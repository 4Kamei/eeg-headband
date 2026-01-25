#![no_std]
#![no_main]

use common::ring_buffer::RingBufferProducer;
use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use defmt::println;
use defmt_rtt as _;
use embassy_executor::task;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_nrf::bind_interrupts;
use embassy_nrf::config::Config;
use embassy_nrf::gpio::Output;
use embassy_nrf::ipc;
use embassy_nrf::ipc::InterruptHandler as IpcInterruptHandler;
use embassy_nrf::ipc::Ipc;
use embassy_nrf::ipc::IpcChannel;
use embassy_nrf::peripherals::IPC;
use embassy_nrf::peripherals::RNG;
use embassy_nrf::rng::InterruptHandler as RngInterruptHandler;
use embassy_nrf::rng::Rng;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch;
use embassy_time::Duration;
use embassy_time::Timer;
use nrf_sdc as sdc;
use nrf_sdc::mpsl::{
    raw as mpsl_raw, ClockInterruptHandler, HighPrioInterruptHandler, LowPrioInterruptHandler,
};
use nrf_sdc::mpsl::{MultiprotocolServiceLayer, Peripherals};
use nrf_sdc::Builder;
use nrf_sdc::SoftdeviceController;
use static_cell::StaticCell;
use trouble_host::advertise;
use trouble_host::prelude::AdStructure;
use trouble_host::prelude::Advertisement;
use trouble_host::prelude::AdvertisementParameters;
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::prelude::BR_EDR_NOT_SUPPORTED;
use trouble_host::prelude::LE_GENERAL_DISCOVERABLE;
use trouble_host::Address;
use trouble_host::Host;
use trouble_host::HostResources;

static IPC_0_WATCH: watch::Watch<CriticalSectionRawMutex, (), 1> = watch::Watch::new();

#[embassy_executor::task]
async fn led_blinker(ipc: ipc::Event<'static>) {
    loop {
        Timer::after_millis(500).await;
        ipc.trigger();
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::info!("Started Network core");
    let mut config = Config::default();
    config.debug = embassy_nrf::config::Debug::Allowed;
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    config.lfclk_source = embassy_nrf::config::LfclkSource::Synthesized;

    let p = embassy_nrf::init(config);
    defmt::info!("Initialized");

    let Ipc {
        event0: mut start_ipc,
        ..
    } = Ipc::new(p.IPC, Irqs);

    start_ipc.configure_trigger([IpcChannel::Channel0]);

    defmt::info!("Triggering start no app core");
    defmt::unwrap!(spawner.spawn(led_blinker(start_ipc)));

    // Your application logic can go here.

    // Create the clock configuration
    let lfclk_cfg = mpsl_raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl_raw::MPSL_CLOCK_LF_SRC_SYNTH as u8,
        rc_ctiv: 0,
        rc_temp_ctiv: 0,
        accuracy_ppm: 50,
        skip_wait_lfclk_started: false,
    };

    let mpsl_p = Peripherals::new(
        p.RTC0, p.TIMER0, p.TIMER1, p.TEMP, p.PPI_CH0, p.PPI_CH1, p.PPI_CH2,
    );

    // Initialize the MPSL
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    let mpsl = MPSL.init(defmt::unwrap!(MultiprotocolServiceLayer::new(
        mpsl_p, Irqs, lfclk_cfg
    )));

    let sdc_p = nrf_sdc::Peripherals::new(
        p.PPI_CH3, p.PPI_CH4, p.PPI_CH5, p.PPI_CH6, p.PPI_CH7, p.PPI_CH8, p.PPI_CH9, p.PPI_CH10,
        p.PPI_CH11, p.PPI_CH12,
    );

    static RNG: StaticCell<Rng<embassy_nrf::mode::Async>> = StaticCell::new();
    let rng = RNG.init(Rng::new(p.RNG, Irqs));

    static SDC_MEM: StaticCell<sdc::Mem<1856>> = StaticCell::new();
    defmt::info!("Initializing the SDC Memory");
    let sdc_mem = SDC_MEM.init(sdc::Mem::new());
    defmt::info!("Initializing the SDC");
    // Initialize the SoftDevice Controller
    let sdc = defmt::unwrap!(nrf_sdc::Builder::new()
        .and_then(Builder::support_peripheral)
        .and_then(Builder::support_central)
        .and_then(Builder::support_ext_scan)
        .and_then(Builder::support_ext_adv)
        .and_then(|b| b.build(sdc_p, rng, mpsl, sdc_mem)));

    defmt::info!("Getting sender");
    // Safety: This is the only place in the codebase where this is called
    let producer = unsafe { common::BLE_QUEUE.get_sender_with_signal(IPC_0_WATCH.sender()) };

    defmt::info!("Spawning tasks");
    // Spawn the MPSL and SDC tasks
    spawner.must_spawn(mpsl_task(mpsl));
    spawner.must_spawn(sdc_task(sdc, producer));
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

#[embassy_executor::task]
async fn sdc_task(
    sdc: SoftdeviceController<'static>,
    _producer: RingBufferProducer<'static, u64, 1>,
) -> ! {
    defmt::info!("In SDC task");

    let address = Address::random([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

    let mut resources: HostResources<DefaultPacketPool, 0, 1> = HostResources::new();

    let stack = trouble_host::new(sdc, &mut resources).set_random_address(address);
    defmt::info!("Created stack");

    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();
    defmt::info!("Built stack");

    let mut adv_data = [0; 64];
    let len = defmt::unwrap!(AdStructure::encode_slice(
        &[
            AdStructure::CompleteLocalName(b"OpenEEG Headband"),
            AdStructure::ShortenedLocalName(b"EEG Headband"),
            AdStructure::ServiceUuids128(&[common::EEG_DATA_SERVICE_UUID]),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        ],
        &mut adv_data[..],
    ));

    defmt::info!("Len {:?}, {:?}", len, adv_data);

    defmt::info!("Starting Advertising");
    join(runner.run(), async {
        loop {
            defmt::info!("Restarting BLE listening loop");
            let params = AdvertisementParameters::default();

            let advertiser = defmt::unwrap!(
                peripheral
                    .advertise(
                        &params,
                        Advertisement::ConnectableScannableUndirected {
                            adv_data: &adv_data[..len],
                            scan_data: &[],
                        },
                    )
                    .await
            );

            defmt::info!("Advertising - waiting for connection");

            let Ok(connection) = advertiser.accept().await else {
                continue;
            };

            defmt::info!("Connection accepted");

            let mut l2cap_config = trouble_host::l2cap::L2capChannelConfig::default();
            l2cap_config.mtu = Some(512);

            let channel = trouble_host::l2cap::L2capChannel::accept(
                &stack,
                &connection,
                &[0, 1],
                &l2cap_config,
            );

            let mut channel = match channel.await {
                Ok(channel) => {
                    defmt::info!("Created L2Cap channel");
                    channel
                }
                Err(error) => {
                    defmt::warn!(
                        "Got error {:?} when creating L2Cap channel - disconnecting",
                        error
                    );
                    continue;
                }
            };

            let mut packet_buffer: [u8; 512] = [0; 512];
            while let Ok(count) = channel.receive(&stack, &mut packet_buffer).await {
                let data = &packet_buffer[..count];
                defmt::info!("Got data {:?}", data);
            }
            defmt::info!("Connection closed");
        }
    })
    .await;

    defmt::info!("Ending");

    loop {
        Timer::after_secs(1).await;
    }
}

#[task]
async fn ipc_handler_task(
    mut event: embassy_nrf::ipc::Event<'static>,
    sender: watch::Sender<'static, CriticalSectionRawMutex, (), 1>,
) {
    loop {
        event.wait().await;
        sender.send(());
    }
}

bind_interrupts! {
    struct Irqs {
        // High-priority interrupts required by MPSL
        RADIO => HighPrioInterruptHandler;
        TIMER0 => HighPrioInterruptHandler;
        RTC0 => HighPrioInterruptHandler;

        // Low-priority interrupt required by MPSL
        // If you have any low-priority interrupts for your multiprotocol service
        // you can add them here. Otherwise leave empty.
        SWI0 => LowPrioInterruptHandler;

        // Clock event interrupt for MPSL
        CLOCK_POWER => ClockInterruptHandler;

        // RNG interrupt
        RNG => RngInterruptHandler<RNG>;

        IPC => IpcInterruptHandler<embassy_nrf::peripherals::IPC>;
    }
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

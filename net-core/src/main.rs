#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use embassy_executor::Spawner;
use embassy_nrf::bind_interrupts;
use embassy_nrf::config::Config;
use embassy_nrf::peripherals::RNG;
use embassy_nrf::rng::InterruptHandler as RngInterruptHandler;
use embassy_nrf::rng::Rng;
use embassy_time::Timer;
use nrf_sdc as sdc;
use nrf_sdc::mpsl::{
    raw as mpsl_raw, ClockInterruptHandler, HighPrioInterruptHandler, LowPrioInterruptHandler,
};
use nrf_sdc::mpsl::{MultiprotocolServiceLayer, Peripherals};
use nrf_sdc::raw as sdc_raw;
use nrf_sdc::SoftdeviceController;
use static_cell::StaticCell;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    config.debug = embassy_nrf::config::Debug::Allowed;
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    config.lfclk_source = embassy_nrf::config::LfclkSource::Synthesized;

    let p = embassy_nrf::init(config);

    // Create the clock configuration
    let lfclk_cfg = mpsl_raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl_raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: 16,
        rc_temp_ctiv: 2,
        accuracy_ppm: mpsl_raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: false,
    };

    // On nrf52 chips, the peripherals needed by MPSL are:
    // RTC0, TIMER0, TEMP, PPI_CH19, PPI_CH30, PPI_CH31
    // The list of peripherals is different for other chips.
    let mpsl_p = Peripherals::new(
        p.RTC0, p.TIMER0, p.TIMER1, p.TEMP, p.PPI_CH0, p.PPI_CH1, p.PPI_CH2,
    );

    // Initialize the MPSL
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    let mpsl = MPSL.init(MultiprotocolServiceLayer::new(mpsl_p, Irqs, lfclk_cfg).unwrap());
    // On nrf52 chips, the peripherals needed by SDC are:
    // PPI_CH17, PPI_CH18, PPI_CH20..=PPI_CH29
    // The list of peripherals is different for other chips.
    let sdc_p = nrf_sdc::Peripherals::new(
        p.PPI_CH3, p.PPI_CH4, p.PPI_CH5, p.PPI_CH6, p.PPI_CH7, p.PPI_CH8, p.PPI_CH9, p.PPI_CH10,
        p.PPI_CH11, p.PPI_CH12,
    );

    static RNG: StaticCell<Rng<embassy_nrf::peripherals::RNG, embassy_nrf::mode::Async>> =
        StaticCell::new();
    let rng = RNG.init(Rng::new(p.RNG, Irqs));
    static SDC_MEM: StaticCell<sdc::Mem<1024>> = StaticCell::new();

    // Initialize the SoftDevice Controller
    let sdc = nrf_sdc::Builder::new()
        .unwrap()
        .support_adv()
        .unwrap()
        .support_peripheral()
        .unwrap()
        .build(sdc_p, rng, mpsl, SDC_MEM.init(sdc::Mem::new()))
        .unwrap();

    // Spawn the MPSL and SDC tasks
    spawner.must_spawn(mpsl_task(mpsl));
    spawner.must_spawn(sdc_task(sdc));

    // Your application logic can go here.
    loop {
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

#[embassy_executor::task]
async fn sdc_task(sdc: SoftdeviceController<'static>) -> ! {
    loop {
        let mut evt_buf = [0; sdc_raw::HCI_MSG_BUFFER_MAX_SIZE as usize];
        match sdc.hci_get(&mut evt_buf).await {
            Ok(_event) => {
                // Handle Bluetooth events
            }
            Err(e) => {
                // Handle errors
                core::panic!("sdc_task error: {:?}", e)
            }
        }
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
    }
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

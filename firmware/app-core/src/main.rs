#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use defmt_rtt as _;
use embassy_executor::{task, Spawner, SpawnerTraceExt};
use embassy_nrf::gpio::Output;
use embassy_nrf::ipc::{self, InterruptHandler as IpcInterruptHandler, Ipc};
use embassy_nrf::pac::SPU;
use embassy_nrf::peripherals::IPC;
use embassy_nrf::{bind_interrupts, reset};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch;
use embassy_time::Timer;
mod bsp;

static BLE_WATCH: watch::Watch<CriticalSectionRawMutex, (), 1> = watch::Watch::new();

#[embassy_executor::task]
async fn led_blinker(mut led: Output<'static>) {
    loop {
        Timer::after_secs(1).await;
        led.toggle();
    }
}

fn init_trustzone() {
    let region_start = (0x2004_0000 - 0x2000_0000) / 0x0000_2000;
    let region_end = (0x2008_0000 - 0x2000_0000) / 0x0000_2000;
    for region in region_start..region_end {
        SPU.ramregion(region as usize).perm().write(|w| {
            w.set_read(true);
            w.set_lock(false);
            w.set_write(true);
        });
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_trustzone();

    reset::hold_network_core();

    defmt::info!("Application core started");
    // SAFETY: We've just started, and we only use the receiver in this code - hence we can assert
    // that we're fine here
    unsafe {
        common::BLE_QUEUE.reset();
        defmt::info!("Reseting receiver");
    };

    let p = bsp::init();
    let Ipc {
        event0: mut start_ipc,
        event1: ble_queue_ipc,
        ..
    } = Ipc::new(p.IPC, Irqs);

    reset::clear_reasons();
    reset::release_network_core();

    let output1 = Output::new(
        p.P0_29,
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );

    defmt::unwrap!(spawner.spawn(led_blinker(output1)));

    defmt::info!("Waiting for network core to start...");
    start_ipc.wait().await;

    defmt::unwrap!(spawner.spawn_named(
        "ble-ipc",
        ipc_handler_task(ble_queue_ipc, BLE_WATCH.sender())
    ));

    let mut ble_receiver =
        common::BLE_QUEUE.get_receiver_with_signal(defmt::unwrap!(BLE_WATCH.receiver()));

    loop {
        defmt::info!("Got event! {:?}", ble_receiver.recv().await);
    }
}

bind_interrupts! {
    struct Irqs {
        IPC => IpcInterruptHandler<embassy_nrf::peripherals::IPC>;
    }
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

#[task]
async fn ipc_handler_task(
    mut event: ipc::Event<'static, IPC>,
    sender: watch::Sender<'static, CriticalSectionRawMutex, (), 1>,
) {
    loop {
        event.wait().await;
        sender.send(());
    }
}

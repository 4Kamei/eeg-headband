#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use defmt_rtt as _;
use embassy_executor::{task, Spawner, SpawnerTraceExt};
use embassy_nrf::bind_interrupts;
use embassy_nrf::ipc::{self, InterruptHandler as IpcInterruptHandler, Ipc};
use embassy_nrf::peripherals::IPC;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch;
use embassy_time::Timer;
mod bsp;

static IPC_0_WATCH: watch::Watch<CriticalSectionRawMutex, (), 1> = watch::Watch::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::println!("Hello world");

    let p = bsp::init();

    let Ipc { event0, .. } = Ipc::new(p.IPC, Irqs);

    defmt::unwrap!(spawner.spawn_named(
        "event0-ipc",
        ipc_handler_task(event0, defmt::unwrap!(IPC_0_WATCH.receiver()))
    ));
    let ble_receiver =
        common::BLE_QUEUE.get_receiver_with_signal(defmt::unwrap!(IPC_0_WATCH.receiver()));

    loop {
        Timer::after_secs(1).await;
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
    event: ipc::Event<'static, IPC>,
    mut receiver: watch::Receiver<'static, CriticalSectionRawMutex, (), 1>,
) {
    loop {
        receiver.changed().await;
        event.trigger();
    }
}

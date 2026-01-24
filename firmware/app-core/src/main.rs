#![no_std]
#![no_main]

use core::ptr;
use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use cortex_m::Peripherals;
use defmt_rtt as _;
use embassy_executor::{task, Spawner, SpawnerTraceExt};
use embassy_nrf::gpio::Output;
use embassy_nrf::ipc::{self, InterruptHandler as IpcInterruptHandler, Ipc};
use embassy_nrf::pac::SPU;
use embassy_nrf::peripherals::IPC;
use embassy_nrf::{bind_interrupts, pac};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch;
use embassy_time::Timer;
mod bsp;

static IPC_0_WATCH: watch::Watch<CriticalSectionRawMutex, (), 1> = watch::Watch::new();

#[embassy_executor::task]
async fn led_blinker(mut led: Output<'static>) {
    loop {
        Timer::after_secs(1).await;
        led.toggle();
    }
}

fn write_reg<T>(addr: usize, value: T) {
    let addr_ptr = unsafe { &mut *(addr as *mut T) };
    *addr_ptr = value;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let region_start = (0x2004_0000 - 0x2000_0000) / 0x0000_2000;
    let region_end = (0x2008_0000 - 0x2000_0000) / 0x0000_2000;

    {
        for region in region_start..region_end {
            defmt::info!("Writing to ram region {:?}", region);
            SPU.ramregion(region as usize).perm().write(|w| {
                w.set_read(true);
                w.set_lock(false);
                w.set_write(true);
            });
            defmt::info!("Complete");
        }
    }

    defmt::println!(
        "Address of stuff: {:?}",
        core::ptr::addr_of!(common::BLE_QUEUE)
    );

    defmt::println!("Hello world");
    // SAFETY: We've just started, and we only use the reciver in this code - hence we can assert
    // that we're fine here
    unsafe {
        common::BLE_QUEUE.reset_receiver();
        defmt::println!("Reseting receiver");
    };

    let p = bsp::init();

    let output1 = Output::new(
        p.P0_31,
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );

    let _output2 = Output::new(
        p.P0_29,
        embassy_nrf::gpio::Level::High,
        embassy_nrf::gpio::OutputDrive::Standard,
    );

    defmt::unwrap!(spawner.spawn(led_blinker(output1)));

    let Ipc { event0, .. } = Ipc::new(p.IPC, Irqs);

    defmt::unwrap!(spawner.spawn_named("ble-ipc", ipc_handler_task(event0, IPC_0_WATCH.sender())));

    let mut ble_receiver =
        common::BLE_QUEUE.get_receiver_with_signal(defmt::unwrap!(IPC_0_WATCH.receiver()));

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

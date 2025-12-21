#![no_std]
#![no_main]

use core::{panic::PanicInfo, sync::atomic::compiler_fence};
use embassy_executor::Spawner;
use embassy_time::Timer;

mod bsp;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = bsp::init();

    loop {
        Timer::after_secs(1).await;
    }
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

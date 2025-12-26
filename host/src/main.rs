// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use gpui::Application;
use tokio::runtime::Builder;

mod ble_driver;
mod gui;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    //let adapter = Adapter::default().await.unwrap();
    //let driver = ble_driver::BleDriver::new(adapter);

    // 1. Build the runtime manually
    let rt = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    // 2. Enter the runtime in this scope
    let _enter = rt.enter(); // runtime is now "current" on this thread

    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(gui::start_application);

    Ok(())
}

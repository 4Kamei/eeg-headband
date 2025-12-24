// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use bluest::{Adapter, Device};
use gpui::{actions, Application};
use std::{str::FromStr, time::Duration};
use tokio::{runtime::Builder, task, time::timeout};
use uuid::Uuid;

mod ble_driver;

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

mod gui {
    use gpui::*;
    use gpui_component::{label::Label, text::Text, Root, TitleBar};

    actions!(main, [Quit]);

    pub fn start_application(cx: &mut App) {
        // Initialize gpui-component before using any components
        gpui_component::init(cx);

        /*
        // Load custom themes from themes directory
        let themes_dir = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("themes");

        if themes_dir.exists() {
            let _ = ThemeRegistry::watch_dir(themes_dir, cx, |cx| {
                // Set Twilight as the default theme after themes are loaded
                let theme_registry = ThemeRegistry::global(cx);
                let twilight_name: SharedString = "Twilight".into();
                if let Some(twilight_theme) = theme_registry.themes().get(&twilight_name) {
                    let twilight_theme = twilight_theme.clone();
                    let theme_mode = twilight_theme.mode;

                    let theme = Theme::global_mut(cx);
                    theme.dark_theme = twilight_theme;
                    Theme::change(theme_mode, None, cx);
                }
            });
        }
        */

        let bounds = Bounds::centered(None, size(px(1600.0), px(900.0)), cx);

        // Set up keyboard bindings
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        // Handle quit action
        cx.on_action(|_: &Quit, cx| cx.quit());

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            let window = cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| TestWindow {});

                cx.new(|cx| Root::new(view, window, cx))
            })?;

            // Get the root view entity and observe when it's released (window closed)
            let root_view = window.update(cx, |_, _, cx| cx.entity())?;
            cx.update(|cx| {
                cx.observe_release(&root_view, |_, cx| cx.quit()).detach();
            })?;

            cx.update(|cx| cx.activate(true))?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    }

    struct TestWindow {}

    impl Render for TestWindow {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                TitleBar::new().child(Label::new("test").text_color(hsla(1.0, 0.5, 0.5, 1.0))),
            )
        }
    }
}

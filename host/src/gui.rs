use gpui::*;
use gpui_component::{
    button::Button,
    chart::LineChart,
    label::Label,
    text::{self, Text},
    Root, Theme, ThemeRegistry, TitleBar,
};
use image::{
    metadata::{Cicp, CicpColorPrimaries},
    Frame, ImageBuffer,
};
use parking_lot::lock_api::GuardSend;
use phosphor::PhosphorHeadless;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{env, ops::DerefMut, path::PathBuf, sync::Arc};

actions!(main, [Quit]);

pub fn start_application(cx: &mut App) {
    // Initialize gpui-component before using any components
    gpui_component::init(cx);

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
            let view = cx.new(|_cx| MainWindow::default());

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

#[derive(Default)]
struct Shared<T> {
    inner: Arc<parking_lot::Mutex<T>>,
}

impl<T> Shared<T> {
    pub fn update<F, U>(&self, func: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        func(self.inner.lock().deref_mut())
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self {
            inner: Arc::new(parking_lot::Mutex::new(value)),
        }
    }
}

#[derive(Clone, Copy)]
enum Tab {
    DeviceState,
    Streaming,
    Recordings,
    Firmware,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::DeviceState => "Device State",
            Tab::Streaming => "Streaming",
            Tab::Recordings => "Recordings",
            Tab::Firmware => "Firmware",
        }
    }

    pub fn in_order() -> impl Iterator<Item = Self> {
        [
            Tab::DeviceState,
            Tab::Streaming,
            Tab::Recordings,
            Tab::Firmware,
        ]
        .into_iter()
    }
}

use proto::{from_edge::FromEdge, to_edge::ToEdge};

const fn pc(value: f32) -> DefiniteLength {
    let _: () = {
        assert!(value <= 100.0, "Value must be between 0 and 100");
        assert!(value >= 0.0, "Value must be between 0 and 100");
    };
    DefiniteLength::Fraction(value)
}

struct GuiState {
    // Tabs, plus static information for each tab
    selected_tab: Tab,
    device_state: Option<device_state::DeviceState>,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_tab: Tab::DeviceState,
            device_state: Default::default(),
        }
    }
}

#[derive(Default)]
struct MainWindow {
    state: Shared<GuiState>,
}

impl MainWindow {
    pub fn send_command(&mut self, command: ToEdge) {
        panic!("Unhandled command {:?}", command);
    }
}
impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .child(navigation_bar(cx, self.state.clone()))
            .child(content_pane(cx, self.state.clone()))
    }
}

pub fn navigation_bar(cx: &mut Context<MainWindow>, shared: Shared<GuiState>) -> impl IntoElement {
    let mut div = div().w(px(300.0)).h(pc(100.0));
    for tab in Tab::in_order() {
        let inner_shared = shared.clone();
        div = div.child(
            Button::new(tab.name())
                .label(tab.name())
                .text_xl()
                .on_click(move |_click_event, _window, _app| {
                    inner_shared.update(|state| state.selected_tab = tab)
                }),
        )
    }
    div
}

pub fn content_pane(cx: &mut Context<MainWindow>, shared: Shared<GuiState>) -> impl IntoElement {
    let selected_tab = match shared.update(|shared| shared.selected_tab) {
        Tab::DeviceState => device_state::device_state(cx, shared),
        Tab::Firmware => unimplemented!(),
        Tab::Recordings => unimplemented!(),
        Tab::Streaming => unimplemented!(),
    };
    div()
        .p(px(16.0))
        .border_3()
        .flex_col()
        .gap(px(12.0))
        .child(selected_tab)
}

mod device_state {
    use crate::gui::{GuiState, MainWindow, Shared};
    use gpui::*;
    use gpui_component::{
        button::Button,
        description_list::{DescriptionItem, DescriptionList},
        label::Label,
    };

    /// Status of the battery we've received from the device
    enum BatteryStatus {
        /// No battery is attached
        None,
        /// We have a battery we're charging, with an estimation of how long to full charge
        Charging(f32, u32),
        /// We have a battery we're discharging, with an estimation of how long until empty
        Discharging(f32, u32),
    }

    impl ToString for BatteryStatus {
        fn to_string(&self) -> String {
            match self {
                BatteryStatus::None => "None".to_string(),
                BatteryStatus::Charging(charging, estimated) => {
                    format!("Charging {:.1?}% ({:?}) to full", charging, estimated)
                }
                BatteryStatus::Discharging(charging, estimated) => {
                    format!("Discharging {:.1?}% ({:?}) to empty", charging, estimated)
                }
            }
        }
    }

    /// Stores the current state of the connected device
    pub struct DeviceState {
        /// String identifying the hardware revision of the board
        hardware_rev: String,
        /// Git hash/tag of the current firmware on the device
        firmware_rev: String,
        /// Status of the device battery
        battery_status: BatteryStatus,
        /// Since last reboot (in seconds)
        uptime: u32,
        /// How long we've been recording for
        current_time_reconding: u32,
        /// The current time of the device (unix timestamp, seconds)
        current_time: u64,
        /// Amount of storage attached to the device (bytes)
        storage_size_total: u32,
        /// Amount of storage that we're currently using
        storage_size_used: u32,
        /// Amount of storage that is currently free
        storage_size_free: u32,
    }

    trait Formatter<T> {
        fn format(value: &T) -> String;
    }

    struct StringFormatter;

    impl<T: ToString> Formatter<T> for StringFormatter {
        fn format(value: &T) -> String {
            value.to_string()
        }
    }

    pub fn text_with_formatter<T, F>(
        text: &str,
        value: &T,
        #[allow(unused_variables)] formatter: F,
    ) -> DescriptionItem
    where
        F: Formatter<T>,
    {
        DescriptionItem::new(text).value(F::format(value)).span(1)
    }

    pub fn device_state(
        _cx: &mut Context<MainWindow>,
        shared: Shared<GuiState>,
    ) -> impl IntoElement {
        let shared_inner = shared.clone();
        let update_button =
            Button::new("update_button")
                .label("Fetch Status")
                .on_click(move |_, _, _| {
                    shared_inner.update(|status| {
                        status.device_state = Some(DeviceState {
                            hardware_rev: "092hns234".into(),
                            firmware_rev: "ht94hr0298349".into(),
                            battery_status: BatteryStatus::Charging(100.0, 0),
                            uptime: 234,
                            current_time_reconding: 0,
                            current_time: 234123523,
                            storage_size_total: 16,
                            storage_size_used: 0,
                            storage_size_free: 16,
                        })
                    });
                });

        let root = div().flex_1().flex_col().child(update_button);
        let root = shared.update(move |state| {
            if let Some(device_state) = &state.device_state {
                let mut state_list = DescriptionList::horizontal().bordered(true).columns(1);
                state_list = state_list.children([
                    text_with_formatter(
                        "Hardware Revision",
                        &device_state.hardware_rev,
                        StringFormatter,
                    ),
                    text_with_formatter(
                        "Firmware Revision",
                        &device_state.firmware_rev,
                        StringFormatter,
                    ),
                    text_with_formatter(
                        "Battery Status",
                        &device_state.battery_status,
                        StringFormatter,
                    ),
                    text_with_formatter("Uptime", &device_state.uptime, StringFormatter),
                    text_with_formatter(
                        "Hardware Revision",
                        &device_state.hardware_rev,
                        StringFormatter,
                    ),
                    text_with_formatter(
                        "Hardware Revision",
                        &device_state.hardware_rev,
                        StringFormatter,
                    ),
                    text_with_formatter(
                        "Hardware Revision",
                        &device_state.hardware_rev,
                        StringFormatter,
                    ),
                ]);
                root.child(state_list)
            } else {
                root.child(Label::new("No status received"))
            }
        });
        root
    }
}

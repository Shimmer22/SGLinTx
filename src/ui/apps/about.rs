use crate::ui::{
    apps::{AppSpec, UiAppContext, UiAppModule},
    input::UiInputEvent,
    model::{AppId, UiFrame},
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::About,
    title: "ABOUT",
    icon_text: "ABT",
    accent: (160, 196, 255),
};

pub struct AboutApp;
pub static ABOUT_APP: AboutApp = AboutApp;

impl UiAppModule for AboutApp {
    fn on_event(&self, _frame: &mut UiFrame, _event: UiInputEvent, _ctx: &UiAppContext<'_>) {}

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
        format!(
            "LinTX\n\nRemote Battery: {}%\nAircraft Battery: {}%\nSignal: {}%\n\nEsc Back",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
        )
    }
}

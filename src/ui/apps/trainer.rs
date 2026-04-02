use crate::ui::{
    apps::{AppSpec, UiAppContext, UiAppModule},
    input::UiInputEvent,
    model::{AppId, UiFrame},
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::Trainer,
    title: "TRAINER",
    icon_text: "TRN",
    accent: (255, 123, 118),
};

pub struct TrainerApp;
pub static TRAINER_APP: TrainerApp = TrainerApp;

impl UiAppModule for TrainerApp {
    fn on_event(&self, _frame: &mut UiFrame, _event: UiInputEvent, _ctx: &UiAppContext<'_>) {}

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
        format!(
            "Remote Battery: {}%\nAircraft Battery: {}%\nSignal: {}%\nBacklight: {}%\nSound: {}%\n\nEsc Back",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        )
    }
}

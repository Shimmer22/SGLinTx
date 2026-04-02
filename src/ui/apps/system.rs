use crate::ui::{
    apps::{common::battery_grade, common::signal_grade, AppSpec, UiAppContext, UiAppModule},
    input::UiInputEvent,
    model::{AppId, UiFrame},
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::System,
    title: "SYSTEM",
    icon_text: "SYS",
    accent: (73, 143, 255),
};

pub struct SystemApp;
pub static SYSTEM_APP: SystemApp = SystemApp;

impl UiAppModule for SystemApp {
    fn on_event(&self, frame: &mut UiFrame, event: UiInputEvent, ctx: &UiAppContext<'_>) {
        match event {
            UiInputEvent::Up => {
                frame.config.backlight_percent =
                    frame.config.backlight_percent.saturating_add(5).min(100);
                ctx.config_tx.send(frame.config);
            }
            UiInputEvent::Down => {
                frame.config.backlight_percent = frame.config.backlight_percent.saturating_sub(5);
                ctx.config_tx.send(frame.config);
            }
            UiInputEvent::Left => {
                frame.config.sound_percent = frame.config.sound_percent.saturating_sub(5);
                ctx.config_tx.send(frame.config);
            }
            UiInputEvent::Right => {
                frame.config.sound_percent = frame.config.sound_percent.saturating_add(5).min(100);
                ctx.config_tx.send(frame.config);
            }
            _ => {}
        }
    }

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
        format!(
            "Remote Battery: {}% ({})\nAircraft Battery: {}% ({})\nSignal: {}% ({})\nClock: {}\n\nBacklight: {}%  (Up/Down)\nSound: {}%  (Left/Right)\n\nEsc Back",
            frame.status.remote_battery_percent,
            battery_grade(frame.status.remote_battery_percent),
            frame.status.aircraft_battery_percent,
            battery_grade(frame.status.aircraft_battery_percent),
            frame.status.signal_strength_percent,
            signal_grade(frame.status.signal_strength_percent),
            frame.status.unix_time_secs,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        )
    }
}

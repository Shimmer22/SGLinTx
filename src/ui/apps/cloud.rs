use crate::ui::{
    apps::{common::signal_grade, AppSpec, UiAppContext, UiAppModule},
    input::UiInputEvent,
    model::{AppId, UiFrame},
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::Cloud,
    title: "CLOUD",
    icon_text: "NET",
    accent: (186, 135, 255),
};

pub struct CloudApp;
pub static CLOUD_APP: CloudApp = CloudApp;

impl UiAppModule for CloudApp {
    fn on_event(&self, frame: &mut UiFrame, event: UiInputEvent, _ctx: &UiAppContext<'_>) {
        if event == UiInputEvent::Open {
            frame.cloud_connected = !frame.cloud_connected;
            if frame.cloud_connected {
                frame.cloud_last_sync_secs = frame.status.unix_time_secs;
            }
        }
    }

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
        let connection = if frame.cloud_connected {
            "ONLINE"
        } else {
            "OFFLINE"
        };
        let sync_secs = if frame.cloud_connected {
            frame
                .status
                .unix_time_secs
                .saturating_sub(frame.cloud_last_sync_secs)
        } else {
            0
        };

        format!(
            "Cloud Link: {}\nLink Quality: {}%\nLast Sync: {}s ago\n\nStatus Summary\nRemote {}% | Aircraft {}%\nSignal Class: {}\n\nEnter: connect/disconnect\nEsc Back",
            connection,
            frame.status.signal_strength_percent,
            sync_secs,
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            signal_grade(frame.status.signal_strength_percent),
        )
    }
}

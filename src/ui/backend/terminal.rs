use std::io::Write;

use super::{
    battery_grade, elrs_list_lines, signal_grade, LvglBackend, MODEL_NAMES, MODEL_PROTOCOLS,
};
use crate::ui::{
    catalog::{app_at, app_spec, page, PAGE_SPECS},
    input::UiInputEvent,
    model::{AppId, UiFrame, UiPage},
};

pub(super) struct TerminalBackend {
    backend_name: String,
}

impl TerminalBackend {
    pub(super) fn new(backend_name: String) -> Self {
        Self { backend_name }
    }
}

fn format_channel_groups(channels: &[i16]) -> String {
    if channels.is_empty() {
        return "No input yet".to_string();
    }

    let mut lines = channels
        .chunks(4)
        .enumerate()
        .take(2)
        .map(|(group_idx, group)| {
            let start = group_idx * 4;
            group
                .iter()
                .enumerate()
                .map(|(offset, value)| format!("CH{}:{}", start + offset + 1, value))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>();

    if channels.len() > 8 {
        lines.push(format!("... +{} more channels", channels.len() - 8));
    }

    lines.join("\n")
}

fn format_app_detail(frame: &UiFrame, app: AppId) -> String {
    match app {
        AppId::System => format!(
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
        ),
        AppId::Control => format!(
            "Input Source: {}\nStatus: {} ({})\nChannels: {}\n{}\n\nMixer Out (0..10000)\nThrust:{}\nDirection:{}\nAileron:{}\nElevator:{}\n\nUse this page to validate input chain.\nEsc Back",
            frame.input_status.source.label(),
            frame.input_status.health.label(),
            frame.input_status.detail,
            frame.input_frame.channels.len(),
            format_channel_groups(&frame.input_frame.channels),
            frame.mixer_out.thrust,
            frame.mixer_out.direction,
            frame.mixer_out.aileron,
            frame.mixer_out.elevator,
        ),
        AppId::Models => {
            let focus = frame.model_focus_idx.min(MODEL_NAMES.len().saturating_sub(1));
            let active = frame.model_active_idx.min(MODEL_NAMES.len().saturating_sub(1));
            format!(
                "Active Model: {} ({})\nFocused Model: {} ({})\n\nModel List\n{} {}\n{} {}\n{} {}\n{} {}\n\nUp/Down: focus model\nEnter: apply focused model\nEsc Back",
                MODEL_NAMES[active],
                MODEL_PROTOCOLS[active],
                MODEL_NAMES[focus],
                MODEL_PROTOCOLS[focus],
                if focus == 0 { ">" } else { " " },
                MODEL_NAMES[0],
                if focus == 1 { ">" } else { " " },
                MODEL_NAMES[1],
                if focus == 2 { ">" } else { " " },
                MODEL_NAMES[2],
                if focus == 3 { ">" } else { " " },
                MODEL_NAMES[3],
            )
        }
        AppId::Cloud => {
            let connection = if frame.cloud_connected { "ONLINE" } else { "OFFLINE" };
            let sync_secs = if frame.cloud_connected {
                frame.status.unix_time_secs.saturating_sub(frame.cloud_last_sync_secs)
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
        AppId::Scripts => {
            let connected = if frame.elrs.connected {
                "CONNECTED"
            } else {
                "OFFLINE"
            };
            let busy = if frame.elrs.busy { "BUSY" } else { "READY" };
            let lines = elrs_list_lines(frame);
            let editor = if frame.elrs.editor_active {
                format!(
                    "\nEdit: {} = {}\nCursor: {}\n",
                    frame.elrs.editor_label,
                    frame.elrs.editor_buffer,
                    frame.elrs.editor_cursor.saturating_add(1)
                )
            } else {
                String::new()
            };
            format!(
                "Link: {} ({})\nModule: {}\nDevice: {}\nVersion: {}\nPath: {}\nStatus: {}\n{}\n{}\n{}\n{}\n{}\n\n{}\nEsc Back",
                connected,
                busy,
                frame.elrs.module_name,
                frame.elrs.device_name,
                frame.elrs.version,
                frame.elrs.path,
                frame.elrs.status_text,
                editor,
                lines[0],
                lines[1],
                lines[2],
                lines[3],
                if frame.elrs.editor_active {
                    "Up/Down: char  Left/Right: move  Enter: save  Esc: cancel"
                } else {
                    "Up/Down: select  Left/Right: adjust  Enter: open/apply  ]: refresh"
                },
            )
        }
        _ => format!(
            "Remote Battery: {}%\nAircraft Battery: {}%\nSignal: {}%\nBacklight: {}%\nSound: {}%\n\nEsc Back",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        ),
    }
}

impl LvglBackend for TerminalBackend {
    fn init(&mut self) {
        println!("[ui] backend={} init", self.backend_name);
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        None
    }

    fn render(&mut self, frame: &UiFrame) {
        print!("\x1B[2J\x1B[1;1H");
        println!("LinTX Launcher [{}]", self.backend_name);
        println!(
            "STATUS | REM:{}% AIR:{}% SIG:{}% T:{}",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.status.unix_time_secs
        );
        println!("-----------------------------------------------");
        match frame.page {
            UiPage::Launcher => {
                let p = page(frame.launcher_page);
                println!(
                    "                LinTX  [Page {}/{}]",
                    p.id + 1,
                    PAGE_SPECS.len()
                );
                for r in 0..p.rows {
                    for c in 0..p.cols {
                        if let Some(app) = app_at(frame.launcher_page, r, c) {
                            let mark = if r == frame.selected_row && c == frame.selected_col {
                                ">"
                            } else {
                                " "
                            };
                            print!("{} {:8}  ", mark, app_spec(app).title);
                        }
                    }
                    println!();
                }
                println!("Arrows move, Enter open, Esc back, [ ] switch page, Q quit");
            }
            UiPage::App(app) => {
                println!("APP: {}", app_spec(app).title);
                println!("{}", format_app_detail(frame, app));
            }
        }
        let _ = std::io::stdout().flush();
    }

    fn shutdown(&mut self) {
        println!("[ui] shutdown");
    }
}

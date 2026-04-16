use crate::{
    config::store,
    messages::{
        ElrsCommandMsg, UiFeedbackMotion, UiFeedbackSeverity, UiFeedbackSlot, UiFeedbackTarget,
        UiInteractionFeedback,
    },
    ui::{
        apps::{common::elrs_list_lines, AppSpec, UiAppContext, UiAppModule},
        input::UiInputEvent,
        model::{AppId, UiFrame},
    },
};

const LOCAL_POWER_LEVELS_MW: [u16; 6] = [10, 25, 100, 250, 500, 1000];
const LOCAL_EDITOR_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-_";
const DEFAULT_BIND_PHRASE: &str = "654321";

pub const SPEC: AppSpec = AppSpec {
    id: AppId::Scripts,
    title: "ELRS",
    icon_text: "ELR",
    accent: (255, 216, 109),
};

pub struct ScriptsApp;
pub static SCRIPTS_APP: ScriptsApp = ScriptsApp;

#[derive(Debug, Clone)]
struct LocalElrsConfig {
    rf_output_enabled: bool,
    wifi_manual_on: bool,
    tx_power_mw: u16,
    bind_phrase: String,
}

impl Default for LocalElrsConfig {
    fn default() -> Self {
        Self {
            rf_output_enabled: false,
            wifi_manual_on: false,
            tx_power_mw: 100,
            bind_phrase: DEFAULT_BIND_PHRASE.to_string(),
        }
    }
}

impl UiAppModule for ScriptsApp {
    fn on_event(&self, frame: &mut UiFrame, event: UiInputEvent, ctx: &UiAppContext<'_>) {
        if is_local_fallback(frame) {
            ensure_local_state(frame);
            if handle_local_event(frame, event) {
                return;
            }
        }

        match event {
            UiInputEvent::Back | UiInputEvent::PagePrev => {
                ctx.elrs_cmd_tx.send(ElrsCommandMsg::Back)
            }
            UiInputEvent::Up => ctx.elrs_cmd_tx.send(ElrsCommandMsg::SelectPrev),
            UiInputEvent::Down => ctx.elrs_cmd_tx.send(ElrsCommandMsg::SelectNext),
            UiInputEvent::Left => ctx.elrs_cmd_tx.send(ElrsCommandMsg::ValueDec),
            UiInputEvent::Right => ctx.elrs_cmd_tx.send(ElrsCommandMsg::ValueInc),
            UiInputEvent::Open => ctx.elrs_cmd_tx.send(ElrsCommandMsg::Activate),
            UiInputEvent::PageNext => ctx.elrs_cmd_tx.send(ElrsCommandMsg::Refresh),
            UiInputEvent::Quit => {}
        }
    }

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
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

    fn intercept_back(&self, frame: &UiFrame) -> bool {
        !frame.elrs.can_leave
    }
}

fn is_local_fallback(frame: &UiFrame) -> bool {
    frame.elrs.path == "/" && frame.elrs.module_name == "ELRS"
}

fn ensure_local_state(frame: &mut UiFrame) {
    let cfg = load_local_config();
    apply_local_state(
        frame,
        &cfg,
        Some("Local config mode (rf_link_service offline)"),
    );
}

fn local_feedback_target(selected_idx: usize) -> UiFeedbackTarget {
    match selected_idx {
        0 => UiFeedbackTarget::FieldId("rf_output".to_string()),
        1 => UiFeedbackTarget::FieldId("wifi_manual".to_string()),
        2 => UiFeedbackTarget::FieldId("bind".to_string()),
        3 => UiFeedbackTarget::FieldId("tx_power".to_string()),
        4 => UiFeedbackTarget::FieldId("bind_phrase".to_string()),
        _ => UiFeedbackTarget::SelectedListRow,
    }
}

fn set_local_feedback(
    frame: &mut UiFrame,
    severity: UiFeedbackSeverity,
    motion: UiFeedbackMotion,
    message: &str,
) {
    let next_seq = frame
        .elrs
        .interaction_feedback
        .as_ref()
        .map(|feedback| feedback.seq.wrapping_add(1))
        .unwrap_or(1);
    frame.elrs.interaction_feedback = Some(UiInteractionFeedback {
        seq: next_seq,
        severity,
        target: local_feedback_target(frame.elrs.selected_idx),
        motion,
        slot: UiFeedbackSlot::TopStatusBar,
        message: message.to_string(),
        ttl_ms: match severity {
            UiFeedbackSeverity::Error => 900,
            UiFeedbackSeverity::Success => 850,
            UiFeedbackSeverity::Busy => 1200,
        },
    });
}

fn clear_local_feedback(frame: &mut UiFrame) {
    frame.elrs.interaction_feedback = None;
}

fn handle_local_event(frame: &mut UiFrame, event: UiInputEvent) -> bool {
    let mut cfg = load_local_config();

    if frame.elrs.editor_active {
        match event {
            UiInputEvent::Back | UiInputEvent::PagePrev => {
                frame.elrs.editor_active = false;
                frame.elrs.can_leave = true;
                frame.elrs.status_text = "Bind phrase edit cancelled".to_string();
                set_local_feedback(
                    frame,
                    UiFeedbackSeverity::Error,
                    UiFeedbackMotion::ShakeX,
                    "Bind phrase edit cancelled",
                );
                true
            }
            UiInputEvent::Up => {
                clear_local_feedback(frame);
                cycle_editor_char(frame, -1);
                true
            }
            UiInputEvent::Down => {
                clear_local_feedback(frame);
                cycle_editor_char(frame, 1);
                true
            }
            UiInputEvent::Left => {
                clear_local_feedback(frame);
                move_editor_cursor(frame, -1);
                true
            }
            UiInputEvent::Right => {
                clear_local_feedback(frame);
                move_editor_cursor(frame, 1);
                true
            }
            UiInputEvent::Open => {
                cfg.bind_phrase = frame.elrs.editor_buffer.clone();
                if save_local_config(&cfg).is_ok() {
                    apply_local_state(frame, &cfg, Some("Bind phrase saved"));
                } else {
                    apply_local_state(frame, &cfg, Some("Bind phrase save failed"));
                }
                frame.elrs.editor_active = false;
                frame.elrs.can_leave = true;
                true
            }
            UiInputEvent::PageNext => true,
            UiInputEvent::Quit => false,
        }
    } else {
        match event {
            UiInputEvent::Up => {
                clear_local_feedback(frame);
                frame.elrs.selected_idx = frame.elrs.selected_idx.saturating_sub(1);
                true
            }
            UiInputEvent::Down => {
                clear_local_feedback(frame);
                frame.elrs.selected_idx = frame.elrs.selected_idx.saturating_add(1).min(4);
                true
            }
            UiInputEvent::Left => {
                apply_local_adjust(frame, &mut cfg, -1);
                true
            }
            UiInputEvent::Right | UiInputEvent::Open => {
                apply_local_adjust(frame, &mut cfg, 1);
                true
            }
            UiInputEvent::PageNext => {
                clear_local_feedback(frame);
                let cfg = load_local_config();
                apply_local_state(frame, &cfg, Some("ELRS config reloaded"));
                true
            }
            UiInputEvent::Back | UiInputEvent::PagePrev => false,
            UiInputEvent::Quit => false,
        }
    }
}

fn apply_local_adjust(frame: &mut UiFrame, cfg: &mut LocalElrsConfig, delta: isize) {
    let (status, severity, motion) = match frame.elrs.selected_idx {
        0 => {
            cfg.rf_output_enabled = !cfg.rf_output_enabled;
            if save_local_config(cfg).is_ok() {
                if cfg.rf_output_enabled {
                    (
                        "RF output enabled",
                        UiFeedbackSeverity::Success,
                        UiFeedbackMotion::Pulse,
                    )
                } else {
                    (
                        "RF output disabled",
                        UiFeedbackSeverity::Success,
                        UiFeedbackMotion::Pulse,
                    )
                }
            } else {
                (
                    "RF output save failed",
                    UiFeedbackSeverity::Error,
                    UiFeedbackMotion::ShakeX,
                )
            }
        }
        1 => {
            cfg.wifi_manual_on = !cfg.wifi_manual_on;
            if save_local_config(cfg).is_ok() {
                if cfg.wifi_manual_on {
                    (
                        "WiFi command armed",
                        UiFeedbackSeverity::Success,
                        UiFeedbackMotion::Pulse,
                    )
                } else {
                    (
                        "WiFi command cleared",
                        UiFeedbackSeverity::Success,
                        UiFeedbackMotion::Pulse,
                    )
                }
            } else {
                (
                    "WiFi config save failed",
                    UiFeedbackSeverity::Error,
                    UiFeedbackMotion::ShakeX,
                )
            }
        }
        2 => (
            "Bind feedback requires rf_link_service",
            UiFeedbackSeverity::Error,
            UiFeedbackMotion::ShakeX,
        ),
        3 => {
            cfg.tx_power_mw = shift_power_level(cfg.tx_power_mw, delta);
            if save_local_config(cfg).is_ok() {
                (
                    "TX power updated",
                    UiFeedbackSeverity::Success,
                    UiFeedbackMotion::Pulse,
                )
            } else {
                (
                    "TX power save failed",
                    UiFeedbackSeverity::Error,
                    UiFeedbackMotion::ShakeX,
                )
            }
        }
        4 => {
            frame.elrs.editor_active = true;
            frame.elrs.can_leave = false;
            frame.elrs.editor_label = "Bind Phrase".to_string();
            frame.elrs.editor_buffer = if cfg.bind_phrase.is_empty() {
                DEFAULT_BIND_PHRASE.to_string()
            } else {
                cfg.bind_phrase.clone()
            };
            frame.elrs.editor_cursor = 0;
            clear_local_feedback(frame);
            return apply_local_state(frame, cfg, Some("Editing bind phrase"));
        }
        _ => {
            clear_local_feedback(frame);
            return apply_local_state(frame, cfg, Some("ELRS"));
        }
    };

    set_local_feedback(frame, severity, motion, status);
    apply_local_state(frame, cfg, Some(status));
}

fn apply_local_state(frame: &mut UiFrame, cfg: &LocalElrsConfig, status: Option<&str>) {
    frame.elrs.module_name = "ELRS".to_string();
    frame.elrs.device_name = "Not Connected".to_string();
    frame.elrs.version = "--".to_string();
    frame.elrs.path = "/".to_string();
    frame.elrs.connected = false;
    frame.elrs.rf_output_enabled = cfg.rf_output_enabled;
    frame.elrs.link_active = false;
    frame.elrs.busy = false;
    frame.elrs.packet_rate = "--".to_string();
    frame.elrs.telemetry_ratio = "--".to_string();
    frame.elrs.tx_power = format!("{}mW", cfg.tx_power_mw);
    frame.elrs.wifi_running = cfg.wifi_manual_on;

    if let Some(status) = status {
        frame.elrs.status_text = status.to_string();
    }

    frame.elrs.params = vec![
        crate::messages::ElrsParamEntry {
            id: "rf_output".to_string(),
            label: "RF Output".to_string(),
            value: if cfg.rf_output_enabled {
                "ON".to_string()
            } else {
                "OFF".to_string()
            },
            selectable: true,
        },
        crate::messages::ElrsParamEntry {
            id: "wifi_manual".to_string(),
            label: "Module WiFi".to_string(),
            value: if cfg.wifi_manual_on {
                "ON".to_string()
            } else {
                "OFF".to_string()
            },
            selectable: true,
        },
        crate::messages::ElrsParamEntry {
            id: "bind".to_string(),
            label: "Bind".to_string(),
            value: "SERVICE".to_string(),
            selectable: true,
        },
        crate::messages::ElrsParamEntry {
            id: "tx_power".to_string(),
            label: "TX Power".to_string(),
            value: format!("{}mW", cfg.tx_power_mw),
            selectable: true,
        },
        crate::messages::ElrsParamEntry {
            id: "bind_phrase".to_string(),
            label: "Bind Phrase".to_string(),
            value: if cfg.bind_phrase.is_empty() {
                DEFAULT_BIND_PHRASE.to_string()
            } else {
                cfg.bind_phrase.clone()
            },
            selectable: true,
        },
        crate::messages::ElrsParamEntry {
            id: "link_state".to_string(),
            label: "Link State".to_string(),
            value: if cfg.rf_output_enabled {
                "SERVICE OFFLINE".to_string()
            } else {
                "RF OFF".to_string()
            },
            selectable: false,
        },
        crate::messages::ElrsParamEntry {
            id: "signal".to_string(),
            label: "Signal".to_string(),
            value: "--".to_string(),
            selectable: false,
        },
        crate::messages::ElrsParamEntry {
            id: "aircraft_battery".to_string(),
            label: "Aircraft Battery".to_string(),
            value: "--".to_string(),
            selectable: false,
        },
        crate::messages::ElrsParamEntry {
            id: "telemetry_fresh".to_string(),
            label: "Telemetry Fresh".to_string(),
            value: "stale".to_string(),
            selectable: false,
        },
        crate::messages::ElrsParamEntry {
            id: "feedback".to_string(),
            label: "Feedback".to_string(),
            value: "start rf_link_service".to_string(),
            selectable: false,
        },
    ];
}

fn load_local_config() -> LocalElrsConfig {
    match store::load_radio_config() {
        Ok(radio) => {
            let bind_phrase = if radio.elrs.bind_phrase.is_empty() {
                DEFAULT_BIND_PHRASE.to_string()
            } else {
                radio.elrs.bind_phrase
            };
            LocalElrsConfig {
                rf_output_enabled: radio.elrs.rf_output_enabled,
                wifi_manual_on: radio.elrs.wifi_manual_on,
                tx_power_mw: normalize_power_level(radio.elrs.tx_power_mw),
                bind_phrase,
            }
        }
        Err(_) => LocalElrsConfig::default(),
    }
}

fn save_local_config(cfg: &LocalElrsConfig) -> Result<(), String> {
    let mut radio = store::load_radio_config().map_err(|err| err.to_string())?;
    radio.elrs.rf_output_enabled = cfg.rf_output_enabled;
    radio.elrs.wifi_manual_on = cfg.wifi_manual_on;
    radio.elrs.tx_power_mw = normalize_power_level(cfg.tx_power_mw);
    radio.elrs.bind_phrase = cfg.bind_phrase.clone();
    store::save_radio_config(&radio).map_err(|err| err.to_string())
}

fn shift_power_level(current: u16, delta: isize) -> u16 {
    let idx = LOCAL_POWER_LEVELS_MW
        .iter()
        .position(|power| *power == normalize_power_level(current))
        .unwrap_or(2) as isize;
    let next = (idx + delta).clamp(0, LOCAL_POWER_LEVELS_MW.len() as isize - 1) as usize;
    LOCAL_POWER_LEVELS_MW[next]
}

fn normalize_power_level(raw: u16) -> u16 {
    LOCAL_POWER_LEVELS_MW
        .iter()
        .min_by_key(|level| level.abs_diff(raw))
        .copied()
        .unwrap_or(100)
}

fn move_editor_cursor(frame: &mut UiFrame, delta: isize) {
    let len = frame.elrs.editor_buffer.len().max(1);
    if delta.is_negative() {
        frame.elrs.editor_cursor = frame
            .elrs
            .editor_cursor
            .saturating_sub(delta.unsigned_abs());
    } else {
        frame.elrs.editor_cursor = frame
            .elrs
            .editor_cursor
            .saturating_add(delta as usize)
            .min(len.saturating_sub(1));
    }
}

fn cycle_editor_char(frame: &mut UiFrame, delta: isize) {
    let mut bytes = frame.elrs.editor_buffer.as_bytes().to_vec();
    if bytes.is_empty() {
        bytes.push(LOCAL_EDITOR_CHARSET[0]);
        frame.elrs.editor_cursor = 0;
    }
    if frame.elrs.editor_cursor >= bytes.len() {
        frame.elrs.editor_cursor = bytes.len().saturating_sub(1);
    }

    let cursor = frame.elrs.editor_cursor;
    let current = bytes[cursor];
    let current_idx = LOCAL_EDITOR_CHARSET
        .iter()
        .position(|ch| *ch == current)
        .unwrap_or(0);
    let next_idx =
        (current_idx as isize + delta).rem_euclid(LOCAL_EDITOR_CHARSET.len() as isize) as usize;
    bytes[cursor] = LOCAL_EDITOR_CHARSET[next_idx];
    frame.elrs.editor_buffer = String::from_utf8_lossy(&bytes).to_string();
}

#[cfg(test)]
mod tests {
    use super::{local_feedback_target, set_local_feedback};
    use crate::{
        messages::{UiFeedbackMotion, UiFeedbackSeverity, UiFeedbackSlot, UiFeedbackTarget},
        ui::model::UiFrame,
    };

    #[test]
    fn test_local_feedback_target_for_wifi_maps_to_wifi_field() {
        assert_eq!(
            local_feedback_target(1),
            UiFeedbackTarget::FieldId("wifi_manual".to_string())
        );
    }

    #[test]
    fn test_set_local_feedback_assigns_top_status_feedback_and_increments_seq() {
        let mut frame = UiFrame::default();
        frame.elrs.selected_idx = 3;

        set_local_feedback(
            &mut frame,
            UiFeedbackSeverity::Success,
            UiFeedbackMotion::Pulse,
            "TX power updated",
        );
        let first = frame
            .elrs
            .interaction_feedback
            .clone()
            .expect("first feedback");
        set_local_feedback(
            &mut frame,
            UiFeedbackSeverity::Error,
            UiFeedbackMotion::ShakeX,
            "TX power save failed",
        );
        let second = frame
            .elrs
            .interaction_feedback
            .clone()
            .expect("second feedback");

        assert_eq!(first.slot, UiFeedbackSlot::TopStatusBar);
        assert_eq!(
            first.target,
            UiFeedbackTarget::FieldId("tx_power".to_string())
        );
        assert!(second.seq > first.seq);
    }
}

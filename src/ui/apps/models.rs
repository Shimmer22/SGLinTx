use crate::{
    config::store,
    messages::ActiveModelMsg,
    ui::{
        apps::{AppSpec, UiAppContext, UiAppModule},
        debug_log,
        input::UiInputEvent,
        model::{AppId, UiFrame},
    },
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::Models,
    title: "MODELS",
    icon_text: "MOD",
    accent: (255, 181, 92),
};

pub struct ModelsApp;
pub static MODELS_APP: ModelsApp = ModelsApp;

impl UiAppModule for ModelsApp {
    fn on_event(&self, frame: &mut UiFrame, event: UiInputEvent, ctx: &UiAppContext<'_>) {
        match event {
            UiInputEvent::Up => {
                frame.model_focus_idx = frame.model_focus_idx.saturating_sub(1);
            }
            UiInputEvent::Down => {
                let max_idx = frame.model_entries.len().saturating_sub(1);
                frame.model_focus_idx = (frame.model_focus_idx + 1).min(max_idx);
            }
            UiInputEvent::Open => {
                if let Some(entry) = frame.model_entries.get(frame.model_focus_idx) {
                    match store::set_active_model(&entry.id) {
                        Ok(_) => {
                            frame.model_active_idx = frame.model_focus_idx;
                            match store::load_active_model() {
                                Ok(model) => ctx.active_model_tx.send(ActiveModelMsg { model }),
                                Err(err) => debug_log(&format!("load_active_model failed: {err}")),
                            }
                        }
                        Err(err) => debug_log(&format!("set_active_model failed: {err}")),
                    }
                }
            }
            _ => {}
        }
    }

    fn render_terminal_detail(&self, frame: &UiFrame) -> String {
        if frame.model_entries.is_empty() {
            return "No model entries found.\n\nEsc Back".to_string();
        }

        let max_idx = frame.model_entries.len().saturating_sub(1);
        let focus = frame.model_focus_idx.min(max_idx);
        let active = frame.model_active_idx.min(max_idx);
        let focus_model = &frame.model_entries[focus];
        let active_model = &frame.model_entries[active];

        let start = focus.saturating_sub(1).min(max_idx.saturating_sub(3));
        let mut lines = Vec::with_capacity(4);
        for idx in start..=(start + 3).min(max_idx) {
            let entry = &frame.model_entries[idx];
            lines.push(format!(
                "{} {} ({})",
                if idx == focus { ">" } else { " " },
                entry.name,
                entry.protocol,
            ));
        }

        while lines.len() < 4 {
            lines.push(String::new());
        }

        format!(
            "Active Model: {} ({})\nFocused Model: {} ({})\n\nModel List\n{}\n{}\n{}\n{}\n\nUp/Down: focus model\nEnter: apply focused model\nEsc Back",
            active_model.name,
            active_model.protocol,
            focus_model.name,
            focus_model.protocol,
            lines[0],
            lines[1],
            lines[2],
            lines[3],
        )
    }
}

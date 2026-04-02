use crate::{
    messages::ElrsCommandMsg,
    ui::{
        apps::{common::elrs_list_lines, AppSpec, UiAppContext, UiAppModule},
        input::UiInputEvent,
        model::{AppId, UiFrame},
    },
};

pub const SPEC: AppSpec = AppSpec {
    id: AppId::Scripts,
    title: "ELRS",
    icon_text: "ELR",
    accent: (255, 216, 109),
};

pub struct ScriptsApp;
pub static SCRIPTS_APP: ScriptsApp = ScriptsApp;

impl UiAppModule for ScriptsApp {
    fn on_event(&self, _frame: &mut UiFrame, event: UiInputEvent, ctx: &UiAppContext<'_>) {
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

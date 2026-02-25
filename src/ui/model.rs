use crate::messages::{SystemConfigMsg, SystemStatusMsg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppId {
    System,
    Control,
    Models,
    Cloud,
}

impl AppId {
    pub const ALL: [AppId; 4] = [AppId::System, AppId::Control, AppId::Models, AppId::Cloud];

    pub fn title(self) -> &'static str {
        match self {
            AppId::System => "SYSTEM",
            AppId::Control => "CONTROL",
            AppId::Models => "MODELS",
            AppId::Cloud => "CLOUD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPage {
    Launcher,
    App(AppId),
}

#[derive(Debug, Clone, Copy)]
pub struct UiFrame {
    pub page: UiPage,
    pub selected_app: AppId,
    pub status: SystemStatusMsg,
    pub config: SystemConfigMsg,
}

impl Default for UiFrame {
    fn default() -> Self {
        Self {
            page: UiPage::Launcher,
            selected_app: AppId::System,
            status: SystemStatusMsg::default(),
            config: SystemConfigMsg::default(),
        }
    }
}

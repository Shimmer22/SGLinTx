use crate::messages::{SystemConfigMsg, SystemStatusMsg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppId {
    System,
    Control,
    Models,
    Cloud,
    Sensor,
    Trainer,
    Scripts,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPage {
    Launcher,
    App(AppId),
}

#[derive(Debug, Clone, Copy)]
pub struct UiFrame {
    pub page: UiPage,
    pub launcher_page: usize,
    pub selected_row: usize,
    pub selected_col: usize,
    pub status: SystemStatusMsg,
    pub config: SystemConfigMsg,
}

impl Default for UiFrame {
    fn default() -> Self {
        Self {
            page: UiPage::Launcher,
            launcher_page: 0,
            selected_row: 0,
            selected_col: 0,
            status: SystemStatusMsg::default(),
            config: SystemConfigMsg::default(),
        }
    }
}

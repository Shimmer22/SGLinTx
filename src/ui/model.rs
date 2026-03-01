use crate::{
    messages::{AdcRawMsg, SystemConfigMsg, SystemStatusMsg},
    mixer::MixerOutMsg,
};

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

#[derive(Debug, Clone)]
pub struct UiFrame {
    pub page: UiPage,
    pub launcher_page: usize,
    pub selected_row: usize,
    pub selected_col: usize,
    pub status: SystemStatusMsg,
    pub config: SystemConfigMsg,
    pub adc_raw: AdcRawMsg,
    pub mixer_out: MixerOutMsg,
    pub model_focus_idx: usize,
    pub model_active_idx: usize,
    pub cloud_connected: bool,
    pub cloud_last_sync_secs: u64,
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
            adc_raw: AdcRawMsg::default(),
            mixer_out: MixerOutMsg {
                thrust: 5000,
                direction: 5000,
                aileron: 5000,
                elevator: 5000,
            },
            model_focus_idx: 0,
            model_active_idx: 0,
            cloud_connected: false,
            cloud_last_sync_secs: 0,
        }
    }
}

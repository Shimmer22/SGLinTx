use crate::config::ModelConfig;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdcRawMsg {
    pub value: [i16; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemStatusMsg {
    pub remote_battery_percent: u8,
    pub aircraft_battery_percent: u8,
    pub signal_strength_percent: u8,
    pub unix_time_secs: u64,
}

impl Default for SystemStatusMsg {
    fn default() -> Self {
        Self {
            remote_battery_percent: 100,
            aircraft_battery_percent: 100,
            signal_strength_percent: 100,
            unix_time_secs: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemConfigMsg {
    pub backlight_percent: u8,
    pub sound_percent: u8,
}

impl Default for SystemConfigMsg {
    fn default() -> Self {
        Self {
            backlight_percent: 70,
            sound_percent: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveModelMsg {
    pub model: ModelConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElrsParamEntry {
    pub id: String,
    pub label: String,
    pub value: String,
    pub selectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElrsStateMsg {
    pub connected: bool,
    pub busy: bool,
    pub can_leave: bool,
    pub path: String,
    pub editor_active: bool,
    pub editor_label: String,
    pub editor_buffer: String,
    pub editor_cursor: usize,
    pub module_name: String,
    pub device_name: String,
    pub version: String,
    pub packet_rate: String,
    pub telemetry_ratio: String,
    pub tx_power: String,
    pub status_text: String,
    pub wifi_running: bool,
    pub selected_idx: usize,
    pub params: Vec<ElrsParamEntry>,
}

impl Default for ElrsStateMsg {
    fn default() -> Self {
        Self {
            connected: false,
            busy: false,
            can_leave: true,
            path: "/".to_string(),
            editor_active: false,
            editor_label: String::new(),
            editor_buffer: String::new(),
            editor_cursor: 0,
            module_name: "ELRS".to_string(),
            device_name: "Not Connected".to_string(),
            version: "--".to_string(),
            packet_rate: "--".to_string(),
            telemetry_ratio: "--".to_string(),
            tx_power: "--".to_string(),
            status_text: "Idle".to_string(),
            wifi_running: false,
            selected_idx: 0,
            params: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElrsCommandMsg {
    Refresh,
    Back,
    SelectPrev,
    SelectNext,
    ValueDec,
    ValueInc,
    Activate,
}

#[rpos::ctor::ctor]
fn register() {
    rpos::msg::add_message::<AdcRawMsg>("adc_raw");
    rpos::msg::add_message::<SystemStatusMsg>("system_status");
    rpos::msg::add_message::<SystemConfigMsg>("system_config");
    rpos::msg::add_message::<ActiveModelMsg>("active_model");
    rpos::msg::add_message::<ElrsStateMsg>("elrs_state");
    rpos::msg::add_message::<ElrsCommandMsg>("elrs_cmd");
}

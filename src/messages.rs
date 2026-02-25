#[derive(Debug, Clone, Copy, Default)]
pub struct AdcRawMsg {
    pub value: [i16; 4],
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
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

#[rpos::ctor::ctor]
fn register() {
    rpos::msg::add_message::<AdcRawMsg>("adc_raw");
    rpos::msg::add_message::<SystemStatusMsg>("system_status");
    rpos::msg::add_message::<SystemConfigMsg>("system_config");
}

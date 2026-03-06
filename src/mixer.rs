use std::{
    fs,
    io::Read,
    sync::{Arc, Mutex},
};

use rpos::thread_logln;

use crate::{
    calibrate::{
        CalibrationData,
        JoystickChannel::{self, *},
    },
    config::{store, ControlRole, ModelConfig, OutputLimits},
    messages::{ActiveModelMsg, AdcRawMsg},
    CALIBRATE_FILENAME,
};

#[derive(Debug, Clone)]
pub struct MixerOutMsg {
    pub thrust: u16,
    pub direction: u16,
    pub aileron: u16,
    pub elevator: u16,
}

fn cal_mixout(channel: JoystickChannel, raw: &AdcRawMsg, cal_data: &CalibrationData) -> u16 {
    let channel_cal_info = &cal_data.channel_infos[channel as usize];

    let raw_val = raw.value[channel_cal_info.index as usize]
        .clamp(channel_cal_info.min, channel_cal_info.max) as i32;

    let mut ret = (raw_val - channel_cal_info.min as i32) as u32 * 10000
        / (channel_cal_info.max as i32 - channel_cal_info.min as i32) as u32;

    if channel_cal_info.rev {
        ret = 10000 - ret;
    }

    ret as u16
}

fn apply_output_profile(value: u16, model: &ModelConfig, role: ControlRole) -> u16 {
    let Some(output) = model.mixer.outputs.iter().find(|output| output.role == role) else {
        return value;
    };

    let centered = value as i32 - 5000;
    let weighted = centered * output.weight as i32 / 100;
    let offset = output.offset as i32 * 5;
    let mut adjusted = 5000 + weighted + offset;
    adjusted = apply_limits(adjusted, &output.limits);
    adjusted.clamp(0, 10000) as u16
}

fn apply_limits(value: i32, limits: &OutputLimits) -> i32 {
    let subtrim = limits.subtrim as i32 * 5;
    let mut adjusted = value + subtrim;
    if limits.reversed {
        adjusted = 10000 - adjusted;
    }

    let low = ((limits.min as i32 + 1000) * 5).clamp(0, 10000);
    let high = ((limits.max as i32 + 1000) * 5).clamp(0, 10000);
    adjusted.clamp(low.min(high), low.max(high))
}

fn load_calibration() -> Option<CalibrationData> {
    let mut toml_str = String::new();
    if let Ok(mut file) = fs::File::open(CALIBRATE_FILENAME) {
        file.read_to_string(&mut toml_str).unwrap();
    } else {
        thread_logln!("no joystick.toml found. please calibrate joysticks first!");
        return None;
    }

    Some(toml::from_str::<CalibrationData>(toml_str.as_str()).unwrap())
}

fn load_initial_model() -> ModelConfig {
    if let Err(err) = store::ensure_default_layout() {
        thread_logln!("config layout init failed: {}", err);
    }
    store::load_active_model().unwrap_or_default()
}

fn mixer_main(_argc: u32, _argv: *const &str) {
    let Some(cal_data) = load_calibration() else {
        return;
    };

    let rx = rpos::msg::get_new_rx_of_message::<AdcRawMsg>("adc_raw").unwrap();
    let tx = rpos::msg::get_new_tx_of_message::<MixerOutMsg>("mixer_out").unwrap();
    let active_model = Arc::new(Mutex::new(load_initial_model()));

    if let Some(active_model_rx) = rpos::msg::get_new_rx_of_message::<ActiveModelMsg>("active_model") {
        let active_model_for_updates = active_model.clone();
        active_model_rx.register_callback("mixer_active_model", move |msg| {
            if let Ok(mut current_model) = active_model_for_updates.lock() {
                *current_model = msg.model.clone();
            }
        });
    }

    rx.register_callback("mixer_callback", move |x| {
        let current_model = active_model.lock().unwrap().clone();
        let mixer_out = MixerOutMsg {
            thrust: apply_output_profile(cal_mixout(Thrust, x, &cal_data), &current_model, ControlRole::Thrust),
            direction: apply_output_profile(
                cal_mixout(Direction, x, &cal_data),
                &current_model,
                ControlRole::Direction,
            ),
            aileron: apply_output_profile(cal_mixout(Aileron, x, &cal_data), &current_model, ControlRole::Aileron),
            elevator: apply_output_profile(
                cal_mixout(Elevator, x, &cal_data),
                &current_model,
                ControlRole::Elevator,
            ),
        };
        tx.send(mixer_out);
    });
}

#[rpos::ctor::ctor]
fn register() {
    rpos::msg::add_message::<MixerOutMsg>("mixer_out");
    rpos::module::Module::register("mixer", mixer_main);
}

#[cfg(test)]
mod tests {
    use crate::calibrate::ChannelInfo;

    use super::*;
    use rand::prelude::*;

    #[test]
    fn test_cal_mixout() {
        let mut rng = thread_rng();
        let mut get_random_channel_value = || rng.gen_range(300..1400) as i16;
        let mut adc_raw = AdcRawMsg {
            value: [500, 100, 1600, get_random_channel_value()],
        };
        let mut cal_data = CalibrationData {
            channel_infos: [
                ChannelInfo {
                    name: "thrust".to_string(),
                    index: 0,
                    min: 200,
                    max: 1500,
                    rev: false,
                },
                ChannelInfo {
                    name: "direction".to_string(),
                    index: 1,
                    min: 200,
                    max: 1500,
                    rev: false,
                },
                ChannelInfo {
                    name: "aliron".to_string(),
                    index: 2,
                    min: 200,
                    max: 1500,
                    rev: false,
                },
                ChannelInfo {
                    name: "ele".to_string(),
                    index: 3,
                    min: 200,
                    max: 1500,
                    rev: false,
                },
            ]
            .to_vec(),
            channel_indexs: [0; 4].to_vec(),
        };

        assert_eq!(
            cal_mixout(JoystickChannel::Thrust, &adc_raw, &cal_data),
            ((500 - 200) as u32 * 10000 / (1500 - 200)) as u16
        );
        assert_eq!(
            cal_mixout(JoystickChannel::Direction, &adc_raw, &cal_data),
            ((200 - 200) as u32 * 10000 / (1500 - 200)) as u16
        );
        assert_eq!(
            cal_mixout(JoystickChannel::Aileron, &adc_raw, &cal_data),
            ((1500 - 200) as u32 * 10000 / (1500 - 200)) as u16
        );

        for _ in 0..1000 {
            assert!(cal_mixout(JoystickChannel::Elevator, &adc_raw, &cal_data) <= 10000);
            adc_raw.value[3] = get_random_channel_value();
        }

        cal_data.channel_infos[0].rev = true;
        assert_eq!(
            cal_mixout(JoystickChannel::Thrust, &adc_raw, &cal_data),
            10000 - ((500 - 200) as u32 * 10000 / (1500 - 200)) as u16
        );
    }

    #[test]
    fn test_apply_output_profile_uses_model_settings() {
        let model = store::load_active_model().unwrap_or_else(|_| ModelConfig::default());
        let mut model = model;
        let elevator = model
            .mixer
            .outputs
            .iter_mut()
            .find(|output| output.role == ControlRole::Elevator)
            .unwrap();
        elevator.weight = 50;
        elevator.offset = 100;
        elevator.limits.reversed = true;

        let value = apply_output_profile(7000, &model, ControlRole::Elevator);
        assert!(value <= 10000);
        assert_ne!(value, 7000);
    }
}

use std::time::Duration;
use std::f32::consts::PI;
use clap::Parser;
use rpos::{msg::get_new_tx_of_message, thread_logln};
use serde::Deserialize;
use crate::{messages::AdcRawMsg, client_process_args};

#[derive(Parser)]
#[command(name="mock_joystick", about = "Mock joystick data generator for testing", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "mock_config.toml")]
    config: String,
}

#[derive(Debug, Deserialize)]
struct MockConfig {
    #[serde(default = "default_mode")]
    mode: String,
    
    #[serde(default = "default_update_rate")]
    update_rate_hz: u32,
    
    #[serde(default)]
    static_config: StaticConfig,
    
    #[serde(default)]
    sine_config: SineConfig,
    
    #[serde(default)]
    step_config: StepConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct StaticConfig {
    #[serde(default = "default_static_channels")]
    channels: Vec<i16>,
}

#[derive(Debug, Deserialize, Clone)]
struct SineConfig {
    #[serde(default = "default_sine_base")]
    base: Vec<i16>,
    
    #[serde(default = "default_sine_amplitude")]
    amplitude: Vec<i16>,
    
    #[serde(default = "default_sine_frequency")]
    frequency_hz: Vec<f32>,
}

#[derive(Debug, Deserialize, Clone)]
struct StepConfig {
    #[serde(default = "default_step_values")]
    values: Vec<Vec<i16>>,
    
    #[serde(default = "default_step_duration")]
    step_duration_ms: u64,
}

// Default value functions
fn default_mode() -> String { "static".to_string() }
fn default_update_rate() -> u32 { 50 }
fn default_static_channels() -> Vec<i16> { vec![992, 992, 0, 992] }
fn default_sine_base() -> Vec<i16> { vec![992, 992, 0, 992] }
fn default_sine_amplitude() -> Vec<i16> { vec![100, 100, 0, 100] }
fn default_sine_frequency() -> Vec<f32> { vec![1.0, 0.5, 0.0, 2.0] }
fn default_step_values() -> Vec<Vec<i16>> { 
    vec![
        vec![0, 0, 0, 0],
        vec![992, 992, 0, 992],
        vec![1984, 1984, 1984, 1984],
    ]
}
fn default_step_duration() -> u64 { 1000 }

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            channels: default_static_channels(),
        }
    }
}

impl Default for SineConfig {
    fn default() -> Self {
        Self {
            base: default_sine_base(),
            amplitude: default_sine_amplitude(),
            frequency_hz: default_sine_frequency(),
        }
    }
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            values: default_step_values(),
            step_duration_ms: default_step_duration(),
        }
    }
}

pub fn mock_joystick_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    // Load configuration
    let config = match std::fs::read_to_string(&args.config) {
        Ok(content) => {
            match toml::from_str::<MockConfig>(&content) {
                Ok(cfg) => {
                    thread_logln!("Loaded mock config from: {}", args.config);
                    cfg
                }
                Err(e) => {
                    thread_logln!("Failed to parse config file: {}, using defaults", e);
                    MockConfig {
                        mode: default_mode(),
                        update_rate_hz: default_update_rate(),
                        static_config: StaticConfig::default(),
                        sine_config: SineConfig::default(),
                        step_config: StepConfig::default(),
                    }
                }
            }
        }
        Err(_) => {
            thread_logln!("Config file not found: {}, using defaults", args.config);
            MockConfig {
                mode: default_mode(),
                update_rate_hz: default_update_rate(),
                static_config: StaticConfig::default(),
                sine_config: SineConfig::default(),
                step_config: StepConfig::default(),
            }
        }
    };

    let adc_raw_tx = get_new_tx_of_message::<AdcRawMsg>("adc_raw").unwrap();
    let update_interval = Duration::from_millis(1000 / config.update_rate_hz as u64);

    thread_logln!("Mock joystick started in '{}' mode @ {} Hz", config.mode, config.update_rate_hz);

    match config.mode.as_str() {
        "static" => run_static_mode(&config.static_config, &adc_raw_tx, update_interval),
        "sine" => run_sine_mode(&config.sine_config, &adc_raw_tx, update_interval),
        "step" => run_step_mode(&config.step_config, &adc_raw_tx, update_interval),
        _ => {
            thread_logln!("Unknown mode: {}, falling back to static", config.mode);
            run_static_mode(&config.static_config, &adc_raw_tx, update_interval);
        }
    }
}

fn run_static_mode(
    config: &StaticConfig,
    tx: &rpos::channel::Sender<AdcRawMsg>,
    interval: Duration,
) {
    let mut channels = [0i16; 4];
    for (i, &val) in config.channels.iter().enumerate().take(4) {
        channels[i] = val;
    }

    thread_logln!("Static mode: channels = {:?}", channels);

    loop {
        tx.send(AdcRawMsg { value: channels });
        std::thread::sleep(interval);
    }
}

fn run_sine_mode(
    config: &SineConfig,
    tx: &rpos::channel::Sender<AdcRawMsg>,
    interval: Duration,
) {
    let mut time = 0.0f32;
    let dt = interval.as_secs_f32();

    thread_logln!("Sine mode: base = {:?}, amplitude = {:?}, freq = {:?}", 
        config.base, config.amplitude, config.frequency_hz);

    loop {
        let mut channels = [0i16; 4];
        
        for i in 0..4 {
            let base = config.base.get(i).copied().unwrap_or(0);
            let amplitude = config.amplitude.get(i).copied().unwrap_or(0);
            let frequency = config.frequency_hz.get(i).copied().unwrap_or(0.0);
            
            let sine_value = (2.0 * PI * frequency * time).sin();
            let value = base + (amplitude as f32 * sine_value) as i16;
            
            channels[i] = value.clamp(-2048, 2047);
        }

        tx.send(AdcRawMsg { value: channels });
        
        time += dt;
        if time > 1000.0 {
            time -= 1000.0; // Prevent overflow
        }
        
        std::thread::sleep(interval);
    }
}

fn run_step_mode(
    config: &StepConfig,
    tx: &rpos::channel::Sender<AdcRawMsg>,
    interval: Duration,
) {
    if config.values.is_empty() {
        thread_logln!("Step mode: no values configured, exiting");
        return;
    }

    thread_logln!("Step mode: {} steps, {} ms per step", 
        config.values.len(), config.step_duration_ms);

    let step_duration = Duration::from_millis(config.step_duration_ms);
    let steps_per_duration = (config.step_duration_ms as f32 / interval.as_millis() as f32).ceil() as usize;
    
    let mut step_index = 0;

    loop {
        let step_values = &config.values[step_index];
        let mut channels = [0i16; 4];
        
        for (i, &val) in step_values.iter().enumerate().take(4) {
            channels[i] = val;
        }

        // Send the same value multiple times for the step duration
        for _ in 0..steps_per_duration {
            tx.send(AdcRawMsg { value: channels });
            std::thread::sleep(interval);
        }

        step_index = (step_index + 1) % config.values.len();
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("mock_joystick", mock_joystick_main);
}

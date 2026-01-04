use std::fs::OpenOptions;
use std::io::Write;
use clap::Parser;
use rpos::{msg::get_new_rx_of_message, thread_logln};
use crate::{mixer::MixerOutMsg, client_process_args};

#[derive(Parser)]
#[command(name="usb_gamepad", about = "USB HID Gamepad output driver", long_about = None)]
struct Cli {
    /// HID device path
    #[arg(short, long, default_value = "/dev/hidg1")]
    device: String,
}

/// USB HID Gamepad Report Format (6 bytes):
/// Byte 0: 8 buttons (bit flags)
/// Byte 1: X axis (Direction, -127~127, 回中)
/// Byte 2: Y axis (Aileron, -127~127, 回中)
/// Byte 3: Z axis (Elevator, -127~127, 回中)
/// Byte 4: Rz axis (spare, -127~127, 回中)
/// Byte 5: Slider (Thrust/Throttle, 0~255, 不回中)
#[repr(C, packed)]
struct HidGamepadReport {
    buttons: u8,       // 8 button bits
    axis_x: i8,        // Direction (左右)
    axis_y: i8,        // Aileron (副翼)
    axis_z: i8,        // Elevator (升降)
    axis_rz: i8,       // Spare (备用)
    slider: u8,        // Thrust/Throttle (油门, 0~255)
}

impl HidGamepadReport {
    fn new() -> Self {
        Self {
            buttons: 0,
            axis_x: 0,
            axis_y: 0,
            axis_z: 0,
            axis_rz: 0,
            slider: 0,  // 油门默认最低
        }
    }

    fn to_bytes(&self) -> [u8; 6] {
        [
            self.buttons,
            self.axis_x as u8,
            self.axis_y as u8,
            self.axis_z as u8,
            self.axis_rz as u8,
            self.slider,
        ]
    }
}

/// Convert mixer value (0~10000) to HID axis value (-127~127)
/// Mixer 输出范围: 0 ~ 10000, 中心值 5000
/// HID 双向轴范围: -127 ~ 127, 中心值 0
fn mixer_to_hid_axis(mixer_value: u16) -> i8 {
    let normalized = (mixer_value as i32 - 5000) as f32 / 5000.0; // -1.0 ~ +1.0
    let hid_value = (normalized * 127.0) as i32;
    hid_value.clamp(-127, 127) as i8
}

/// Convert mixer value (0~10000) to HID throttle value (0~255)
/// Mixer 输出范围: 0 ~ 10000
/// HID 油门范围: 0 ~ 255 (单向，不回中)
fn mixer_to_hid_throttle(mixer_value: u16) -> u8 {
    let normalized = mixer_value as f32 / 10000.0;  // 0.0 ~ 1.0
    let hid_value = (normalized * 255.0) as u32;
    hid_value.clamp(0, 255) as u8
}

pub fn usb_gamepad_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    thread_logln!("USB Gamepad driver starting...");
    thread_logln!("  Device: {}", args.device);

    // 订阅 mixer 输出消息
    let mut mixer_rx = match get_new_rx_of_message::<MixerOutMsg>("mixer_out") {
        Some(rx) => rx,
        None => {
            thread_logln!("Failed to subscribe to mixer_out");
            return;
        }
    };

    // 打开 HID 设备
    let mut hid_device = match OpenOptions::new()
        .write(true)
        .open(&args.device)
    {
        Ok(f) => f,
        Err(e) => {
            thread_logln!("Failed to open HID device {}: {}", args.device, e);
            thread_logln!("Please run gamepad_composite.sh first!");
            return;
        }
    };

    thread_logln!("USB Gamepad ready, waiting for mixer data...");

    let mut counter = 0u32;
    loop {
        // rpos::Receiver 使用 .read() 方法，而不是 .recv()
        let msg = mixer_rx.read();
        
        counter += 1;
        if counter == 1 {
            thread_logln!("✓ Received first mixer data! thrust={}, dir={}, ail={}, elev={}", 
                msg.thrust, msg.direction, msg.aileron, msg.elevator);
        }
        
        let mut report = HidGamepadReport::new();

        // 映射 mixer 输出到 HID 轴
        // MixerOutMsg 字段: thrust, direction, aileron, elevator
        report.axis_x = mixer_to_hid_axis(msg.direction);  // X轴 = 左右方向
        report.axis_y = mixer_to_hid_axis(msg.aileron);    // Y轴 = 副翼
        report.axis_z = mixer_to_hid_axis(msg.elevator);   // Z轴 = 升降
        report.axis_rz = 0;  // 备用轴，暂时不用
        report.slider = mixer_to_hid_throttle(msg.thrust); // Slider = 油门 (0~255)

        // 暂时没有按键数据，保持为0
        report.buttons = 0;

        // 发送 HID 报告
        let report_bytes = report.to_bytes();
        if let Err(e) = hid_device.write_all(&report_bytes) {
            thread_logln!("Failed to write HID report: {}", e);
        }
        
        // 打印调试信息（每100次打印一次）
        if counter % 100 == 0 {
            thread_logln!("HID sent {} reports. Latest: Dir={}, Ail={}, Elev={}, Thr={}", 
                counter, report.axis_x, report.axis_y, report.axis_z, report.slider);
        }
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("usb_gamepad", usb_gamepad_main);
}

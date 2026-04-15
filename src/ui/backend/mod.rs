use super::{input::UiInputEvent, model::UiFrame};

mod terminal;

#[cfg(any(feature = "sdl_ui", all(feature = "lvgl_ui", target_os = "linux")))]
mod pointer;

#[cfg(feature = "lvgl_ui")]
mod lvgl_core;

#[cfg(feature = "sdl_ui")]
mod sdl;

#[cfg(all(feature = "lvgl_ui", target_os = "linux"))]
mod fbdev;
#[cfg(all(feature = "lvgl_ui", target_os = "linux", target_arch = "riscv64"))]
mod vo;

use terminal::TerminalBackend;

#[cfg(all(feature = "lvgl_ui", target_os = "linux"))]
use fbdev::FbdevBackend;
#[cfg(feature = "sdl_ui")]
use sdl::SdlBackend;
#[cfg(all(feature = "lvgl_ui", target_os = "linux", target_arch = "riscv64"))]
use vo::VoBackend;

pub trait LvglBackend {
    fn init(&mut self);
    fn poll_event(&mut self) -> Option<UiInputEvent>;
    fn render(&mut self, frame: &UiFrame);
    fn shutdown(&mut self);
}

pub enum BackendKind {
    PcApi,
    PcSdl {
        width: u32,
        height: u32,
    },
    Fbdev {
        device: String,
        touch_device: Option<String>,
        width: u32,
        height: u32,
    },
    Vo {
        touch_device: Option<String>,
        width: u32,
        height: u32,
    },
}

impl BackendKind {
    pub fn parse(
        name: &str,
        fb_device: &str,
        touch_device: Option<&str>,
        width: u32,
        height: u32,
    ) -> Self {
        match name {
            "pc_sdl" | "sdl" => Self::PcSdl { width, height },
            "fb" | "fbdev" => Self::Fbdev {
                device: fb_device.to_string(),
                touch_device: touch_device
                    .map(|path| path.trim().to_string())
                    .filter(|path| !path.is_empty()),
                width,
                height,
            },
            "vo" => Self::Vo {
                touch_device: touch_device
                    .map(|path| path.trim().to_string())
                    .filter(|path| !path.is_empty()),
                width,
                height,
            },
            _ => Self::PcApi,
        }
    }
}

fn signal_grade(v: u8) -> &'static str {
    match v {
        75..=100 => "SOLID",
        45..=74 => "FAIR",
        20..=44 => "WEAK",
        _ => "LOST",
    }
}

fn elrs_list_lines(frame: &UiFrame) -> [String; 4] {
    let total = frame.elrs.params.len();
    if total == 0 {
        return [
            "No ELRS params available".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];
    }

    let selected = frame.elrs.selected_idx.min(total.saturating_sub(1));
    let start = selected.saturating_sub(1).min(total.saturating_sub(4));
    let mut lines = Vec::with_capacity(4);
    for idx in start..(start + 4).min(total) {
        let entry = &frame.elrs.params[idx];
        lines.push(format!(
            "{} {}: {}",
            if idx == selected { ">" } else { " " },
            entry.label,
            entry.value
        ));
    }
    while lines.len() < 4 {
        lines.push(String::new());
    }
    [
        lines[0].clone(),
        lines[1].clone(),
        lines[2].clone(),
        lines[3].clone(),
    ]
}

pub fn new_backend(kind: BackendKind) -> Box<dyn LvglBackend> {
    match kind {
        BackendKind::PcApi => {
            eprintln!("[lintx-ui] new_backend -> PcApi");
            super::debug_log("new_backend -> PcApi");
            Box::new(TerminalBackend::new("pc-api".to_string()))
        }
        BackendKind::PcSdl { width, height } => {
            eprintln!("[lintx-ui] new_backend -> PcSdl {width}x{height}");
            super::debug_log(&format!("new_backend -> PcSdl {width}x{height}"));
            #[cfg(feature = "sdl_ui")]
            {
                return Box::new(SdlBackend::new(width, height));
            }
            #[cfg(not(feature = "sdl_ui"))]
            {
                let name = format!("pc-sdl-disabled({}x{})", width, height);
                return Box::new(TerminalBackend::new(name));
            }
        }
        BackendKind::Fbdev {
            device,
            touch_device,
            width,
            height,
        } => {
            eprintln!(
                "[lintx-ui] new_backend -> Fbdev device={device} touch_device={touch_device:?} size={width}x{height}"
            );
            super::debug_log(&format!(
                "new_backend -> Fbdev device={device} touch_device={touch_device:?} size={width}x{height}"
            ));
            #[cfg(all(feature = "lvgl_ui", target_os = "linux"))]
            {
                return Box::new(FbdevBackend::new(device, touch_device, width, height));
            }
            #[cfg(not(all(feature = "lvgl_ui", target_os = "linux")))]
            {
                return Box::new(TerminalBackend::new(format!("fbdev:{device}")));
            }
        }
        BackendKind::Vo {
            touch_device,
            width,
            height,
        } => {
            eprintln!(
                "[lintx-ui] new_backend -> Vo touch_device={touch_device:?} size={width}x{height}"
            );
            super::debug_log(&format!(
                "new_backend -> Vo touch_device={touch_device:?} size={width}x{height}"
            ));
            #[cfg(all(feature = "lvgl_ui", target_os = "linux", target_arch = "riscv64"))]
            {
                return Box::new(VoBackend::new(touch_device, width, height));
            }
            #[cfg(not(all(feature = "lvgl_ui", target_os = "linux", target_arch = "riscv64")))]
            {
                return Box::new(TerminalBackend::new("vo-disabled".to_string()));
            }
        }
    }
}

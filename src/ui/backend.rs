use std::io::Write;

use super::{
    catalog::{app_at, app_spec, page, PAGE_SPECS},
    input::UiInputEvent,
    model::{AppId, UiFrame, UiPage},
};

pub trait LvglBackend {
    fn init(&mut self);
    fn poll_event(&mut self) -> Option<UiInputEvent>;
    fn render(&mut self, frame: &UiFrame);
    fn shutdown(&mut self);
}

pub enum BackendKind {
    PcApi,
    PcSdl { width: u32, height: u32 },
    Fbdev { device: String },
}

impl BackendKind {
    pub fn parse(name: &str, fb_device: &str, width: u32, height: u32) -> Self {
        match name {
            "pc_sdl" | "sdl" => Self::PcSdl { width, height },
            "fb" | "fbdev" => Self::Fbdev {
                device: fb_device.to_string(),
            },
            _ => Self::PcApi,
        }
    }
}

pub struct TerminalBackend {
    backend_name: String,
}

impl TerminalBackend {
    pub fn new(backend_name: String) -> Self {
        Self { backend_name }
    }
}

const MODEL_NAMES: [&str; 4] = ["Quad X", "Fixed Wing", "Rover", "Boat"];
const MODEL_PROTOCOLS: [&str; 4] = ["CRSF 250Hz", "CRSF 150Hz", "PWM 100Hz", "PWM 50Hz"];

fn battery_grade(v: u8) -> &'static str {
    match v {
        80..=100 => "GOOD",
        50..=79 => "OK",
        20..=49 => "LOW",
        _ => "CRITICAL",
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

fn format_app_detail(frame: &UiFrame, app: AppId) -> String {
    match app {
        AppId::System => format!(
            "Remote Battery: {}% ({})\nAircraft Battery: {}% ({})\nSignal: {}% ({})\nClock: {}\n\nBacklight: {}%  (Up/Down)\nSound: {}%  (Left/Right)\n\nEsc Back",
            frame.status.remote_battery_percent,
            battery_grade(frame.status.remote_battery_percent),
            frame.status.aircraft_battery_percent,
            battery_grade(frame.status.aircraft_battery_percent),
            frame.status.signal_strength_percent,
            signal_grade(frame.status.signal_strength_percent),
            frame.status.unix_time_secs,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        ),
        AppId::Control => format!(
            "ADC Raw\nCH1:{}  CH2:{}\nCH3:{}  CH4:{}\n\nMixer Out (0..10000)\nThrust:{}\nDirection:{}\nAileron:{}\nElevator:{}\n\nUse this page to validate input chain.\nEsc Back",
            frame.adc_raw.value[0],
            frame.adc_raw.value[1],
            frame.adc_raw.value[2],
            frame.adc_raw.value[3],
            frame.mixer_out.thrust,
            frame.mixer_out.direction,
            frame.mixer_out.aileron,
            frame.mixer_out.elevator,
        ),
        AppId::Models => {
            let focus = frame.model_focus_idx.min(MODEL_NAMES.len().saturating_sub(1));
            let active = frame.model_active_idx.min(MODEL_NAMES.len().saturating_sub(1));
            format!(
                "Active Model: {} ({})\nFocused Model: {} ({})\n\nModel List\n{} {}\n{} {}\n{} {}\n{} {}\n\nUp/Down: focus model\nEnter: apply focused model\nEsc Back",
                MODEL_NAMES[active],
                MODEL_PROTOCOLS[active],
                MODEL_NAMES[focus],
                MODEL_PROTOCOLS[focus],
                if focus == 0 { ">" } else { " " },
                MODEL_NAMES[0],
                if focus == 1 { ">" } else { " " },
                MODEL_NAMES[1],
                if focus == 2 { ">" } else { " " },
                MODEL_NAMES[2],
                if focus == 3 { ">" } else { " " },
                MODEL_NAMES[3],
            )
        }
        AppId::Cloud => {
            let connection = if frame.cloud_connected { "ONLINE" } else { "OFFLINE" };
            let sync_secs = if frame.cloud_connected {
                frame.status.unix_time_secs.saturating_sub(frame.cloud_last_sync_secs)
            } else {
                0
            };
            format!(
                "Cloud Link: {}\nLink Quality: {}%\nLast Sync: {}s ago\n\nStatus Summary\nRemote {}% | Aircraft {}%\nSignal Class: {}\n\nEnter: connect/disconnect\nEsc Back",
                connection,
                frame.status.signal_strength_percent,
                sync_secs,
                frame.status.remote_battery_percent,
                frame.status.aircraft_battery_percent,
                signal_grade(frame.status.signal_strength_percent),
            )
        }
        _ => format!(
            "Remote Battery: {}%\nAircraft Battery: {}%\nSignal: {}%\nBacklight: {}%\nSound: {}%\n\nEsc Back",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        ),
    }
}

impl LvglBackend for TerminalBackend {
    fn init(&mut self) {
        println!("[ui] backend={} init", self.backend_name);
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        None
    }

    fn render(&mut self, frame: &UiFrame) {
        print!("\x1B[2J\x1B[1;1H");
        println!("LinTX Launcher [{}]", self.backend_name);
        println!(
            "STATUS | REM:{}% AIR:{}% SIG:{}% T:{}",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.status.unix_time_secs
        );
        println!("-----------------------------------------------");
        match frame.page {
            UiPage::Launcher => {
                let p = page(frame.launcher_page);
                println!(
                    "                LinTX  [Page {}/{}]",
                    p.id + 1,
                    PAGE_SPECS.len()
                );
                for r in 0..p.rows {
                    for c in 0..p.cols {
                        if let Some(app) = app_at(frame.launcher_page, r, c) {
                            let mark = if r == frame.selected_row && c == frame.selected_col {
                                ">"
                            } else {
                                " "
                            };
                            print!("{} {:8}  ", mark, app_spec(app).title);
                        }
                    }
                    println!();
                }
                println!("Arrows move, Enter open, Esc back, [ ] switch page, Q quit");
            }
            UiPage::App(app) => {
                println!("APP: {}", app_spec(app).title);
                println!("{}", format_app_detail(frame, app));
            }
        }
        let _ = std::io::stdout().flush();
    }

    fn shutdown(&mut self) {
        println!("[ui] shutdown");
    }
}

pub fn new_backend(kind: BackendKind) -> Box<dyn LvglBackend> {
    match kind {
        BackendKind::PcApi => {
            super::debug_log("new_backend -> PcApi");
            Box::new(TerminalBackend::new("pc-api".to_string()))
        }
        BackendKind::PcSdl { width, height } => {
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
        BackendKind::Fbdev { device } => {
            super::debug_log(&format!("new_backend -> Fbdev device={device}"));
            Box::new(TerminalBackend::new(format!("fbdev:{}", device)))
        }
    }
}

#[cfg(feature = "sdl_ui")]
const TOP_BAR_HEIGHT: i32 = 44;
#[cfg(feature = "sdl_ui")]
const LVGL_DRAW_BUF_PIXELS: usize = 800 * 80;

#[cfg(feature = "sdl_ui")]
struct LvglUiObjects {
    status_label: *mut lvgl_sys::lv_obj_t,
    clock_label: *mut lvgl_sys::lv_obj_t,
    page_label: *mut lvgl_sys::lv_obj_t,
    launcher_panel: *mut lvgl_sys::lv_obj_t,
    app_panel: *mut lvgl_sys::lv_obj_t,
    app_header_card: *mut lvgl_sys::lv_obj_t,
    app_badge_label: *mut lvgl_sys::lv_obj_t,
    app_title_label: *mut lvgl_sys::lv_obj_t,
    app_subtitle_label: *mut lvgl_sys::lv_obj_t,
    app_metric_cards: [*mut lvgl_sys::lv_obj_t; 2],
    app_metric_titles: [*mut lvgl_sys::lv_obj_t; 2],
    app_metric_values: [*mut lvgl_sys::lv_obj_t; 2],
    app_metric_bars: [*mut lvgl_sys::lv_obj_t; 2],
    app_list_title: *mut lvgl_sys::lv_obj_t,
    app_list_lines: [*mut lvgl_sys::lv_obj_t; 4],
    app_hint_label: *mut lvgl_sys::lv_obj_t,
    branding_label: *mut lvgl_sys::lv_obj_t,
    app_cards: [*mut lvgl_sys::lv_obj_t; 8],
    app_icon_boxes: [*mut lvgl_sys::lv_obj_t; 8],
    app_icon_labels: [*mut lvgl_sys::lv_obj_t; 8],
    app_title_labels: [*mut lvgl_sys::lv_obj_t; 8],
}

#[cfg(feature = "sdl_ui")]
struct AppTemplateData {
    accent: (u8, u8, u8),
    badge: String,
    title: String,
    subtitle: String,
    metric_titles: [String; 2],
    metric_values: [String; 2],
    metric_progress: [u8; 2],
    list_title: String,
    list_lines: [String; 4],
    hint: String,
}

#[cfg(feature = "sdl_ui")]
struct SdlBackend {
    width: u32,
    height: u32,
    sdl_ctx: Option<sdl2::Sdl>,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
    event_pump: Option<sdl2::EventPump>,
    framebuffer: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    display: Option<lvgl::Display>,
    ui: Option<LvglUiObjects>,
    last_tick: std::time::Instant,
}

#[cfg(feature = "sdl_ui")]
#[derive(Clone, Copy, Debug)]
enum SdlRenderMode {
    Software,
    Accelerated,
}

#[cfg(feature = "sdl_ui")]
impl SdlBackend {
    fn to_coord(v: i32) -> lvgl_sys::lv_coord_t {
        v.clamp(i16::MIN as i32, i16::MAX as i32) as lvgl_sys::lv_coord_t
    }

    fn new(width: u32, height: u32) -> Self {
        let fb_size = width as usize * height as usize * 3;
        Self {
            width,
            height,
            sdl_ctx: None,
            canvas: None,
            event_pump: None,
            framebuffer: std::rc::Rc::new(std::cell::RefCell::new(vec![0; fb_size])),
            display: None,
            ui: None,
            last_tick: std::time::Instant::now(),
        }
    }

    fn is_wsl() -> bool {
        if std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::env::var_os("WSL_INTEROP").is_some()
        {
            return true;
        }

        std::fs::read_to_string("/proc/version")
            .map(|v| v.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
    }

    fn select_render_mode() -> SdlRenderMode {
        match std::env::var("LINTX_SDL_RENDERER")
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or_else(|_| "auto".to_string())
            .as_str()
        {
            "software" | "sw" => SdlRenderMode::Software,
            "accelerated" | "gpu" => SdlRenderMode::Accelerated,
            _ => {
                if Self::is_wsl() {
                    SdlRenderMode::Software
                } else {
                    SdlRenderMode::Accelerated
                }
            }
        }
    }

    fn set_label_text(label: *mut lvgl_sys::lv_obj_t, text: &str) {
        let sanitized = text.replace('\0', " ");
        if let Ok(c_text) = std::ffi::CString::new(sanitized) {
            unsafe {
                lvgl_sys::lv_label_set_text(label, c_text.as_ptr());
            }
        }
    }

    fn clamp_pct(v: i32) -> u8 {
        v.clamp(0, 100) as u8
    }

    fn set_latest_display_default() {
        unsafe {
            let mut last = std::ptr::null_mut();
            let mut cur = lvgl_sys::lv_disp_get_next(std::ptr::null_mut());
            while !cur.is_null() {
                last = cur;
                cur = lvgl_sys::lv_disp_get_next(cur);
            }
            if !last.is_null() {
                lvgl_sys::lv_disp_set_default(last);
            }
        }
    }

    fn app_template_data(&self, frame: &UiFrame, app: AppId) -> AppTemplateData {
        let spec = app_spec(app);
        match app {
            AppId::System => AppTemplateData {
                accent: spec.accent,
                badge: "SYSTEM".to_string(),
                title: "Power & Device Health".to_string(),
                subtitle: "Live status and quick parameter tuning".to_string(),
                metric_titles: ["Remote Battery".to_string(), "Aircraft Battery".to_string()],
                metric_values: [
                    format!("{}%", frame.status.remote_battery_percent),
                    format!("{}%", frame.status.aircraft_battery_percent),
                ],
                metric_progress: [
                    frame.status.remote_battery_percent,
                    frame.status.aircraft_battery_percent,
                ],
                list_title: "Quick Info".to_string(),
                list_lines: [
                    format!(
                        "Signal: {}% ({})",
                        frame.status.signal_strength_percent,
                        signal_grade(frame.status.signal_strength_percent)
                    ),
                    format!("Unix Time: {}", frame.status.unix_time_secs),
                    format!("Backlight: {}%", frame.config.backlight_percent),
                    format!("Sound: {}%", frame.config.sound_percent),
                ],
                hint: "UP/DOWN: Backlight   LEFT/RIGHT: Sound   ESC: Back".to_string(),
            },
            AppId::Control => {
                let left_avg =
                    ((frame.adc_raw.value[0] as i32 + frame.adc_raw.value[1] as i32) / 2).max(0);
                let right_avg =
                    ((frame.adc_raw.value[2] as i32 + frame.adc_raw.value[3] as i32) / 2).max(0);
                AppTemplateData {
                    accent: spec.accent,
                    badge: "CONTROL".to_string(),
                    title: "Input Pipeline Monitor".to_string(),
                    subtitle: "Sensor input and mixer output diagnostics".to_string(),
                    metric_titles: ["ADC CH1/2".to_string(), "ADC CH3/4".to_string()],
                    metric_values: [
                        format!("{}/{}", frame.adc_raw.value[0], frame.adc_raw.value[1]),
                        format!("{}/{}", frame.adc_raw.value[2], frame.adc_raw.value[3]),
                    ],
                    metric_progress: [
                        Self::clamp_pct(left_avg * 100 / 2048),
                        Self::clamp_pct(right_avg * 100 / 2048),
                    ],
                    list_title: "Mixer Out".to_string(),
                    list_lines: [
                        format!("Thrust: {}", frame.mixer_out.thrust),
                        format!("Direction: {}", frame.mixer_out.direction),
                        format!("Aileron: {}", frame.mixer_out.aileron),
                        format!("Elevator: {}", frame.mixer_out.elevator),
                    ],
                    hint: "Use for ADC -> mixer chain validation   ESC: Back".to_string(),
                }
            }
            AppId::Models => {
                let names = ["Quad X", "Fixed Wing", "Rover", "Boat"];
                let protocols = ["CRSF 250Hz", "CRSF 150Hz", "PWM 100Hz", "PWM 50Hz"];
                let focus = frame.model_focus_idx.min(3);
                let active = frame.model_active_idx.min(3);
                AppTemplateData {
                    accent: spec.accent,
                    badge: "MODELS".to_string(),
                    title: "Model Profile Manager".to_string(),
                    subtitle: "Profile selection and activation".to_string(),
                    metric_titles: ["Active Profile".to_string(), "Focused Profile".to_string()],
                    metric_values: [
                        format!("{} · {}", names[active], protocols[active]),
                        format!("{} · {}", names[focus], protocols[focus]),
                    ],
                    metric_progress: [
                        Self::clamp_pct(((active + 1) * 25) as i32),
                        Self::clamp_pct(((focus + 1) * 25) as i32),
                    ],
                    list_title: "Profiles".to_string(),
                    list_lines: [
                        format!(
                            "{} {}",
                            if active == 0 { "[A]" } else { "   " },
                            if focus == 0 { "> Quad X" } else { "  Quad X" }
                        ),
                        format!(
                            "{} {}",
                            if active == 1 { "[A]" } else { "   " },
                            if focus == 1 {
                                "> Fixed Wing"
                            } else {
                                "  Fixed Wing"
                            }
                        ),
                        format!(
                            "{} {}",
                            if active == 2 { "[A]" } else { "   " },
                            if focus == 2 { "> Rover" } else { "  Rover" }
                        ),
                        format!(
                            "{} {}",
                            if active == 3 { "[A]" } else { "   " },
                            if focus == 3 { "> Boat" } else { "  Boat" }
                        ),
                    ],
                    hint: "UP/DOWN: Focus Profile   ENTER: Apply   ESC: Back".to_string(),
                }
            }
            AppId::Cloud => {
                let online = frame.cloud_connected;
                let sync_secs = if online {
                    frame
                        .status
                        .unix_time_secs
                        .saturating_sub(frame.cloud_last_sync_secs)
                } else {
                    0
                };
                AppTemplateData {
                    accent: spec.accent,
                    badge: "CLOUD".to_string(),
                    title: "Telemetry Link".to_string(),
                    subtitle: "Sync state and link quality".to_string(),
                    metric_titles: ["Connection".to_string(), "Link Quality".to_string()],
                    metric_values: [
                        if online {
                            "ONLINE".to_string()
                        } else {
                            "OFFLINE".to_string()
                        },
                        format!("{}%", frame.status.signal_strength_percent),
                    ],
                    metric_progress: [
                        if online { 100 } else { 0 },
                        frame.status.signal_strength_percent,
                    ],
                    list_title: "Sync Status".to_string(),
                    list_lines: [
                        format!("Last Sync: {}s ago", sync_secs),
                        format!("Remote Battery: {}%", frame.status.remote_battery_percent),
                        format!(
                            "Aircraft Battery: {}%",
                            frame.status.aircraft_battery_percent
                        ),
                        format!(
                            "Signal Class: {}",
                            signal_grade(frame.status.signal_strength_percent)
                        ),
                    ],
                    hint: "ENTER: Connect/Disconnect   ESC: Back".to_string(),
                }
            }
            _ => AppTemplateData {
                accent: spec.accent,
                badge: spec.title.to_string(),
                title: format!("{} Workspace", spec.title),
                subtitle: "Template placeholder".to_string(),
                metric_titles: ["Metric A".to_string(), "Metric B".to_string()],
                metric_values: ["--".to_string(), "--".to_string()],
                metric_progress: [0, 0],
                list_title: "Details".to_string(),
                list_lines: [
                    "No data".to_string(),
                    "No data".to_string(),
                    "No data".to_string(),
                    "No data".to_string(),
                ],
                hint: "ESC: Back".to_string(),
            },
        }
    }

    fn build_ui(&mut self) {
        let width = self.width as i32;
        let height = self.height as i32;

        unsafe {
            let root = lvgl_sys::lv_disp_get_scr_act(std::ptr::null_mut());
            lvgl_sys::lv_obj_clean(root);
            lvgl_sys::lv_obj_set_style_bg_color(root, lvgl_sys::_LV_COLOR_MAKE(20, 20, 22), 0);
            lvgl_sys::lv_obj_clear_flag(root, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_scrollbar_mode(
                root,
                lvgl_sys::LV_SCROLLBAR_MODE_OFF as lvgl_sys::lv_scrollbar_mode_t,
            );

            let status_label = lvgl_sys::lv_label_create(root);
            lvgl_sys::lv_obj_set_style_text_color(
                status_label,
                lvgl_sys::_LV_COLOR_MAKE(180, 180, 185),
                0,
            );
            lvgl_sys::lv_obj_set_pos(status_label, Self::to_coord(8), Self::to_coord(10));

            let page_label = lvgl_sys::lv_label_create(root);
            lvgl_sys::lv_obj_set_style_text_color(
                page_label,
                lvgl_sys::_LV_COLOR_MAKE(180, 180, 185),
                0,
            );
            lvgl_sys::lv_obj_set_pos(
                page_label,
                Self::to_coord(width / 2 - 34),
                Self::to_coord(10),
            );

            let clock_label = lvgl_sys::lv_label_create(root);
            lvgl_sys::lv_obj_set_style_text_color(
                clock_label,
                lvgl_sys::_LV_COLOR_MAKE(180, 180, 185),
                0,
            );
            lvgl_sys::lv_obj_set_pos(clock_label, Self::to_coord(width - 90), Self::to_coord(10));

            let launcher_panel = lvgl_sys::lv_obj_create(root);
            lvgl_sys::lv_obj_set_pos(
                launcher_panel,
                Self::to_coord(0),
                Self::to_coord(TOP_BAR_HEIGHT),
            );
            lvgl_sys::lv_obj_set_size(
                launcher_panel,
                Self::to_coord(width),
                Self::to_coord(height - TOP_BAR_HEIGHT),
            );
            lvgl_sys::lv_obj_set_style_bg_color(
                launcher_panel,
                lvgl_sys::_LV_COLOR_MAKE(30, 30, 32),
                0,
            );
            lvgl_sys::lv_obj_set_style_border_width(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_radius(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_top(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_bottom(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_left(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_right(launcher_panel, 0, 0);
            lvgl_sys::lv_obj_clear_flag(launcher_panel, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_scrollbar_mode(
                launcher_panel,
                lvgl_sys::LV_SCROLLBAR_MODE_OFF as lvgl_sys::lv_scrollbar_mode_t,
            );

            let branding_label = lvgl_sys::lv_label_create(launcher_panel);
            Self::set_label_text(branding_label, "LinTX");
            lvgl_sys::lv_obj_set_style_text_color(
                branding_label,
                lvgl_sys::lv_color_t { full: 0xFFFF },
                0,
            );
            lvgl_sys::lv_obj_set_style_text_font(
                branding_label,
                &lvgl_sys::lv_font_montserrat_48 as *const _ as *const lvgl_sys::lv_font_t,
                0,
            );
            lvgl_sys::lv_obj_align(
                branding_label,
                lvgl_sys::LV_ALIGN_TOP_MID as lvgl_sys::lv_align_t,
                0,
                60,
            );

            let mut app_cards = [std::ptr::null_mut(); 8];
            let mut app_icon_boxes = [std::ptr::null_mut(); 8];
            let mut app_icon_labels = [std::ptr::null_mut(); 8];
            let mut app_title_labels = [std::ptr::null_mut(); 8];

            for i in 0..8 {
                let card = lvgl_sys::lv_obj_create(launcher_panel);
                lvgl_sys::lv_obj_set_style_bg_opa(card, 0, 0);
                lvgl_sys::lv_obj_set_style_border_width(card, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_top(card, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_bottom(card, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_left(card, 0, 0);
                lvgl_sys::lv_obj_set_style_pad_right(card, 0, 0);
                lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                let icon_box = lvgl_sys::lv_obj_create(card);
                lvgl_sys::lv_obj_set_style_radius(icon_box, 16, 0);
                lvgl_sys::lv_obj_set_style_border_width(icon_box, 0, 0);
                lvgl_sys::lv_obj_clear_flag(icon_box, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);

                let icon_label = lvgl_sys::lv_label_create(icon_box);
                lvgl_sys::lv_obj_set_style_text_color(
                    icon_label,
                    lvgl_sys::_LV_COLOR_MAKE(255, 255, 255),
                    0,
                );
                lvgl_sys::lv_obj_align(
                    icon_label,
                    lvgl_sys::LV_ALIGN_CENTER as lvgl_sys::lv_align_t,
                    0,
                    0,
                );

                let title_label = lvgl_sys::lv_label_create(card);
                lvgl_sys::lv_obj_set_style_text_color(
                    title_label,
                    lvgl_sys::_LV_COLOR_MAKE(220, 220, 220),
                    0,
                );
                lvgl_sys::lv_obj_set_style_text_align(
                    title_label,
                    lvgl_sys::LV_TEXT_ALIGN_CENTER as lvgl_sys::lv_text_align_t,
                    0,
                );

                app_cards[i] = card;
                app_icon_boxes[i] = icon_box;
                app_icon_labels[i] = icon_label;
                app_title_labels[i] = title_label;
            }

            let app_panel = lvgl_sys::lv_obj_create(root);
            lvgl_sys::lv_obj_set_pos(
                app_panel,
                Self::to_coord(width + 20),
                Self::to_coord(TOP_BAR_HEIGHT),
            );
            lvgl_sys::lv_obj_set_size(
                app_panel,
                Self::to_coord(width),
                Self::to_coord(height - TOP_BAR_HEIGHT),
            );
            lvgl_sys::lv_obj_set_style_bg_color(app_panel, lvgl_sys::_LV_COLOR_MAKE(22, 24, 28), 0);
            lvgl_sys::lv_obj_set_style_border_width(app_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_top(app_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_bottom(app_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_left(app_panel, 0, 0);
            lvgl_sys::lv_obj_set_style_pad_right(app_panel, 0, 0);
            lvgl_sys::lv_obj_clear_flag(app_panel, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_scrollbar_mode(
                app_panel,
                lvgl_sys::LV_SCROLLBAR_MODE_OFF as lvgl_sys::lv_scrollbar_mode_t,
            );

            let app_header_card = lvgl_sys::lv_obj_create(app_panel);
            lvgl_sys::lv_obj_set_pos(app_header_card, Self::to_coord(14), Self::to_coord(14));
            lvgl_sys::lv_obj_set_size(
                app_header_card,
                Self::to_coord(width - 28),
                Self::to_coord(92),
            );
            lvgl_sys::lv_obj_set_style_bg_color(
                app_header_card,
                lvgl_sys::_LV_COLOR_MAKE(45, 62, 92),
                0,
            );
            lvgl_sys::lv_obj_set_style_bg_opa(app_header_card, 240, 0);
            lvgl_sys::lv_obj_set_style_radius(app_header_card, 16, 0);
            lvgl_sys::lv_obj_set_style_border_width(app_header_card, 0, 0);
            lvgl_sys::lv_obj_clear_flag(app_header_card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
            lvgl_sys::lv_obj_set_scrollbar_mode(
                app_header_card,
                lvgl_sys::LV_SCROLLBAR_MODE_OFF as lvgl_sys::lv_scrollbar_mode_t,
            );

            let app_badge_label = lvgl_sys::lv_label_create(app_header_card);
            lvgl_sys::lv_obj_set_style_text_color(
                app_badge_label,
                lvgl_sys::_LV_COLOR_MAKE(228, 236, 255),
                0,
            );
            lvgl_sys::lv_obj_set_style_text_font(
                app_badge_label,
                &lvgl_sys::lv_font_montserrat_14 as *const _ as *const lvgl_sys::lv_font_t,
                0,
            );
            lvgl_sys::lv_obj_set_pos(app_badge_label, Self::to_coord(14), Self::to_coord(6));

            let app_title_label = lvgl_sys::lv_label_create(app_header_card);
            lvgl_sys::lv_obj_set_style_text_color(
                app_title_label,
                lvgl_sys::_LV_COLOR_MAKE(255, 255, 255),
                0,
            );
            lvgl_sys::lv_obj_set_style_text_font(
                app_title_label,
                &lvgl_sys::lv_font_montserrat_20 as *const _ as *const lvgl_sys::lv_font_t,
                0,
            );
            lvgl_sys::lv_obj_set_pos(app_title_label, Self::to_coord(14), Self::to_coord(22));

            let app_subtitle_label = lvgl_sys::lv_label_create(app_header_card);
            lvgl_sys::lv_obj_set_style_text_color(
                app_subtitle_label,
                lvgl_sys::_LV_COLOR_MAKE(200, 200, 200),
                0,
            );
            lvgl_sys::lv_obj_set_pos(app_subtitle_label, Self::to_coord(14), Self::to_coord(50));
            lvgl_sys::lv_obj_set_width(app_subtitle_label, Self::to_coord(width - 56));

            let mut app_metric_cards = [std::ptr::null_mut(); 2];
            let mut app_metric_titles = [std::ptr::null_mut(); 2];
            let mut app_metric_values = [std::ptr::null_mut(); 2];
            let mut app_metric_bars = [std::ptr::null_mut(); 2];

            for i in 0..2 {
                let card = lvgl_sys::lv_obj_create(app_panel);
                let card_w = ((width - 42) / 2).max(120);
                let x = 14 + i as i32 * (card_w + 14);
                lvgl_sys::lv_obj_set_pos(card, Self::to_coord(x), Self::to_coord(122));
                lvgl_sys::lv_obj_set_size(card, Self::to_coord(card_w), Self::to_coord(110));
                lvgl_sys::lv_obj_set_style_radius(card, 14, 0);
                lvgl_sys::lv_obj_set_style_bg_color(card, lvgl_sys::_LV_COLOR_MAKE(34, 36, 42), 0);
                lvgl_sys::lv_obj_set_style_bg_opa(card, 255, 0);
                lvgl_sys::lv_obj_set_style_border_width(card, 0, 0);
                lvgl_sys::lv_obj_clear_flag(card, lvgl_sys::LV_OBJ_FLAG_SCROLLABLE);
                lvgl_sys::lv_obj_set_scrollbar_mode(
                    card,
                    lvgl_sys::LV_SCROLLBAR_MODE_OFF as lvgl_sys::lv_scrollbar_mode_t,
                );

                let title = lvgl_sys::lv_label_create(card);
                lvgl_sys::lv_obj_set_style_text_color(
                    title,
                    lvgl_sys::_LV_COLOR_MAKE(168, 176, 188),
                    0,
                );
                lvgl_sys::lv_obj_set_pos(title, Self::to_coord(10), Self::to_coord(10));

                let value = lvgl_sys::lv_label_create(card);
                lvgl_sys::lv_obj_set_style_text_color(
                    value,
                    lvgl_sys::_LV_COLOR_MAKE(255, 255, 255),
                    0,
                );
                lvgl_sys::lv_obj_set_style_text_font(
                    value,
                    &lvgl_sys::lv_font_montserrat_20 as *const _ as *const lvgl_sys::lv_font_t,
                    0,
                );
                lvgl_sys::lv_obj_set_pos(value, Self::to_coord(10), Self::to_coord(36));

                let bar = lvgl_sys::lv_bar_create(card);
                lvgl_sys::lv_obj_set_pos(bar, Self::to_coord(10), Self::to_coord(82));
                lvgl_sys::lv_obj_set_size(bar, Self::to_coord(card_w - 20), Self::to_coord(12));
                lvgl_sys::lv_bar_set_range(bar, 0, 100);
                lvgl_sys::lv_bar_set_value(bar, 0, lvgl_sys::lv_anim_enable_t_LV_ANIM_OFF);
                lvgl_sys::lv_obj_set_style_bg_color(bar, lvgl_sys::_LV_COLOR_MAKE(56, 58, 64), 0);
                lvgl_sys::lv_obj_set_style_bg_color(
                    bar,
                    lvgl_sys::_LV_COLOR_MAKE(120, 196, 255),
                    lvgl_sys::LV_PART_INDICATOR,
                );
                lvgl_sys::lv_obj_set_style_radius(bar, 6, 0);

                app_metric_cards[i] = card;
                app_metric_titles[i] = title;
                app_metric_values[i] = value;
                app_metric_bars[i] = bar;
            }

            let app_list_title = lvgl_sys::lv_label_create(app_panel);
            lvgl_sys::lv_obj_set_style_text_color(
                app_list_title,
                lvgl_sys::_LV_COLOR_MAKE(174, 182, 196),
                0,
            );
            lvgl_sys::lv_obj_set_pos(app_list_title, Self::to_coord(14), Self::to_coord(248));

            let mut app_list_lines = [std::ptr::null_mut(); 4];
            for (i, line) in app_list_lines.iter_mut().enumerate() {
                let row = lvgl_sys::lv_label_create(app_panel);
                lvgl_sys::lv_obj_set_style_text_color(
                    row,
                    lvgl_sys::_LV_COLOR_MAKE(228, 232, 238),
                    0,
                );
                lvgl_sys::lv_obj_set_pos(
                    row,
                    Self::to_coord(14),
                    Self::to_coord(274 + i as i32 * 28),
                );
                lvgl_sys::lv_obj_set_width(row, Self::to_coord(width - 28));
                *line = row;
            }

            let app_hint_label = lvgl_sys::lv_label_create(app_panel);
            lvgl_sys::lv_obj_set_style_text_color(
                app_hint_label,
                lvgl_sys::_LV_COLOR_MAKE(170, 174, 182),
                0,
            );
            lvgl_sys::lv_obj_set_pos(
                app_hint_label,
                Self::to_coord(14),
                Self::to_coord(height - TOP_BAR_HEIGHT - 34),
            );
            lvgl_sys::lv_obj_set_width(app_hint_label, Self::to_coord(width - 28));

            self.ui = Some(LvglUiObjects {
                status_label,
                clock_label,
                page_label,
                launcher_panel,
                app_panel,
                app_header_card,
                app_badge_label,
                app_title_label,
                app_subtitle_label,
                app_metric_cards,
                app_metric_titles,
                app_metric_values,
                app_metric_bars,
                app_list_title,
                app_list_lines,
                app_hint_label,
                branding_label,
                app_cards,
                app_icon_boxes,
                app_icon_labels,
                app_title_labels,
            });
        }
    }

    fn update_launcher(&self, frame: &UiFrame, ui: &LvglUiObjects) {
        let p = page(frame.launcher_page);
        let panel_h = (self.height as i32 - TOP_BAR_HEIGHT - 20).max(120);
        let panel_w = self.width as i32 - 40;

        let is_home = frame.launcher_page == 0;

        unsafe {
            if is_home {
                lvgl_sys::lv_obj_clear_flag(ui.branding_label, lvgl_sys::LV_OBJ_FLAG_HIDDEN);
            } else {
                lvgl_sys::lv_obj_add_flag(ui.branding_label, lvgl_sys::LV_OBJ_FLAG_HIDDEN);
            }
        }

        let col_gap = 20;
        let row_gap = 25;
        let cols = p.cols.max(1) as i32;

        let cell_w = (panel_w - (cols - 1) * col_gap) / cols;
        let cell_h = 140;

        for idx in 0..8 {
            let row = idx / 4;
            let col = idx % 4;

            let card = ui.app_cards[idx];
            let icon_box = ui.app_icon_boxes[idx];
            let title_label = ui.app_title_labels[idx];
            let icon_label = ui.app_icon_labels[idx];

            if row < p.rows {
                if let Some(app) = app_at(frame.launcher_page, row, col) {
                    let spec = app_spec(app);
                    let is_selected = row == frame.selected_row && col == frame.selected_col;

                    let x = 20 + col as i32 * (cell_w + col_gap);
                    let mut y = 20 + row as i32 * (cell_h + row_gap);

                    if is_home {
                        y = panel_h - cell_h - 40;
                    }

                    // App iconography style
                    let mut icon_size = (cell_h as i32 - 25).min(cell_w as i32).min(80);
                    if is_selected {
                        icon_size += 14;
                    }

                    unsafe {
                        lvgl_sys::lv_obj_set_pos(card, Self::to_coord(x), Self::to_coord(y));
                        lvgl_sys::lv_obj_set_size(
                            card,
                            Self::to_coord(cell_w),
                            Self::to_coord(cell_h),
                        );

                        // Icon box
                        lvgl_sys::lv_obj_set_size(
                            icon_box,
                            Self::to_coord(icon_size),
                            Self::to_coord(icon_size),
                        );
                        lvgl_sys::lv_obj_align(
                            icon_box,
                            lvgl_sys::LV_ALIGN_TOP_MID as lvgl_sys::lv_align_t,
                            0,
                            0,
                        );
                        lvgl_sys::lv_obj_set_style_bg_color(
                            icon_box,
                            lvgl_sys::_LV_COLOR_MAKE(spec.accent.0, spec.accent.1, spec.accent.2),
                            0,
                        );
                        lvgl_sys::lv_obj_set_style_bg_opa(icon_box, 255, 0);

                        // Title & Text Colors - Opaque White (0xFFFF)
                        lvgl_sys::lv_obj_set_style_text_color(
                            title_label,
                            lvgl_sys::lv_color_t { full: 0xFFFF },
                            0,
                        );
                        lvgl_sys::lv_obj_align(
                            title_label,
                            lvgl_sys::LV_ALIGN_BOTTOM_MID as lvgl_sys::lv_align_t,
                            0,
                            0,
                        );
                        Self::set_label_text(title_label, spec.title);
                        Self::set_label_text(icon_label, spec.icon_text);

                        // Selection effect - White Border and Larger Font
                        if is_selected {
                            lvgl_sys::lv_obj_set_style_border_width(icon_box, 4, 0);
                            lvgl_sys::lv_obj_set_style_border_color(
                                icon_box,
                                lvgl_sys::lv_color_t { full: 0xFFFF },
                                0,
                            );
                            lvgl_sys::lv_obj_set_style_border_opa(icon_box, 255, 0);
                            lvgl_sys::lv_obj_set_style_text_font(
                                title_label,
                                &lvgl_sys::lv_font_montserrat_20 as *const _
                                    as *const lvgl_sys::lv_font_t,
                                0,
                            );
                        } else {
                            lvgl_sys::lv_obj_set_style_border_width(icon_box, 0, 0);
                            lvgl_sys::lv_obj_set_style_outline_width(icon_box, 0, 0);
                            lvgl_sys::lv_obj_set_style_text_font(
                                title_label,
                                &lvgl_sys::lv_font_montserrat_14 as *const _
                                    as *const lvgl_sys::lv_font_t,
                                0,
                            );
                        }
                    }
                    continue;
                }
            }

            unsafe {
                lvgl_sys::lv_obj_set_pos(
                    card,
                    Self::to_coord(self.width as i32 + 100),
                    Self::to_coord(self.height as i32 + 100),
                );
            }
        }
    }

    fn update_app_page(&self, frame: &UiFrame, ui: &LvglUiObjects, app: AppId) {
        let data = self.app_template_data(frame, app);

        unsafe {
            lvgl_sys::lv_obj_set_style_bg_color(
                ui.app_header_card,
                lvgl_sys::_LV_COLOR_MAKE(data.accent.0, data.accent.1, data.accent.2),
                0,
            );

            for card in ui.app_metric_cards {
                lvgl_sys::lv_obj_set_style_border_width(card, 1, 0);
                lvgl_sys::lv_obj_set_style_border_color(
                    card,
                    lvgl_sys::_LV_COLOR_MAKE(
                        data.accent.0 / 2,
                        data.accent.1 / 2,
                        data.accent.2 / 2,
                    ),
                    0,
                );
            }
            for bar in ui.app_metric_bars {
                lvgl_sys::lv_obj_set_style_bg_color(
                    bar,
                    lvgl_sys::_LV_COLOR_MAKE(data.accent.0, data.accent.1, data.accent.2),
                    lvgl_sys::LV_PART_INDICATOR,
                );
            }
        }

        Self::set_label_text(ui.app_badge_label, &data.badge);
        Self::set_label_text(ui.app_title_label, &data.title);
        Self::set_label_text(ui.app_subtitle_label, &data.subtitle);

        for i in 0..2 {
            Self::set_label_text(ui.app_metric_titles[i], &data.metric_titles[i]);
            Self::set_label_text(ui.app_metric_values[i], &data.metric_values[i]);
            unsafe {
                lvgl_sys::lv_bar_set_value(
                    ui.app_metric_bars[i],
                    data.metric_progress[i].into(),
                    lvgl_sys::lv_anim_enable_t_LV_ANIM_OFF,
                );
            }
        }

        Self::set_label_text(ui.app_list_title, &data.list_title);
        for i in 0..4 {
            Self::set_label_text(ui.app_list_lines[i], &data.list_lines[i]);
        }
        Self::set_label_text(ui.app_hint_label, &data.hint);
    }

    fn sync_ui(&mut self, frame: &UiFrame) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };

        let status = format!(
            "R {}%  A {}%  S {}%",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
        );
        Self::set_label_text(ui.status_label, &status);

        let secs = frame.status.unix_time_secs % 86400;
        let clock = format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        );
        Self::set_label_text(ui.clock_label, &clock);

        let page_txt = format!("Page {}/{}", frame.launcher_page + 1, PAGE_SPECS.len());
        Self::set_label_text(ui.page_label, &page_txt);

        match frame.page {
            UiPage::Launcher => {
                unsafe {
                    lvgl_sys::lv_obj_set_pos(
                        ui.launcher_panel,
                        Self::to_coord(0),
                        Self::to_coord(TOP_BAR_HEIGHT),
                    );
                    lvgl_sys::lv_obj_set_pos(
                        ui.app_panel,
                        Self::to_coord(self.width as i32 + 20),
                        Self::to_coord(TOP_BAR_HEIGHT),
                    );
                }
                self.update_launcher(frame, ui);
            }
            UiPage::App(app) => {
                unsafe {
                    lvgl_sys::lv_obj_set_pos(
                        ui.launcher_panel,
                        Self::to_coord(self.width as i32 + 20),
                        Self::to_coord(TOP_BAR_HEIGHT),
                    );
                    lvgl_sys::lv_obj_set_pos(
                        ui.app_panel,
                        Self::to_coord(0),
                        Self::to_coord(TOP_BAR_HEIGHT),
                    );
                }
                self.update_app_page(frame, ui, app);
            }
        }
    }

    fn blit_refresh<const N: usize>(
        framebuffer: &std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
        width: u32,
        height: u32,
        refresh: &lvgl::DisplayRefresh<N>,
    ) {
        let area = &refresh.area;
        if area.x2 < area.x1 || area.y2 < area.y1 {
            return;
        }

        let x1 = i32::from(area.x1);
        let y1 = i32::from(area.y1);
        let area_w = (i32::from(area.x2) - x1 + 1) as usize;
        let area_h = (i32::from(area.y2) - y1 + 1) as usize;
        let width_i32 = width as i32;
        let height_i32 = height as i32;

        let mut fb = framebuffer.borrow_mut();
        let mut idx = 0usize;

        for row in 0..area_h {
            let y = y1 + row as i32;
            for col in 0..area_w {
                if idx >= refresh.colors.len() {
                    return;
                }

                let x = x1 + col as i32;
                let color = refresh.colors[idx];
                idx += 1;

                if x < 0 || y < 0 || x >= width_i32 || y >= height_i32 {
                    continue;
                }

                let color_raw: lvgl_sys::lv_color_t = color.into();
                let full = unsafe { color_raw.full };
                // Manual RGB565 unpacking (assuming standard RGB565 in LVGL)
                let r5 = ((full >> 11) & 0x1F) as u8;
                let g6 = ((full >> 5) & 0x3F) as u8;
                let b5 = (full & 0x1F) as u8;

                let r8 = (r5 << 3) | (r5 >> 2);
                let g8 = (g6 << 2) | (g6 >> 4);
                let b8 = (b5 << 3) | (b5 >> 2);

                let offset = ((y as usize * width as usize) + x as usize) * 3;
                fb[offset] = r8;
                fb[offset + 1] = g8;
                fb[offset + 2] = b8;
            }
        }
    }
}

#[cfg(feature = "sdl_ui")]
impl LvglBackend for SdlBackend {
    fn init(&mut self) {
        super::debug_log(&format!(
            "SdlBackend::init begin size={}x{}",
            self.width, self.height
        ));
        unsafe {
            // UI thread can be force-cancelled by server on client disconnect.
            // Reset LVGL to a known-good state before registering a fresh display.
            lvgl_sys::lv_deinit();
            lvgl_sys::lv_init();
        }
        let sdl_ctx = sdl2::init().expect("failed to init sdl");
        super::debug_log("SdlBackend::init sdl2::init ok");
        let video = sdl_ctx.video().expect("failed to init sdl video");
        super::debug_log("SdlBackend::init sdl video ok");
        let render_mode = Self::select_render_mode();
        super::debug_log(&format!(
            "SdlBackend::init renderer mode={:?} (override with LINTX_SDL_RENDERER=software|accelerated)",
            render_mode
        ));

        let create_window = || {
            video
                .window("LinTX LVGL", self.width, self.height)
                .position_centered()
                .resizable()
                .build()
                .expect("failed to create window")
        };

        let window = create_window();
        super::debug_log("SdlBackend::init window ok");
        let canvas = match render_mode {
            SdlRenderMode::Software => {
                super::debug_log("SdlBackend::init creating software canvas");
                window
                    .into_canvas()
                    .software()
                    .build()
                    .expect("failed to create software canvas")
            }
            SdlRenderMode::Accelerated => {
                super::debug_log("SdlBackend::init creating accelerated canvas");
                match window.into_canvas().accelerated().present_vsync().build() {
                    Ok(c) => c,
                    Err(err) => {
                        super::debug_log(&format!(
                            "SdlBackend::init accelerated canvas failed: {}; retry with software canvas",
                            err
                        ));
                        create_window()
                            .into_canvas()
                            .software()
                            .build()
                            .expect("failed to create software canvas after accelerated fallback")
                    }
                }
            }
        };
        super::debug_log("SdlBackend::init canvas ok");
        let event_pump = sdl_ctx.event_pump().expect("failed to get event pump");
        super::debug_log("SdlBackend::init event pump ok");

        self.sdl_ctx = Some(sdl_ctx);
        self.canvas = Some(canvas);
        self.event_pump = Some(event_pump);

        let draw_buf = lvgl::DrawBuffer::<LVGL_DRAW_BUF_PIXELS>::default();
        let framebuffer = std::rc::Rc::clone(&self.framebuffer);
        let width = self.width;
        let height = self.height;

        let display = lvgl::Display::register(draw_buf, self.width, self.height, move |refresh| {
            Self::blit_refresh(&framebuffer, width, height, refresh);
        })
        .expect("failed to register lvgl display");
        super::debug_log("SdlBackend::init lvgl display registered");

        self.display = Some(display);
        Self::set_latest_display_default();
        self.build_ui();
        self.last_tick = std::time::Instant::now();
        super::debug_log("SdlBackend::init done");
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;

        let event_pump = self.event_pump.as_mut()?;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Some(UiInputEvent::Quit),
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => return Some(UiInputEvent::Quit),
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Some(UiInputEvent::Back),
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    ..
                } => return Some(UiInputEvent::Open),
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => return Some(UiInputEvent::Left),
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => return Some(UiInputEvent::Right),
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => return Some(UiInputEvent::Up),
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => return Some(UiInputEvent::Down),
                Event::KeyDown {
                    keycode: Some(Keycode::LeftBracket),
                    ..
                } => return Some(UiInputEvent::PagePrev),
                Event::KeyDown {
                    keycode: Some(Keycode::RightBracket),
                    ..
                } => return Some(UiInputEvent::PageNext),
                _ => {}
            }
        }
        None
    }

    fn render(&mut self, frame: &UiFrame) {
        self.sync_ui(frame);

        let now = std::time::Instant::now();
        let tick_ms = now
            .saturating_duration_since(self.last_tick)
            .as_millis()
            .clamp(1, 1000) as u32;
        self.last_tick = now;

        lvgl::tick_inc(std::time::Duration::from_millis(tick_ms as u64));
        lvgl::task_handler();

        let Some(canvas) = self.canvas.as_mut() else {
            return;
        };

        use sdl2::{pixels::PixelFormatEnum, rect::Rect};

        let texture_creator = canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, self.width, self.height)
            .expect("failed to create texture");

        {
            let fb = self.framebuffer.borrow();
            texture
                .update(None, &fb, self.width as usize * 3)
                .expect("failed to upload frame");
        }

        let (ow, oh) = canvas.output_size().unwrap_or((self.width, self.height));
        canvas.clear();
        let _ = canvas.copy(&texture, None, Rect::new(0, 0, ow, oh));
        canvas.present();
    }

    fn shutdown(&mut self) {
        self.ui = None;
        self.display = None;
        self.event_pump = None;
        self.canvas = None;
        self.sdl_ctx = None;
        self.framebuffer.borrow_mut().fill(0);
    }
}

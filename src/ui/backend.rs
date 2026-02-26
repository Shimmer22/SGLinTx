use std::io::Write;

use super::{
    catalog::{app_at, app_spec, page, PAGE_SPECS},
    input::UiInputEvent,
    model::{UiFrame, UiPage},
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
                println!("(placeholder page)");
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
    app_title_label: *mut lvgl_sys::lv_obj_t,
    app_status_label: *mut lvgl_sys::lv_obj_t,
    app_buttons: [*mut lvgl_sys::lv_obj_t; 8],
    app_labels: [*mut lvgl_sys::lv_obj_t; 8],
}

#[cfg(feature = "sdl_ui")]
struct SdlBackend {
    width: u32,
    height: u32,
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
            canvas: None,
            event_pump: None,
            framebuffer: std::rc::Rc::new(std::cell::RefCell::new(vec![0; fb_size])),
            display: None,
            ui: None,
            last_tick: std::time::Instant::now(),
        }
    }

    fn is_wsl() -> bool {
        if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some()
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

    fn build_ui(&mut self) {
        let width = self.width as i32;
        let height = self.height as i32;

        unsafe {
            let root = lvgl_sys::lv_disp_get_scr_act(std::ptr::null_mut());
            lvgl_sys::lv_obj_clean(root);

            let status_label = lvgl_sys::lv_label_create(root);
            lvgl_sys::lv_obj_set_pos(status_label, Self::to_coord(8), Self::to_coord(10));

            let page_label = lvgl_sys::lv_label_create(root);
            lvgl_sys::lv_obj_set_pos(
                page_label,
                Self::to_coord(width / 2 - 34),
                Self::to_coord(10),
            );

            let clock_label = lvgl_sys::lv_label_create(root);
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

            let mut app_buttons = [std::ptr::null_mut(); 8];
            let mut app_labels = [std::ptr::null_mut(); 8];
            for i in 0..8 {
                let btn = lvgl_sys::lv_btn_create(launcher_panel);
                let label = lvgl_sys::lv_label_create(btn);
                lvgl_sys::lv_obj_align(
                    label,
                    lvgl_sys::LV_ALIGN_CENTER as lvgl_sys::lv_align_t,
                    Self::to_coord(0),
                    Self::to_coord(0),
                );
                app_buttons[i] = btn;
                app_labels[i] = label;
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

            let app_title_label = lvgl_sys::lv_label_create(app_panel);
            lvgl_sys::lv_obj_set_pos(app_title_label, Self::to_coord(12), Self::to_coord(10));

            let app_status_label = lvgl_sys::lv_label_create(app_panel);
            lvgl_sys::lv_obj_set_pos(app_status_label, Self::to_coord(12), Self::to_coord(56));
            lvgl_sys::lv_obj_set_width(app_status_label, Self::to_coord(width - 24));

            self.ui = Some(LvglUiObjects {
                status_label,
                clock_label,
                page_label,
                launcher_panel,
                app_panel,
                app_title_label,
                app_status_label,
                app_buttons,
                app_labels,
            });
        }
    }

    fn update_launcher(&self, frame: &UiFrame, ui: &LvglUiObjects) {
        let p = page(frame.launcher_page);
        let panel_h = (self.height as i32 - TOP_BAR_HEIGHT - 8).max(120);

        let col_gap = 10;
        let row_gap = 10;
        let cols = p.cols.max(1) as i32;
        let rows = p.rows.max(1) as i32;

        let cell_w = (self.width as i32 - 24 - (cols - 1) * col_gap) / cols;
        let cell_h = (panel_h - 16 - (rows - 1) * row_gap) / rows;

        for idx in 0..8 {
            let row = idx / 4;
            let col = idx % 4;

            let btn = ui.app_buttons[idx];
            let label = ui.app_labels[idx];

            if row < p.rows {
                if let Some(app) = app_at(frame.launcher_page, row, col) {
                    let x = 12 + col as i32 * (cell_w + col_gap);
                    let y = 8 + row as i32 * (cell_h + row_gap);
                    let w = (cell_w - 2).max(52);
                    let h = (cell_h - 2).max(42);

                    unsafe {
                        lvgl_sys::lv_obj_set_pos(btn, Self::to_coord(x), Self::to_coord(y));
                        lvgl_sys::lv_obj_set_size(btn, Self::to_coord(w), Self::to_coord(h));
                    }

                    let spec = app_spec(app);
                    let title = if row == frame.selected_row && col == frame.selected_col {
                        format!("> {}", spec.title)
                    } else {
                        spec.title.to_string()
                    };
                    Self::set_label_text(label, &title);
                    continue;
                }
            }

            unsafe {
                lvgl_sys::lv_obj_set_pos(
                    btn,
                    Self::to_coord(self.width as i32 + 50),
                    Self::to_coord(self.height as i32 + 50),
                );
            }
            Self::set_label_text(label, "");
        }
    }

    fn update_app_page(&self, frame: &UiFrame, ui: &LvglUiObjects, app: super::model::AppId) {
        let app_name = app_spec(app).title;
        let title = format!("{} APP", app_name);
        Self::set_label_text(ui.app_title_label, &title);

        let detail = format!(
            "Remote Battery: {}%\nAircraft Battery: {}%\nSignal: {}%\nBacklight: {}%\nSound: {}%\n\nEsc Back",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent,
            frame.config.backlight_percent,
            frame.config.sound_percent,
        );
        Self::set_label_text(ui.app_status_label, &detail);
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

                let offset = ((y as usize * width as usize) + x as usize) * 3;
                fb[offset] = color.r();
                fb[offset + 1] = color.g();
                fb[offset + 2] = color.b();
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

    fn shutdown(&mut self) {}
}

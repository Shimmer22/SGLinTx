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
                println!("                LinTX  [Page {}/{}]", p.id + 1, PAGE_SPECS.len());
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
        BackendKind::PcApi => Box::new(TerminalBackend::new("pc-api".to_string())),
        BackendKind::PcSdl { width, height } => {
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
            Box::new(TerminalBackend::new(format!("fbdev:{}", device)))
        }
    }
}

#[cfg(feature = "sdl_ui")]
struct SdlBackend {
    width: u32,
    height: u32,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
    event_pump: Option<sdl2::EventPump>,
}

#[cfg(feature = "sdl_ui")]
impl SdlBackend {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            canvas: None,
            event_pump: None,
        }
    }

    fn glyph(ch: char) -> [u8; 7] {
        match ch {
            'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
            'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
            'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
            'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
            'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
            'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
            'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
            'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
            'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
            'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
            'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
            'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
            'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
            'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
            'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
            'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
            'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
            'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
            '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
            '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
            '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
            '3' => [0x1E, 0x01, 0x01, 0x06, 0x01, 0x01, 0x1E],
            '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
            '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
            '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
            '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
            '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
            '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
            ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
            '%' => [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13],
            '[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
            ']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
            '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
            '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
            ' ' => [0x00; 7],
            _ => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x00, 0x08],
        }
    }

    fn measure_text(text: &str, scale: i32) -> i32 {
        let chars = text.chars().count() as i32;
        chars * (5 * scale + scale)
    }

    fn draw_text(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        x: i32,
        y: i32,
        text: &str,
        scale: i32,
        color: sdl2::pixels::Color,
    ) {
        use sdl2::rect::Rect;
        canvas.set_draw_color(color);
        let mut cx = x;
        for ch in text.chars() {
            let g = Self::glyph(ch.to_ascii_uppercase());
            for (row, bits) in g.iter().enumerate() {
                for col in 0..5 {
                    if (bits >> (4 - col)) & 1 == 1 {
                        let _ = canvas.fill_rect(Rect::new(
                            cx + col * scale,
                            y + row as i32 * scale,
                            scale as u32,
                            scale as u32,
                        ));
                    }
                }
            }
            cx += 5 * scale + scale;
        }
    }

    fn draw_bar(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        percent: u8,
        color: sdl2::pixels::Color,
    ) {
        use sdl2::{pixels::Color, rect::Rect};
        canvas.set_draw_color(Color::RGB(52, 58, 70));
        let _ = canvas.fill_rect(Rect::new(x, y, w, h));
        canvas.set_draw_color(color);
        let fill_w = (w as u64 * percent as u64 / 100) as u32;
        let _ = canvas.fill_rect(Rect::new(x, y, fill_w, h));
    }

    fn fill_circle(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        cx: i32,
        cy: i32,
        r: i32,
    ) {
        use sdl2::rect::Rect;
        for dy in -r..=r {
            let dx = ((r * r - dy * dy) as f32).sqrt() as i32;
            let _ = canvas.fill_rect(Rect::new(cx - dx, cy + dy, (dx * 2 + 1) as u32, 1));
        }
    }

    fn fill_rounded_rect(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        radius: i32,
        color: sdl2::pixels::Color,
    ) {
        use sdl2::rect::Rect;
        let r = radius.max(0).min((w.min(h)) / 2);
        canvas.set_draw_color(color);
        let _ = canvas.fill_rect(Rect::new(x + r, y, (w - 2 * r) as u32, h as u32));
        let _ = canvas.fill_rect(Rect::new(x, y + r, r as u32, (h - 2 * r) as u32));
        let _ = canvas.fill_rect(Rect::new(x + w - r, y + r, r as u32, (h - 2 * r) as u32));

        Self::fill_circle(canvas, x + r, y + r, r);
        Self::fill_circle(canvas, x + w - r - 1, y + r, r);
        Self::fill_circle(canvas, x + r, y + h - r - 1, r);
        Self::fill_circle(canvas, x + w - r - 1, y + h - r - 1, r);
    }

    fn draw_status(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, frame: &UiFrame, w: i32) {
        use sdl2::{pixels::Color, rect::Rect};
        canvas.set_draw_color(Color::RGB(23, 27, 34));
        let _ = canvas.fill_rect(Rect::new(0, 0, w as u32, 44));

        let left = format!(
            "R {}%  A {}%  S {}%",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent
        );
        Self::draw_text(canvas, 12, 12, &left, 2, Color::RGB(236, 238, 244));

        let secs = frame.status.unix_time_secs % 86400;
        let right = format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);
        let rw = Self::measure_text(&right, 2);
        Self::draw_text(canvas, w - rw - 12, 12, &right, 2, Color::RGB(236, 238, 244));
    }

    fn draw_launcher(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, frame: &UiFrame, w: i32, h: i32) {
        use sdl2::pixels::Color;

        let title = "LinTX";
        let t_scale = (w / 210).clamp(4, 10);
        let tw = Self::measure_text(title, t_scale);
        Self::draw_text(canvas, (w - tw) / 2, 62, title, t_scale, Color::RGB(245, 247, 255));

        let p = page(frame.launcher_page);
        let page_txt = format!("[{} / {}]", p.id + 1, PAGE_SPECS.len());
        let pw = Self::measure_text(&page_txt, 2);
        Self::draw_text(canvas, (w - pw) / 2, 126, &page_txt, 2, Color::RGB(170, 178, 196));

        let grid_top = 164;
        let grid_bottom = h - 26;
        let grid_h = (grid_bottom - grid_top).max(120);
        let cols = p.cols as i32;
        let rows = p.rows as i32;

        let cell_w = (w - 36 * 2) / cols.max(1);
        let cell_h = grid_h / rows.max(1);

        for r in 0..rows {
            for c in 0..cols {
                let app = match app_at(frame.launcher_page, r as usize, c as usize) {
                    Some(a) => a,
                    None => continue,
                };
                let spec = app_spec(app);
                let (ar, ag, ab) = spec.accent;

                let cx = 36 + c * cell_w + cell_w / 2;
                let cy = grid_top + r * cell_h + cell_h / 2 - 10;
                let icon_w = (cell_w - 36).clamp(76, 110);
                let icon_h = icon_w;
                let ix = cx - icon_w / 2;
                let iy = cy - icon_h / 2;

                let selected = frame.selected_row == r as usize && frame.selected_col == c as usize;

                Self::fill_rounded_rect(
                    canvas,
                    ix,
                    iy,
                    icon_w,
                    icon_h,
                    icon_w / 4,
                    if selected {
                        Color::RGB(ar, ag, ab)
                    } else {
                        Color::RGB((ar as f32 * 0.75) as u8, (ag as f32 * 0.75) as u8, (ab as f32 * 0.75) as u8)
                    },
                );

                if selected {
                    Self::fill_rounded_rect(
                        canvas,
                        ix - 4,
                        iy - 4,
                        icon_w + 8,
                        icon_h + 8,
                        icon_w / 4,
                        Color::RGBA(250, 239, 170, 70),
                    );
                }

                let icon_scale = (icon_w / 46).clamp(2, 4);
                let iw = Self::measure_text(spec.icon_text, icon_scale);
                Self::draw_text(
                    canvas,
                    cx - iw / 2,
                    iy + icon_h / 2 - 10,
                    spec.icon_text,
                    icon_scale,
                    Color::RGB(250, 252, 255),
                );

                let label_scale = (cell_w / 90).clamp(2, 3);
                let lw = Self::measure_text(spec.title, label_scale);
                Self::draw_text(
                    canvas,
                    cx - lw / 2,
                    iy + icon_h + 12,
                    spec.title,
                    label_scale,
                    Color::RGB(224, 228, 236),
                );
            }
        }
    }
}

#[cfg(feature = "sdl_ui")]
impl LvglBackend for SdlBackend {
    fn init(&mut self) {
        let sdl_ctx = sdl2::init().expect("failed to init sdl");
        let video = sdl_ctx.video().expect("failed to init sdl video");
        let window = video
            .window("LinTX", self.width, self.height)
            .position_centered()
            .resizable()
            .build()
            .expect("failed to create window");
        let mut canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .expect("failed to create canvas");
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        let event_pump = sdl_ctx.event_pump().expect("failed to get event pump");
        self.canvas = Some(canvas);
        self.event_pump = Some(event_pump);
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;

        let event_pump = self.event_pump.as_mut()?;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Some(UiInputEvent::Quit),
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => return Some(UiInputEvent::Quit),
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Some(UiInputEvent::Back),
                Event::KeyDown { keycode: Some(Keycode::Return), .. } => return Some(UiInputEvent::Open),
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => return Some(UiInputEvent::Left),
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => return Some(UiInputEvent::Right),
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => return Some(UiInputEvent::Up),
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => return Some(UiInputEvent::Down),
                Event::KeyDown { keycode: Some(Keycode::LeftBracket), .. } => return Some(UiInputEvent::PagePrev),
                Event::KeyDown { keycode: Some(Keycode::RightBracket), .. } => return Some(UiInputEvent::PageNext),
                _ => {}
            }
        }
        None
    }

    fn render(&mut self, frame: &UiFrame) {
        use sdl2::{pixels::Color, rect::Rect};

        let canvas = self.canvas.as_mut().expect("canvas not initialized");
        let (w_u, h_u) = canvas.output_size().unwrap_or((self.width, self.height));
        let w = w_u as i32;
        let h = h_u as i32;

        canvas.set_draw_color(Color::RGB(14, 16, 21));
        canvas.clear();

        canvas.set_draw_color(Color::RGB(22, 25, 32));
        let _ = canvas.fill_rect(Rect::new(0, 44, w as u32, (h - 44) as u32));

        Self::draw_status(canvas, frame, w);

        match frame.page {
            UiPage::Launcher => Self::draw_launcher(canvas, frame, w, h),
            UiPage::App(app) => {
                let name = format!("{} APP", app_spec(app).title);
                let nw = Self::measure_text(&name, 5);
                Self::draw_text(canvas, (w - nw) / 2, 92, &name, 5, Color::RGB(242, 246, 255));

                canvas.set_draw_color(Color::RGB(33, 38, 49));
                let _ = canvas.fill_rect(Rect::new(60, 182, (w - 120) as u32, (h - 250) as u32));

                let tip = "ESC BACK";
                let tw = Self::measure_text(tip, 3);
                Self::draw_text(canvas, (w - tw) / 2, h - 52, tip, 3, Color::RGB(210, 214, 224));

                if app == super::model::AppId::System {
                    Self::draw_text(canvas, 86, 210, "BACKLIGHT", 3, Color::RGB(224, 228, 238));
                    Self::draw_bar(canvas, 86, 242, (w - 172) as u32, 24, frame.config.backlight_percent, Color::RGB(250, 211, 104));
                    Self::draw_text(canvas, 86, 290, "SOUND", 3, Color::RGB(224, 228, 238));
                    Self::draw_bar(canvas, 86, 322, (w - 172) as u32, 24, frame.config.sound_percent, Color::RGB(160, 134, 255));
                }
            }
        }

        canvas.present();
    }

    fn shutdown(&mut self) {}
}

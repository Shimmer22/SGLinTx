use std::io::Write;

use super::{
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
        println!("STATUS | REM:{}% AIR:{}% SIG:{}% T:{}", frame.status.remote_battery_percent, frame.status.aircraft_battery_percent, frame.status.signal_strength_percent, frame.status.unix_time_secs);
        println!("-----------------------------------------------");
        match frame.page {
            UiPage::Launcher => {
                println!("                 LinTX");
                for app in AppId::ALL {
                    let mark = if app == frame.selected_app { ">" } else { " " };
                    println!("{} {}", mark, app.title());
                }
                println!("TAB/ARROWS select, ENTER open, ESC back, Q quit");
            }
            UiPage::App(app) => {
                println!("APP: {}", app.title());
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
        BackendKind::Fbdev { device } => Box::new(TerminalBackend::new(format!("fbdev:{}", device))),
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
            'J' => [0x1F, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
            'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
            'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
            'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
            'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
            'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
            'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
            'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
            'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
            'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
            'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
            'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
            'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
            'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
            'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
            'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
            'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
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
            '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
            '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
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
                        let rx = cx + col * scale;
                        let ry = y + row as i32 * scale;
                        let _ = canvas.fill_rect(Rect::new(rx, ry, scale as u32, scale as u32));
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

    fn draw_status(
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        frame: &UiFrame,
        width: i32,
    ) {
        use sdl2::{pixels::Color, rect::Rect};
        canvas.set_draw_color(Color::RGB(24, 28, 36));
        let _ = canvas.fill_rect(Rect::new(0, 0, width as u32, 44));

        let left = format!(
            "R {}%  A {}%  S {}%",
            frame.status.remote_battery_percent,
            frame.status.aircraft_battery_percent,
            frame.status.signal_strength_percent
        );
        Self::draw_text(canvas, 12, 12, &left, 2, Color::RGB(236, 238, 244));

        let secs = frame.status.unix_time_secs % 86400;
        let hh = secs / 3600;
        let mm = (secs % 3600) / 60;
        let ss = secs % 60;
        let right = format!("{:02}:{:02}:{:02}", hh, mm, ss);
        let text_w = Self::measure_text(&right, 2);
        Self::draw_text(canvas, width - text_w - 12, 12, &right, 2, Color::RGB(236, 238, 244));
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
                    keycode: Some(Keycode::Tab),
                    ..
                }
                | Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                }
                | Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => return Some(UiInputEvent::Next),
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                }
                | Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => return Some(UiInputEvent::Prev),
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

        canvas.set_draw_color(Color::RGB(16, 18, 23));
        canvas.clear();

        Self::draw_status(canvas, frame, w);

        match frame.page {
            UiPage::Launcher => {
                let title = "LinTX";
                let title_scale = (w / 220).clamp(4, 10);
                let tw = Self::measure_text(title, title_scale);
                Self::draw_text(
                    canvas,
                    (w - tw) / 2,
                    64,
                    title,
                    title_scale,
                    Color::RGB(245, 247, 255),
                );

                let grid_top = 170;
                let margin = 28;
                let gap = 24;
                let card_w = ((w - margin * 2 - gap) / 2).max(120);
                let card_h = ((h - grid_top - 24 - gap) / 2).max(90);

                for (idx, app) in AppId::ALL.iter().enumerate() {
                    let col = (idx % 2) as i32;
                    let row = (idx / 2) as i32;
                    let x = margin + col * (card_w + gap);
                    let y = grid_top + row * (card_h + gap);

                    let selected = *app == frame.selected_app;
                    canvas.set_draw_color(if selected {
                        Color::RGB(52, 84, 145)
                    } else {
                        Color::RGB(36, 42, 54)
                    });
                    let _ = canvas.fill_rect(Rect::new(x, y, card_w as u32, card_h as u32));

                    if selected {
                        canvas.set_draw_color(Color::RGB(236, 197, 95));
                        let _ = canvas.draw_rect(Rect::new(x - 2, y - 2, (card_w + 4) as u32, (card_h + 4) as u32));
                    }

                    let title_scale = (card_w / 90).clamp(2, 4);
                    let app_name = app.title();
                    let aw = Self::measure_text(app_name, title_scale);
                    Self::draw_text(
                        canvas,
                        x + (card_w - aw) / 2,
                        y + card_h / 2 - 8,
                        app_name,
                        title_scale,
                        Color::RGB(242, 244, 252),
                    );
                }
            }
            UiPage::App(app) => {
                let head = format!("{} APP", app.title());
                let hw = Self::measure_text(&head, 5);
                Self::draw_text(canvas, (w - hw) / 2, 88, &head, 5, Color::RGB(240, 243, 252));

                canvas.set_draw_color(Color::RGB(33, 38, 49));
                let _ = canvas.fill_rect(Rect::new(60, 180, (w - 120) as u32, (h - 260) as u32));

                let tip = "ESC BACK";
                let tw = Self::measure_text(tip, 3);
                Self::draw_text(canvas, (w - tw) / 2, h - 56, tip, 3, Color::RGB(210, 214, 224));

                if app == AppId::System {
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

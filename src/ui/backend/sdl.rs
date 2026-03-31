use crate::ui::{input::UiInputEvent, model::UiFrame};

use super::{
    lvgl_core::{LvglUiCore, LVGL_DRAW_BUF_PIXELS},
    pointer::PointerInputAdapter,
    LvglBackend,
};

#[derive(Clone, Copy, Debug)]
enum SdlRenderMode {
    Software,
    Accelerated,
}

pub(super) struct SdlBackend {
    core: LvglUiCore,
    sdl_ctx: Option<sdl2::Sdl>,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
    event_pump: Option<sdl2::EventPump>,
    framebuffer: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    pointer: PointerInputAdapter,
}

impl SdlBackend {
    pub(super) fn new(width: u32, height: u32) -> Self {
        let fb_size = width as usize * height as usize * 3;
        Self {
            core: LvglUiCore::new(width, height),
            sdl_ctx: None,
            canvas: None,
            event_pump: None,
            framebuffer: std::rc::Rc::new(std::cell::RefCell::new(vec![0; fb_size])),
            pointer: PointerInputAdapter::default(),
        }
    }

    fn window_to_logical(
        width: u32,
        height: u32,
        window_w: u32,
        window_h: u32,
        x: i32,
        y: i32,
    ) -> (i32, i32) {
        let logical_x = x.saturating_mul(width as i32) / window_w.max(1) as i32;
        let logical_y = y.saturating_mul(height as i32) / window_h.max(1) as i32;
        (
            logical_x.clamp(0, width.saturating_sub(1) as i32),
            logical_y.clamp(0, height.saturating_sub(1) as i32),
        )
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
                let (mut r8, g8, mut b8) = Self::expand_lv_color(color_raw);
                if Self::swap_rb_enabled() {
                    std::mem::swap(&mut r8, &mut b8);
                }

                let offset = ((y as usize * width as usize) + x as usize) * 3;
                fb[offset] = r8;
                fb[offset + 1] = g8;
                fb[offset + 2] = b8;
            }
        }
    }

    fn expand_channel(v: u8, max_in: u16) -> u8 {
        (((v as u16) * 255 + (max_in / 2)) / max_in) as u8
    }

    fn expand_lv_color(color_raw: lvgl_sys::lv_color_t) -> (u8, u8, u8) {
        let r = unsafe { lvgl_sys::_LV_COLOR_GET_R(color_raw) as u8 };
        let g = unsafe { lvgl_sys::_LV_COLOR_GET_G(color_raw) as u8 };
        let b = unsafe { lvgl_sys::_LV_COLOR_GET_B(color_raw) as u8 };
        match lvgl_sys::LV_COLOR_DEPTH {
            16 => (
                Self::expand_channel(r, 31),
                Self::expand_channel(g, 63),
                Self::expand_channel(b, 31),
            ),
            8 => (
                Self::expand_channel(r, 7),
                Self::expand_channel(g, 7),
                Self::expand_channel(b, 3),
            ),
            _ => (r, g, b),
        }
    }

    fn swap_rb_enabled() -> bool {
        std::env::var("LINTX_SDL_SWAP_RB")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
            .unwrap_or(false)
    }
}

impl LvglBackend for SdlBackend {
    fn init(&mut self) {
        super::super::debug_log(&format!(
            "SdlBackend::init begin size={}x{}",
            self.core.width, self.core.height
        ));
        unsafe {
            lvgl_sys::lv_deinit();
            lvgl_sys::lv_init();
        }
        let sdl_ctx = sdl2::init().expect("failed to init sdl");
        super::super::debug_log("SdlBackend::init sdl2::init ok");
        let video = sdl_ctx.video().expect("failed to init sdl video");
        super::super::debug_log("SdlBackend::init sdl video ok");
        let render_mode = Self::select_render_mode();
        super::super::debug_log(&format!(
            "SdlBackend::init renderer mode={:?} (override with LINTX_SDL_RENDERER=software|accelerated)",
            render_mode
        ));

        let create_window = || {
            video
                .window("LinTX LVGL", self.core.width, self.core.height)
                .position_centered()
                .resizable()
                .build()
                .expect("failed to create window")
        };

        let window = create_window();
        super::super::debug_log("SdlBackend::init window ok");
        let canvas = match render_mode {
            SdlRenderMode::Software => {
                super::super::debug_log("SdlBackend::init creating software canvas");
                window
                    .into_canvas()
                    .software()
                    .build()
                    .expect("failed to create software canvas")
            }
            SdlRenderMode::Accelerated => {
                super::super::debug_log("SdlBackend::init creating accelerated canvas");
                match window.into_canvas().accelerated().present_vsync().build() {
                    Ok(c) => c,
                    Err(err) => {
                        super::super::debug_log(&format!(
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
        super::super::debug_log("SdlBackend::init canvas ok");
        let event_pump = sdl_ctx.event_pump().expect("failed to get event pump");
        super::super::debug_log("SdlBackend::init event pump ok");

        self.sdl_ctx = Some(sdl_ctx);
        self.canvas = Some(canvas);
        self.event_pump = Some(event_pump);

        let draw_buf = lvgl::DrawBuffer::<LVGL_DRAW_BUF_PIXELS>::default();
        let framebuffer = std::rc::Rc::clone(&self.framebuffer);
        let width = self.core.width;
        let height = self.core.height;

        let display = lvgl::Display::register(
            draw_buf,
            self.core.width,
            self.core.height,
            move |refresh| {
                Self::blit_refresh(&framebuffer, width, height, refresh);
            },
        )
        .expect("failed to register lvgl display");
        super::super::debug_log("SdlBackend::init lvgl display registered");

        self.core.display = Some(display);
        LvglUiCore::set_latest_display_default();
        self.core.build_ui();
        self.core.last_tick = std::time::Instant::now();
        super::super::debug_log("SdlBackend::init done");
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        use sdl2::event::Event;
        use sdl2::event::WindowEvent;
        use sdl2::keyboard::Keycode;

        if let Some(evt) = self.pointer.pop_event() {
            return Some(evt);
        }

        let (window_w, window_h) = self
            .canvas
            .as_ref()
            .and_then(|canvas| canvas.output_size().ok())
            .unwrap_or((self.core.width, self.core.height));
        let logical_size = (self.core.width, self.core.height);
        let event_pump = self.event_pump.as_mut()?;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Some(UiInputEvent::Quit),
                Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => return Some(UiInputEvent::Quit),
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
                Event::MouseButtonDown {
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    let (x, y) = Self::window_to_logical(
                        logical_size.0,
                        logical_size.1,
                        window_w,
                        window_h,
                        x,
                        y,
                    );
                    self.pointer.begin(x, y);
                }
                Event::MouseMotion {
                    x, y, mousestate, ..
                } if mousestate.left() => {
                    let (x, y) = Self::window_to_logical(
                        logical_size.0,
                        logical_size.1,
                        window_w,
                        window_h,
                        x,
                        y,
                    );
                    self.pointer.update(x, y);
                }
                Event::MouseButtonUp {
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    let (x, y) = Self::window_to_logical(
                        logical_size.0,
                        logical_size.1,
                        window_w,
                        window_h,
                        x,
                        y,
                    );
                    self.pointer.end(x, y);
                    if let Some(evt) = self.pointer.pop_event() {
                        return Some(evt);
                    }
                }
                _ => {}
            }
        }
        self.pointer.pop_event()
    }

    fn render(&mut self, frame: &UiFrame) {
        self.pointer
            .update_snapshot(frame, self.core.width, self.core.height);
        self.core.set_drag_offset(self.pointer.drag_offset_x());
        self.core.sync_ui(frame);
        self.core.start_frame();

        let Some(canvas) = self.canvas.as_mut() else {
            return;
        };

        use sdl2::{pixels::PixelFormatEnum, rect::Rect};

        let texture_creator = canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, self.core.width, self.core.height)
            .expect("failed to create texture");

        {
            let fb = self.framebuffer.borrow();
            texture
                .update(None, &fb, self.core.width as usize * 3)
                .expect("failed to upload frame");
        }

        let (ow, oh) = canvas
            .output_size()
            .unwrap_or((self.core.width, self.core.height));
        canvas.clear();
        let _ = canvas.copy(&texture, None, Rect::new(0, 0, ow, oh));
        canvas.present();
    }

    fn shutdown(&mut self) {
        self.core.ui = None;
        self.core.display = None;
        self.event_pump = None;
        self.canvas = None;
        self.sdl_ctx = None;
        self.framebuffer.borrow_mut().fill(0);
    }
}

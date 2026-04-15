use std::{cell::RefCell, fs::OpenOptions, io, os::fd::AsRawFd, ptr::NonNull, rc::Rc};

use crate::ui::{input::UiInputEvent, model::UiFrame};

use super::{
    lvgl_core::{LvglUiCore, LVGL_DRAW_BUF_PIXELS},
    pointer::PointerInputAdapter,
    LvglBackend,
};

unsafe extern "C" {
    fn lintx_vo_create(
        logical_width: u32,
        logical_height: u32,
        panel_width: u32,
        panel_height: u32,
        rotate_degrees: u16,
    ) -> *mut libc::c_void;
    fn lintx_vo_framebuffer_ptr(handle: *mut libc::c_void) -> *mut u8;
    fn lintx_vo_framebuffer_len(handle: *mut libc::c_void) -> usize;
    fn lintx_vo_framebuffer_stride(handle: *mut libc::c_void) -> u32;
    fn lintx_vo_present(handle: *mut libc::c_void) -> libc::c_int;
    fn lintx_vo_destroy(handle: *mut libc::c_void);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct LinuxInputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct LinuxInputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

struct EvdevAxisCalibration {
    min: i32,
    max: i32,
}

struct EvdevTouchInput {
    file: std::fs::File,
    x_axis: Option<EvdevAxisCalibration>,
    y_axis: Option<EvdevAxisCalibration>,
    rotate_degrees: u16,
    pressed: bool,
    x: i32,
    y: i32,
}

struct MappedRgbSurface {
    ptr: NonNull<u8>,
    len: usize,
    stride: usize,
}

impl MappedRgbSurface {
    unsafe fn bytes_mut(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
    }
}

pub(super) struct VoBackend {
    core: LvglUiCore,
    surface: Option<Rc<RefCell<MappedRgbSurface>>>,
    vo: *mut libc::c_void,
    pointer: PointerInputAdapter,
    touch_input: Option<EvdevTouchInput>,
    present_count: u64,
}

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0;
const BTN_TOUCH: u16 = 0x14a;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;

const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_READ: u32 = 2;

const fn ioc(dir: u32, type_: u32, nr: u32, size: u32) -> libc::c_ulong {
    ((dir << IOC_DIRSHIFT)
        | (type_ << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)) as libc::c_ulong
}

const fn eviocgabs(axis: u16) -> libc::c_ulong {
    ioc(
        IOC_READ,
        b'E' as u32,
        0x40 + axis as u32,
        std::mem::size_of::<LinuxInputAbsInfo>() as u32,
    )
}

impl VoBackend {
    pub(super) fn new(touch_device: Option<String>, width: u32, height: u32) -> Self {
        Self {
            core: LvglUiCore::new(width, height),
            surface: None,
            vo: std::ptr::null_mut(),
            pointer: PointerInputAdapter::default(),
            touch_input: touch_device
                .as_deref()
                .and_then(|path| EvdevTouchInput::open(path).ok()),
            present_count: 0,
        }
    }

    fn rotate_degrees() -> u16 {
        std::env::var("LINTX_VO_ROTATE")
            .ok()
            .or_else(|| std::env::var("LINTX_FB_ROTATE").ok())
            .and_then(|v| v.parse::<u16>().ok())
            .map(|v| v % 360)
            .unwrap_or(270)
    }

    fn panel_size(width: u32, height: u32, rotate_degrees: u16) -> (u32, u32) {
        let env_w = std::env::var("LINTX_VO_PANEL_WIDTH")
            .ok()
            .and_then(|v| v.parse::<u32>().ok());
        let env_h = std::env::var("LINTX_VO_PANEL_HEIGHT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok());
        match (env_w, env_h) {
            (Some(w), Some(h)) => (w, h),
            _ if matches!(rotate_degrees, 90 | 270) => (height, width),
            _ => (width, height),
        }
    }

    fn blit_refresh<const N: usize>(
        surface: &Rc<RefCell<MappedRgbSurface>>,
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

        let mut surface = surface.borrow_mut();
        let stride = surface.stride;
        let fb = unsafe { surface.bytes_mut() };
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
                let (r8, g8, b8) = Self::expand_lv_color(color_raw);
                let offset = y as usize * stride + x as usize * 3;
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
}

impl LvglBackend for VoBackend {
    fn init(&mut self) {
        let rotate_degrees = Self::rotate_degrees();
        let (panel_width, panel_height) =
            Self::panel_size(self.core.width, self.core.height, rotate_degrees);
        eprintln!(
            "[lintx-ui] VoBackend::init logical={}x{} panel={}x{} rotate={} touch={}",
            self.core.width,
            self.core.height,
            panel_width,
            panel_height,
            rotate_degrees,
            self.touch_input.is_some()
        );

        unsafe {
            lvgl_sys::lv_deinit();
            lvgl_sys::lv_init();
        }

        self.vo = unsafe {
            lintx_vo_create(
                self.core.width,
                self.core.height,
                panel_width,
                panel_height,
                rotate_degrees,
            )
        };
        if self.vo.is_null() {
            panic!("failed to initialize VO backend");
        }

        let ptr = NonNull::new(unsafe { lintx_vo_framebuffer_ptr(self.vo) })
            .expect("vo framebuffer ptr is null");
        let len = unsafe { lintx_vo_framebuffer_len(self.vo) };
        let stride = unsafe { lintx_vo_framebuffer_stride(self.vo) } as usize;
        eprintln!(
            "[lintx-ui] VoBackend::init vo_handle={:p} framebuffer={:p} len={} stride={}",
            self.vo,
            ptr.as_ptr(),
            len,
            stride
        );

        let surface = Rc::new(RefCell::new(MappedRgbSurface { ptr, len, stride }));
        let draw_buf = lvgl::DrawBuffer::<LVGL_DRAW_BUF_PIXELS>::default();
        let surface_for_draw = Rc::clone(&surface);
        let width = self.core.width;
        let height = self.core.height;

        let display = lvgl::Display::register(draw_buf, width, height, move |refresh| {
            Self::blit_refresh(&surface_for_draw, width, height, refresh);
        })
        .expect("failed to register lvgl display");

        self.surface = Some(surface);
        self.core.display = Some(display);
        LvglUiCore::set_latest_display_default();
        self.core.build_ui();
        self.core.last_tick = std::time::Instant::now();
        eprintln!("[lintx-ui] VoBackend::init done");
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        if let Some(evt) = self.pointer.pop_event() {
            return Some(evt);
        }

        if let Some(touch_input) = self.touch_input.as_mut() {
            touch_input.read_events(self.core.width, self.core.height, &mut self.pointer);
        }

        self.pointer.pop_event()
    }

    fn render(&mut self, frame: &UiFrame) {
        self.core.sync_ui(frame);
        let ret = unsafe { lintx_vo_present(self.vo) };
        if ret != 0 {
            let err = io::Error::last_os_error();
            eprintln!(
                "[lintx-ui] vo present failed: ret={} errno={:?} logical={}x{} handle={:p}",
                ret, err, self.core.width, self.core.height, self.vo
            );
            return;
        }

        self.present_count = self.present_count.saturating_add(1);
        if self.present_count <= 3 || self.present_count % 120 == 0 {
            eprintln!(
                "[lintx-ui] vo present ok: count={} logical={}x{} handle={:p}",
                self.present_count, self.core.width, self.core.height, self.vo
            );
        }
    }

    fn shutdown(&mut self) {
        eprintln!("[lintx-ui] VoBackend::shutdown called");
        if let Some(surface) = self.surface.as_ref() {
            unsafe {
                surface.borrow_mut().bytes_mut().fill(0);
            }
        }
        if !self.vo.is_null() {
            eprintln!(
                "[lintx-ui] VoBackend::shutdown destroying handle={:p}",
                self.vo
            );
            unsafe { lintx_vo_destroy(self.vo) };
            self.vo = std::ptr::null_mut();
        }
        self.surface = None;
    }
}

impl Drop for VoBackend {
    fn drop(&mut self) {
        eprintln!("[lintx-ui] VoBackend::drop called");
        if !self.vo.is_null() {
            eprintln!("[lintx-ui] VoBackend::drop destroying handle={:p}", self.vo);
            unsafe { lintx_vo_destroy(self.vo) };
            self.vo = std::ptr::null_mut();
        }
        self.surface = None;
    }
}

impl EvdevTouchInput {
    fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).open(path)?;
        let fd = file.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            x_axis: Self::axis_calibration(fd, ABS_MT_POSITION_X)
                .or_else(|| Self::axis_calibration(fd, ABS_X)),
            y_axis: Self::axis_calibration(fd, ABS_MT_POSITION_Y)
                .or_else(|| Self::axis_calibration(fd, ABS_Y)),
            rotate_degrees: std::env::var("LINTX_TOUCH_ROTATE")
                .ok()
                .or_else(|| std::env::var("LINTX_VO_ROTATE").ok())
                .or_else(|| std::env::var("LINTX_FB_ROTATE").ok())
                .and_then(|v| v.parse::<u16>().ok())
                .map(|v| v % 360)
                .unwrap_or(270),
            file,
            pressed: false,
            x: 0,
            y: 0,
        })
    }

    fn axis_calibration(fd: libc::c_int, axis: u16) -> Option<EvdevAxisCalibration> {
        let mut abs = LinuxInputAbsInfo::default();
        let req = eviocgabs(axis);
        let ret = unsafe { libc::ioctl(fd, req as _, &mut abs) };
        if ret < 0 || abs.maximum <= abs.minimum {
            return None;
        }
        Some(EvdevAxisCalibration {
            min: abs.minimum,
            max: abs.maximum,
        })
    }

    fn read_events(&mut self, width: u32, height: u32, pointer: &mut PointerInputAdapter) {
        loop {
            let mut event = LinuxInputEvent::default();
            let read_len = unsafe {
                libc::read(
                    self.file.as_raw_fd(),
                    &mut event as *mut _ as *mut libc::c_void,
                    std::mem::size_of::<LinuxInputEvent>(),
                )
            };

            if read_len == 0 {
                return;
            }
            if read_len < 0 {
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::WouldBlock {
                    super::super::debug_log(&format!("touch read failed: {err}"));
                }
                return;
            }
            if read_len as usize != std::mem::size_of::<LinuxInputEvent>() {
                return;
            }

            match event.type_ {
                EV_KEY if event.code == BTN_TOUCH => {
                    self.pressed = event.value > 0;
                }
                EV_ABS => match event.code {
                    ABS_X | ABS_MT_POSITION_X => {
                        let (phys_w, _) = self.touch_extent(width, height);
                        self.x = self.scale_axis(event.value, self.x_axis.as_ref(), phys_w)
                    }
                    ABS_Y | ABS_MT_POSITION_Y => {
                        let (_, phys_h) = self.touch_extent(width, height);
                        self.y = self.scale_axis(event.value, self.y_axis.as_ref(), phys_h)
                    }
                    _ => {}
                },
                EV_SYN if event.code == SYN_REPORT => {
                    let (logical_x, logical_y) = self.map_touch_to_logical(width, height);
                    if self.pressed && !pointer.gesture.pressed {
                        pointer.begin(logical_x, logical_y);
                    } else if self.pressed {
                        pointer.update(logical_x, logical_y);
                    } else if pointer.gesture.pressed {
                        pointer.end(logical_x, logical_y);
                    }
                }
                _ => {}
            }
        }
    }

    fn touch_extent(&self, width: u32, height: u32) -> (u32, u32) {
        if matches!(self.rotate_degrees, 90 | 270) {
            (height, width)
        } else {
            (width, height)
        }
    }

    fn scale_axis(&self, value: i32, axis: Option<&EvdevAxisCalibration>, extent: u32) -> i32 {
        let Some(axis) = axis else {
            return value;
        };
        let span = (axis.max - axis.min).max(1);
        let clamped = value.clamp(axis.min, axis.max) - axis.min;
        ((clamped as i64 * extent.saturating_sub(1) as i64) / span as i64) as i32
    }

    fn map_touch_to_logical(&self, width: u32, height: u32) -> (i32, i32) {
        let max_x = width.saturating_sub(1) as i32;
        let max_y = height.saturating_sub(1) as i32;
        let x = self.x.clamp(0, max_x);
        let y = self.y.clamp(0, max_y);
        match self.rotate_degrees {
            90 => (max_x - y, x),
            180 => (max_x - x, max_y - y),
            270 => (y, max_y - x),
            _ => (x, y),
        }
    }
}

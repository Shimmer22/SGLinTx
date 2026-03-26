use std::{fs::OpenOptions, io, mem::MaybeUninit, os::fd::AsRawFd};

use crate::ui::{input::UiInputEvent, model::UiFrame};

use super::{
    lvgl_core::{LvglUiCore, LVGL_DRAW_BUF_PIXELS},
    pointer::PointerInputAdapter,
    LvglBackend,
};

const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;
const FBIOPAN_DISPLAY: libc::c_ulong = 0x4606;

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFbFixScreeninfo {
    id: [libc::c_char; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFbVarScreeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: LinuxFbBitfield,
    green: LinuxFbBitfield,
    blue: LinuxFbBitfield,
    transp: LinuxFbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

struct LinuxFramebuffer {
    file: std::fs::File,
    var_info: LinuxFbVarScreeninfo,
    map: *mut u8,
    map_len: usize,
    frame_shadow: Vec<u8>,
    dirty: Option<(u32, u32, u32, u32)>,
    previous_dirty: Option<(u32, u32, u32, u32)>,
    row_scratch: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
    bits_per_pixel: u32,
    bytes_per_pixel: usize,
    xoffset: u32,
    current_yoffset: u32,
    render_yoffset: u32,
    page_flip: bool,
    red: LinuxFbBitfield,
    green: LinuxFbBitfield,
    blue: LinuxFbBitfield,
    transp: LinuxFbBitfield,
    rotate_degrees: u16,
    swap_rb: bool,
    rgb565_to_native32_lut: Option<Box<[u32]>>,
    last_pan_elapsed: std::time::Duration,
    native32_format: Option<Native32Format>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Native32Format {
    LvglArgb8888,
    LvglArgb8888SwapRb,
}

impl Drop for LinuxFramebuffer {
    fn drop(&mut self) {
        if !self.map.is_null() && self.map_len != 0 {
            unsafe {
                libc::munmap(self.map.cast(), self.map_len);
            }
        }
    }
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

pub(super) struct FbdevBackend {
    core: LvglUiCore,
    framebuffer: LinuxFramebuffer,
    touch_input: Option<EvdevTouchInput>,
    pointer: PointerInputAdapter,
    perf: FbdevPerfStats,
}

#[derive(Default)]
struct FbdevPerfStats {
    enabled: bool,
    /// Detailed per-flush rectangle logging (LINTX_UI_FLUSH_RECTS=1)
    flush_rects_enabled: bool,
    window_start: Option<std::time::Instant>,
    frames: u32,
    flush_calls: u32,
    flush_pixels: u64,
    flush_ns: u128,
    sync_ui_ns: u128,
    task_handler_ns: u128,
    present_ns: u128,
    pan_calls: u32,
    pan_ns: u128,
    /// Track largest flush rect in current window for summary
    max_flush_width: u32,
    max_flush_height: u32,
    max_flush_pixels: u64,
    /// Track full-screen flush ratio (area >= 80% of screen)
    fullscreen_flush_count: u32,
    screen_pixels: u64,
}

impl FbdevPerfStats {
    fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            enabled: std::env::var("LINTX_UI_PERF_TRACE")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
                .unwrap_or(false),
            flush_rects_enabled: std::env::var("LINTX_UI_FLUSH_RECTS")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
                .unwrap_or(false),
            screen_pixels: u64::from(screen_width) * u64::from(screen_height),
            ..Self::default()
        }
    }

    fn begin_window_if_needed(&mut self, now: std::time::Instant) {
        if self.window_start.is_none() {
            self.window_start = Some(now);
        }
    }

    fn record_flush_area(
        &mut self,
        x1: lvgl_sys::lv_coord_t,
        y1: lvgl_sys::lv_coord_t,
        x2: lvgl_sys::lv_coord_t,
        y2: lvgl_sys::lv_coord_t,
    ) {
        if !self.enabled || x2 < x1 || y2 < y1 {
            return;
        }
        let width = (i32::from(x2) - i32::from(x1) + 1).max(0) as u32;
        let height = (i32::from(y2) - i32::from(y1) + 1).max(0) as u32;
        if width == 0 || height == 0 {
            return;
        }
        let pixels = u64::from(width) * u64::from(height);

        self.flush_calls = self.flush_calls.saturating_add(1);
        self.flush_pixels = self.flush_pixels.saturating_add(pixels);

        // Track max flush rect
        if pixels > self.max_flush_pixels {
            self.max_flush_width = width;
            self.max_flush_height = height;
            self.max_flush_pixels = pixels;
        }

        // Track full-screen flushes (>= 80% of screen area)
        let threshold = self.screen_pixels * 80 / 100;
        if pixels >= threshold {
            self.fullscreen_flush_count = self.fullscreen_flush_count.saturating_add(1);
        }

        // Per-flush rectangle logging
        if self.flush_rects_enabled {
            let pct = if self.screen_pixels > 0 {
                (pixels as f64 / self.screen_pixels as f64) * 100.0
            } else {
                0.0
            };
            super::super::debug_log(&format!(
                "fb-flush rect=({},{})..({},{}) size={}x{} pixels={} pct={:.1}%",
                x1, y1, x2, y2, width, height, pixels, pct
            ));
        }
    }

    fn record_flush_work(&mut self, elapsed: std::time::Duration) {
        if !self.enabled {
            return;
        }
        self.flush_ns = self.flush_ns.saturating_add(elapsed.as_nanos());
    }

    fn record_frame(
        &mut self,
        now: std::time::Instant,
        sync_ui: std::time::Duration,
        task_handler: std::time::Duration,
        present: std::time::Duration,
        pan: std::time::Duration,
    ) {
        if !self.enabled {
            return;
        }
        self.begin_window_if_needed(now);
        self.frames = self.frames.saturating_add(1);
        self.sync_ui_ns = self.sync_ui_ns.saturating_add(sync_ui.as_nanos());
        self.task_handler_ns = self.task_handler_ns.saturating_add(task_handler.as_nanos());
        self.present_ns = self.present_ns.saturating_add(present.as_nanos());
        if !pan.is_zero() {
            self.pan_calls = self.pan_calls.saturating_add(1);
            self.pan_ns = self.pan_ns.saturating_add(pan.as_nanos());
        }

        let Some(start) = self.window_start else {
            return;
        };
        let elapsed = now.saturating_duration_since(start);
        if elapsed < std::time::Duration::from_secs(1) {
            return;
        }

        let secs = elapsed.as_secs_f64().max(0.001);
        let fps = self.frames as f64 / secs;
        let flush_mpix = self.flush_pixels as f64 / 1_000_000.0 / secs;
        let fullscreen_pct = if self.flush_calls > 0 {
            (self.fullscreen_flush_count as f64 / self.flush_calls as f64) * 100.0
        } else {
            0.0
        };
        super::super::debug_log(&format!(
            "fb-perf fps={fps:.1} flush_calls={} flush_mpix_s={flush_mpix:.2} flush_ms={:.2} sync_ui_ms={:.2} task_ms={:.2} present_ms={:.2} pan_calls={} pan_ms={:.2} max_rect={}x{} fullscreen_pct={:.0}%",
            self.flush_calls,
            self.flush_ns as f64 / 1_000_000.0 / self.frames.max(1) as f64,
            self.sync_ui_ns as f64 / 1_000_000.0 / self.frames.max(1) as f64,
            self.task_handler_ns as f64 / 1_000_000.0 / self.frames.max(1) as f64,
            self.present_ns as f64 / 1_000_000.0 / self.frames.max(1) as f64,
            self.pan_calls,
            self.pan_ns as f64 / 1_000_000.0 / self.frames.max(1) as f64,
            self.max_flush_width,
            self.max_flush_height,
            fullscreen_pct,
        ));

        self.window_start = Some(now);
        self.frames = 0;
        self.flush_calls = 0;
        self.flush_pixels = 0;
        self.flush_ns = 0;
        self.sync_ui_ns = 0;
        self.task_handler_ns = 0;
        self.present_ns = 0;
        self.pan_calls = 0;
        self.pan_ns = 0;
        self.max_flush_width = 0;
        self.max_flush_height = 0;
        self.max_flush_pixels = 0;
        self.fullscreen_flush_count = 0;
    }
}

impl EvdevTouchInput {
    fn touch_debug_enabled() -> bool {
        std::env::var("LINTX_TOUCH_DEBUG")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
            .unwrap_or(false)
    }

    fn touch_debug_log(msg: &str) {
        if Self::touch_debug_enabled() {
            super::super::debug_log(msg);
        }
    }

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
            rotate_degrees: std::env::var("LINTX_FB_ROTATE")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .map(|v| v % 360)
                .unwrap_or(0),
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
                    Self::touch_debug_log(&format!(
                        "evdev key code=0x{:x} value={}",
                        event.code, event.value
                    ));
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
                    Self::touch_debug_log(&format!(
                        "evdev syn pressed={} raw=({}, {}) mapped=({}, {}) rotate={}",
                        self.pressed, self.x, self.y, logical_x, logical_y, self.rotate_degrees
                    ));
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
        match self.rotate_degrees {
            90 | 270 => (height, width),
            _ => (width, height),
        }
    }

    fn map_touch_to_logical(&self, width: u32, height: u32) -> (i32, i32) {
        let logical_w = width as i32;
        let logical_h = height as i32;
        let phys_h = self.touch_extent(width, height).1 as i32;

        let (x, y) = match self.rotate_degrees {
            90 => (
                self.y,
                (logical_h - 1 - self.x).clamp(0, logical_h.saturating_sub(1)),
            ),
            180 => (
                (logical_w - 1 - self.x).clamp(0, logical_w.saturating_sub(1)),
                (logical_h - 1 - self.y).clamp(0, logical_h.saturating_sub(1)),
            ),
            270 => (
                (phys_h - 1 - self.y).clamp(0, logical_w.saturating_sub(1)),
                (logical_h - 1 - self.x).clamp(0, logical_h.saturating_sub(1)),
            ),
            _ => (
                self.x.clamp(0, logical_w.saturating_sub(1)),
                self.y.clamp(0, logical_h.saturating_sub(1)),
            ),
        };

        (x, y)
    }

    fn scale_axis(&self, value: i32, axis: Option<&EvdevAxisCalibration>, extent: u32) -> i32 {
        let Some(axis) = axis else {
            return value.clamp(0, extent.saturating_sub(1) as i32);
        };
        let span = (axis.max - axis.min).max(1);
        let logical = (value - axis.min) * (extent.saturating_sub(1) as i32) / span;
        logical.clamp(0, extent.saturating_sub(1) as i32)
    }
}

impl LinuxFramebuffer {
    fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let fd = file.as_raw_fd();

        let mut finfo = MaybeUninit::<LinuxFbFixScreeninfo>::zeroed();
        let mut vinfo = MaybeUninit::<LinuxFbVarScreeninfo>::zeroed();

        let fix_ret = unsafe { libc::ioctl(fd, FBIOGET_FSCREENINFO as _, finfo.as_mut_ptr()) };
        if fix_ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let var_ret = unsafe { libc::ioctl(fd, FBIOGET_VSCREENINFO as _, vinfo.as_mut_ptr()) };
        if var_ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let finfo = unsafe { finfo.assume_init() };
        let vinfo = unsafe { vinfo.assume_init() };
        let bytes_per_pixel = vinfo.bits_per_pixel.div_ceil(8) as usize;
        let page_flip = vinfo.yres_virtual >= vinfo.yres.saturating_mul(2) && finfo.ypanstep > 0;
        let map_len = finfo.smem_len as usize;
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let stride = finfo.line_length as usize;
        let frame_bytes = stride.saturating_mul(vinfo.yres as usize);
        let current_offset = vinfo.yoffset as usize * stride;
        if current_offset.saturating_add(frame_bytes) > map_len {
            unsafe {
                libc::munmap(map, map_len);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "framebuffer current page out of range",
            ));
        }
        let frame_shadow = unsafe {
            std::slice::from_raw_parts(map.cast::<u8>().add(current_offset), frame_bytes)
        }
        .to_vec();
        let rgb565_to_native32_lut = if bytes_per_pixel == 4 {
            Some(Self::build_rgb565_to_native32_lut(
                vinfo.red,
                vinfo.green,
                vinfo.blue,
                vinfo.transp,
                std::env::var_os("LINTX_FB_SWAP_RB").is_some(),
            ))
        } else {
            None
        };

        Ok(Self {
            file,
            var_info: vinfo,
            map: map.cast(),
            map_len,
            frame_shadow,
            dirty: None,
            previous_dirty: None,
            row_scratch: Vec::new(),
            width: vinfo.xres,
            height: vinfo.yres,
            stride,
            bits_per_pixel: vinfo.bits_per_pixel,
            bytes_per_pixel,
            xoffset: vinfo.xoffset,
            current_yoffset: vinfo.yoffset,
            render_yoffset: vinfo.yoffset,
            page_flip,
            red: vinfo.red,
            green: vinfo.green,
            blue: vinfo.blue,
            transp: vinfo.transp,
            rotate_degrees: std::env::var("LINTX_FB_ROTATE")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .map(|v| v % 360)
                .unwrap_or(0),
            swap_rb: std::env::var_os("LINTX_FB_SWAP_RB").is_some(),
            rgb565_to_native32_lut,
            last_pan_elapsed: std::time::Duration::ZERO,
            native32_format: Self::detect_native32_format(vinfo),
        })
    }

    fn detect_native32_format(vinfo: LinuxFbVarScreeninfo) -> Option<Native32Format> {
        if vinfo.bits_per_pixel != 32
            || vinfo.red.length != 8
            || vinfo.green.length != 8
            || vinfo.blue.length != 8
            || vinfo.transp.length != 8
        {
            return None;
        }

        if vinfo.red.offset == 16
            && vinfo.green.offset == 8
            && vinfo.blue.offset == 0
            && vinfo.transp.offset == 24
        {
            return Some(Native32Format::LvglArgb8888);
        }

        if vinfo.red.offset == 0
            && vinfo.green.offset == 8
            && vinfo.blue.offset == 16
            && vinfo.transp.offset == 24
        {
            return Some(Native32Format::LvglArgb8888SwapRb);
        }

        None
    }

    fn build_rgb565_to_native32_lut(
        red: LinuxFbBitfield,
        green: LinuxFbBitfield,
        blue: LinuxFbBitfield,
        transp: LinuxFbBitfield,
        swap_rb: bool,
    ) -> Box<[u32]> {
        let alpha_mask = if transp.length > 0 {
            ((1u32 << transp.length.min(16)) - 1) << transp.offset
        } else {
            0
        };

        let mut lut = vec![0u32; 1 << 16];
        for (rgb565, slot) in lut.iter_mut().enumerate() {
            let rgb565 = rgb565 as u16;
            let r5 = ((rgb565 >> 11) & 0x1F) as u8;
            let g6 = ((rgb565 >> 5) & 0x3F) as u8;
            let b5 = (rgb565 & 0x1F) as u8;

            let r8 = (r5 << 3) | (r5 >> 2);
            let g8 = (g6 << 2) | (g6 >> 4);
            let b8 = (b5 << 3) | (b5 >> 2);
            let (r8, b8) = if swap_rb { (b8, r8) } else { (r8, b8) };

            *slot = alpha_mask
                | (Self::scale_channel(r8, red.length) << red.offset)
                | (Self::scale_channel(g8, green.length) << green.offset)
                | (Self::scale_channel(b8, blue.length) << blue.offset);
        }

        lut.into_boxed_slice()
    }

    fn begin_frame(&mut self) {
        self.dirty = None;
        self.last_pan_elapsed = std::time::Duration::ZERO;
        if !self.page_flip {
            self.render_yoffset = self.current_yoffset;
            return;
        }

        let next_yoffset = if self.current_yoffset >= self.height {
            0
        } else {
            self.height
        };
        self.render_yoffset = next_yoffset;
        if let Some((x1, y1, x2, y2)) = self.previous_dirty {
            self.copy_shadow_rect_to_page(next_yoffset, x1, y1, x2, y2);
        }
    }

    fn scale_channel(value: u8, length: u32) -> u32 {
        if length == 0 {
            return 0;
        }
        let max = (1u32 << length.min(16)) - 1;
        ((u32::from(value) * max) + 127) / 255
    }

    fn pack_color(&self, color: lvgl_sys::lv_color_t) -> u32 {
        if lvgl_sys::LV_COLOR_DEPTH == 32 {
            let full = unsafe { color.full as u32 };
            if let Some(format) = self.native32_format {
                return match format {
                    Native32Format::LvglArgb8888 => full,
                    Native32Format::LvglArgb8888SwapRb => {
                        (full & 0xFF00_FF00)
                            | ((full & 0x00FF_0000) >> 16)
                            | ((full & 0x0000_00FF) << 16)
                    }
                };
            }

            let mut r8 = ((full >> 16) & 0xFF) as u8;
            let g8 = ((full >> 8) & 0xFF) as u8;
            let mut b8 = (full & 0xFF) as u8;
            if self.swap_rb {
                std::mem::swap(&mut r8, &mut b8);
            }

            let mut pixel = 0u32;
            pixel |= Self::scale_channel(r8, self.red.length) << self.red.offset;
            pixel |= Self::scale_channel(g8, self.green.length) << self.green.offset;
            pixel |= Self::scale_channel(b8, self.blue.length) << self.blue.offset;
            if self.transp.length > 0 {
                let alpha = ((full >> 24) & 0xFF) as u8;
                pixel |= Self::scale_channel(alpha, self.transp.length) << self.transp.offset;
            }
            return pixel;
        }

        let rgb565 = unsafe { color.full } as u16;
        if let Some(lut) = self.rgb565_to_native32_lut.as_ref() {
            return lut[rgb565 as usize];
        }
        let r5 = ((rgb565 >> 11) & 0x1F) as u8;
        let g6 = ((rgb565 >> 5) & 0x3F) as u8;
        let b5 = (rgb565 & 0x1F) as u8;

        let r8 = (r5 << 3) | (r5 >> 2);
        let g8 = (g6 << 2) | (g6 >> 4);
        let b8 = (b5 << 3) | (b5 >> 2);
        let (r8, b8) = if self.swap_rb { (b8, r8) } else { (r8, b8) };

        let mut pixel = 0u32;
        pixel |= Self::scale_channel(r8, self.red.length) << self.red.offset;
        pixel |= Self::scale_channel(g8, self.green.length) << self.green.offset;
        pixel |= Self::scale_channel(b8, self.blue.length) << self.blue.offset;
        if self.transp.length > 0 {
            pixel |= ((1u32 << self.transp.length.min(16)) - 1) << self.transp.offset;
        }
        pixel
    }

    fn copy_shadow_rect_to_page(&mut self, page_yoffset: u32, x1: u32, y1: u32, x2: u32, y2: u32) {
        let row_bytes = (x2 - x1 + 1) as usize * self.bytes_per_pixel;
        for y in y1..=y2 {
            let shadow_offset = self.shadow_offset(y, x1);
            let page_offset = self.page_offset(page_yoffset, y, x1);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.frame_shadow.as_ptr().add(shadow_offset),
                    self.map.add(page_offset),
                    row_bytes,
                );
            }
        }
    }

    fn shadow_offset(&self, dst_y: u32, dst_x: u32) -> usize {
        (dst_y as usize * self.stride) + ((dst_x + self.xoffset) as usize * self.bytes_per_pixel)
    }

    fn page_offset(&self, page_yoffset: u32, dst_y: u32, dst_x: u32) -> usize {
        ((page_yoffset as usize + dst_y as usize) * self.stride)
            + ((dst_x + self.xoffset) as usize * self.bytes_per_pixel)
    }

    fn resize_row_scratch(&mut self, pixels: usize) {
        let byte_len = pixels.saturating_mul(self.bytes_per_pixel);
        if self.row_scratch.len() < byte_len {
            self.row_scratch.resize(byte_len, 0);
        }
    }

    fn span_offset(&self, dst_y: u32, dst_x: u32, pixels: usize) -> io::Result<(usize, usize)> {
        let byte_len = pixels * self.bytes_per_pixel;
        let offset = ((u64::from(dst_y) + u64::from(self.render_yoffset)) * self.stride as u64)
            + ((u64::from(dst_x) + u64::from(self.xoffset)) * self.bytes_per_pixel as u64);
        let end = offset as usize + byte_len;
        if end > self.map_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "framebuffer span out of range",
            ));
        }
        Ok((offset as usize, byte_len))
    }

    fn mark_dirty(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        self.dirty = Some(match self.dirty.take() {
            Some((dx1, dy1, dx2, dy2)) => (dx1.min(x1), dy1.min(y1), dx2.max(x2), dy2.max(y2)),
            None => (x1, y1, x2, y2),
        });
    }

    fn write_span(&mut self, dst_y: u32, dst_x: u32, pixels: usize) -> io::Result<()> {
        let (offset, byte_len) = self.span_offset(dst_y, dst_x, pixels)?;
        let shadow_offset = self.shadow_offset(dst_y, dst_x);
        self.frame_shadow[shadow_offset..shadow_offset + byte_len]
            .copy_from_slice(&self.row_scratch[..byte_len]);
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.row_scratch.as_ptr(),
                self.map.add(offset),
                byte_len,
            );
        }
        self.mark_dirty(dst_x, dst_y, dst_x + pixels as u32 - 1, dst_y);
        Ok(())
    }

    fn present(&mut self) -> io::Result<()> {
        if self.page_flip {
            if self.render_yoffset != self.current_yoffset {
                let mut next = self.var_info;
                next.yoffset = self.render_yoffset;
                let pan_start = std::time::Instant::now();
                let ret = unsafe {
                    libc::ioctl(
                        self.file.as_raw_fd(),
                        FBIOPAN_DISPLAY as _,
                        &mut next as *mut LinuxFbVarScreeninfo,
                    )
                };
                self.last_pan_elapsed = pan_start.elapsed();
                if ret < 0 {
                    return Err(io::Error::last_os_error());
                }
                self.current_yoffset = self.render_yoffset;
                self.var_info.yoffset = self.render_yoffset;
            }
            self.previous_dirty = self.dirty.take();
            return Ok(());
        }

        let Some((x1, y1, x2, y2)) = self.dirty.take() else {
            return Ok(());
        };

        let row_bytes = (x2 - x1 + 1) as usize * self.bytes_per_pixel;
        for y in y1..=y2 {
            let offset = self.page_offset(self.current_yoffset, y, x1);
            let shadow_offset = self.shadow_offset(y, x1);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.frame_shadow.as_ptr().add(shadow_offset),
                    self.map.add(offset),
                    row_bytes,
                );
            }
        }
        Ok(())
    }

    fn last_pan_elapsed(&self) -> std::time::Duration {
        self.last_pan_elapsed
    }

    fn write_refresh<const N: usize>(
        &mut self,
        refresh: &lvgl::DisplayRefresh<N>,
    ) -> io::Result<()> {
        let area = &refresh.area;
        if area.x2 < area.x1 || area.y2 < area.y1 {
            return Ok(());
        }

        let x1 = i32::from(area.x1).max(0);
        let y1 = i32::from(area.y1).max(0);
        let x2 = i32::from(area.x2);
        let y2 = i32::from(area.y2);
        if x2 < x1 || y2 < y1 {
            return Ok(());
        }

        let src_width = (i32::from(area.x2) - i32::from(area.x1) + 1) as usize;
        let area_x1 = i32::from(area.x1);
        let area_y1 = i32::from(area.y1);
        let bytes_per_pixel = self.bytes_per_pixel;
        match self.rotate_degrees {
            90 => {
                let span = (y2 - y1 + 1) as usize;
                self.resize_row_scratch(span);
                for x in x1..=x2 {
                    let dst_y = x as u32;
                    let dst_x = (self.width as i32 - 1 - y2) as u32;
                    let mut offset = 0;
                    for y in (y1..=y2).rev() {
                        let src_row = (y - area_y1) as usize;
                        let src_col = (x - area_x1) as usize;
                        let idx = src_row * src_width + src_col;
                        let raw: lvgl_sys::lv_color_t = refresh.colors[idx].into();
                        let pixel = self.pack_color(raw).to_ne_bytes();
                        let end = offset + bytes_per_pixel;
                        self.row_scratch[offset..end].copy_from_slice(&pixel[..bytes_per_pixel]);
                        offset = end;
                    }
                    self.write_span(dst_y, dst_x, span)?;
                }
            }
            180 => {
                let span = (x2 - x1 + 1) as usize;
                self.resize_row_scratch(span);
                for y in y1..=y2 {
                    let src_row = (y - area_y1) as usize;
                    let dst_y = (self.height as i32 - 1 - y) as u32;
                    let dst_x = (self.width as i32 - 1 - x2) as u32;
                    let mut offset = 0;
                    for x in (x1..=x2).rev() {
                        let src_col = (x - area_x1) as usize;
                        let idx = src_row * src_width + src_col;
                        let raw: lvgl_sys::lv_color_t = refresh.colors[idx].into();
                        let pixel = self.pack_color(raw).to_ne_bytes();
                        let end = offset + bytes_per_pixel;
                        self.row_scratch[offset..end].copy_from_slice(&pixel[..bytes_per_pixel]);
                        offset = end;
                    }
                    self.write_span(dst_y, dst_x, span)?;
                }
            }
            270 => {
                let span = (y2 - y1 + 1) as usize;
                self.resize_row_scratch(span);
                for x in x1..=x2 {
                    let dst_y = (self.height as i32 - 1 - x) as u32;
                    let dst_x = y1 as u32;
                    let mut offset = 0;
                    for y in y1..=y2 {
                        let src_row = (y - area_y1) as usize;
                        let src_col = (x - area_x1) as usize;
                        let idx = src_row * src_width + src_col;
                        let raw: lvgl_sys::lv_color_t = refresh.colors[idx].into();
                        let pixel = self.pack_color(raw).to_ne_bytes();
                        let end = offset + bytes_per_pixel;
                        self.row_scratch[offset..end].copy_from_slice(&pixel[..bytes_per_pixel]);
                        offset = end;
                    }
                    self.write_span(dst_y, dst_x, span)?;
                }
            }
            _ => {
                let span = (x2 - x1 + 1) as usize;
                self.resize_row_scratch(span);
                for y in y1..=y2 {
                    let src_row = (y - area_y1) as usize;
                    let dst_y = y as u32;
                    let dst_x = x1 as u32;
                    let mut offset = 0;
                    for x in x1..=x2 {
                        let src_col = (x - area_x1) as usize;
                        let idx = src_row * src_width + src_col;
                        let raw: lvgl_sys::lv_color_t = refresh.colors[idx].into();
                        let pixel = self.pack_color(raw).to_ne_bytes();
                        let end = offset + bytes_per_pixel;
                        self.row_scratch[offset..end].copy_from_slice(&pixel[..bytes_per_pixel]);
                        offset = end;
                    }
                    self.write_span(dst_y, dst_x, span)?;
                }
            }
        }

        Ok(())
    }
}

impl FbdevBackend {
    pub(super) fn new(
        device: String,
        touch_device: Option<String>,
        width: u32,
        height: u32,
    ) -> Self {
        let framebuffer = LinuxFramebuffer::open(&device)
            .unwrap_or_else(|err| panic!("failed to open {device}: {err}"));
        let touch_input =
            touch_device
                .as_deref()
                .and_then(|path| match EvdevTouchInput::open(path) {
                    Ok(input) => {
                        super::super::debug_log(&format!(
                            "FbdevBackend::new touch input opened: {path}"
                        ));
                        Some(input)
                    }
                    Err(err) => {
                        super::super::debug_log(&format!(
                            "FbdevBackend::new touch input open failed path={path}: {err}"
                        ));
                        None
                    }
                });
        super::super::debug_log(&format!(
            "FbdevBackend::new device={} actual={}x{} bpp={} stride={} rotate={} swap_rb={}",
            device,
            framebuffer.width,
            framebuffer.height,
            framebuffer.bits_per_pixel,
            framebuffer.stride,
            framebuffer.rotate_degrees,
            framebuffer.swap_rb
        ));
        Self {
            core: LvglUiCore::new(width, height),
            framebuffer,
            touch_input,
            pointer: PointerInputAdapter::default(),
            perf: FbdevPerfStats::new(width, height),
        }
    }
}

impl LvglBackend for FbdevBackend {
    fn init(&mut self) {
        super::super::debug_log(&format!(
            "FbdevBackend::init begin logical={}x{} fb={}x{}",
            self.core.width, self.core.height, self.framebuffer.width, self.framebuffer.height
        ));
        unsafe {
            lvgl_sys::lv_deinit();
            lvgl_sys::lv_init();
        }

        let draw_buf = lvgl::DrawBuffer::<LVGL_DRAW_BUF_PIXELS>::default();
        let fb_ptr: *mut LinuxFramebuffer = &mut self.framebuffer;
        let perf_ptr: *mut FbdevPerfStats = &mut self.perf;
        let display = lvgl::Display::register(
            draw_buf,
            self.core.width,
            self.core.height,
            move |refresh| {
                let fb = unsafe { &mut *fb_ptr };
                let perf = unsafe { &mut *perf_ptr };
                perf.record_flush_area(
                    refresh.area.x1,
                    refresh.area.y1,
                    refresh.area.x2,
                    refresh.area.y2,
                );
                let flush_start = std::time::Instant::now();
                if let Err(err) = fb.write_refresh(refresh) {
                    super::super::debug_log(&format!("FbdevBackend::write_refresh failed: {err}"));
                }
                perf.record_flush_work(flush_start.elapsed());
            },
        )
        .expect("failed to register lvgl display");

        self.core.display = Some(display);
        LvglUiCore::set_latest_display_default();
        self.core.build_ui();
        self.core.last_tick = std::time::Instant::now();
        super::super::debug_log("FbdevBackend::init done");
    }

    fn poll_event(&mut self) -> Option<UiInputEvent> {
        if let Some(evt) = self.pointer.pop_event() {
            return Some(evt);
        }
        if let Some(touch) = self.touch_input.as_mut() {
            touch.read_events(self.core.width, self.core.height, &mut self.pointer);
        }
        self.pointer.pop_event()
    }

    fn render(&mut self, frame: &UiFrame) {
        let now = std::time::Instant::now();
        self.pointer
            .update_snapshot(frame, self.core.width, self.core.height);
        self.core.set_drag_offset(self.pointer.drag_offset_x());
        let sync_ui_start = std::time::Instant::now();
        self.core.sync_ui(frame);
        let sync_ui_elapsed = sync_ui_start.elapsed();
        self.framebuffer.begin_frame();
        let task_handler_start = std::time::Instant::now();
        self.core.start_frame();
        let task_handler_elapsed = task_handler_start.elapsed();
        let present_start = std::time::Instant::now();
        if let Err(err) = self.framebuffer.present() {
            super::super::debug_log(&format!("FbdevBackend::present failed: {err}"));
        }
        let present_elapsed = present_start.elapsed();
        let pan_elapsed = self.framebuffer.last_pan_elapsed();
        self.perf.record_frame(
            now,
            sync_ui_elapsed,
            task_handler_elapsed,
            present_elapsed,
            pan_elapsed,
        );
    }

    fn shutdown(&mut self) {
        self.core.ui = None;
        self.core.display = None;
    }
}

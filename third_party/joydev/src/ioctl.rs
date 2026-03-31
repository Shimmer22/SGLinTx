//! Unsafe wrappers for the underling ioctls

use input_event_codes::{ABS_CNT, BTN_MISC, KEY_CNT};
use libc::{__u16, __u32, __u8, c_char, c_int, ioctl};

use sys::{
	js_corr, JSIOCGAXES, JSIOCGAXMAP, JSIOCGBTNMAP, JSIOCGBUTTONS, JSIOCGCORR, JSIOCGNAME, JSIOCGVERSION, JSIOCSAXMAP,
	JSIOCSBTNMAP, JSIOCSCORR,
};

use crate::result::convert_ioctl_result;
use crate::Result;

/// `JSIOCGAXES` ioctl.
pub(crate) unsafe fn jsiocgaxes(fd: c_int, data: *mut __u8) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGAXES as _, data))
}

/// `JSIOCGAXMAP` ioctl.
pub(crate) unsafe fn jsiocgaxmap(fd: c_int, data: *mut [__u8; ABS_CNT as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGAXMAP as _, data))
}

/// `JSIOCGBTNMAP` ioctl.
pub(crate) unsafe fn jsiocgbtnmap(fd: c_int, data: *mut [__u16; (KEY_CNT - BTN_MISC) as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGBTNMAP as _, data))
}

/// `JSIOCGBUTTONS` ioctl.
pub(crate) unsafe fn jsiocgbuttons(fd: c_int, data: *mut __u8) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGBUTTONS as _, data))
}

/// `JSIOCGCORR` ioctl.
pub(crate) unsafe fn jsiocgcorr(fd: c_int, data: *mut [js_corr; ABS_CNT as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGCORR as _, data))
}

/// `JSIOCGNAME` ioctl.
pub(crate) unsafe fn jsiocgname(fd: c_int, data: &mut [c_char]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGNAME(data.len()) as _, data))
}

/// `JSIOCGVERSION` ioctl.
pub(crate) unsafe fn jsiocgversion(fd: c_int, data: *mut __u32) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCGVERSION as _, data))
}

/// `JSIOCSAXMAP` ioctl.
pub(crate) unsafe fn jsiocsaxmap(fd: c_int, data: *const [__u8; ABS_CNT as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCSAXMAP as _, data))
}

/// `JSIOCSBTNMAP` ioctl.
pub(crate) unsafe fn jsiocsbtnmap(fd: c_int, data: *const [__u16; (KEY_CNT - BTN_MISC) as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCSBTNMAP as _, data))
}

/// `JSIOCSCORR` ioctl.
pub(crate) unsafe fn jsiocscorr(fd: c_int, data: *const [js_corr; ABS_CNT as usize]) -> Result<c_int> {
	convert_ioctl_result(ioctl(fd, JSIOCSCORR as _, data))
}

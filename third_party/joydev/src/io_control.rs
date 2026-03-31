//! Safe wrappers for the underling ioctls
//!
//! This module is intended for making custom device abstractions. If you're using the provided [`Device`](struct.Device.html) abstraction you
//! don't have to use this module however mixing is not prohibited and should work.

use std::ffi::CStr;
use std::mem::size_of;
use std::os::unix::prelude::*;

use input_event_codes::{ABS_CNT, BTN_MISC, KEY_CNT};
use libc::{c_char, read};
use nix::errno::Errno;

use crate::event_codes::{AbsoluteAxis, Key};
use crate::ioctl::{
	jsiocgaxes, jsiocgaxmap, jsiocgbtnmap, jsiocgbuttons, jsiocgcorr, jsiocgname, jsiocgversion, jsiocsaxmap,
	jsiocsbtnmap, jsiocscorr,
};
use crate::{Correction, Error, Event, Result};

/// Retrieve number of axis present. Calls `JSIOCGAXES` ioctl.
pub fn get_axis_count(fd: RawFd) -> Result<u8> {
	let mut result = 0u8;
	unsafe { jsiocgaxes(fd, &mut result) }?;
	Ok(result)
}

/// Retrieve axes mapping. Calls `JSIOCGAXMAP` ioctl.
pub fn get_axis_mapping(fd: RawFd) -> Result<Vec<AbsoluteAxis>> {
	let mut result = vec![AbsoluteAxis::default(); ABS_CNT as usize];
	unsafe { jsiocgaxmap(fd, result.as_mut_ptr() as *mut _) }?;
	Ok(result)
}

/// Retrieve number of buttons present. Calls `JSIOCGBUTTONS` ioctl.
pub fn get_button_count(fd: RawFd) -> Result<u8> {
	let mut result = 0u8;
	unsafe { jsiocgbuttons(fd, &mut result) }?;
	Ok(result)
}

/// Retrieve buttons mapping. Calls `JSIOCGBTNMAP` ioctl.
pub fn get_button_mapping(fd: RawFd) -> Result<Vec<Key>> {
	let mut result = vec![Key::default(); (KEY_CNT - BTN_MISC) as usize];
	unsafe { jsiocgbtnmap(fd, result.as_mut_ptr() as *mut _) }?;
	Ok(result)
}

/// Retrieve axes correction values. Calls `JSIOCGCORR` ioctl.
pub fn get_correction_values(fd: RawFd) -> Result<Vec<Correction>> {
	let mut result = vec![Correction::default(); ABS_CNT as usize];
	unsafe { jsiocgcorr(fd, result.as_mut_ptr() as *mut _) }?;
	Ok(result)
}

/// Retrieve driver version. Calls `JSIOCGVERSION` ioctl.
pub fn get_driver_version(fd: RawFd) -> Result<u32> {
	let mut result = 0u32;
	unsafe { jsiocgversion(fd, &mut result) }?;
	Ok(result)
}

/// Read an event. Calls `read`.
///
/// If read fails with EAGAIN return [`Error::QueueEmpty`](../enum.Error.html#variant.QueueEmpty). In this context EAGAIN
/// is not an error but rather an indicator that the device has no events left. This only applies if the device is
/// opened in non-blocking mode for blocking mode EAGAIN shouldn't be ever returned.
pub fn get_event(fd: RawFd) -> Result<Event> {
	let mut result = Event::default();
	if unsafe { read(fd, (&mut result as *mut _) as *mut _, size_of::<Event>()) } > 0 {
		Ok(result)
	} else {
		let errno = Errno::last();
		match errno {
			Errno::EAGAIN => Err(Error::QueueEmpty),
			_ => Err(Error::from(errno)),
		}
	}
}

// TODO: Maybe add support for joydev 0.x
/*/// Read an event the legacy version. Calls `read`.
pub fn get_event_legacy(fd: RawFd) -> Result<JS_DATA_TYPE> {
	unimplemented!()
}*/

/// Retrieve device identifier. Calls `JSIOCGNAME` ioctl.
pub fn get_identifier(fd: RawFd) -> Result<String> {
	// TODO: Maybe change to vector?
	let mut result: [c_char; 128] = [0; 128];
	unsafe { jsiocgname(fd, &mut result) }?;
	Ok(unsafe { CStr::from_ptr(&result as *const _) }.to_str()?.to_owned())
}

/// Set axes mapping. Calls `JSIOCSAXMAP` ioctl.
pub fn set_axis_mapping(fd: RawFd, mapping: &[AbsoluteAxis; ABS_CNT as usize]) -> Result<()> {
	unsafe { jsiocsaxmap(fd, mapping as *const _ as *const _) }?;
	Ok(())
}

/// Set buttons mapping. Calls `JSIOCSBTNMAP` ioctl.
pub fn set_button_mapping(fd: RawFd, mapping: &[Key; (KEY_CNT - BTN_MISC) as usize]) -> Result<()> {
	unsafe { jsiocsbtnmap(fd, mapping as *const _ as *const _) }?;
	Ok(())
}

/// Set axes correction values. Calls `JSIOCSCORR` ioctl.
pub fn set_correction_values(fd: RawFd, correction: &[Correction; ABS_CNT as usize]) -> Result<()> {
	unsafe { jsiocscorr(fd, correction as *const _ as *const _) }?;
	Ok(())
}

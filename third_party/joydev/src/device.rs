use std::fs::{File, OpenOptions};
use std::os::unix::prelude::*;
use std::path::Path;

use arrayref::array_ref;
use input_event_codes::{ABS_CNT, BTN_MISC, KEY_CNT};

use crate::event_codes::{AbsoluteAxis, Key};
use crate::io_control::{
	get_axis_count, get_axis_mapping, get_button_count, get_button_mapping, get_correction_values, get_driver_version,
	get_event, get_identifier, set_axis_mapping, set_button_mapping, set_correction_values,
};
use crate::{Correction, DeviceEvent, Result};

/// Default device abstraction
#[derive(Debug)]
pub struct Device {
	axis_count: u8,
	axis_mapping: Vec<AbsoluteAxis>,
	button_count: u8,
	button_mapping: Vec<Key>,
	driver_version: u32,
	file: File,
	identifier: String,
}

impl Device {
	/// Get axis count
	pub const fn axis_count(&self) -> u8 {
		self.axis_count
	}

	/// Get axes mapping
	pub fn axis_mapping(&self) -> &[AbsoluteAxis; ABS_CNT as usize] {
		array_ref!(self.axis_mapping, 0, ABS_CNT as usize)
	}

	/// Get axis mapping at index
	pub fn axis_mapping_at(&self, number: u8) -> AbsoluteAxis {
		self.axis_mapping[number as usize]
	}

	/// Get button count
	pub const fn button_count(&self) -> u8 {
		self.button_count
	}

	/// Get buttons mapping
	pub fn button_mapping(&self) -> &[Key; (KEY_CNT - BTN_MISC) as usize] {
		array_ref!(self.button_mapping, 0, (KEY_CNT - BTN_MISC) as usize)
	}

	/// Get button mapping at index
	pub fn button_mapping_at(&self, number: u8) -> Key {
		self.button_mapping[number as usize]
	}

	/// Get driver version
	pub fn driver_version(&self) -> u32 {
		self.driver_version
	}

	/// Create new device from raw `fd`
	///
	/// **This function expects the file to be opened at least in read mode. Other flags are optional.** Non-blocking
	/// mode is recommended unless you really don't want it. Other flags shouldn't have any impact.
	///
	/// If the file is not a valid device node the will fail gracefully.
	///
	/// # Safety
	///
	/// Safety is equivalent to that of `std::fs::File::from_raw_fd`.
	pub unsafe fn from_raw_fd(fd: RawFd) -> Result<Self> {
		Self::new(File::from_raw_fd(fd))
	}

	/// Retrieve axes correction values. Wraps [`get_correction_values`](io_control/fn.get_correction_values.html).
	pub fn get_correction_values(&self) -> Result<Vec<Correction>> {
		get_correction_values(self.as_raw_fd())
	}

	/// Read en event. Wraps [`get_event`](io_control/fn.get_event.html).
	pub fn get_event(&self) -> Result<DeviceEvent> {
		Ok(DeviceEvent::from_event(self, get_event(self.as_raw_fd())?))
	}

	/// Get device identifier
	pub fn identifier(&self) -> &str {
		self.identifier.as_str()
	}

	/// Create new device from `file`
	///
	/// **This function expects the file to be opened at least in read mode. Other flags are optional.** Non-blocking
	/// mode is recommended unless you really don't want it. Other flags shouldn't have any impact.
	///
	/// If the file is not a valid device node the will fail gracefully.
	pub fn new(file: File) -> Result<Self> {
		let axis_count = get_axis_count(file.as_raw_fd())?;
		let axis_mapping = get_axis_mapping(file.as_raw_fd())?;
		let button_count = get_button_count(file.as_raw_fd())?;
		let button_mapping = get_button_mapping(file.as_raw_fd())?;
		let driver_version = get_driver_version(file.as_raw_fd())?;
		let identifier = get_identifier(file.as_raw_fd()).unwrap_or_else(|_| "Unknown".to_owned());
		Ok(Self {
			axis_count,
			axis_mapping,
			button_count,
			button_mapping,
			driver_version,
			file,
			identifier,
		})
	}

	/// Create new device by opening file at `path`
	///
	/// **This function always tries opening the file in read-only and non-blocking mode.**
	///
	/// If the file is not a valid device node the will fail gracefully.
	pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
		Self::new(
			OpenOptions::new()
				.custom_flags(libc::O_NONBLOCK)
				.read(true)
				.open(path)?,
		)
	}

	/// Refresh axis mapping. Wraps [`get_axis_mapping`](io_control/fn.get_axis_mapping.html).
	pub fn refresh_axis_mapping(&mut self) -> Result<()> {
		self.axis_mapping = get_axis_mapping(self.as_raw_fd())?;
		Ok(())
	}

	/// Refresh button mapping. Wraps [`get_button_mapping`](io_control/fn.get_button_mapping.html).
	pub fn refresh_button_mapping(&mut self) -> Result<()> {
		self.button_mapping = get_button_mapping(self.as_raw_fd())?;
		Ok(())
	}

	/// Refresh mapping for both axis and buttons
	pub fn refresh_mapping(&mut self) -> Result<()> {
		self.refresh_axis_mapping()?;
		self.refresh_button_mapping()
	}

	/// Set axes mapping. Wraps [`set_axis_mapping`](io_control/fn.set_axis_mapping.html).
	pub fn set_axis_mapping(&mut self, mapping: &[AbsoluteAxis; ABS_CNT as usize]) -> Result<()> {
		set_axis_mapping(self.as_raw_fd(), mapping)?;
		self.refresh_axis_mapping()
	}

	/// Set axis mapping at index. Wraps [`set_axis_mapping`](io_control/fn.set_axis_mapping.html).
	pub fn set_axis_mapping_at(&mut self, number: u8, axis: AbsoluteAxis) -> Result<()> {
		let mut mapping = self.axis_mapping.clone();
		mapping[number as usize] = axis;
		self.set_axis_mapping(array_ref!(mapping, 0, ABS_CNT as usize))
	}

	/// Set buttons mapping. Wraps [`set_button_mapping`](io_control/fn.set_button_mapping.html).
	pub fn set_button_mapping(&mut self, mapping: &[Key; (KEY_CNT - BTN_MISC) as usize]) -> Result<()> {
		set_button_mapping(self.as_raw_fd(), mapping)?;
		self.refresh_button_mapping()
	}

	/// Set button mapping at index. Wraps [`set_button_mapping`](io_control/fn.set_button_mapping.html).
	pub fn set_button_mapping_at(&mut self, number: u8, button: Key) -> Result<()> {
		let mut mapping = self.button_mapping.clone();
		mapping[number as usize] = button;
		self.set_button_mapping(array_ref!(mapping, 0, (KEY_CNT - BTN_MISC) as usize))
	}

	/// Set axes correction values. Wraps [`set_correction_values`](io_control/fn.set_correction_values.html).
	pub fn set_correction_values(&self, mapping: &[Correction; ABS_CNT as usize]) -> Result<()> {
		set_correction_values(self.as_raw_fd(), mapping)
	}
}

impl AsRawFd for Device {
	fn as_raw_fd(&self) -> RawFd {
		self.file.as_raw_fd()
	}
}

impl IntoRawFd for Device {
	fn into_raw_fd(self) -> RawFd {
		self.file.into_raw_fd()
	}
}

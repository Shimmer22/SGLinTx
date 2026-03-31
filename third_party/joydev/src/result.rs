use std::result;

use nix::errno::{Errno, ErrnoSentinel};

use crate::Error;

/// Joydev Result type
pub type Result<T> = result::Result<T, Error>;

pub(crate) fn convert_ioctl_result<S: ErrnoSentinel + PartialEq<S>>(value: S) -> Result<S> {
	if value == S::sentinel() {
		Err(Error::Sys(Errno::last()))
	} else {
		Ok(value)
	}
}

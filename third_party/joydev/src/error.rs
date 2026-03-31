use std::io::Error as IoError;
use std::str::Utf8Error;
use std::{error, fmt};

use nix::errno::Errno;

/// Joydev Error type
#[derive(Debug)]
pub enum Error {
	/// Errors caused by Rust standard io (File::open, Read, etc.).
	Io(IoError),
	/// This can be returned from [`io_control::get_event`](io_control/fn.get_event.html) when the device is open in non-blocking mode and there aren't any events left on the device.
	QueueEmpty,
	/// Errors caused by C string to Rust string conversion.
	String(Utf8Error),
	/// Errors caused by ioctls.
	Sys(Errno),
}

impl error::Error for Error {
	fn description(&self) -> &str {
		match self {
			Self::Io(_) => "IO error",
			Self::QueueEmpty => "Queue empty",
			Self::String(_) => "String error",
			Self::Sys(ref errno) => errno.desc(),
		}
	}

	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Self::Io(ref error) => Some(error),
			Self::String(ref error) => Some(error),
			_ => None,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Io(ref error) => write!(f, "{}", error),
			Self::QueueEmpty => write!(f, "Queue empty"),
			Self::String(ref error) => write!(f, "{}", error),
			Self::Sys(ref errno) => write!(f, "{:?}: {}", errno, errno.desc()),
		}
	}
}

impl From<Errno> for Error {
	fn from(errno: Errno) -> Self {
		Error::Sys(errno)
	}
}

impl From<IoError> for Error {
	fn from(error: IoError) -> Self {
		Self::Io(error)
	}
}

impl From<Utf8Error> for Error {
	fn from(error: Utf8Error) -> Self {
		Error::String(error)
	}
}

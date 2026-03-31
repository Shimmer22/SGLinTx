use std::fmt;
use std::mem::{transmute, zeroed};

use sys::js_event;

use crate::{EventType, GenericEvent};

/// Raw event
///
/// Events lack device context and as a result doesn't provide axis/button mapping. If the mapping is required the event
/// can be mapped using [`DeviceEvent`](enum.DeviceEvent.html) or manually.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Event {
	pub(crate) event: js_event,
}

impl Event {
	/// Returns the event's axis/button.
	pub const fn number(self) -> u8 {
		self.event.number
	}

	/// Returns the event's type.
	pub fn type_(self) -> EventType {
		unsafe { transmute(self.event.type_) }
	}
}

impl Default for Event {
	fn default() -> Self {
		Self {
			event: unsafe { zeroed() },
		}
	}
}

impl fmt::Debug for Event {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"Event {{ number: {:?}, time: {:?}, type: {:?}, value: {:?} }}",
			self.number(),
			self.time(),
			self.type_(),
			self.value()
		)
	}
}

impl GenericEvent for Event {
	fn is_real(&self) -> bool {
		match self.type_() {
			EventType::Axis | EventType::Button => true,
			EventType::AxisSynthetic | EventType::ButtonSynthetic => false,
		}
	}

	fn is_synthetic(&self) -> bool {
		match self.type_() {
			EventType::Axis | EventType::Button => false,
			EventType::AxisSynthetic | EventType::ButtonSynthetic => true,
		}
	}

	fn time(&self) -> u32 {
		self.event.time
	}

	fn value(&self) -> i16 {
		self.event.value
	}
}

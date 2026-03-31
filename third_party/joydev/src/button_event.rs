use crate::event_codes::Key;
use crate::GenericEvent;

/// Button event
///
/// This event is wrapped with mappings for a specific device.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ButtonEvent {
	button: Key,
	is_synthetic: bool,
	time: u32,
	value: i16,
}

impl ButtonEvent {
	/// Returns the event's mapped axis
	pub const fn button(&self) -> Key {
		self.button
	}

	pub(crate) const fn new(button: Key, is_synthetic: bool, time: u32, value: i16) -> Self {
		ButtonEvent {
			button,
			is_synthetic,
			time,
			value,
		}
	}
}

impl GenericEvent for ButtonEvent {
	fn is_real(&self) -> bool {
		!self.is_synthetic
	}

	fn is_synthetic(&self) -> bool {
		self.is_synthetic
	}

	fn time(&self) -> u32 {
		self.time
	}

	fn value(&self) -> i16 {
		self.value
	}
}

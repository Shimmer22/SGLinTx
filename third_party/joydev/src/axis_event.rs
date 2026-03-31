use crate::event_codes::AbsoluteAxis;
use crate::GenericEvent;

/// Axis event
///
/// This event is wrapped with mappings for a specific device.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AxisEvent {
	axis: AbsoluteAxis,
	is_synthetic: bool,
	time: u32,
	value: i16,
}

impl AxisEvent {
	/// Returns the event's mapped axis
	pub const fn axis(self) -> AbsoluteAxis {
		self.axis
	}

	pub(crate) const fn new(axis: AbsoluteAxis, is_synthetic: bool, time: u32, value: i16) -> Self {
		AxisEvent {
			axis,
			is_synthetic,
			time,
			value,
		}
	}
}

impl GenericEvent for AxisEvent {
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

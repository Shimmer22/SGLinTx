use crate::{AxisEvent, ButtonEvent, Device, Event, EventType, GenericEvent};

/// Wrapped event
///
/// Device events wrap axis and button events. This is intended for use with the builtin [`Device`](struct.Device.html) abstraction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeviceEvent {
	/// Axis event
	Axis(AxisEvent),
	/// Button event
	Button(ButtonEvent),
}

impl DeviceEvent {
	/// Creates new `DeviceEvent` from `event` using mappings form `device`.
	pub fn from_event(device: &Device, event: Event) -> Self {
		match event.type_() {
			EventType::Axis => Self::Axis(AxisEvent::new(
				device.axis_mapping_at(event.number()),
				false,
				event.time(),
				event.value(),
			)),
			EventType::AxisSynthetic => Self::Axis(AxisEvent::new(
				device.axis_mapping_at(event.number()),
				true,
				event.time(),
				event.value(),
			)),
			EventType::Button => Self::Button(ButtonEvent::new(
				device.button_mapping_at(event.number()),
				false,
				event.time(),
				event.value(),
			)),
			EventType::ButtonSynthetic => Self::Button(ButtonEvent::new(
				device.button_mapping_at(event.number()),
				true,
				event.time(),
				event.value(),
			)),
		}
	}
}

impl GenericEvent for DeviceEvent {
	fn is_real(&self) -> bool {
		match self {
			Self::Axis(ref event) => event.is_real(),
			Self::Button(ref event) => event.is_real(),
		}
	}

	fn is_synthetic(&self) -> bool {
		match self {
			Self::Axis(ref event) => event.is_synthetic(),
			Self::Button(ref event) => event.is_synthetic(),
		}
	}

	fn time(&self) -> u32 {
		match self {
			Self::Axis(ref event) => event.time(),
			Self::Button(ref event) => event.time(),
		}
	}

	fn value(&self) -> i16 {
		match self {
			Self::Axis(ref event) => event.value(),
			Self::Button(ref event) => event.value(),
		}
	}
}

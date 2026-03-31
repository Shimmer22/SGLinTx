use sys::{JS_EVENT_AXIS, JS_EVENT_BUTTON, JS_EVENT_INIT};

/// Event types
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum EventType {
	/// Real axis event
	Axis = JS_EVENT_AXIS,
	/// Synthetic axis event
	AxisSynthetic = JS_EVENT_AXIS | JS_EVENT_INIT,
	/// Real button event
	Button = JS_EVENT_BUTTON,
	/// Synthetic button event
	ButtonSynthetic = JS_EVENT_BUTTON | JS_EVENT_INIT,
}

use std::marker::PhantomData;

use input_event_codes::*;

use crate::event_codes::{EventCode, IntoIter};

/// Event codes for absolute axes
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum AbsoluteAxis {
	/// Left X axis
	LeftX = ABS_X as u8,
	/// Left Y axis
	LeftY = ABS_Y as u8,
	/// Left Z axis
	LeftZ = ABS_Z as u8,
	/// Right X axis
	RightX = ABS_RX as u8,
	/// Right Y axis
	RightY = ABS_RY as u8,
	/// Right Z axis
	RightZ = ABS_RZ as u8,
	/// Throttle axis
	Throttle = ABS_THROTTLE as u8,
	/// Rudder axis
	Rudder = ABS_RUDDER as u8,
	/// Wheel axis
	Wheel = ABS_WHEEL as u8,
	/// Gas axis
	Gas = ABS_GAS as u8,
	/// Break axis
	Break = ABS_BRAKE as u8,
	/// Unknown axis 0B
	Unknown0B = 0x0b,
	/// Unknown axis 0C
	Unknown0C = 0x0c,
	/// Unknown axis 0D
	Unknown0D = 0x0d,
	/// Unknown axis 0E
	Unknown0E = 0x0e,
	/// Unknown axis 0F
	Unknown0F = 0x0f,
	/// Hat0 X axis
	Hat0X = ABS_HAT0X as u8,
	/// Hat0 Y axis
	Hat0Y = ABS_HAT0Y as u8,
	/// Hat1 X axis
	Hat1X = ABS_HAT1X as u8,
	/// Hat1 Y axis
	Hat1Y = ABS_HAT1Y as u8,
	/// Hat2 X axis
	Hat2X = ABS_HAT2X as u8,
	/// Hat2 Y axis
	Hat2Y = ABS_HAT2Y as u8,
	/// Hat3 X axis
	Hat3X = ABS_HAT3X as u8,
	/// Hat3 Y axis
	Hat3Y = ABS_HAT3Y as u8,
	/// Pressure axis
	Pressure = ABS_PRESSURE as u8,
	/// Distance axis
	Distance = ABS_DISTANCE as u8,
	/// Tilt X axis
	TiltX = ABS_TILT_X as u8,
	/// Tilt Y axis
	TiltY = ABS_TILT_Y as u8,
	/// Tool width axis
	ToolWidth = ABS_TOOL_WIDTH as u8,
	/// Unknown axis 1D
	Unknown1D = 0x1d,
	/// Unknown axis 1E
	Unknown1E = 0x1e,
	/// Unknown axis 1F
	Unknown1F = 0x1f,
	/// Volume axis
	Volume = ABS_VOLUME as u8,
	/// Unknown axis 21
	Unknown21 = 0x21,
	/// Unknown axis 22
	Unknown22 = 0x22,
	/// Unknown axis 23
	Unknown23 = 0x23,
	/// Unknown axis 24
	Unknown24 = 0x24,
	/// Unknown axis 25
	Unknown25 = 0x25,
	/// Unknown axis 26
	Unknown26 = 0x26,
	/// Unknown axis 27
	Unknown27 = 0x27,
	/// Miscellaneous axis
	Miscellaneous = ABS_MISC as u8,
	/// Unknown axis 29
	Unknown29 = 0x29,
	/// Unknown axis 2A
	Unknown2A = 0x2a,
	/// Unknown axis 2B
	Unknown2B = 0x2b,
	/// Unknown axis 2C
	Unknown2C = 0x2c,
	/// Unknown axis 2D
	Unknown2D = 0x2d,
	/// Reserved axis
	Reserved = ABS_RESERVED as u8,
	/// Multi-touch slot axis
	MultiTouchSlot = ABS_MT_SLOT as u8,
	/// Multi-touch touch major axis
	MultiTouchTouchMajor = ABS_MT_TOUCH_MAJOR as u8,
	/// Multi-touch touch minor axis
	MultiTouchTouchMinor = ABS_MT_TOUCH_MINOR as u8,
	/// Multi-touch width major axis
	MultiTouchWidthMajor = ABS_MT_WIDTH_MAJOR as u8,
	/// Multi-touch width minor axis
	MultiTouchWidthMinor = ABS_MT_WIDTH_MINOR as u8,
	/// Multi-touch orientation axis
	MultiTouchOrientation = ABS_MT_ORIENTATION as u8,
	/// Multi-touch position X axis
	MultiTouchPositionX = ABS_MT_POSITION_X as u8,
	/// Multi-touch position Y axis
	MultiTouchPositionY = ABS_MT_POSITION_Y as u8,
	/// Multi-touch tool type axis
	MultiTouchToolType = ABS_MT_TOOL_TYPE as u8,
	/// Multi-touch blob id axis
	MultiTouchBlobId = ABS_MT_BLOB_ID as u8,
	/// Multi-touch tracking id axis
	MultiTouchTrackingId = ABS_MT_TRACKING_ID as u8,
	/// Multi-touch pressure axis
	MultiTouchPressure = ABS_MT_PRESSURE as u8,
	/// Multi-touch distance axis
	MultiTouchDistance = ABS_MT_DISTANCE as u8,
	/// Multi-touch tool X axis
	MultiTouchToolX = ABS_MT_TOOL_X as u8,
	/// Multi-touch tool Y axis
	MultiTouchToolY = ABS_MT_TOOL_Y as u8,
	/// Unknown axis 3E
	Unknown3E = 0x3e,
}

impl Default for AbsoluteAxis {
	fn default() -> Self {
		AbsoluteAxis::LeftX
	}
}

impl EventCode<u8> for AbsoluteAxis {
	const COUNT: u8 = ABS_CNT as u8;
	const MAX: u8 = ABS_MAX as u8;
}

impl IntoIterator for AbsoluteAxis {
	type Item = AbsoluteAxis;
	type IntoIter = IntoIter<AbsoluteAxis, u8>;

	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter {
			phantom: PhantomData,
			value: self as u8,
		}
	}
}

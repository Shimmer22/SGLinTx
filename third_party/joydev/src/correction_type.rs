use sys::{JS_CORR_BROKEN, JS_CORR_NONE};

/// Correction type
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum CorrectionType {
	/// The axis needs correction
	Broken = JS_CORR_BROKEN,
	/// The axis doesn't require correction
	None = JS_CORR_NONE,
}

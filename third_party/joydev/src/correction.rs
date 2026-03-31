use std::fmt;
use std::mem::{transmute, zeroed};

use sys::js_corr;

use crate::CorrectionType;

/// Axis correction
///
/// Correction is used for axis calibration.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Correction {
	pub(crate) corr: js_corr,
}

impl Correction {
	/// Returns coefficient values
	pub const fn coefficient(&self) -> &[i32; 8] {
		&self.corr.coef
	}

	/// Returns coefficient values for editing
	pub fn coefficient_mut(&mut self) -> &mut [i32; 8] {
		&mut self.corr.coef
	}

	/// Creates new correction from `coefficient` values, `precision` and `type`
	pub const fn new(coefficient: &[i32; 8], precision: i16, type_: CorrectionType) -> Self {
		Self {
			corr: js_corr {
				coef: *coefficient,
				prec: precision,
				type_: type_ as u16,
			},
		}
	}

	/// Returns precision
	pub const fn precision(&self) -> i16 {
		self.corr.prec
	}

	/// Set precision
	pub fn set_precision(&mut self, precision: i16) {
		self.corr.prec = precision;
	}

	/// Set type
	pub fn set_type(&mut self, type_: CorrectionType) {
		self.corr.prec = type_ as i16;
	}

	/// Returns type
	pub fn type_(&self) -> CorrectionType {
		unsafe { transmute(self.corr.type_) }
	}
}

impl Default for Correction {
	fn default() -> Self {
		Self {
			corr: unsafe { zeroed() },
		}
	}
}

impl fmt::Debug for Correction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"Correction {{ coefficient: {:?}, precision: {:?}, type: {:?} }}",
			self.coefficient(),
			self.precision(),
			self.type_()
		)
	}
}

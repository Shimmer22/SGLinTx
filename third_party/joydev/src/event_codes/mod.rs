//! Event codes sent by devices
//!
//! Those codes are used for axis and button mapping.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::AddAssign;

pub use self::absolute_axis::AbsoluteAxis;
pub use self::key::Key;
use std::marker::PhantomData;

mod absolute_axis;
mod key;

/// Trait common to all event codes
pub trait EventCode<T>
where
	Self: Clone + Copy + Debug + Default + Eq + Hash + Sized,
	T: EventCodeValue,
{
	/// Code count
	const COUNT: T;
	/// Maximum value
	const MAX: T;

	/// Return the default event code iterator
	fn iter() -> IntoIter<Self, T> {
		IntoIter {
			phantom: PhantomData,
			value: unsafe { *(&Self::default() as *const Self as *const T) },
		}
	}
}

/// Trait common to all event code values
pub trait EventCodeValue
where
	Self: AddAssign + Clone + Copy + Debug + Eq + Hash + Ord + Sized,
{
	/// Value of one
	const ONE: Self;
}

impl EventCodeValue for u8 {
	const ONE: Self = 1u8;
}

impl EventCodeValue for u16 {
	const ONE: Self = 1u16;
}

/// Event code iterator
pub struct IntoIter<T, U>
where
	T: EventCode<U>,
	U: EventCodeValue,
{
	pub(crate) phantom: std::marker::PhantomData<T>,
	pub(crate) value: U,
}

impl<T, U> Iterator for IntoIter<T, U>
where
	T: EventCode<U>,
	U: EventCodeValue,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		if self.value >= T::MAX {
			None
		} else {
			let result = Some(unsafe { *(&self.value as *const U as *const T) });
			self.value += U::ONE;
			result
		}
	}
}

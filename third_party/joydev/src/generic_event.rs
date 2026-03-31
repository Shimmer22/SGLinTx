use std::fmt::Debug;
use std::hash::Hash;

/// Trait common to all events
pub trait GenericEvent
where
	Self: Clone + Copy + Debug + Eq + Hash + PartialEq,
{
	/// Returns `true` if the event is real
	fn is_real(&self) -> bool;

	/// Returns `true` if the event is synthetic
	fn is_synthetic(&self) -> bool;

	/// Returns the event's timestamp
	fn time(&self) -> u32;

	/// Returns the event's value
	fn value(&self) -> i16;
}

//! A Rust wrapper library for joydev devices
//!
//! ## Usage
//!
//! Add this to your `Cargo.tml`:
//!
//! ```toml
//! [dependencies]
//! joydev = "^0.3.0"
//! ```
//!
//! and this to your crate root:
//!
//! ```rust
//! extern crate joydev;
//! ```
//!
//! to get started open a device:
//!
//! ```rust
//! use joydev::Device;
//!
//! // You should probably check what devices are available
//! // by reading /dev/input directory or using udev.
//! if let Ok(device) = Device::open("/dev/input/js0") {
//!     // Get an event and print it.
//!     println!("{:?}", device.get_event());
//! }
//! ```
//!
//! or run the example:
//!
//! ```nix
//! cargo run --example=device
//! ```
#![cfg(target_os = "linux")]
#![deny(missing_docs)]

extern crate arrayref;
extern crate input_event_codes;
extern crate joydev_sys as sys;
extern crate libc;
extern crate nix;

pub use self::axis_event::AxisEvent;
pub use self::button_event::ButtonEvent;
pub use self::correction::Correction;
pub use self::correction_type::CorrectionType;
pub use self::device::Device;
pub use self::device_event::DeviceEvent;
pub use self::error::Error;
pub use self::event::Event;
pub use self::event_type::EventType;
pub use self::generic_event::GenericEvent;
pub use self::result::Result;

mod axis_event;
mod button_event;
mod correction;
mod correction_type;
mod device;
mod device_event;
mod error;
mod event;
pub mod event_codes;
mod event_type;
mod generic_event;
pub mod io_control;
mod ioctl;
mod result;

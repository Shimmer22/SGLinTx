extern crate ctrlc;
extern crate joydev;

use std::fs::OpenOptions;
use std::os::unix::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use joydev::io_control::{
	get_axis_count, get_axis_mapping, get_button_count, get_button_mapping, get_correction_values, get_driver_version,
	get_event, get_identifier,
};
use joydev::Error;

fn main() -> Result<(), Error> {
	let running = Arc::new(AtomicBool::new(true));

	{
		let r = running.clone();
		ctrlc::set_handler(move || {
			r.store(false, Ordering::SeqCst);
		})
		.expect("Error setting Ctrl-C handler");
	}

	let file = OpenOptions::new()
		.custom_flags(libc::O_NONBLOCK)
		.read(true)
		.open("/dev/input/js0")
		.unwrap();
	println!("Axis count: {}", get_axis_count(file.as_raw_fd())?);
	println!("Axis mapping: {:#?}", get_axis_mapping(file.as_raw_fd())?);
	println!("Button count: {}", get_button_count(file.as_raw_fd()).unwrap());
	println!("Button mapping: {:#?}", get_button_mapping(file.as_raw_fd())?);
	println!("Correction values: {:#?}", get_correction_values(file.as_raw_fd())?);
	println!("Driver version: {:x}", get_driver_version(file.as_raw_fd())?);
	println!("Identifier: {}", get_identifier(file.as_raw_fd())?);

	while running.load(Ordering::SeqCst) {
		'inner: loop {
			let event = match get_event(file.as_raw_fd()) {
				Err(error) => match error {
					Error::QueueEmpty => break 'inner,
					_ => panic!("{}: {:?}", "called `Result::unwrap()` on an `Err` value", &error),
				},
				Ok(event) => event,
			};
			println!("{:?}", event);
		}
		//println!("Queue empty");
	}

	Ok(())
}

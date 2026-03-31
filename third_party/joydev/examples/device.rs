extern crate ctrlc;
extern crate joydev;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use joydev::{Device, DeviceEvent, Error};

fn main() -> Result<(), Error> {
	let running = Arc::new(AtomicBool::new(true));

	{
		let r = running.clone();
		ctrlc::set_handler(move || {
			r.store(false, Ordering::SeqCst);
		})
		.expect("Error setting Ctrl-C handler");
	}

	let device = Device::open("/dev/input/js0")?;
	println!("{:#?}", device);

	while running.load(Ordering::SeqCst) {
		'inner: loop {
			let event = match device.get_event() {
				Err(error) => match error {
					Error::QueueEmpty => break 'inner,
					_ => panic!("{}: {:?}", "called `Result::unwrap()` on an `Err` value", &error),
				},
				Ok(event) => event,
			};
			match event {
				DeviceEvent::Axis(ref event) => println!("{:?}", event),
				DeviceEvent::Button(ref event) => println!("{:?}", event),
			}
		}
		//println!("Queue empty");
	}

	Ok(())
}

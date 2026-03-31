[![Pipeline status](https://gitlab.com/gm666q/joydev-rs/badges/master/pipeline.svg)](https://gitlab.com/gm666q/joydev-rs/commits/master)
[![Coverage report](https://gitlab.com/gm666q/joydev-rs/badges/master/coverage.svg)](https://gitlab.com/gm666q/joydev-rs/commits/master)
[![Crates.io](https://img.shields.io/crates/v/joydev)](https://crates.io/crates/joydev)
[![Documentation](https://docs.rs/joydev/badge.svg)](https://docs.rs/joydev)
![License](https://img.shields.io/crates/l/joydev)

# joydev

A Rust wrapper library for joydev devices

## Usage

Add this to your `Cargo.tml`:

```toml
[dependencies]
joydev = "^0.3.0"
```

and this to your crate root:

```rust
extern crate joydev;
```

to get started open a device:

```rust
use joydev::Device;

fn main() {
    // You should probably check what devices are available
    // by reading /dev/input directory or using udev.
    if let Ok(device) = Device::open("/dev/input/js0") {
        // Get an event and print it.
        println!("{:?}", device.get_event());
    }
}
```

or run the example:

```nix
cargo run --example=device
```
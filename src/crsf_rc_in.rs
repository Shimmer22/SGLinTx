use std::time::Duration;
use clap::Parser;
use crsf::{PacketAddress, PacketType, PacketParser, RcChannels};
use rpos::{msg::get_new_tx_of_message, thread_logln};
use crate::{adc::AdcRawMsg, client_process_args};

#[derive(Parser)]
#[command(name="crsf_rc_in", about = "Read RC data from STM32 via CRSF Protocol", long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 420000)] // CRSF typically uses 420k, but can be 115200. Defaulting to standard CRSF.
    baudrate: u32,

    dev_name: String,
}

fn handle_channels(channels: &RcChannels, tx: &rpos::channel::Sender<AdcRawMsg>) {
    // CRSF channels are 0-1984 (11-bit), typically centered at 992.
    // LinTx internal AdcRawMsg typically expects values.
    // Let's assume AdcRawMsg expects raw values similar to what we got before.
    // Previous code: [CH1_L, CH1_H] u16.
    // Let's map CRSF channels [0..15] to AdcRawMsg.value array.
    
    // Note: AdcRawMsg strictly has `value: [i16; 4]` based on previous analysis of stm32_serial.rs and adc.rs
    // But we should probably expand AdcRawMsg to support more channels later.
    // For now, we map the first 4 channels to the existing 4 slots to maintain compatibility.
    
    let mut mapped_values = [0i16; 4];
    
    // CRSF channel order is typically AETR (Aileron, Elevator, Throttle, Rudder) or TAER.
    // Standard CRSF is AETR.
    // LinTx mixer expects:
    // Index 0: Thrust (Throttle)
    // Index 1: Direction (Rudder)
    // Index 2: Aileron
    // Index 3: Elevator
    
    // We need to know the channel mapping of the STM32 source. 
    // Assuming STM32 sends AETR (std CRSF):
    // Ch 0: Aileron
    // Ch 1: Elevator
    // Ch 2: Throttle
    // Ch 3: Rudder
    
    // Mapping to LinTx AdcRawMsg (based on calibrate.rs/mixer.rs usage):
    // AdcRawMsg[0] -> used for Thrust
    // AdcRawMsg[1] -> used for Direction
    // AdcRawMsg[2] -> used for Aileron
    // AdcRawMsg[3] -> used for Elevator
    
    mapped_values[2] = channels.0[0] as i16; // Aileron
    mapped_values[3] = channels.0[1] as i16; // Elevator
    mapped_values[0] = channels.0[2] as i16; // Throttle
    mapped_values[1] = channels.0[3] as i16; // Rudder
    
    tx.send(AdcRawMsg { value: mapped_values });
}

pub fn crsf_rc_in_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    let serial = serialport::new(&args.dev_name, args.baudrate);
    match serial.timeout(Duration::from_millis(100)).open() {
        Ok(mut port) => {
            thread_logln!("crsf_rc_in start on {} @ {} baud", args.dev_name, args.baudrate);
            
            let adc_raw_tx = get_new_tx_of_message::<AdcRawMsg>("adc_raw").unwrap();
            let mut parser = PacketParser::<1024>::new(); // Internal buffer size
            let mut buf = [0u8; 1024];

            loop {
                match port.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        parser.push_bytes(&buf[..n]);
                        while let Some(packet) = parser.next_packet() {
                            match packet {
                                Ok((_addr, crsf::Packet::RcChannels(channels))) => {
                                    handle_channels(&channels, &adc_raw_tx);
                                }
                                Ok(_) =>   {
                                    // Ignore telemetry or other packets for now
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    Ok(_) => {
                        // EOF or no data, sleep a bit
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        // Timeout is fine, just continue
                    }
                    Err(e) => {
                        thread_logln!("Serial read error: {}", e);
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        Err(e) => {
            thread_logln!("Failed to open serial port {}: {}", args.dev_name, e);
        }
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("crsf_rc_in", crsf_rc_in_main);
}

use std::time::Duration;
use clap::Parser;
use crc::{Crc, CRC_8_DVB_S2};
use rpos::{msg::get_new_tx_of_message, thread_logln};
use crate::{messages::AdcRawMsg, client_process_args};

#[derive(Parser)]
#[command(name="stm32_serial", about = "Read data from STM32 via Serial (CRSF-like)", long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 115200)]
    baudrate: u32,

    dev_name: String,
}

#[derive(Debug)]
enum State {
    WaitSync,
    WaitLen,
    WaitPayload,
}

pub fn stm32_serial_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    let serial = serialport::new(&args.dev_name, args.baudrate);
    let mut port = serial.timeout(Duration::from_millis(10)).open().expect("Failed to open serial port");

    let adc_raw_tx = get_new_tx_of_message::<AdcRawMsg>("adc_raw").unwrap();
    let crc_alg = Crc::<u8>::new(&CRC_8_DVB_S2);

    let mut read_buffer = [0u8; 64];
    let mut payload = Vec::with_capacity(64);
    let mut state = State::WaitSync;
    let mut target_len = 0;

    thread_logln!("stm32_serial start on {}!", args.dev_name);

    loop {
        match port.read(&mut read_buffer) {
            Ok(count) => {
                for i in 0..count {
                    let byte = read_buffer[i];
                    match state {
                        State::WaitSync => {
                            if byte == 0x5A {
                                state = State::WaitLen;
                            }
                        }
                        State::WaitLen => {
                            target_len = byte as usize;
                            if target_len > 60 || target_len < 2 {
                                // Invalid length, back to sync
                                state = State::WaitSync;
                            } else {
                                payload.clear();
                                state = State::WaitPayload;
                            }
                        }
                        State::WaitPayload => {
                            payload.push(byte);
                            if payload.len() == target_len {
                                // Validate CRC
                                let data_len = payload.len();
                                let received_crc = payload[data_len - 1];
                                let computed_crc = crc_alg.checksum(&payload[0..data_len - 1]);

                                if computed_crc == received_crc {
                                    handle_packet(&payload, &adc_raw_tx);
                                } else {
                                    // CRC Error, don't log every time to avoid spamming if baudrate is wrong
                                }
                                state = State::WaitSync;
                            }
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => {
                thread_logln!("Serial read error: {}", e);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_packet(payload: &[u8], tx: &rpos::channel::Sender<AdcRawMsg>) {
    if payload.is_empty() { return; }
    let msg_type = payload[0];
    
    match msg_type {
        0x01 => {
            // Joystick packet
            // Payload structure: [Type(1)] [CH1_L, CH1_H] [CH2_L, CH2_H] [CH3_L, CH3_H] [CH4_L, CH4_H] [Buttons(1)] [Reserve(2)] [CRC(1)]
            // Payload passed here contains [Type ... CRC]
            if payload.len() >= 10 { // Type(1) + 4*U16(8) + CRC(1) = 10
                let mut channels = [0i16; 4];
                for i in 0..4 {
                    let start = 1 + i * 2;
                    if start + 1 < payload.len() {
                        channels[i] = u16::from_le_bytes([payload[start], payload[start+1]]) as i16;
                    }
                }
                tx.send(AdcRawMsg { value: channels });
            }
        }
        _ => {
            // Other packet types can be handled here
        }
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("stm32_serial", stm32_serial_main);
}

use crate::{
    client_process_args,
    messages::{
        publish_input_frame, publish_input_status, AdcRawMsg, InputFrameMsg, InputHealth,
        InputSource, InputStatusMsg,
    },
};
use clap::Parser;
use crc::{Crc, CRC_8_DVB_S2};
use rpos::{msg::get_new_tx_of_message, thread_logln};
use serialport::SerialPort;
use std::time::Duration;

const STM32_SYNC_BYTE: u8 = 0x5A;
const STM32_JOYSTICK_MSG_TYPE: u8 = 0x01;
const STM32_JOYSTICK_PAYLOAD_LEN: usize = 12;
const STM32_JOYSTICK_CHANNEL_COUNT: usize = 4;
const STM32_MAX_FRAME_LEN: usize = 60;
const STM32_MIN_FRAME_LEN: usize = 2;
const STM32_MAX_BUFFERED_BYTES: usize = STM32_MAX_FRAME_LEN * 8;
const STM32_REOPEN_DELAY: Duration = Duration::from_millis(500);

#[derive(Parser)]
#[command(
    name="stm32_serial",
    about = "Read TX stick data forwarded by STM32 over the custom serial protocol",
    long_about = None
)]
struct Cli {
    #[arg(short, long, default_value_t = 115200)]
    baudrate: u32,

    dev_name: String,
}

struct Stm32FrameParser {
    crc_alg: Crc<u8>,
    buffer: Vec<u8>,
}

impl Stm32FrameParser {
    fn new() -> Self {
        Self {
            crc_alg: Crc::<u8>::new(&CRC_8_DVB_S2),
            buffer: Vec::with_capacity(STM32_MAX_BUFFERED_BYTES),
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        self.buffer.extend_from_slice(bytes);
        self.trim_buffer_tail();
    }

    fn next_channels(&mut self) -> Option<[i16; STM32_JOYSTICK_CHANNEL_COUNT]> {
        loop {
            self.discard_until_sync();

            if self.buffer.len() < 2 {
                return None;
            }

            let payload_len = self.buffer[1] as usize;
            if !(STM32_MIN_FRAME_LEN..=STM32_MAX_FRAME_LEN).contains(&payload_len) {
                self.buffer.drain(..1);
                continue;
            }

            let frame_len = 2 + payload_len;
            if self.buffer.len() < frame_len {
                return None;
            }

            let payload = &self.buffer[2..frame_len];
            let data_len = payload.len();
            let received_crc = payload[data_len - 1];
            let computed_crc = self.crc_alg.checksum(&payload[..data_len - 1]);
            if computed_crc != received_crc {
                self.buffer.drain(..1);
                continue;
            }

            let parsed = parse_joystick_channels(payload);
            self.buffer.drain(..frame_len);
            if let Some(channels) = parsed {
                return Some(channels);
            }
        }
    }

    fn discard_until_sync(&mut self) {
        if let Some(sync_pos) = self.buffer.iter().position(|byte| *byte == STM32_SYNC_BYTE) {
            if sync_pos > 0 {
                self.buffer.drain(..sync_pos);
            }
        } else {
            self.buffer.clear();
        }
    }

    fn trim_buffer_tail(&mut self) {
        if self.buffer.len() <= STM32_MAX_BUFFERED_BYTES {
            return;
        }

        let keep_from = self.buffer.len().saturating_sub(STM32_MAX_BUFFERED_BYTES);
        if let Some(sync_pos) = self.buffer[keep_from..]
            .iter()
            .position(|byte| *byte == STM32_SYNC_BYTE)
        {
            self.buffer.drain(..keep_from + sync_pos);
        } else {
            self.buffer.clear();
        }
    }
}

pub fn stm32_serial_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    let input_status_tx = get_new_tx_of_message::<InputStatusMsg>("input_status").unwrap();
    let adc_raw_tx = get_new_tx_of_message::<AdcRawMsg>("adc_raw").unwrap();
    let input_frame_tx = get_new_tx_of_message::<InputFrameMsg>("input_frame").unwrap();

    thread_logln!("stm32_serial start on {}!", args.dev_name);

    loop {
        match open_stm32_port(&args.dev_name, args.baudrate) {
            Ok(mut port) => {
                publish_input_status(
                    &input_status_tx,
                    InputSource::Stm32Serial,
                    InputHealth::Running,
                    format!("{} @ {} baud", args.dev_name, args.baudrate),
                    4,
                );

                if let Err(err) = read_stm32_stream(&mut *port, &input_frame_tx, &adc_raw_tx) {
                    publish_input_status(
                        &input_status_tx,
                        InputSource::Stm32Serial,
                        InputHealth::Error,
                        format!("read {} failed: {}", args.dev_name, err),
                        4,
                    );
                    thread_logln!("Serial read error on {}: {}", args.dev_name, err);
                }
            }
            Err(err) => {
                publish_input_status(
                    &input_status_tx,
                    InputSource::Stm32Serial,
                    InputHealth::Error,
                    format!("open {} failed: {}", args.dev_name, err),
                    4,
                );
                thread_logln!("Failed to open serial port {}: {}", args.dev_name, err);
            }
        }

        std::thread::sleep(STM32_REOPEN_DELAY);
    }
}

fn open_stm32_port(
    dev_name: &str,
    baudrate: u32,
) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(dev_name, baudrate)
        .timeout(Duration::from_millis(10))
        .open()
}

fn read_stm32_stream(
    port: &mut dyn SerialPort,
    input_frame_tx: &rpos::channel::Sender<InputFrameMsg>,
    adc_raw_tx: &rpos::channel::Sender<AdcRawMsg>,
) -> std::io::Result<()> {
    let mut read_buffer = [0u8; 64];
    let mut parser = Stm32FrameParser::new();

    loop {
        match port.read(&mut read_buffer) {
            Ok(count) if count > 0 => {
                parser.push_bytes(&read_buffer[..count]);
                while let Some(channels) = parser.next_channels() {
                    publish_input_frame(
                        input_frame_tx,
                        Some(adc_raw_tx),
                        InputSource::Stm32Serial,
                        &channels,
                    );
                }
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => return Err(e),
        }
    }
}

fn parse_joystick_channels(payload: &[u8]) -> Option<[i16; STM32_JOYSTICK_CHANNEL_COUNT]> {
    if payload.len() != STM32_JOYSTICK_PAYLOAD_LEN || payload[0] != STM32_JOYSTICK_MSG_TYPE {
        return None;
    }

    let mut channels = [0i16; STM32_JOYSTICK_CHANNEL_COUNT];
    for (index, channel) in channels.iter_mut().enumerate() {
        let start = 1 + index * 2;
        *channel = u16::from_le_bytes([payload[start], payload[start + 1]]) as i16;
    }

    Some(channels)
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("stm32_serial", stm32_serial_main);
}

#[cfg(test)]
mod tests {
    use super::*;

    const STM32_JOYSTICK_RESERVED_LEN: usize = 2;

    fn build_frame(channels: [u16; STM32_JOYSTICK_CHANNEL_COUNT]) -> Vec<u8> {
        let crc_alg = Crc::<u8>::new(&CRC_8_DVB_S2);
        let mut frame = vec![
            STM32_SYNC_BYTE,
            STM32_JOYSTICK_PAYLOAD_LEN as u8,
            STM32_JOYSTICK_MSG_TYPE,
        ];
        for channel in channels {
            frame.extend_from_slice(&channel.to_le_bytes());
        }
        frame.extend_from_slice(&[0u8; STM32_JOYSTICK_RESERVED_LEN]);
        let crc = crc_alg.checksum(&frame[2..]);
        frame.push(crc);
        frame
    }

    #[test]
    fn test_parse_joystick_channels_from_board_frame() {
        let frame = build_frame([2088, 1541, 2059, 2061]);

        assert_eq!(frame.len(), 14);
        assert_eq!(frame[1], STM32_JOYSTICK_PAYLOAD_LEN as u8);
        assert_eq!(
            parse_joystick_channels(&frame[2..]),
            Some([2088, 1541, 2059, 2061])
        );
    }

    #[test]
    fn test_parse_joystick_channels_rejects_non_joystick_payload() {
        let mut frame = build_frame([2088, 1541, 2059, 2061]);
        frame[2] = 0x02;

        assert_eq!(parse_joystick_channels(&frame[2..]), None);
    }

    #[test]
    fn test_parser_resynchronizes_after_noise_and_bad_crc() {
        let mut parser = Stm32FrameParser::new();
        let frame = build_frame([2088, 1541, 2059, 2061]);
        let mut corrupted = frame.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0xFF;

        let mut input = vec![0x00, 0x11, 0x22, STM32_SYNC_BYTE];
        input.extend_from_slice(&corrupted);
        input.extend_from_slice(&frame);

        parser.push_bytes(&input);

        assert_eq!(parser.next_channels(), Some([2088, 1541, 2059, 2061]));
        assert_eq!(parser.next_channels(), None);
    }

    #[test]
    fn test_parser_handles_fragmented_frame() {
        let mut parser = Stm32FrameParser::new();
        let frame = build_frame([1000, 1200, 1400, 1600]);

        parser.push_bytes(&frame[..5]);
        assert_eq!(parser.next_channels(), None);

        parser.push_bytes(&frame[5..]);
        assert_eq!(parser.next_channels(), Some([1000, 1200, 1400, 1600]));
    }
}

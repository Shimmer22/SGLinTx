use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use crc::{Crc, CRC_8_DVB_S2};
use crsf::{PacketAddress, RawPacket};
use rpos::{
    msg::{get_new_rx_of_message, get_new_tx_of_message},
    pthread_scheduler::SchedulePthread,
    thread_logln,
};

use crate::{client_process_args, messages::SystemStatusMsg, mixer::MixerOutMsg};

const CRSF_SYNC: u8 = 0xC8;
const CRSF_FRAME_BATTERY_ID: u8 = 0x08;
const CRSF_FRAME_LINK_ID: u8 = 0x14;
const CRSF_FRAME_LINK_RX_ID: u8 = 0x1C;
const CRSF_MAX_PACKET_SIZE: usize = 66;
const CRSF_CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_DVB_S2);

#[derive(Parser)]
#[command(name="erls_tx", about = None, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 115200)]
    baudrate: u32,

    dev_name: String,
}

fn new_rc_channel_packet(channel_vals: &[u16; 16]) -> RawPacket {
    let chn = crsf::RcChannels(*channel_vals);
    let packet = crsf::Packet::RcChannels(chn);
    packet.into_raw(PacketAddress::Transmitter)
}

fn gen_magic_packet() -> [u8; 8] {
    let mut data = [0; 8];
    let crc8_alg = Crc::<u8>::new(&CRC_8_DVB_S2);
    data[0] = 0xEE; //ELRS_ADDRESS
    data[1] = 6;
    data[2] = 0x2D; //TYPE_SETTINGS_WRITE
    data[3] = 0xEE;
    data[4] = 0xEA; //  Radio Transmitter
    data[5] = 0x1;
    data[6] = 0x00;
    data[7] = crc8_alg.checksum(&data[2..7]);
    data
}

#[inline]
fn mxier_out_2_crsf(val: u16) -> u16 {
    (val as u32
        * (crsf::RcChannels::CHANNEL_VALUE_MAX - crsf::RcChannels::CHANNEL_VALUE_MIN) as u32
        / 10000
        + crsf::RcChannels::CHANNEL_VALUE_MIN as u32) as u16
}

fn elrs_tx_main(argc: u32, argv: *const &str) {
    let arg_ret = client_process_args::<Cli>(argc, argv);
    if arg_ret.is_none() {
        return;
    }

    let args = arg_ret.unwrap();

    let dev_name = &args.dev_name;
    let serial = serialport::new(dev_name, args.baudrate);
    let mut dev = serial.timeout(Duration::from_millis(1000)).open().unwrap();
    let mut rx = get_new_rx_of_message::<MixerOutMsg>("mixer_out").unwrap();
    let system_status_tx = get_new_tx_of_message::<SystemStatusMsg>("system_status").unwrap();

    let magic_cmd = gen_magic_packet();
    for _ in 0..10 {
        dev.write(&magic_cmd).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    thread_logln!("elrs_tx start!");

    SchedulePthread::new_simple(Box::new(move |_| {
        let mut status_state = TelemetryStatusState::default();
        let mut serial_buf = Vec::<u8>::new();
        let mut read_buf = [0u8; 256];
        let mut crsf_chn_values: [u16; 16] = [0; 16];
        loop {
            if let Some(msg) = rx.try_read() {
                crsf_chn_values[0] = mxier_out_2_crsf(msg.aileron);
                crsf_chn_values[1] = mxier_out_2_crsf(msg.elevator);
                crsf_chn_values[2] = mxier_out_2_crsf(msg.thrust);
                crsf_chn_values[3] = mxier_out_2_crsf(msg.direction);
                let raw_packet = new_rc_channel_packet(&crsf_chn_values);
                if let Err(err) = dev.write(raw_packet.data()) {
                    thread_logln!("elrs_tx write failed: {}", err);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
            }

            match dev.read(&mut read_buf) {
                Ok(n) if n > 0 => {
                    serial_buf.extend_from_slice(&read_buf[..n]);
                    for frame in extract_crsf_frames(&mut serial_buf) {
                        if let Some(status) = status_state.consume_frame(&frame) {
                            system_status_tx.send(status);
                        }
                    }
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
                Err(err) => {
                    thread_logln!("elrs_tx read failed: {}", err);
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }));
}

#[derive(Debug, Clone, Default)]
struct TelemetryStatusState {
    remote_battery_percent: Option<u8>,
    aircraft_battery_percent: Option<u8>,
    signal_strength_percent: Option<u8>,
}

impl TelemetryStatusState {
    fn consume_frame(&mut self, frame: &[u8]) -> Option<SystemStatusMsg> {
        if frame.len() < 5 {
            return None;
        }

        match frame[2] {
            // EdgeTX mapping: LINK_ID payload byte 2 is RX quality (percentage).
            // Full packet index = 3(header offset) + 2 = 5.
            CRSF_FRAME_LINK_ID => {
                if let Some(rx_quality) = frame.get(5).copied() {
                    self.signal_strength_percent = Some(rx_quality.min(100));
                }
            }
            // EdgeTX mapping: LINK_RX_ID payload byte 1 carries RX RSSI percentage.
            // Full packet index = 3(header offset) + 1 = 4.
            CRSF_FRAME_LINK_RX_ID => {
                if let Some(rx_rssi_percent) = frame.get(4).copied() {
                    self.signal_strength_percent = Some(rx_rssi_percent.min(100));
                }
            }
            // EdgeTX mapping: BATTERY_ID payload byte 7 is battery remaining percentage.
            // Full packet index = 3(header offset) + 7 = 10.
            CRSF_FRAME_BATTERY_ID => {
                if let Some(remaining) = frame.get(10).copied() {
                    self.aircraft_battery_percent = Some(remaining.min(100));
                }
            }
            _ => {}
        }

        if self.signal_strength_percent.is_none() && self.aircraft_battery_percent.is_none() {
            return None;
        }

        Some(SystemStatusMsg {
            remote_battery_percent: self.remote_battery_percent.unwrap_or(100),
            aircraft_battery_percent: self.aircraft_battery_percent.unwrap_or(100),
            signal_strength_percent: self.signal_strength_percent.unwrap_or(100),
            unix_time_secs: now_unix_secs(),
        })
    }
}

fn extract_crsf_frames(buf: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut cursor = 0usize;
    while buf.len().saturating_sub(cursor) >= 3 {
        if !matches!(buf[cursor], CRSF_SYNC | 0xEA | 0xEE) {
            cursor += 1;
            continue;
        }

        let packet_len = buf[cursor + 1] as usize + 2;
        if packet_len < 5 || packet_len > CRSF_MAX_PACKET_SIZE {
            cursor += 1;
            continue;
        }
        if cursor + packet_len > buf.len() {
            break;
        }

        let frame = &buf[cursor..cursor + packet_len];
        if check_frame_crc(frame) {
            frames.push(frame.to_vec());
            cursor += packet_len;
        } else {
            cursor += 1;
        }
    }

    if cursor > 0 {
        buf.drain(..cursor);
    }
    frames
}

fn check_frame_crc(frame: &[u8]) -> bool {
    if frame.len() < 5 {
        return false;
    }
    let len = frame[1] as usize;
    let expected = *frame.last().unwrap_or(&0);
    CRSF_CRC.checksum(&frame[2..2 + len - 1]) == expected
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{
        check_frame_crc, extract_crsf_frames, TelemetryStatusState, CRSF_CRC,
        CRSF_FRAME_BATTERY_ID, CRSF_FRAME_LINK_ID, CRSF_SYNC,
    };

    fn build_frame(frame_type: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(CRSF_SYNC);
        frame.push((payload.len() + 2) as u8);
        frame.push(frame_type);
        frame.extend_from_slice(payload);
        frame.push(CRSF_CRC.checksum(&frame[2..]));
        frame
    }

    #[test]
    fn test_extract_crsf_frames_with_noise() {
        let frame = build_frame(CRSF_FRAME_LINK_ID, &[1, 2, 80, 4, 5, 6, 7, 8, 9]);
        let mut buf = vec![0x00, 0x11, 0x22];
        buf.extend_from_slice(&frame);
        let frames = extract_crsf_frames(&mut buf);
        assert_eq!(frames.len(), 1);
        assert!(check_frame_crc(&frames[0]));
        assert!(buf.is_empty());
    }

    #[test]
    fn test_status_update_from_link_and_battery_frames() {
        let mut state = TelemetryStatusState::default();
        let link = build_frame(CRSF_FRAME_LINK_ID, &[0, 0, 75, 0, 0, 0, 0, 0, 0]);
        let batt = build_frame(CRSF_FRAME_BATTERY_ID, &[0, 0, 0, 0, 0, 0, 0, 44]);

        let status1 = state.consume_frame(&link).expect("link status");
        assert_eq!(status1.signal_strength_percent, 75);
        assert_eq!(status1.aircraft_battery_percent, 100);

        let status2 = state.consume_frame(&batt).expect("battery status");
        assert_eq!(status2.signal_strength_percent, 75);
        assert_eq!(status2.aircraft_battery_percent, 44);
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("elrs_tx", elrs_tx_main);
}

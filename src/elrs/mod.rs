use std::time::{Duration, Instant};

use crc::{Crc, CRC_8_DVB_S2};

pub const CRSF_SYNC: u8 = 0xC8;
pub const MODULE_ADDRESS: u8 = 0xEE;
pub const RECEIVER_ADDRESS: u8 = 0xEC;
pub const RADIO_ADDRESS: u8 = 0xEA;
pub const COMMAND_ID: u8 = 0x32;
pub const SUBCOMMAND_CRSF: u8 = 0x10;
pub const SUBCOMMAND_CRSF_BIND: u8 = 0x01;

const CRSF_D5_CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_DVB_S2);
const BIND_FRAME_BURST_COUNT: u8 = 6;
const BIND_FRAME_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElrsOperation {
    EnterBind,
    SetWifiManual(bool),
    SetTxPower(u16),
    SetBindPhrase(String),
    RefreshParams,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElrsOperationStatus {
    Queued(&'static str),
    Busy(&'static str),
    Unsupported(&'static str),
}

impl ElrsOperationStatus {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Queued(msg) | Self::Busy(msg) | Self::Unsupported(msg) => msg,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ElrsProtocolRuntime {
    bind_frames_remaining: u8,
    bind_frames_total: u8,
    last_bind_frame_at: Option<Instant>,
    last_status: Option<String>,
}

impl Default for ElrsProtocolRuntime {
    fn default() -> Self {
        Self {
            bind_frames_remaining: 0,
            bind_frames_total: 0,
            last_bind_frame_at: None,
            last_status: None,
        }
    }
}

impl ElrsProtocolRuntime {
    pub fn request(&mut self, op: ElrsOperation) -> ElrsOperationStatus {
        match op {
            ElrsOperation::EnterBind => {
                if self.bind_active() {
                    ElrsOperationStatus::Busy("Bind already in progress")
                } else {
                    self.bind_frames_remaining = BIND_FRAME_BURST_COUNT;
                    self.bind_frames_total = BIND_FRAME_BURST_COUNT;
                    self.last_bind_frame_at = None;
                    self.last_status = Some("Bind request queued".to_string());
                    ElrsOperationStatus::Queued("Bind request queued")
                }
            }
            ElrsOperation::SetWifiManual(_)
            | ElrsOperation::SetTxPower(_)
            | ElrsOperation::SetBindPhrase(_)
            | ElrsOperation::RefreshParams => {
                ElrsOperationStatus::Unsupported("Operation not wired to CRSF yet")
            }
        }
    }

    pub fn bind_active(&self) -> bool {
        self.bind_frames_remaining > 0
    }

    pub fn clear_ephemeral(&mut self) {
        self.bind_frames_remaining = 0;
        self.bind_frames_total = 0;
        self.last_bind_frame_at = None;
    }

    pub fn poll_outgoing_frame(&mut self, now: Instant) -> Option<Vec<u8>> {
        if self.bind_frames_remaining == 0 {
            return None;
        }

        if let Some(last) = self.last_bind_frame_at {
            if now.saturating_duration_since(last) < BIND_FRAME_INTERVAL {
                return None;
            }
        }

        self.last_bind_frame_at = Some(now);
        self.bind_frames_remaining = self.bind_frames_remaining.saturating_sub(1);
        if self.bind_frames_remaining == 0 {
            self.last_status = Some("Bind command burst sent".to_string());
        }

        Some(build_crossfire_bind_frame(MODULE_ADDRESS))
    }

    pub fn take_status_text(&mut self) -> Option<String> {
        self.last_status.take()
    }

    pub fn bind_progress(&self) -> Option<(u8, u8)> {
        if self.bind_frames_total == 0 {
            None
        } else {
            Some((
                self.bind_frames_total
                    .saturating_sub(self.bind_frames_remaining),
                self.bind_frames_total,
            ))
        }
    }
}

pub fn build_crossfire_bind_frame(destination: u8) -> Vec<u8> {
    let mut frame = Vec::with_capacity(9);
    frame.push(CRSF_SYNC);
    frame.push(7);
    frame.push(COMMAND_ID);
    frame.push(destination);
    frame.push(RADIO_ADDRESS);
    frame.push(SUBCOMMAND_CRSF);
    frame.push(SUBCOMMAND_CRSF_BIND);
    frame.push(crc8_ba(&frame[2..7]));
    frame.push(CRSF_D5_CRC.checksum(&frame[2..8]));
    frame
}

fn crc8_ba(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for byte in data {
        crc ^= *byte;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0xBA;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::{build_crossfire_bind_frame, crc8_ba, ElrsOperation, ElrsProtocolRuntime, MODULE_ADDRESS};

    #[test]
    fn test_crc8_ba_known_value() {
        assert_eq!(crc8_ba(&[0x32, 0xEE, 0xEA, 0x10, 0x01]), 0x14);
    }

    #[test]
    fn test_build_crossfire_bind_frame_matches_expected_layout() {
        assert_eq!(
            build_crossfire_bind_frame(MODULE_ADDRESS),
            vec![0xC8, 0x07, 0x32, 0xEE, 0xEA, 0x10, 0x01, 0x14, 0xEB]
        );
    }

    #[test]
    fn test_bind_request_queues_burst() {
        let mut runtime = ElrsProtocolRuntime::default();
        let status = runtime.request(ElrsOperation::EnterBind);
        assert_eq!(status.message(), "Bind request queued");
        assert!(runtime.bind_active());
    }
}

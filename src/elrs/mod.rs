use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crc::{Crc, CRC_8_DVB_S2};

pub const CRSF_SYNC: u8 = 0xC8;
pub const MODULE_ADDRESS: u8 = 0xEE;
pub const RADIO_ADDRESS: u8 = 0xEA;
pub const ELRS_HANDSET_ADDRESS: u8 = 0xEF;
pub const COMMAND_ID: u8 = 0x32;
pub const SUBCOMMAND_CRSF: u8 = 0x10;
pub const SUBCOMMAND_CRSF_BIND: u8 = 0x01;
pub const COMMAND_MODEL_SELECT_ID: u8 = 0x05;

const CRSF_D5_CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_DVB_S2);
const BIND_FRAME_INTERVAL: Duration = Duration::from_millis(120);
const BIND_SETTLE_INTERVAL: Duration = Duration::from_millis(400);
const PARAM_FRAME_INTERVAL: Duration = Duration::from_millis(80);
const PARAM_MAX_FIELD_ID: u8 = 64;
const PARAM_DEVICE_PING_ID: u8 = 0x28;
const PARAM_DEVICE_INFO_ID: u8 = 0x29;
const PARAM_REQUEST_SETTINGS_ID: u8 = 0x2A;
const PARAM_SETTINGS_ENTRY_ID: u8 = 0x2B;
const PARAM_READ_ID: u8 = 0x2C;
const PARAM_WRITE_ID: u8 = 0x2D;
const PARAM_COMMAND_ID: u8 = 0x2E;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: String,
    pub serial: u32,
    pub hw_version: u32,
    pub sw_version: u32,
    pub field_count: u8,
    pub is_elrs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Folder,
    Select,
    Command,
    Info,
    String,
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamEntry {
    pub id: u8,
    pub parent: u8,
    pub kind: ParamKind,
    pub label: String,
    pub options: Vec<String>,
    pub value: String,
    pub command_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ElrsProtocolRuntime {
    bind_frames_remaining: u8,
    bind_frames_total: u8,
    bind_command_pending: bool,
    last_bind_frame_at: Option<Instant>,
    bind_settle_until: Option<Instant>,
    last_param_frame_at: Option<Instant>,
    last_ping_at: Option<Instant>,
    last_status: Option<String>,
    param_refresh_requested: bool,
    model_id_pending: bool,
    settings_request_pending: bool,
    current_model_id: u8,
    next_param_read_id: u8,
    outgoing_queue: VecDeque<Vec<u8>>,
    device_info: Option<DeviceInfo>,
    params: HashMap<u8, ParamEntry>,
}

impl Default for ElrsProtocolRuntime {
    fn default() -> Self {
        Self {
            bind_frames_remaining: 0,
            bind_frames_total: 0,
            bind_command_pending: false,
            last_bind_frame_at: None,
            bind_settle_until: None,
            last_param_frame_at: None,
            last_ping_at: None,
            last_status: None,
            param_refresh_requested: true,
            model_id_pending: true,
            settings_request_pending: true,
            current_model_id: 0,
            next_param_read_id: 1,
            outgoing_queue: VecDeque::new(),
            device_info: None,
            params: HashMap::new(),
        }
    }
}

impl ElrsProtocolRuntime {
    pub fn request(&mut self, op: ElrsOperation) -> ElrsOperationStatus {
        match op {
            ElrsOperation::EnterBind => {
                if self.bind_active() {
                    ElrsOperationStatus::Busy("Bind already in progress")
                } else if let Some(id) = self.find_command_param(&["Bind"]) {
                    self.outgoing_queue.push_back(build_parameter_command_frame(
                        MODULE_ADDRESS,
                        id,
                        1,
                    ));
                    self.bind_frames_remaining = 1;
                    self.bind_frames_total = 1;
                    self.bind_command_pending = true;
                    self.last_bind_frame_at = None;
                    self.bind_settle_until = None;
                    self.last_status = Some("Bind command queued".to_string());
                    ElrsOperationStatus::Queued("Bind command queued")
                } else {
                    self.request_refresh();
                    ElrsOperationStatus::Unsupported("Bind command unavailable")
                }
            }
            ElrsOperation::SetWifiManual(enable) => {
                let labels: &[&str] = if enable {
                    &["Enable WiFi", "Enter WiFi", "WiFi Update"]
                } else {
                    &["Disable WiFi", "Exit WiFi"]
                };
                if let Some(id) = self.find_command_param(&labels) {
                    self.outgoing_queue.push_back(build_parameter_write_frame(
                        MODULE_ADDRESS,
                        id,
                        &[1],
                    ));
                    self.last_status = Some(if enable {
                        "WiFi command queued".to_string()
                    } else {
                        "WiFi disable queued".to_string()
                    });
                    ElrsOperationStatus::Queued("WiFi command queued")
                } else {
                    self.request_refresh();
                    ElrsOperationStatus::Unsupported("WiFi command unavailable")
                }
            }
            ElrsOperation::SetTxPower(power) => {
                let Some((param_id, param_options)) = self
                    .find_select_param(&["Max Power", "TX Power"])
                    .map(|param| (param.id, param.options.clone()))
                else {
                    self.request_refresh();
                    return ElrsOperationStatus::Unsupported("TX power parameter unavailable");
                };
                let Some(index) = nearest_option_index(&param_options, power) else {
                    self.request_refresh();
                    return ElrsOperationStatus::Unsupported("TX power options unavailable");
                };
                self.outgoing_queue.push_back(build_parameter_write_frame(
                    MODULE_ADDRESS,
                    param_id,
                    &[index as u8],
                ));
                self.last_status =
                    Some(format!("TX power request queued: {}", param_options[index]));
                ElrsOperationStatus::Queued("TX power request queued")
            }
            ElrsOperation::SetBindPhrase(value) => {
                if let Some(id) = self.find_string_param(&["Bind Phrase"]) {
                    self.outgoing_queue.push_back(build_parameter_write_frame(
                        MODULE_ADDRESS,
                        id,
                        value.as_bytes(),
                    ));
                    self.last_status = Some("Bind phrase write queued".to_string());
                    ElrsOperationStatus::Queued("Bind phrase write queued")
                } else {
                    self.request_refresh();
                    ElrsOperationStatus::Unsupported("Bind phrase parameter unavailable")
                }
            }
            ElrsOperation::RefreshParams => {
                self.request_refresh();
                ElrsOperationStatus::Queued("ELRS params refresh queued")
            }
        }
    }

    pub fn bind_active(&self) -> bool {
        self.bind_frames_remaining > 0
            || self
                .bind_settle_until
                .map(|deadline| Instant::now() < deadline)
                .unwrap_or(false)
            || self.bind_command_pending
    }

    pub fn clear_ephemeral(&mut self) {
        self.bind_frames_remaining = 0;
        self.bind_frames_total = 0;
        self.bind_command_pending = false;
        self.last_bind_frame_at = None;
        self.bind_settle_until = None;
        self.outgoing_queue.clear();
    }

    pub fn request_refresh(&mut self) {
        self.param_refresh_requested = true;
        self.model_id_pending = true;
        self.settings_request_pending = true;
        self.last_ping_at = None;
    }

    pub fn set_model_id(&mut self, model_id: u8) {
        if self.current_model_id != model_id {
            self.current_model_id = model_id;
            self.model_id_pending = true;
            self.last_status = Some(format!("ELRS model id set to {}", model_id));
        }
    }

    pub fn poll_outgoing_frame(&mut self, now: Instant) -> Option<Vec<u8>> {
        if self.bind_frames_remaining > 0 && !self.bind_command_pending {
            if let Some(last) = self.last_bind_frame_at {
                if now.saturating_duration_since(last) < BIND_FRAME_INTERVAL {
                    return None;
                }
            }
            self.last_bind_frame_at = Some(now);
            self.bind_frames_remaining = self.bind_frames_remaining.saturating_sub(1);
            if self.bind_frames_remaining == 0 {
                self.bind_settle_until = Some(now + BIND_SETTLE_INTERVAL);
                self.last_status = Some("Bind command burst sent".to_string());
            }
            return Some(build_crossfire_bind_frame(MODULE_ADDRESS));
        }

        if let Some(deadline) = self.bind_settle_until {
            if now < deadline {
                return None;
            }
            self.bind_settle_until = None;
        }

        if let Some(last) = self.last_param_frame_at {
            if now.saturating_duration_since(last) < PARAM_FRAME_INTERVAL {
                return None;
            }
        }

        if let Some(frame) = self.outgoing_queue.pop_front() {
            if self.bind_command_pending {
                self.bind_command_pending = false;
                self.bind_frames_remaining = self.bind_frames_remaining.saturating_sub(1);
                self.bind_settle_until = Some(now + BIND_SETTLE_INTERVAL);
                self.last_status = Some("Bind command sent".to_string());
            }
            self.last_param_frame_at = Some(now);
            return Some(frame);
        }

        if self.model_id_pending {
            self.model_id_pending = false;
            self.last_param_frame_at = Some(now);
            return Some(build_crossfire_model_id_frame(
                MODULE_ADDRESS,
                self.current_model_id,
            ));
        }

        if self.settings_request_pending && self.device_info.is_some() {
            self.settings_request_pending = false;
            self.last_param_frame_at = Some(now);
            return Some(build_request_settings_frame(MODULE_ADDRESS));
        }

        let ping_due = self
            .last_ping_at
            .map(|last| now.saturating_duration_since(last) >= Duration::from_secs(2))
            .unwrap_or(true);
        if (self.param_refresh_requested || self.device_info.is_none()) && ping_due {
            self.last_ping_at = Some(now);
            self.last_param_frame_at = Some(now);
            if self.param_refresh_requested {
                self.param_refresh_requested = false;
                self.next_param_read_id = 1;
                self.params.clear();
            }
            return Some(build_device_ping_frame(MODULE_ADDRESS));
        }

        if self.device_info.is_some() && self.next_param_read_id <= PARAM_MAX_FIELD_ID {
            let frame = build_parameter_read_frame(MODULE_ADDRESS, self.next_param_read_id, 0);
            self.next_param_read_id = self.next_param_read_id.saturating_add(1);
            self.last_param_frame_at = Some(now);
            return Some(frame);
        }

        None
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

    pub fn consume_frame(&mut self, frame: &[u8]) {
        if frame.len() < 5 {
            return;
        }
        match frame[2] {
            PARAM_DEVICE_INFO_ID => {
                if let Some(info) = parse_device_info(frame) {
                    self.next_param_read_id = 1;
                    self.device_info = Some(info.clone());
                    self.last_status = Some(format!("ELRS module detected: {}", info.name));
                }
            }
            PARAM_SETTINGS_ENTRY_ID => {
                if let Some(entry) = parse_parameter_entry(frame) {
                    self.params.insert(entry.id, entry);
                }
            }
            PARAM_COMMAND_ID => {
                if let Some(status) = parse_command_status(frame) {
                    self.last_status = Some(status);
                }
            }
            _ => {}
        }
    }

    pub fn device_info(&self) -> Option<&DeviceInfo> {
        self.device_info.as_ref()
    }

    pub fn select_value_for_labels(&self, labels: &[&str]) -> Option<String> {
        self.find_select_param(labels)
            .map(|param| param.value.clone())
    }

    fn find_command_param(&self, labels: &[&str]) -> Option<u8> {
        self.params
            .values()
            .find(|param| {
                matches!(param.kind, ParamKind::Command)
                    && labels.iter().any(|label| param.label == *label)
            })
            .map(|param| param.id)
    }

    fn find_select_param(&self, labels: &[&str]) -> Option<&ParamEntry> {
        self.params.values().find(|param| {
            matches!(param.kind, ParamKind::Select)
                && labels.iter().any(|label| param.label == *label)
        })
    }

    fn find_string_param(&self, labels: &[&str]) -> Option<u8> {
        self.params
            .values()
            .find(|param| {
                matches!(param.kind, ParamKind::String)
                    && labels.iter().any(|label| param.label == *label)
            })
            .map(|param| param.id)
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

pub fn build_crossfire_model_id_frame(destination: u8, model_id: u8) -> Vec<u8> {
    let mut frame = Vec::with_capacity(10);
    frame.push(CRSF_SYNC);
    frame.push(8);
    frame.push(COMMAND_ID);
    frame.push(destination);
    frame.push(RADIO_ADDRESS);
    frame.push(SUBCOMMAND_CRSF);
    frame.push(COMMAND_MODEL_SELECT_ID);
    frame.push(model_id);
    frame.push(crc8_ba(&frame[2..8]));
    frame.push(CRSF_D5_CRC.checksum(&frame[2..9]));
    frame
}

fn build_device_ping_frame(destination: u8) -> Vec<u8> {
    let _ = destination;
    let mut frame = Vec::with_capacity(6);
    frame.push(CRSF_SYNC);
    frame.push(4);
    frame.push(PARAM_DEVICE_PING_ID);
    frame.push(0x00);
    frame.push(RADIO_ADDRESS);
    frame.push(CRSF_D5_CRC.checksum(&frame[2..]));
    frame
}

fn build_request_settings_frame(destination: u8) -> Vec<u8> {
    build_extended_frame(
        PARAM_REQUEST_SETTINGS_ID,
        &[destination, parameter_handset_address(destination)],
    )
}

fn build_parameter_read_frame(destination: u8, field_id: u8, chunk: u8) -> Vec<u8> {
    build_extended_frame(
        PARAM_READ_ID,
        &[
            destination,
            parameter_handset_address(destination),
            field_id,
            chunk,
        ],
    )
}

fn build_parameter_write_frame(destination: u8, field_id: u8, value: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + value.len());
    payload.extend_from_slice(&[
        destination,
        parameter_handset_address(destination),
        field_id,
    ]);
    payload.extend_from_slice(value);
    build_extended_frame(PARAM_WRITE_ID, &payload)
}

fn build_parameter_command_frame(destination: u8, field_id: u8, status: u8) -> Vec<u8> {
    build_parameter_write_frame(destination, field_id, &[status])
}

fn parameter_handset_address(destination: u8) -> u8 {
    if destination == MODULE_ADDRESS {
        ELRS_HANDSET_ADDRESS
    } else {
        RADIO_ADDRESS
    }
}

fn build_extended_frame(frame_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 6);
    frame.push(CRSF_SYNC);
    frame.push((payload.len() + 2) as u8);
    frame.push(frame_type);
    frame.extend_from_slice(payload);
    frame.push(CRSF_D5_CRC.checksum(&frame[2..]));
    frame
}

fn parse_device_info(frame: &[u8]) -> Option<DeviceInfo> {
    if frame.len() < 10
        || frame[2] != PARAM_DEVICE_INFO_ID
        || frame.get(4).copied()? != MODULE_ADDRESS
    {
        return None;
    }
    let name_size = frame[1].checked_sub(18)? as usize;
    let name_end = 5usize.checked_add(name_size)?;
    let name = String::from_utf8_lossy(frame.get(5..name_end)?).to_string();
    let is_elrs = frame.get(name_end..name_end + 4) == Some(b"ELRS".as_slice());
    let major = frame.get(14 + name_size).copied().unwrap_or_default() as u32;
    let minor = frame.get(15 + name_size).copied().unwrap_or_default() as u32;
    let revision = frame.get(16 + name_size).copied().unwrap_or_default() as u32;
    Some(DeviceInfo {
        is_elrs,
        name,
        serial: 0,
        hw_version: major,
        sw_version: (major << 16) | (minor << 8) | revision,
        field_count: PARAM_MAX_FIELD_ID,
    })
}

fn parse_parameter_entry(frame: &[u8]) -> Option<ParamEntry> {
    if frame.len() < 11 || frame[2] != PARAM_SETTINGS_ENTRY_ID {
        return None;
    }
    let field_id = *frame.get(5)?;
    let chunk = *frame.get(6)?;
    if chunk != 0 {
        return None;
    }
    let parent = *frame.get(7)?;
    let kind_raw = *frame.get(8)?;
    let kind = parse_param_kind(kind_raw);
    let mut cursor = 9usize;
    let label_end = frame[cursor..].iter().position(|byte| *byte == 0)? + cursor;
    let label = String::from_utf8_lossy(&frame[cursor..label_end]).to_string();
    cursor = label_end + 1;

    let mut options = Vec::new();
    let mut value = String::new();
    let mut command_status = None;

    match kind {
        ParamKind::Select => {
            let options_end = frame[cursor..].iter().position(|byte| *byte == 0)? + cursor;
            options = frame[cursor..options_end]
                .split(|byte| *byte == b';')
                .map(|slice| String::from_utf8_lossy(slice).to_string())
                .filter(|item| !item.is_empty())
                .collect();
            cursor = options_end + 1;
            let current = *frame.get(cursor)?;
            value = options
                .get(current as usize)
                .cloned()
                .unwrap_or_else(|| current.to_string());
        }
        ParamKind::Command => {
            if let Some(status_start) = cursor.checked_add(2) {
                if status_start < frame.len().saturating_sub(1) {
                    let status_end = frame[status_start..]
                        .iter()
                        .position(|byte| *byte == 0)
                        .map(|offset| status_start + offset)
                        .unwrap_or(frame.len().saturating_sub(1));
                    let status =
                        String::from_utf8_lossy(&frame[status_start..status_end]).to_string();
                    if !status.is_empty() {
                        value = status.clone();
                        command_status = Some(status);
                    }
                }
            }
        }
        ParamKind::Info | ParamKind::String => {
            let value_end = frame[cursor..]
                .iter()
                .position(|byte| *byte == 0)
                .map(|offset| cursor + offset)
                .unwrap_or(frame.len().saturating_sub(1));
            value = String::from_utf8_lossy(&frame[cursor..value_end]).to_string();
        }
        ParamKind::Folder | ParamKind::Unknown(_) => {}
    }

    Some(ParamEntry {
        id: field_id,
        parent,
        kind,
        label,
        options,
        value,
        command_status,
    })
}

fn parse_command_status(frame: &[u8]) -> Option<String> {
    if frame.len() < 8 || frame[2] != PARAM_COMMAND_ID {
        return None;
    }
    let field_id = *frame.get(5)?;
    let status = *frame.get(6)?;
    let msg = match status {
        0 => format!("ELRS command {} acknowledged", field_id),
        1 => format!("ELRS command {} executing", field_id),
        2 => format!("ELRS command {} finished", field_id),
        3 => format!("ELRS command {} failed", field_id),
        _ => format!("ELRS command {} status {}", field_id, status),
    };
    Some(msg)
}

fn parse_param_kind(kind: u8) -> ParamKind {
    match kind & 0x5F {
        9 => ParamKind::Select,
        10 => ParamKind::String,
        11 => ParamKind::Folder,
        12 => ParamKind::Info,
        13 => ParamKind::Command,
        _ => ParamKind::Unknown(kind),
    }
}

fn nearest_option_index(options: &[String], target_mw: u16) -> Option<usize> {
    options
        .iter()
        .enumerate()
        .filter_map(|(idx, option)| parse_power_value(option).map(|value| (idx, value)))
        .min_by_key(|(_, value)| value.abs_diff(target_mw))
        .map(|(idx, _)| idx)
}

fn parse_power_value(raw: &str) -> Option<u16> {
    let digits: String = raw.chars().filter(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<u16>().ok()
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
    use super::{
        build_crossfire_bind_frame, build_crossfire_model_id_frame, build_parameter_read_frame,
        build_parameter_write_frame, crc8_ba, parse_parameter_entry, ElrsOperation,
        ElrsProtocolRuntime, ParamEntry, ParamKind, ELRS_HANDSET_ADDRESS, MODULE_ADDRESS,
        PARAM_SETTINGS_ENTRY_ID,
    };
    use std::time::{Duration, Instant};

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
    fn test_build_crossfire_model_id_frame_matches_expected_layout() {
        assert_eq!(
            build_crossfire_model_id_frame(MODULE_ADDRESS, 0),
            vec![0xC8, 0x08, 0x32, 0xEE, 0xEA, 0x10, 0x05, 0x00, 0xDC, 0x4D]
        );
    }

    #[test]
    fn test_bind_request_queues_command_when_available() {
        let mut runtime = ElrsProtocolRuntime::default();
        runtime.params.insert(
            17,
            ParamEntry {
                id: 17,
                parent: 0,
                kind: ParamKind::Command,
                label: "Bind".to_string(),
                options: Vec::new(),
                value: String::new(),
                command_status: None,
            },
        );
        let status = runtime.request(ElrsOperation::EnterBind);
        assert_eq!(status.message(), "Bind command queued");
        assert!(runtime.bind_active());
    }

    #[test]
    fn test_parse_select_parameter_entry() {
        let frame = vec![
            0xC8,
            0x18,
            PARAM_SETTINGS_ENTRY_ID,
            0xEA,
            0xEE,
            0x07,
            0x00,
            0x06,
            0x09,
            b'M',
            b'a',
            b'x',
            b' ',
            b'P',
            b'o',
            b'w',
            b'e',
            b'r',
            0,
            b'1',
            b'0',
            b';',
            b'2',
            b'5',
            0,
            0x01,
            0x00,
            0x02,
            0x00,
        ];
        let entry = parse_parameter_entry(&frame).expect("entry");
        assert_eq!(entry.label, "Max Power");
        assert_eq!(entry.options, vec!["10".to_string(), "25".to_string()]);
        assert_eq!(entry.value, "25");
    }

    #[test]
    fn test_build_parameter_read_frame_uses_device_then_handset() {
        assert_eq!(
            build_parameter_read_frame(MODULE_ADDRESS, 0x3E, 0),
            vec![0xC8, 0x06, 0x2C, 0xEE, 0xEF, 0x3E, 0x00, 0x1A]
        );
    }

    #[test]
    fn test_build_parameter_write_frame_contains_field_id_and_value() {
        let frame = build_parameter_write_frame(MODULE_ADDRESS, 7, &[2]);
        assert_eq!(frame[2], 0x2D);
        assert_eq!(frame[3], MODULE_ADDRESS);
        assert_eq!(frame[4], ELRS_HANDSET_ADDRESS);
        assert_eq!(frame[5], 7);
        assert_eq!(frame[6], 2);
    }

    #[test]
    fn test_bind_active_stays_true_during_settle_window() {
        let mut runtime = ElrsProtocolRuntime::default();
        runtime.params.insert(
            17,
            ParamEntry {
                id: 17,
                parent: 0,
                kind: ParamKind::Command,
                label: "Bind".to_string(),
                options: Vec::new(),
                value: String::new(),
                command_status: None,
            },
        );
        runtime.request(ElrsOperation::EnterBind);
        let frame = runtime.poll_outgoing_frame(Instant::now());
        assert!(frame.is_some());
        assert!(runtime.bind_active());
    }

    #[test]
    fn test_model_id_update_changes_next_model_id_frame() {
        let mut runtime = ElrsProtocolRuntime::default();
        runtime.set_model_id(7);
        let frame = runtime
            .poll_outgoing_frame(Instant::now() + Duration::from_secs(1))
            .expect("model id frame");
        assert_eq!(frame, build_crossfire_model_id_frame(MODULE_ADDRESS, 7));
    }

    #[test]
    fn test_bind_phrase_write_queues_string_param_when_available() {
        let mut runtime = ElrsProtocolRuntime::default();
        runtime.params.insert(
            9,
            ParamEntry {
                id: 9,
                parent: 0,
                kind: ParamKind::String,
                label: "Bind Phrase".to_string(),
                options: Vec::new(),
                value: String::new(),
                command_status: None,
            },
        );
        let status = runtime.request(ElrsOperation::SetBindPhrase("abc123".to_string()));
        assert_eq!(status.message(), "Bind phrase write queued");
        let frame = runtime
            .poll_outgoing_frame(Instant::now() + Duration::from_secs(1))
            .expect("queued frame");
        assert_eq!(frame[2], 0x2D);
        assert_eq!(frame[5], 9);
        assert_eq!(&frame[6..12], b"abc123");
    }
}

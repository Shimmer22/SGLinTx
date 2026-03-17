use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

use clap::Parser;
use crc::{Crc, CRC_8_DVB_S2};
use rpos::{
    msg::{get_new_rx_of_message, get_new_tx_of_message},
    thread_logln,
};

use crate::{
    client_process_args,
    messages::{ElrsCommandMsg, ElrsParamEntry, ElrsStateMsg},
};

const CRSF_CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_DVB_S2);
const CRSF_SYNC: u8 = 0xC8;
const CRSF_ADDRESS_BROADCAST: u8 = 0x00;
const CRSF_ADDRESS_RADIO: u8 = 0xEA;
const CRSF_ADDRESS_MODULE: u8 = 0xEE;
const CRSF_ADDRESS_HANDSET: u8 = 0xEF;
const CRSF_ADDRESS_RECEIVER: u8 = 0xEC;
const CRSF_FRAME_PING_DEVICES: u8 = 0x28;
const CRSF_FRAME_DEVICE_INFO: u8 = 0x29;
const CRSF_FRAME_PARAM_ENTRY: u8 = 0x2B;
const CRSF_FRAME_PARAM_READ: u8 = 0x2C;
const CRSF_FRAME_PARAM_WRITE: u8 = 0x2D;
const CRSF_FRAME_COMMAND: u8 = 0x32;
const CRSF_FRAME_ELRS_INFO: u8 = 0x2E;
const CRSF_SUBCMD_CRSF: u8 = 0x10;
const CRSF_SUBCMD_BIND: u8 = 0x01;
const CRSF_MAX_FRAME_LEN: usize = 64;
const CRSF_MAX_PACKET_SIZE: usize = CRSF_MAX_FRAME_LEN + 2;
const CRSF_PARAM_HIDDEN_MASK: u8 = 0x80;
const CRSF_PARAM_TYPE_UINT8: u8 = 0x00;
const CRSF_PARAM_TYPE_TEXT_SELECTION: u8 = 0x09;
const CRSF_PARAM_TYPE_STRING: u8 = 0x0A;
const CRSF_PARAM_TYPE_FOLDER: u8 = 0x0B;
const CRSF_PARAM_TYPE_INFO: u8 = 0x0C;
const CRSF_PARAM_TYPE_COMMAND: u8 = 0x0D;
const DISCOVERY_MISS_LIMIT: u8 = 16;
const STRING_EDIT_MAX_LEN: usize = 32;
const STRING_EDIT_CHARSET: &[u8] =
    b" abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.";

#[derive(Parser)]
#[command(name = "elrs_agent", about = "ELRS configuration state service")]
struct Cli {
    #[arg(long, default_value = "mock")]
    mode: String,

    #[arg(long, default_value = "/dev/ttyS3")]
    dev_name: String,

    #[arg(short, long, default_value_t = 420000)]
    baudrate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockFieldKind {
    Folder,
    Select,
    String,
    Command,
}

#[derive(Debug, Clone)]
struct MockField {
    field_id: u8,
    parent: u8,
    label: &'static str,
    hidden: bool,
    kind: MockFieldKind,
    choices: &'static [&'static str],
    current_idx: usize,
    string_value: String,
}

impl MockField {
    fn display_value(&self) -> String {
        match self.kind {
            MockFieldKind::Folder => ">".to_string(),
            MockFieldKind::Select => self
                .choices
                .get(self.current_idx)
                .copied()
                .unwrap_or("--")
                .to_string(),
            MockFieldKind::String => self.string_value.clone(),
            MockFieldKind::Command => "Run".to_string(),
        }
    }

    fn is_selectable(&self) -> bool {
        !self.hidden
    }
}

#[derive(Debug, Clone)]
struct StringEditState {
    field_id: u8,
    label: String,
    buffer: Vec<u8>,
    cursor: usize,
}

impl StringEditState {
    fn new(field_id: u8, label: String, value: &str) -> Self {
        let mut buffer = value.as_bytes().to_vec();
        if buffer.is_empty() {
            buffer.push(b' ');
        }
        buffer.truncate(STRING_EDIT_MAX_LEN);
        Self {
            field_id,
            label,
            buffer,
            cursor: 0,
        }
    }

    fn buffer_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).trim_end().to_string()
    }

    fn move_cursor(&mut self, delta: isize) {
        if delta < 0 {
            self.cursor = self.cursor.saturating_sub(delta.unsigned_abs());
            return;
        }

        let next = self.cursor.saturating_add(delta as usize);
        if next >= self.buffer.len() && self.buffer.len() < STRING_EDIT_MAX_LEN {
            self.buffer.push(b' ');
        }
        self.cursor = next.min(self.buffer.len().saturating_sub(1));
    }

    fn cycle_char(&mut self, delta: isize) {
        if self.buffer.is_empty() {
            self.buffer.push(b' ');
            self.cursor = 0;
        }
        let current = self.buffer[self.cursor];
        let pos = STRING_EDIT_CHARSET
            .iter()
            .position(|ch| *ch == current)
            .unwrap_or(0) as isize;
        let len = STRING_EDIT_CHARSET.len() as isize;
        let next = (pos + delta).rem_euclid(len) as usize;
        self.buffer[self.cursor] = STRING_EDIT_CHARSET[next];
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleEntry {
    Refresh,
    FolderUp,
    Field(u8),
}

#[derive(Debug, Clone)]
struct MockElrsAgent {
    dev_name: String,
    baudrate: u32,
    selected_idx: usize,
    current_folder: u8,
    folder_stack: Vec<u8>,
    fields: Vec<MockField>,
    edit_state: Option<StringEditState>,
    wifi_running: bool,
    bind_until: Option<Instant>,
    status_text: String,
}

impl MockElrsAgent {
    fn new(dev_name: String, baudrate: u32) -> Self {
        Self {
            dev_name,
            baudrate,
            selected_idx: 1,
            current_folder: 0,
            folder_stack: Vec::new(),
            fields: vec![
                MockField {
                    field_id: 1,
                    parent: 0,
                    label: "General",
                    hidden: false,
                    kind: MockFieldKind::Folder,
                    choices: &[],
                    current_idx: 0,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 2,
                    parent: 0,
                    label: "Tools",
                    hidden: false,
                    kind: MockFieldKind::Folder,
                    choices: &[],
                    current_idx: 0,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 3,
                    parent: 1,
                    label: "Packet Rate",
                    hidden: false,
                    kind: MockFieldKind::Select,
                    choices: &["50Hz", "150Hz", "250Hz", "500Hz"],
                    current_idx: 2,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 4,
                    parent: 1,
                    label: "Telemetry Ratio",
                    hidden: false,
                    kind: MockFieldKind::Select,
                    choices: &["Off", "1:128", "1:64", "1:32", "1:16"],
                    current_idx: 3,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 5,
                    parent: 1,
                    label: "TX Power",
                    hidden: false,
                    kind: MockFieldKind::Select,
                    choices: &["25mW", "100mW", "250mW", "500mW", "1000mW"],
                    current_idx: 1,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 6,
                    parent: 1,
                    label: "Bind Phrase",
                    hidden: false,
                    kind: MockFieldKind::String,
                    choices: &[],
                    current_idx: 0,
                    string_value: "lin-tx-demo".to_string(),
                },
                MockField {
                    field_id: 7,
                    parent: 2,
                    label: "Bind",
                    hidden: false,
                    kind: MockFieldKind::Command,
                    choices: &[],
                    current_idx: 0,
                    string_value: String::new(),
                },
                MockField {
                    field_id: 8,
                    parent: 2,
                    label: "WiFi Update",
                    hidden: false,
                    kind: MockFieldKind::Command,
                    choices: &[],
                    current_idx: 0,
                    string_value: String::new(),
                },
            ],
            edit_state: None,
            wifi_running: false,
            bind_until: None,
            status_text: "ELRS agent ready".to_string(),
        }
    }

    fn handle_command(&mut self, cmd: ElrsCommandMsg) {
        if self.edit_state.is_some() {
            self.handle_edit_command(cmd);
            return;
        }

        match cmd {
            ElrsCommandMsg::Refresh => {
                self.status_text = "Refreshed mock ELRS tree".to_string();
            }
            ElrsCommandMsg::Back => {
                if self.current_folder != 0 {
                    self.current_folder = self.folder_stack.pop().unwrap_or(0);
                    self.focus_first_field();
                    self.status_text = format!("Back to {}", self.current_path());
                }
            }
            ElrsCommandMsg::SelectPrev => {
                self.selected_idx = self.selected_idx.saturating_sub(1);
            }
            ElrsCommandMsg::SelectNext => {
                let max_idx = self.visible_entries().len().saturating_sub(1);
                self.selected_idx = (self.selected_idx + 1).min(max_idx);
            }
            ElrsCommandMsg::ValueDec => self.adjust_selected(-1),
            ElrsCommandMsg::ValueInc => self.adjust_selected(1),
            ElrsCommandMsg::Activate => self.activate_selected(),
        }
        self.normalize_selection();
    }

    fn handle_edit_command(&mut self, cmd: ElrsCommandMsg) {
        let Some(edit) = self.edit_state.as_mut() else {
            return;
        };
        match cmd {
            ElrsCommandMsg::Back => {
                self.status_text = format!("Canceled edit for `{}`", edit.label);
                self.edit_state = None;
            }
            ElrsCommandMsg::SelectPrev => edit.cycle_char(-1),
            ElrsCommandMsg::SelectNext => edit.cycle_char(1),
            ElrsCommandMsg::ValueDec => edit.move_cursor(-1),
            ElrsCommandMsg::ValueInc => edit.move_cursor(1),
            ElrsCommandMsg::Activate => {
                let value = edit.buffer_string();
                if let Some(field) = self
                    .fields
                    .iter_mut()
                    .find(|field| field.field_id == edit.field_id)
                {
                    field.string_value = value.clone();
                }
                self.status_text = format!("Updated `{}` = {}", edit.label, value);
                self.edit_state = None;
            }
            ElrsCommandMsg::Refresh => {}
        }
    }

    fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut entries = vec![VisibleEntry::Refresh];
        if self.current_folder != 0 {
            entries.push(VisibleEntry::FolderUp);
        }
        entries.extend(
            self.fields
                .iter()
                .filter(|field| !field.hidden && field.parent == self.current_folder)
                .map(|field| VisibleEntry::Field(field.field_id)),
        );
        entries
    }

    fn params(&self) -> Vec<ElrsParamEntry> {
        self.visible_entries()
            .into_iter()
            .map(|entry| match entry {
                VisibleEntry::Refresh => ElrsParamEntry {
                    id: "refresh".to_string(),
                    label: "Refresh".to_string(),
                    value: "Scan".to_string(),
                    selectable: true,
                },
                VisibleEntry::FolderUp => ElrsParamEntry {
                    id: "folder_up".to_string(),
                    label: "..".to_string(),
                    value: "Back".to_string(),
                    selectable: true,
                },
                VisibleEntry::Field(field_id) => {
                    let field = self.field(field_id).unwrap();
                    ElrsParamEntry {
                        id: format!("field_{field_id}"),
                        label: field.label.to_string(),
                        value: field.display_value(),
                        selectable: field.is_selectable(),
                    }
                }
            })
            .collect()
    }

    fn activate_selected(&mut self) {
        let selected = self
            .visible_entries()
            .get(self.selected_idx)
            .copied()
            .unwrap_or(VisibleEntry::Refresh);
        match selected {
            VisibleEntry::Refresh => {
                self.status_text = "Refreshed mock ELRS tree".to_string();
            }
            VisibleEntry::FolderUp => {
                self.handle_command(ElrsCommandMsg::Back);
            }
            VisibleEntry::Field(field_id) => {
                let Some(field) = self.field(field_id).cloned() else {
                    return;
                };
                match field.kind {
                    MockFieldKind::Folder => {
                        self.folder_stack.push(self.current_folder);
                        self.current_folder = field.field_id;
                        self.focus_first_field();
                        self.status_text = format!("Entered {}", self.current_path());
                    }
                    MockFieldKind::String => {
                        self.edit_state = Some(StringEditState::new(
                            field.field_id,
                            field.label.to_string(),
                            &field.string_value,
                        ));
                        self.status_text = format!("Editing `{}`", field.label);
                    }
                    MockFieldKind::Command => {
                        if field.label == "Bind" {
                            self.bind_until = Some(Instant::now() + Duration::from_secs(2));
                            self.status_text = "Binding request sent".to_string();
                        } else if field.label == "WiFi Update" {
                            self.wifi_running = !self.wifi_running;
                            self.status_text = if self.wifi_running {
                                "WiFi update mode enabled".to_string()
                            } else {
                                "WiFi update mode disabled".to_string()
                            };
                        }
                    }
                    MockFieldKind::Select => {
                        self.status_text = format!("Applied {}", field.label);
                    }
                }
            }
        }
    }

    fn adjust_selected(&mut self, delta: isize) {
        let Some(VisibleEntry::Field(field_id)) =
            self.visible_entries().get(self.selected_idx).copied()
        else {
            return;
        };
        let Some(field) = self
            .fields
            .iter_mut()
            .find(|field| field.field_id == field_id)
        else {
            return;
        };
        if field.kind != MockFieldKind::Select {
            return;
        }
        let len = field.choices.len() as isize;
        let next = (field.current_idx as isize + delta).clamp(0, len - 1) as usize;
        if next != field.current_idx {
            field.current_idx = next;
            self.status_text = format!("{} -> {}", field.label, field.display_value());
        }
    }

    fn tick(&mut self) {
        if let Some(deadline) = self.bind_until {
            if Instant::now() >= deadline {
                self.bind_until = None;
                self.status_text = "Bind window closed".to_string();
            }
        }
    }

    fn current_path(&self) -> String {
        let mut parts = Vec::new();
        for folder_id in self
            .folder_stack
            .iter()
            .copied()
            .filter(|folder_id| *folder_id != 0)
            .chain(std::iter::once(self.current_folder).filter(|folder_id| *folder_id != 0))
        {
            if let Some(field) = self.field(folder_id) {
                parts.push(field.label.to_string());
            }
        }
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }

    fn focus_first_field(&mut self) {
        self.selected_idx = if self.current_folder == 0 { 1 } else { 2 };
        self.normalize_selection();
    }

    fn normalize_selection(&mut self) {
        let max_idx = self.visible_entries().len().saturating_sub(1);
        self.selected_idx = self.selected_idx.min(max_idx);
    }

    fn field(&self, field_id: u8) -> Option<&MockField> {
        self.fields.iter().find(|field| field.field_id == field_id)
    }

    fn state(&self) -> ElrsStateMsg {
        let packet_rate = self
            .field(3)
            .map(MockField::display_value)
            .unwrap_or_default();
        let telemetry_ratio = self
            .field(4)
            .map(MockField::display_value)
            .unwrap_or_default();
        let tx_power = self
            .field(5)
            .map(MockField::display_value)
            .unwrap_or_default();

        ElrsStateMsg {
            connected: true,
            busy: self.bind_until.is_some(),
            can_leave: self.current_folder == 0 && self.edit_state.is_none(),
            path: self.current_path(),
            editor_active: self.edit_state.is_some(),
            editor_label: self
                .edit_state
                .as_ref()
                .map(|edit| edit.label.clone())
                .unwrap_or_default(),
            editor_buffer: self
                .edit_state
                .as_ref()
                .map(StringEditState::buffer_string)
                .unwrap_or_default(),
            editor_cursor: self
                .edit_state
                .as_ref()
                .map(|edit| edit.cursor)
                .unwrap_or(0),
            module_name: "ExpressLRS TX".to_string(),
            device_name: self.dev_name.clone(),
            version: format!("mock @ {} baud", self.baudrate),
            packet_rate,
            telemetry_ratio,
            tx_power,
            status_text: self.status_text.clone(),
            wifi_running: self.wifi_running,
            selected_idx: self.selected_idx,
            params: self.params(),
        }
    }
}

#[derive(Debug, Clone)]
struct CrsfRuntime {
    dev_name: String,
    baudrate: u32,
    selected_idx: usize,
    connected: bool,
    module_name: String,
    device_name: String,
    version: String,
    packet_rate: String,
    telemetry_ratio: String,
    tx_power: String,
    wifi_running: bool,
    status_text: String,
    current_folder: u8,
    folder_stack: Vec<u8>,
    edit_state: Option<StringEditState>,
    fields: Vec<CrsfField>,
    pending_reads: Vec<u8>,
    active_read: Option<PendingRead>,
    pending_chunks: Option<PendingChunk>,
    discovery_cursor: u8,
    discovery_miss_streak: u8,
    discovery_complete: bool,
    highest_field_id_seen: u8,
    last_ping_at: Instant,
    last_info_at: Option<Instant>,
    last_refresh_at: Instant,
}

impl CrsfRuntime {
    fn new(dev_name: String, baudrate: u32) -> Self {
        Self {
            dev_name: dev_name.clone(),
            baudrate,
            selected_idx: 1,
            connected: false,
            module_name: "ExpressLRS TX".to_string(),
            device_name: dev_name,
            version: "--".to_string(),
            packet_rate: "--".to_string(),
            telemetry_ratio: "--".to_string(),
            tx_power: "--".to_string(),
            wifi_running: false,
            status_text: "Waiting for CRSF device info".to_string(),
            current_folder: 0,
            folder_stack: Vec::new(),
            edit_state: None,
            fields: Vec::new(),
            pending_reads: Vec::new(),
            active_read: None,
            pending_chunks: None,
            discovery_cursor: 1,
            discovery_miss_streak: 0,
            discovery_complete: false,
            highest_field_id_seen: 0,
            last_ping_at: Instant::now() - Duration::from_secs(10),
            last_info_at: None,
            last_refresh_at: Instant::now(),
        }
    }

    fn reset_scan(&mut self, clear_fields: bool) {
        self.pending_reads.clear();
        self.active_read = None;
        self.pending_chunks = None;
        self.discovery_cursor = 1;
        self.discovery_miss_streak = 0;
        self.discovery_complete = false;
        self.highest_field_id_seen = 0;
        self.current_folder = 0;
        self.folder_stack.clear();
        self.edit_state = None;
        self.selected_idx = 1;
        if clear_fields {
            self.fields.clear();
            self.packet_rate = "--".to_string();
            self.telemetry_ratio = "--".to_string();
            self.tx_power = "--".to_string();
        }
    }

    fn visible_entries(&self) -> Vec<VisibleEntry> {
        let mut entries = vec![VisibleEntry::Refresh];
        if self.current_folder != 0 {
            entries.push(VisibleEntry::FolderUp);
        }
        entries.extend(
            self.fields
                .iter()
                .filter(|field| !field.hidden && field.parent == self.current_folder)
                .map(|field| VisibleEntry::Field(field.field_id)),
        );
        entries
    }

    fn params(&self) -> Vec<ElrsParamEntry> {
        self.visible_entries()
            .into_iter()
            .filter_map(|entry| match entry {
                VisibleEntry::Refresh => Some(ElrsParamEntry {
                    id: "refresh".to_string(),
                    label: "Refresh".to_string(),
                    value: if self.discovery_complete {
                        "Done".to_string()
                    } else {
                        format!("Scan {}", self.discovery_cursor)
                    },
                    selectable: true,
                }),
                VisibleEntry::FolderUp => Some(ElrsParamEntry {
                    id: "folder_up".to_string(),
                    label: "..".to_string(),
                    value: "Back".to_string(),
                    selectable: true,
                }),
                VisibleEntry::Field(field_id) => {
                    self.field(field_id).and_then(CrsfField::to_param_entry)
                }
            })
            .collect()
    }

    fn field(&self, field_id: u8) -> Option<&CrsfField> {
        self.fields.iter().find(|field| field.field_id == field_id)
    }

    fn field_mut(&mut self, field_id: u8) -> Option<&mut CrsfField> {
        self.fields
            .iter_mut()
            .find(|field| field.field_id == field_id)
    }

    fn focus_first_field(&mut self) {
        self.selected_idx = if self.current_folder == 0 { 1 } else { 2 };
        self.normalize_selection();
    }

    fn normalize_selection(&mut self) {
        let max_idx = self.visible_entries().len().saturating_sub(1);
        self.selected_idx = self.selected_idx.min(max_idx);
    }

    fn current_path(&self) -> String {
        let mut parts = Vec::new();
        for folder_id in self
            .folder_stack
            .iter()
            .copied()
            .filter(|folder_id| *folder_id != 0)
            .chain(std::iter::once(self.current_folder).filter(|folder_id| *folder_id != 0))
        {
            if let Some(field) = self.field(folder_id) {
                parts.push(field.label.clone());
            }
        }
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }

    fn selected_visible_entry(&self) -> VisibleEntry {
        self.visible_entries()
            .get(self.selected_idx)
            .copied()
            .unwrap_or(VisibleEntry::Refresh)
    }

    fn state(&self, busy: bool) -> ElrsStateMsg {
        ElrsStateMsg {
            connected: self.connected,
            busy,
            can_leave: self.current_folder == 0 && self.edit_state.is_none(),
            path: self.current_path(),
            editor_active: self.edit_state.is_some(),
            editor_label: self
                .edit_state
                .as_ref()
                .map(|edit| edit.label.clone())
                .unwrap_or_default(),
            editor_buffer: self
                .edit_state
                .as_ref()
                .map(StringEditState::buffer_string)
                .unwrap_or_default(),
            editor_cursor: self
                .edit_state
                .as_ref()
                .map(|edit| edit.cursor)
                .unwrap_or(0),
            module_name: self.module_name.clone(),
            device_name: self.device_name.clone(),
            version: self.version.clone(),
            packet_rate: self.packet_rate.clone(),
            telemetry_ratio: self.telemetry_ratio.clone(),
            tx_power: self.tx_power.clone(),
            status_text: self.status_text.clone(),
            wifi_running: self.wifi_running,
            selected_idx: self.selected_idx,
            params: self.params(),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingRead {
    field_id: u8,
    requested_at: Instant,
    chunk: u8,
}

#[derive(Debug, Clone)]
struct PendingChunk {
    field_id: u8,
    next_chunk: u8,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
enum CrsfFieldValue {
    Uint8 {
        current: u8,
        min: u8,
        max: u8,
        default: u8,
        unit: String,
    },
    TextSelection {
        current: u8,
        min: u8,
        max: u8,
        default: u8,
        unit: String,
        options: Vec<String>,
    },
    String {
        current: String,
    },
    Folder,
    Info {
        text: String,
    },
    Command {
        step: u8,
        timeout_10ms: u8,
        info: String,
    },
    Unknown {
        raw_type: u8,
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
struct CrsfField {
    field_id: u8,
    parent: u8,
    hidden: bool,
    label: String,
    value: CrsfFieldValue,
}

impl CrsfField {
    fn to_param_entry(&self) -> Option<ElrsParamEntry> {
        if self.hidden {
            return None;
        }

        Some(ElrsParamEntry {
            id: format!("field_{}", self.field_id),
            label: self.label.clone(),
            value: self.display_value(),
            selectable: self.is_selectable(),
        })
    }

    fn display_value(&self) -> String {
        match &self.value {
            CrsfFieldValue::Uint8 { current, unit, .. } => {
                if unit.is_empty() {
                    current.to_string()
                } else {
                    format!("{current} {unit}")
                }
            }
            CrsfFieldValue::TextSelection {
                current, options, ..
            } => options
                .get(*current as usize)
                .cloned()
                .unwrap_or_else(|| current.to_string()),
            CrsfFieldValue::String { current } => current.clone(),
            CrsfFieldValue::Folder => ">".to_string(),
            CrsfFieldValue::Info { text } => text.clone(),
            CrsfFieldValue::Command { info, step, .. } => {
                if info.is_empty() {
                    format!("Step {step}")
                } else {
                    info.clone()
                }
            }
            CrsfFieldValue::Unknown { raw_type, bytes } => {
                format!("type=0x{raw_type:02x} [{}B]", bytes.len())
            }
        }
    }

    fn is_selectable(&self) -> bool {
        matches!(
            self.value,
            CrsfFieldValue::Uint8 { .. }
                | CrsfFieldValue::TextSelection { .. }
                | CrsfFieldValue::String { .. }
                | CrsfFieldValue::Folder
                | CrsfFieldValue::Command { .. }
        )
    }
}

struct CrsfPort {
    port: Box<dyn serialport::SerialPort>,
    rx_buf: Vec<u8>,
}

impl CrsfPort {
    fn open(path: &str, baudrate: u32) -> std::io::Result<Self> {
        let port = serialport::new(path, baudrate)
            .timeout(Duration::from_millis(20))
            .open()?;
        Ok(Self {
            port,
            rx_buf: Vec::with_capacity(256),
        })
    }

    fn write_frame(&mut self, frame: &[u8]) -> std::io::Result<()> {
        elrs_debug_log(&format!("tx {}", hex_bytes(frame)));
        self.port.write_all(frame)?;
        self.port.flush()
    }

    fn poll_frames(&mut self) -> std::io::Result<Vec<Vec<u8>>> {
        let mut scratch = [0u8; 128];
        match self.port.read(&mut scratch) {
            Ok(n) if n > 0 => self.rx_buf.extend_from_slice(&scratch[..n]),
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => return Err(err),
        }

        Ok(extract_crsf_frames(&mut self.rx_buf))
    }
}

fn elrs_agent_main(argc: u32, argv: *const &str) {
    let Some(args) = client_process_args::<Cli>(argc, argv) else {
        return;
    };

    let state_tx = get_new_tx_of_message::<ElrsStateMsg>("elrs_state").unwrap();
    let mut cmd_rx = get_new_rx_of_message::<ElrsCommandMsg>("elrs_cmd").unwrap();

    match args.mode.as_str() {
        "crsf" | "real" => run_crsf_agent(args, state_tx, &mut cmd_rx),
        "mock" => run_mock_agent(args, state_tx, &mut cmd_rx),
        other => {
            thread_logln!(
                "elrs_agent mode `{}` not implemented, falling back to mock",
                other
            );
            run_mock_agent(args, state_tx, &mut cmd_rx);
        }
    }
}

fn run_mock_agent(
    args: Cli,
    state_tx: rpos::channel::Sender<ElrsStateMsg>,
    cmd_rx: &mut rpos::channel::Receiver<ElrsCommandMsg>,
) {
    let mut agent = MockElrsAgent::new(args.dev_name, args.baudrate);
    state_tx.send(agent.state());
    thread_logln!("elrs_agent start in mock mode");

    loop {
        while let Some(cmd) = cmd_rx.try_read() {
            agent.handle_command(cmd);
        }

        agent.tick();
        state_tx.send(agent.state());
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn run_crsf_agent(
    args: Cli,
    state_tx: rpos::channel::Sender<ElrsStateMsg>,
    cmd_rx: &mut rpos::channel::Receiver<ElrsCommandMsg>,
) {
    let mut port = match CrsfPort::open(&args.dev_name, args.baudrate) {
        Ok(port) => port,
        Err(err) => {
            thread_logln!(
                "elrs_agent failed to open {} @ {}: {}",
                args.dev_name,
                args.baudrate,
                err
            );
            let state = ElrsStateMsg {
                device_name: args.dev_name,
                version: format!("open failed @ {} baud", args.baudrate),
                status_text: err.to_string(),
                ..ElrsStateMsg::default()
            };
            state_tx.send(state);
            return;
        }
    };

    let mut runtime = CrsfRuntime::new(args.dev_name, args.baudrate);
    runtime.status_text = "CRSF port opened, probing module".to_string();
    state_tx.send(runtime.state(false));
    thread_logln!("elrs_agent start in crsf mode");

    let mut bind_busy_until: Option<Instant> = None;
    loop {
        while let Some(cmd) = cmd_rx.try_read() {
            handle_crsf_command(cmd, &mut runtime, &mut port, &mut bind_busy_until);
        }

        match port.poll_frames() {
            Ok(frames) => {
                for frame in frames {
                    elrs_debug_log(&format!("rx {}", hex_bytes(&frame)));
                    handle_crsf_frame(&frame, &mut runtime);
                }
            }
            Err(err) => {
                runtime.connected = false;
                runtime.status_text = format!("Serial read error: {}", err);
            }
        }

        if runtime.last_ping_at.elapsed() >= Duration::from_secs(1) {
            if let Err(err) = port.write_frame(&build_ping_frame()) {
                runtime.connected = false;
                runtime.status_text = format!("Ping failed: {}", err);
            } else {
                runtime.last_ping_at = Instant::now();
            }
        }

        if runtime.connected {
            drive_param_discovery(&mut runtime, &mut port);
        }

        if let Some(last_info) = runtime.last_info_at {
            if last_info.elapsed() > Duration::from_secs(3) {
                runtime.connected = false;
                runtime.status_text = "ELRS module info timeout".to_string();
            }
        }

        if let Some(until) = bind_busy_until {
            if Instant::now() >= until {
                bind_busy_until = None;
                runtime.status_text = "Bind command sent".to_string();
            }
        }

        state_tx.send(runtime.state(bind_busy_until.is_some()));
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn handle_crsf_command(
    cmd: ElrsCommandMsg,
    runtime: &mut CrsfRuntime,
    port: &mut CrsfPort,
    bind_busy_until: &mut Option<Instant>,
) {
    if runtime.edit_state.is_some() {
        handle_string_edit_command(cmd, runtime, port);
        return;
    }

    match cmd {
        ElrsCommandMsg::SelectPrev => {
            runtime.selected_idx = runtime.selected_idx.saturating_sub(1);
        }
        ElrsCommandMsg::SelectNext => {
            let max_idx = runtime.visible_entries().len().saturating_sub(1);
            runtime.selected_idx = (runtime.selected_idx + 1).min(max_idx);
        }
        ElrsCommandMsg::Back => {
            if runtime.current_folder != 0 {
                runtime.current_folder = runtime.folder_stack.pop().unwrap_or(0);
                runtime.focus_first_field();
                runtime.status_text = format!("Back to {}", runtime.current_path());
            }
        }
        ElrsCommandMsg::Refresh => {
            runtime.reset_scan(true);
            if let Err(err) = port.write_frame(&build_ping_frame()) {
                runtime.status_text = format!("Refresh failed: {}", err);
            } else {
                runtime.last_ping_at = Instant::now();
                runtime.last_refresh_at = Instant::now();
                runtime.status_text = "CRSF ping sent, parameter scan reset".to_string();
            }
        }
        ElrsCommandMsg::Activate => match runtime.selected_visible_entry() {
            VisibleEntry::Refresh => {
                if let Err(err) = port.write_frame(&build_ping_frame()) {
                    runtime.status_text = format!("Refresh failed: {}", err);
                } else {
                    runtime.last_ping_at = Instant::now();
                    runtime.status_text = "CRSF ping sent".to_string();
                }
            }
            VisibleEntry::FolderUp => {
                if runtime.current_folder != 0 {
                    runtime.current_folder = runtime.folder_stack.pop().unwrap_or(0);
                    runtime.focus_first_field();
                    runtime.status_text = format!("Back to {}", runtime.current_path());
                }
            }
            VisibleEntry::Field(field_id) => {
                let Some(field) = runtime.field(field_id).cloned() else {
                    return;
                };
                match &field.value {
                    CrsfFieldValue::Folder => {
                        runtime.folder_stack.push(runtime.current_folder);
                        runtime.current_folder = field.field_id;
                        runtime.focus_first_field();
                        schedule_folder_refresh(runtime, field.field_id);
                        runtime.status_text = format!("Entered {}", runtime.current_path());
                    }
                    CrsfFieldValue::String { current } => {
                        runtime.edit_state = Some(StringEditState::new(
                            field.field_id,
                            field.label.clone(),
                            current,
                        ));
                        runtime.status_text = format!("Editing `{}`", field.label);
                    }
                    CrsfFieldValue::Command { step, .. } => {
                        let next_step = if *step == 3 { 4 } else { 1 };
                        match port
                            .write_frame(&build_param_write_u8_frame(field.field_id, next_step))
                        {
                            Ok(_) => {
                                *bind_busy_until =
                                    Some(Instant::now() + Duration::from_millis(500));
                                runtime.status_text = format!("Command `{}` sent", field.label);
                                schedule_sibling_refresh(runtime, field.parent);
                            }
                            Err(err) => {
                                runtime.status_text =
                                    format!("Command `{}` failed: {}", field.label, err);
                            }
                        }
                    }
                    _ => {
                        runtime.status_text = format!("Field `{}` is not an action", field.label);
                    }
                }
            }
        },
        ElrsCommandMsg::ValueDec | ElrsCommandMsg::ValueInc => {
            let delta = if matches!(cmd, ElrsCommandMsg::ValueInc) {
                1
            } else {
                -1
            };
            let VisibleEntry::Field(field_id) = runtime.selected_visible_entry() else {
                return;
            };
            let Some(field) = runtime.field(field_id).cloned() else {
                return;
            };
            match next_field_value(&field, delta) {
                Some(next) => match port
                    .write_frame(&build_param_write_u8_frame(field.field_id, next))
                {
                    Ok(_) => {
                        apply_numeric_update(runtime, field.field_id, next);
                        runtime.status_text =
                            format!("Update `{}` -> {}", field.label, field.display_value());
                        schedule_sibling_refresh(runtime, field.parent);
                    }
                    Err(err) => {
                        runtime.status_text = format!("Update `{}` failed: {}", field.label, err);
                    }
                },
                None => {
                    runtime.status_text = format!("Field `{}` is read-only", field.label);
                }
            }
        }
    }

    runtime.normalize_selection();
}

fn handle_string_edit_command(cmd: ElrsCommandMsg, runtime: &mut CrsfRuntime, port: &mut CrsfPort) {
    match cmd {
        ElrsCommandMsg::Back => {
            if let Some(edit) = runtime.edit_state.as_ref() {
                runtime.status_text = format!("Canceled edit for `{}`", edit.label);
                runtime.edit_state = None;
            }
        }
        ElrsCommandMsg::SelectPrev => {
            if let Some(edit) = runtime.edit_state.as_mut() {
                edit.cycle_char(-1);
            }
        }
        ElrsCommandMsg::SelectNext => {
            if let Some(edit) = runtime.edit_state.as_mut() {
                edit.cycle_char(1);
            }
        }
        ElrsCommandMsg::ValueDec => {
            if let Some(edit) = runtime.edit_state.as_mut() {
                edit.move_cursor(-1);
            }
        }
        ElrsCommandMsg::ValueInc => {
            if let Some(edit) = runtime.edit_state.as_mut() {
                edit.move_cursor(1);
            }
        }
        ElrsCommandMsg::Refresh => {}
        ElrsCommandMsg::Activate => {
            let Some((field_id, label, value)) = runtime
                .edit_state
                .as_ref()
                .map(|edit| (edit.field_id, edit.label.clone(), edit.buffer_string()))
            else {
                return;
            };
            match port.write_frame(&build_param_write_string_frame(field_id, &value)) {
                Ok(_) => {
                    if let Some(field) = runtime.field_mut(field_id) {
                        field.value = CrsfFieldValue::String {
                            current: value.clone(),
                        };
                    }
                    schedule_field_refresh(runtime, field_id);
                    runtime.status_text = format!("Updated `{}` = {}", label, value);
                    runtime.edit_state = None;
                }
                Err(err) => {
                    runtime.status_text = format!("String write failed: {}", err);
                }
            }
        }
    }
}

fn handle_crsf_frame(frame: &[u8], runtime: &mut CrsfRuntime) {
    if frame.len() < 5 {
        return;
    }

    match frame[2] {
        CRSF_FRAME_DEVICE_INFO => parse_device_info(frame, runtime),
        CRSF_FRAME_ELRS_INFO => parse_elrs_info(frame, runtime),
        CRSF_FRAME_PARAM_ENTRY => parse_param_entry(frame, runtime),
        _ => {}
    }
}

fn parse_device_info(frame: &[u8], runtime: &mut CrsfRuntime) {
    if frame.len() < 8 || frame.get(4).copied() != Some(CRSF_ADDRESS_MODULE) {
        return;
    }

    let payload_len = frame[1] as usize;
    if payload_len < 18 || frame.len() < payload_len + 2 {
        return;
    }

    let name_end = frame[5..frame.len().saturating_sub(4)]
        .iter()
        .position(|byte| *byte == 0)
        .map(|idx| 5 + idx)
        .unwrap_or(5);
    let name = String::from_utf8_lossy(&frame[5..name_end])
        .trim()
        .to_string();
    let name_size = payload_len.saturating_sub(18);
    let version_idx = 5 + name_size + 9;
    if version_idx + 2 < frame.len() {
        runtime.version = format!(
            "{}.{}.{}",
            frame[version_idx],
            frame[version_idx + 1],
            frame[version_idx + 2]
        );
    }
    if !name.is_empty() {
        runtime.module_name = name.clone();
        runtime.device_name = name;
    }
    runtime.connected = true;
    runtime.last_info_at = Some(Instant::now());
    runtime.status_text = format!(
        "ELRS device detected on {} @ {}",
        runtime.dev_name, runtime.baudrate
    );
    if runtime.fields.is_empty() {
        runtime.reset_scan(false);
    }
}

fn parse_elrs_info(frame: &[u8], runtime: &mut CrsfRuntime) {
    if frame.len() < 8 {
        return;
    }

    runtime.connected = true;
    runtime.last_info_at = Some(Instant::now());

    let bad_packets = frame.get(4).copied().unwrap_or(0);
    let good_packets = frame
        .get(5..7)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .unwrap_or(0);
    let flags = frame.get(7).copied().unwrap_or(0);
    runtime.wifi_running = flags & 0x01 != 0;
    runtime.status_text = format!(
        "ELRS info good={} bad={} flags=0x{:02x}",
        good_packets, bad_packets, flags
    );
}

fn parse_param_entry(frame: &[u8], runtime: &mut CrsfRuntime) {
    if frame.len() < 8 || frame[3] != CRSF_ADDRESS_RADIO {
        return;
    }

    let field_id = frame[5];
    let chunks_remaining = frame[6];
    let continuation = frame[7..frame.len() - 1].to_vec();

    match &mut runtime.pending_chunks {
        Some(pending) if pending.field_id == field_id => {
            pending.bytes.extend_from_slice(&continuation);
            if chunks_remaining == 0 {
                let assembled = pending.bytes.clone();
                runtime.pending_chunks = None;
                finish_field_parse(runtime, field_id, &assembled);
                runtime.active_read = None;
            } else {
                let chunk_num = pending.next_chunk;
                pending.next_chunk = pending.next_chunk.saturating_add(1);
                runtime.active_read = Some(PendingRead {
                    field_id,
                    requested_at: Instant::now(),
                    chunk: chunk_num,
                });
            }
        }
        _ => {
            if chunks_remaining == 0 {
                finish_field_parse(runtime, field_id, &continuation);
                runtime.active_read = None;
            } else {
                runtime.pending_chunks = Some(PendingChunk {
                    field_id,
                    next_chunk: 1,
                    bytes: continuation,
                });
                runtime.active_read = Some(PendingRead {
                    field_id,
                    requested_at: Instant::now(),
                    chunk: 1,
                });
            }
        }
    }
}

fn finish_field_parse(runtime: &mut CrsfRuntime, field_id: u8, bytes: &[u8]) {
    match parse_field_bytes(field_id, bytes) {
        Some(field) => {
            runtime.discovery_miss_streak = 0;
            runtime.discovery_complete = false;
            runtime.highest_field_id_seen = runtime.highest_field_id_seen.max(field_id);
            runtime.discovery_cursor = runtime.discovery_cursor.max(field_id.saturating_add(1));
            upsert_field(runtime, field);
        }
        None => {
            elrs_debug_log(&format!(
                "parse field failed id={} payload={}",
                field_id,
                hex_bytes(bytes)
            ));
        }
    }
}

fn parse_field_bytes(field_id: u8, bytes: &[u8]) -> Option<CrsfField> {
    if bytes.len() < 2 {
        return None;
    }

    let parent = bytes[0];
    let hidden = bytes[1] & CRSF_PARAM_HIDDEN_MASK != 0;
    let field_type = bytes[1] & !CRSF_PARAM_HIDDEN_MASK;
    let mut idx = 2usize;
    let label = read_c_string(bytes, &mut idx)?;

    let value = match field_type {
        CRSF_PARAM_TYPE_UINT8 => {
            if idx + 3 >= bytes.len() {
                return None;
            }
            let current = bytes[idx];
            let min = bytes[idx + 1];
            let max = bytes[idx + 2];
            let default = bytes[idx + 3];
            idx += 4;
            let unit = read_c_string(bytes, &mut idx).unwrap_or_default();
            CrsfFieldValue::Uint8 {
                current,
                min,
                max,
                default,
                unit,
            }
        }
        CRSF_PARAM_TYPE_TEXT_SELECTION => {
            let options_raw = read_c_string(bytes, &mut idx).unwrap_or_default();
            if idx + 3 >= bytes.len() {
                return None;
            }
            let current = bytes[idx];
            let min = bytes[idx + 1];
            let max = bytes[idx + 2];
            let default = bytes[idx + 3];
            idx += 4;
            let unit = read_c_string(bytes, &mut idx).unwrap_or_default();
            CrsfFieldValue::TextSelection {
                current,
                min,
                max,
                default,
                unit,
                options: options_raw.split(';').map(|s| s.to_string()).collect(),
            }
        }
        CRSF_PARAM_TYPE_STRING => {
            let current = read_c_string(bytes, &mut idx).unwrap_or_default();
            CrsfFieldValue::String { current }
        }
        CRSF_PARAM_TYPE_FOLDER => CrsfFieldValue::Folder,
        CRSF_PARAM_TYPE_INFO => {
            let text = read_c_string(bytes, &mut idx).unwrap_or_default();
            CrsfFieldValue::Info { text }
        }
        CRSF_PARAM_TYPE_COMMAND => {
            if idx + 1 >= bytes.len() {
                return None;
            }
            let step = bytes[idx];
            let timeout_10ms = bytes[idx + 1];
            idx += 2;
            let info = read_c_string(bytes, &mut idx).unwrap_or_default();
            CrsfFieldValue::Command {
                step,
                timeout_10ms,
                info,
            }
        }
        raw_type => CrsfFieldValue::Unknown {
            raw_type,
            bytes: bytes[idx..].to_vec(),
        },
    };

    Some(CrsfField {
        field_id,
        parent,
        hidden,
        label,
        value,
    })
}

fn upsert_field(runtime: &mut CrsfRuntime, field: CrsfField) {
    let label_lower = field.label.to_ascii_lowercase();
    match label_lower.as_str() {
        "packet rate" => runtime.packet_rate = field.display_value(),
        "telem ratio" | "telemetry ratio" => runtime.telemetry_ratio = field.display_value(),
        "tx power" | "max power" => runtime.tx_power = field.display_value(),
        _ => {}
    }

    if let Some(existing) = runtime
        .fields
        .iter_mut()
        .find(|entry| entry.field_id == field.field_id)
    {
        *existing = field;
    } else {
        runtime.fields.push(field);
        runtime.fields.sort_by_key(|entry| entry.field_id);
    }
    runtime.normalize_selection();
}

fn drive_param_discovery(runtime: &mut CrsfRuntime, port: &mut CrsfPort) {
    if let Some(active) = &runtime.active_read {
        if active.requested_at.elapsed() < Duration::from_millis(250) {
            return;
        }

        if active.chunk > 0 {
            if let Err(err) =
                port.write_frame(&build_param_read_frame(active.field_id, active.chunk))
            {
                runtime.status_text = format!("Chunk read failed: {}", err);
            } else {
                runtime.active_read = Some(PendingRead {
                    field_id: active.field_id,
                    requested_at: Instant::now(),
                    chunk: active.chunk,
                });
            }
            return;
        }

        runtime.discovery_miss_streak = runtime.discovery_miss_streak.saturating_add(1);
        if runtime.discovery_miss_streak >= DISCOVERY_MISS_LIMIT
            && runtime.discovery_cursor > runtime.highest_field_id_seen.saturating_add(4)
        {
            runtime.discovery_complete = true;
        }
        runtime.active_read = None;
    }

    if runtime.pending_chunks.is_some() {
        return;
    }

    let next_field_id = if let Some(field_id) = runtime.pending_reads.first().copied() {
        runtime.pending_reads.remove(0);
        Some(field_id)
    } else if !runtime.discovery_complete && runtime.discovery_cursor != u8::MAX {
        let field_id = runtime.discovery_cursor;
        runtime.discovery_cursor = runtime.discovery_cursor.saturating_add(1);
        Some(field_id)
    } else {
        None
    };

    if let Some(field_id) = next_field_id {
        if let Err(err) = port.write_frame(&build_param_read_frame(field_id, 0)) {
            runtime.status_text = format!("Parameter read failed: {}", err);
        } else {
            runtime.active_read = Some(PendingRead {
                field_id,
                requested_at: Instant::now(),
                chunk: 0,
            });
        }
    }
}

fn schedule_field_refresh(runtime: &mut CrsfRuntime, field_id: u8) {
    push_pending_read(runtime, field_id);
    if let Some(field) = runtime.field(field_id) {
        schedule_sibling_refresh(runtime, field.parent);
    }
}

fn schedule_sibling_refresh(runtime: &mut CrsfRuntime, parent: u8) {
    let mut ids = runtime
        .fields
        .iter()
        .filter(|field| field.parent == parent || field.field_id == parent)
        .map(|field| field.field_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    for id in ids.into_iter().rev() {
        push_pending_read_front(runtime, id);
    }
    runtime.active_read = None;
    runtime.pending_chunks = None;
}

fn schedule_folder_refresh(runtime: &mut CrsfRuntime, folder_id: u8) {
    push_pending_read_front(runtime, folder_id);
    for field_id in runtime
        .fields
        .iter()
        .filter(|field| field.parent == folder_id)
        .map(|field| field.field_id)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        push_pending_read_front(runtime, field_id);
    }
}

fn push_pending_read(runtime: &mut CrsfRuntime, field_id: u8) {
    if !runtime.pending_reads.contains(&field_id) {
        runtime.pending_reads.push(field_id);
    }
}

fn push_pending_read_front(runtime: &mut CrsfRuntime, field_id: u8) {
    if let Some(idx) = runtime.pending_reads.iter().position(|id| *id == field_id) {
        runtime.pending_reads.remove(idx);
    }
    runtime.pending_reads.insert(0, field_id);
}

fn next_field_value(field: &CrsfField, delta: i16) -> Option<u8> {
    match &field.value {
        CrsfFieldValue::Uint8 {
            current, min, max, ..
        } => Some((*current as i16 + delta).clamp(*min as i16, *max as i16) as u8),
        CrsfFieldValue::TextSelection {
            current, min, max, ..
        } => Some((*current as i16 + delta).clamp(*min as i16, *max as i16) as u8),
        _ => None,
    }
}

fn apply_numeric_update(runtime: &mut CrsfRuntime, field_id: u8, next: u8) {
    if let Some(field) = runtime.field_mut(field_id) {
        match &mut field.value {
            CrsfFieldValue::Uint8 { current, .. } => *current = next,
            CrsfFieldValue::TextSelection { current, .. } => *current = next,
            _ => {}
        }
        let label_lower = field.label.to_ascii_lowercase();
        match label_lower.as_str() {
            "packet rate" => runtime.packet_rate = field.display_value(),
            "telem ratio" | "telemetry ratio" => runtime.telemetry_ratio = field.display_value(),
            "tx power" | "max power" => runtime.tx_power = field.display_value(),
            _ => {}
        }
    }
}

fn read_c_string(bytes: &[u8], idx: &mut usize) -> Option<String> {
    let start = *idx;
    let end = bytes.get(start..)?.iter().position(|byte| *byte == 0)? + start;
    *idx = end + 1;
    Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
}

fn extract_crsf_frames(buf: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut cursor = 0usize;

    while buf.len().saturating_sub(cursor) >= 3 {
        if !matches!(
            buf[cursor],
            CRSF_ADDRESS_RADIO | CRSF_SYNC | CRSF_ADDRESS_MODULE
        ) {
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

fn build_ping_frame() -> Vec<u8> {
    build_crsf_frame(
        CRSF_SYNC,
        CRSF_FRAME_PING_DEVICES,
        &[CRSF_ADDRESS_BROADCAST, CRSF_ADDRESS_RADIO],
    )
}

fn build_bind_frame(telemetry_streaming: bool) -> Vec<u8> {
    let destination = if telemetry_streaming {
        CRSF_ADDRESS_RECEIVER
    } else {
        CRSF_ADDRESS_MODULE
    };
    build_crsf_command_frame(
        CRSF_SYNC,
        &[
            destination,
            CRSF_ADDRESS_RADIO,
            CRSF_SUBCMD_CRSF,
            CRSF_SUBCMD_BIND,
        ],
    )
}

fn build_param_read_frame(field_id: u8, chunk: u8) -> Vec<u8> {
    build_crsf_frame(
        CRSF_SYNC,
        CRSF_FRAME_PARAM_READ,
        &[CRSF_ADDRESS_MODULE, CRSF_ADDRESS_HANDSET, field_id, chunk],
    )
}

fn build_param_write_u8_frame(field_id: u8, value: u8) -> Vec<u8> {
    build_crsf_frame(
        CRSF_SYNC,
        CRSF_FRAME_PARAM_WRITE,
        &[CRSF_ADDRESS_MODULE, CRSF_ADDRESS_HANDSET, field_id, value],
    )
}

fn build_param_write_string_frame(field_id: u8, value: &str) -> Vec<u8> {
    let mut payload = vec![CRSF_ADDRESS_MODULE, CRSF_ADDRESS_HANDSET, field_id];
    payload.extend_from_slice(value.as_bytes());
    payload.push(0);
    build_crsf_frame(CRSF_SYNC, CRSF_FRAME_PARAM_WRITE, &payload)
}

fn build_crsf_frame(address: u8, frame_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(address);
    out.push((payload.len() + 2) as u8);
    out.push(frame_type);
    out.extend_from_slice(payload);
    out.push(CRSF_CRC.checksum(&out[2..]));
    out
}

fn build_crsf_command_frame(address: u8, payload_without_crc: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(payload_without_crc.len() + 1);
    payload.extend_from_slice(payload_without_crc);
    payload.push(crc8_ba(payload_without_crc));
    build_crsf_frame(address, CRSF_FRAME_COMMAND, &payload)
}

fn crc8_ba(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for byte in data {
        crc = CRC8_BA_TABLE[(crc ^ byte) as usize];
    }
    crc
}

fn elrs_debug_enabled() -> bool {
    std::env::var_os("LINTX_ELRS_DEBUG").is_some()
}

fn elrs_debug_log(msg: &str) {
    if elrs_debug_enabled() {
        thread_logln!("[elrs_debug] {}", msg);
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

const CRC8_BA_TABLE: [u8; 256] = [
    0x00, 0xBA, 0xCE, 0x74, 0x26, 0x9C, 0xE8, 0x52, 0x4C, 0xF6, 0x82, 0x38, 0x6A, 0xD0, 0xA4, 0x1E,
    0x98, 0x22, 0x56, 0xEC, 0xBE, 0x04, 0x70, 0xCA, 0xD4, 0x6E, 0x1A, 0xA0, 0xF2, 0x48, 0x3C, 0x86,
    0x8A, 0x30, 0x44, 0xFE, 0xAC, 0x16, 0x62, 0xD8, 0xC6, 0x7C, 0x08, 0xB2, 0xE0, 0x5A, 0x2E, 0x94,
    0x12, 0xA8, 0xDC, 0x66, 0x34, 0x8E, 0xFA, 0x40, 0x5E, 0xE4, 0x90, 0x2A, 0x78, 0xC2, 0xB6, 0x0C,
    0xAE, 0x14, 0x60, 0xDA, 0x88, 0x32, 0x46, 0xFC, 0xE2, 0x58, 0x2C, 0x96, 0xC4, 0x7E, 0x0A, 0xB0,
    0x36, 0x8C, 0xF8, 0x42, 0x10, 0xAA, 0xDE, 0x64, 0x7A, 0xC0, 0xB4, 0x0E, 0x5C, 0xE6, 0x92, 0x28,
    0x24, 0x9E, 0xEA, 0x50, 0x02, 0xB8, 0xCC, 0x76, 0x68, 0xD2, 0xA6, 0x1C, 0x4E, 0xF4, 0x80, 0x3A,
    0xBC, 0x06, 0x72, 0xC8, 0x9A, 0x20, 0x54, 0xEE, 0xF0, 0x4A, 0x3E, 0x84, 0xD6, 0x6C, 0x18, 0xA2,
    0xE6, 0x5C, 0x28, 0x92, 0xC0, 0x7A, 0x0E, 0xB4, 0xAA, 0x10, 0x64, 0xDE, 0x8C, 0x36, 0x42, 0xF8,
    0x7E, 0xC4, 0xB0, 0x0A, 0x58, 0xE2, 0x96, 0x2C, 0x32, 0x88, 0xFC, 0x46, 0x14, 0xAE, 0xDA, 0x60,
    0x6C, 0xD6, 0xA2, 0x18, 0x4A, 0xF0, 0x84, 0x3E, 0x20, 0x9A, 0xEE, 0x54, 0x06, 0xBC, 0xC8, 0x72,
    0xF4, 0x4E, 0x3A, 0x80, 0xD2, 0x68, 0x1C, 0xA6, 0xB8, 0x02, 0x76, 0xCC, 0x9E, 0x24, 0x50, 0xEA,
    0x48, 0xF2, 0x86, 0x3C, 0x6E, 0xD4, 0xA0, 0x1A, 0x04, 0xBE, 0xCA, 0x70, 0x22, 0x98, 0xEC, 0x56,
    0xD0, 0x6A, 0x1E, 0xA4, 0xF6, 0x4C, 0x38, 0x82, 0x9C, 0x26, 0x52, 0xE8, 0xBA, 0x00, 0x74, 0xCE,
    0xC2, 0x78, 0x0C, 0xB6, 0xE4, 0x5E, 0x2A, 0x90, 0x8E, 0x34, 0x40, 0xFA, 0xA8, 0x12, 0x66, 0xDC,
    0x5A, 0xE0, 0x94, 0x2E, 0x7C, 0xC6, 0xB2, 0x08, 0x16, 0xAC, 0xD8, 0x62, 0x30, 0x8A, 0xFE, 0x44,
];

#[cfg(test)]
mod tests {
    use super::{
        build_bind_frame, build_param_write_string_frame, build_ping_frame, check_frame_crc,
        extract_crsf_frames, StringEditState, CRSF_FRAME_COMMAND, CRSF_FRAME_PING_DEVICES,
    };

    #[test]
    fn test_ping_frame_crc() {
        let frame = build_ping_frame();
        assert_eq!(frame[2], CRSF_FRAME_PING_DEVICES);
        assert!(check_frame_crc(&frame));
    }

    #[test]
    fn test_bind_frame_crc() {
        let frame = build_bind_frame(false);
        assert_eq!(frame[2], CRSF_FRAME_COMMAND);
        assert!(check_frame_crc(&frame));
    }

    #[test]
    fn test_extract_crsf_frames_skips_invalid_bytes() {
        let mut buf = vec![0x00, 0x01];
        let ping = build_ping_frame();
        buf.extend_from_slice(&ping);
        let frames = extract_crsf_frames(&mut buf);
        assert_eq!(frames.len(), 1);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_string_edit_state_moves_and_cycles() {
        let mut edit = StringEditState::new(1, "Bind Phrase".to_string(), "ab");
        edit.move_cursor(1);
        edit.cycle_char(1);
        assert_eq!(edit.buffer_string(), "ac");
    }

    #[test]
    fn test_string_write_frame_crc() {
        let frame = build_param_write_string_frame(6, "lin-tx");
        assert!(check_frame_crc(&frame));
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("elrs_agent", elrs_agent_main);
}

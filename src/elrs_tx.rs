use std::io::{Read, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use crc::{Crc, CRC_8_DVB_S2};
use crsf::{PacketAddress, RawPacket};
use rpos::{
    msg::{get_new_rx_of_message, get_new_tx_of_message},
    pthread_scheduler::SchedulePthread,
    thread_logln,
};
use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::{
    client_process_args,
    config::{store, ElrsUiConfig},
    messages::{ElrsCommandMsg, ElrsFeedbackMsg, ElrsStateMsg, SystemStatusMsg},
    mixer::MixerOutMsg,
};

const CRSF_SYNC: u8 = 0xC8;
const CRSF_FRAME_BATTERY_ID: u8 = 0x08;
const CRSF_FRAME_LINK_ID: u8 = 0x14;
const CRSF_FRAME_LINK_RX_ID: u8 = 0x1C;
const CRSF_MAX_PACKET_SIZE: usize = 66;
const CRSF_CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_DVB_S2);
const REOPEN_DELAY: Duration = Duration::from_millis(500);
const TELEMETRY_STALE_AFTER: Duration = Duration::from_secs(2);
const SERIAL_IO_TIMEOUT: Duration = Duration::from_millis(100);
const WRITE_TIMEOUT_LOG_EVERY: u32 = 50;
const DEFAULT_RF_UART: &str = "/dev/ttyS3";
const ELRS_POWER_LEVELS_MW: [u16; 6] = [10, 25, 100, 250, 500, 1000];
const EDITOR_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Parser)]
#[command(name = "rf_link_service", about = "Unified ELRS/CRSF RF link service", long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 420000)]
    baudrate: u32,

    #[arg(default_value = DEFAULT_RF_UART)]
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
    data[0] = 0xEE;
    data[1] = 6;
    data[2] = 0x2D;
    data[3] = 0xEE;
    data[4] = 0xEA;
    data[5] = 0x1;
    data[6] = 0x00;
    data[7] = crc8_alg.checksum(&data[2..7]);
    data
}

#[inline]
fn mixer_out_to_crsf(value: u16) -> u16 {
    (value as u32
        * (crsf::RcChannels::CHANNEL_VALUE_MAX - crsf::RcChannels::CHANNEL_VALUE_MIN) as u32
        / 10000
        + crsf::RcChannels::CHANNEL_VALUE_MIN as u32) as u16
}

#[derive(Debug, Clone)]
struct EditorState {
    buffer: Vec<u8>,
    cursor: usize,
}

impl EditorState {
    fn new(initial: &str) -> Self {
        let mut buffer = initial.as_bytes().to_vec();
        sanitize_editor_buffer(&mut buffer);
        if buffer.is_empty() {
            buffer.push(EDITOR_CHARSET[0]);
        }
        Self { buffer, cursor: 0 }
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.buffer.is_empty() {
            self.cursor = 0;
            return;
        }

        if delta.is_negative() {
            self.cursor = self.cursor.saturating_sub(delta.unsigned_abs());
        } else {
            self.cursor = self
                .cursor
                .saturating_add(delta as usize)
                .min(self.buffer.len().saturating_sub(1));
        }
    }

    fn cycle_char(&mut self, delta: isize) {
        if self.buffer.is_empty() {
            self.buffer.push(EDITOR_CHARSET[0]);
            self.cursor = 0;
            return;
        }

        let current = self.buffer[self.cursor];
        let current_idx = char_index(current).unwrap_or(0);
        let len = EDITOR_CHARSET.len() as isize;
        let next_idx = (current_idx as isize + delta).rem_euclid(len) as usize;
        self.buffer[self.cursor] = EDITOR_CHARSET[next_idx];
    }

    fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }
}

#[derive(Debug, Clone)]
struct ElrsUiState {
    config: ElrsUiConfig,
    selected_idx: usize,
    editor: Option<EditorState>,
}

impl ElrsUiState {
    const SELECTABLE_PARAM_COUNT: usize = 4;

    fn load() -> Self {
        let config = store::load_radio_config()
            .map(|radio| radio.elrs)
            .unwrap_or_default();
        Self {
            config,
            selected_idx: 0,
            editor: None,
        }
    }

    fn editor_label(&self) -> &'static str {
        "Bind Phrase"
    }

    fn handle_command(&mut self, cmd: ElrsCommandMsg) -> String {
        if self.editor.is_some() {
            return self.handle_editor_command(cmd);
        }

        match cmd {
            ElrsCommandMsg::Back => "Back".to_string(),
            ElrsCommandMsg::Refresh => self.reload_from_disk(),
            ElrsCommandMsg::SelectPrev => {
                self.selected_idx = self.selected_idx.saturating_sub(1);
                self.current_item_status()
            }
            ElrsCommandMsg::SelectNext => {
                let max_idx = Self::SELECTABLE_PARAM_COUNT.saturating_sub(1);
                self.selected_idx = self.selected_idx.saturating_add(1).min(max_idx);
                self.current_item_status()
            }
            ElrsCommandMsg::ValueDec => self.adjust_selected(-1),
            ElrsCommandMsg::ValueInc => self.adjust_selected(1),
            ElrsCommandMsg::Activate => self.activate_selected(),
        }
    }

    fn handle_editor_command(&mut self, cmd: ElrsCommandMsg) -> String {
        if self.editor.is_none() {
            return "Edit unavailable".to_string();
        }

        match cmd {
            ElrsCommandMsg::Back => {
                self.editor = None;
                "Bind phrase edit cancelled".to_string()
            }
            ElrsCommandMsg::SelectPrev => {
                let editor = self.editor.as_mut().expect("editor checked");
                editor.cycle_char(-1);
                format!("Editing bind phrase: {}", editor.as_string())
            }
            ElrsCommandMsg::SelectNext => {
                let editor = self.editor.as_mut().expect("editor checked");
                editor.cycle_char(1);
                format!("Editing bind phrase: {}", editor.as_string())
            }
            ElrsCommandMsg::ValueDec => {
                let editor = self.editor.as_mut().expect("editor checked");
                editor.move_cursor(-1);
                format!("Cursor {}", editor.cursor.saturating_add(1))
            }
            ElrsCommandMsg::ValueInc => {
                let editor = self.editor.as_mut().expect("editor checked");
                editor.move_cursor(1);
                format!("Cursor {}", editor.cursor.saturating_add(1))
            }
            ElrsCommandMsg::Activate => {
                let Some(editor) = self.editor.as_ref() else {
                    return "Edit unavailable".to_string();
                };
                self.config.bind_phrase = editor.as_string();
                self.editor = None;
                match self.persist() {
                    Ok(()) => "Bind phrase saved".to_string(),
                    Err(err) => format!("Bind phrase save failed: {err}"),
                }
            }
            ElrsCommandMsg::Refresh => "Editing bind phrase".to_string(),
        }
    }

    fn adjust_selected(&mut self, delta: isize) -> String {
        match self.selected_idx {
            0 => {
                self.config.wifi_manual_on = !self.config.wifi_manual_on;
                match self.persist() {
                    Ok(()) => {
                        if self.config.wifi_manual_on {
                            "ELRS WiFi enabled".to_string()
                        } else {
                            "ELRS WiFi disabled".to_string()
                        }
                    }
                    Err(err) => format!("WiFi config save failed: {err}"),
                }
            }
            1 => {
                self.config.bind_mode = !self.config.bind_mode;
                match self.persist() {
                    Ok(()) => {
                        if self.config.bind_mode {
                            "Bind mode entered".to_string()
                        } else {
                            "Bind mode exited".to_string()
                        }
                    }
                    Err(err) => format!("Bind mode save failed: {err}"),
                }
            }
            2 => {
                self.config.tx_power_mw = shift_power_level(self.config.tx_power_mw, delta);
                match self.persist() {
                    Ok(()) => format!("TX power set to {}mW", self.config.tx_power_mw),
                    Err(err) => format!("TX power save failed: {err}"),
                }
            }
            3 => {
                self.editor = Some(EditorState::new(&self.config.bind_phrase));
                "Editing bind phrase".to_string()
            }
            _ => self.current_item_status(),
        }
    }

    fn activate_selected(&mut self) -> String {
        match self.selected_idx {
            0..=2 => self.adjust_selected(1),
            3 => {
                self.editor = Some(EditorState::new(&self.config.bind_phrase));
                "Editing bind phrase".to_string()
            }
            _ => self.current_item_status(),
        }
    }

    fn current_item_status(&self) -> String {
        match self.selected_idx {
            0 => {
                if self.config.wifi_manual_on {
                    "Manual WiFi is ON".to_string()
                } else {
                    "Manual WiFi is OFF".to_string()
                }
            }
            1 => {
                if self.config.bind_mode {
                    "Bind mode ACTIVE".to_string()
                } else {
                    "Bind mode IDLE".to_string()
                }
            }
            2 => format!("TX power {}mW", self.config.tx_power_mw),
            3 => "Bind phrase".to_string(),
            _ => String::new(),
        }
    }

    fn reload_from_disk(&mut self) -> String {
        match store::load_radio_config() {
            Ok(radio) => {
                self.config = radio.elrs;
                self.editor = None;
                "ELRS config reloaded".to_string()
            }
            Err(err) => format!("ELRS config reload failed: {err}"),
        }
    }

    fn persist(&self) -> Result<(), String> {
        let mut radio = store::load_radio_config().map_err(|err| err.to_string())?;
        radio.elrs = self.config.clone();
        store::save_radio_config(&radio).map_err(|err| err.to_string())
    }
}

fn sanitize_editor_buffer(buffer: &mut Vec<u8>) {
    for ch in buffer.iter_mut() {
        if char_index(*ch).is_none() {
            *ch = EDITOR_CHARSET[0];
        }
    }
    if buffer.len() > 32 {
        buffer.truncate(32);
    }
}

fn char_index(ch: u8) -> Option<usize> {
    EDITOR_CHARSET.iter().position(|candidate| *candidate == ch)
}

fn shift_power_level(current: u16, delta: isize) -> u16 {
    let current = normalize_power_level(current);
    let idx = ELRS_POWER_LEVELS_MW
        .iter()
        .position(|power| *power == current)
        .unwrap_or(0) as isize;
    let next = (idx + delta).clamp(0, ELRS_POWER_LEVELS_MW.len() as isize - 1) as usize;
    ELRS_POWER_LEVELS_MW[next]
}

fn normalize_power_level(raw: u16) -> u16 {
    ELRS_POWER_LEVELS_MW
        .iter()
        .min_by_key(|level| level.abs_diff(raw))
        .copied()
        .unwrap_or(100)
}

fn write_packet_with_timeout_tolerance(
    dev: &mut dyn serialport::SerialPort,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut sent = 0usize;
    while sent < payload.len() {
        match dev.write(&payload[sent..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "serial write returned 0",
                ))
            }
            Ok(n) => {
                sent = sent.saturating_add(n);
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn rf_link_service_main(argc: u32, argv: *const &str) {
    let args = match client_process_args::<Cli>(argc, argv) {
        Some(v) => v,
        None => return,
    };

    let mut mixer_out_rx = get_new_rx_of_message::<MixerOutMsg>("mixer_out").unwrap();
    let mut elrs_cmd_rx = get_new_rx_of_message::<ElrsCommandMsg>("elrs_cmd").unwrap();
    let system_status_tx = get_new_tx_of_message::<SystemStatusMsg>("system_status").unwrap();
    let elrs_state_tx = get_new_tx_of_message::<ElrsStateMsg>("elrs_state").unwrap();
    let elrs_feedback_tx = get_new_tx_of_message::<ElrsFeedbackMsg>("elrs_feedback").unwrap();

    thread_logln!(
        "rf_link_service start on {} @ {} baud",
        args.dev_name,
        args.baudrate
    );

    SchedulePthread::new_simple(Box::new(move |_| {
        let mut status_state = TelemetryStatusState::default();
        let mut ui_state = ElrsUiState::load();
        let mut serial_buf = Vec::<u8>::new();
        let mut read_buf = [0u8; 256];
        let mut crsf_chn_values: [u16; 16] = [0; 16];
        let mut last_state = ElrsStateMsg::default();
        let mut write_timeout_count: u32 = 0;

        loop {
            let serial = serialport::new(&args.dev_name, args.baudrate)
                .timeout(SERIAL_IO_TIMEOUT)
                .data_bits(DataBits::Eight)
                .parity(Parity::None)
                .stop_bits(StopBits::One)
                .flow_control(FlowControl::None);
            let mut dev = match serial.open() {
                Ok(port) => port,
                Err(err) => {
                    let err_text = format!("open failed: {}", err);
                    while let Some(cmd) = elrs_cmd_rx.try_read() {
                        let cmd_status = ui_state.handle_command(cmd);
                        let cmd_state = build_elrs_state(
                            false,
                            cmd_status,
                            &status_state,
                            &ui_state,
                            &args.dev_name,
                        );
                        if cmd_state != last_state {
                            elrs_state_tx.send(cmd_state.clone());
                            last_state = cmd_state;
                        }
                    }
                    let state = build_elrs_state(
                        false,
                        err_text.clone(),
                        &status_state,
                        &ui_state,
                        &args.dev_name,
                    );
                    elrs_feedback_tx.send(build_feedback(false, err_text.clone(), &status_state));
                    if state != last_state {
                        elrs_state_tx.send(state.clone());
                        last_state = state;
                    }
                    thread_logln!("rf_link_service open failed on {}: {}", args.dev_name, err);
                    std::thread::sleep(REOPEN_DELAY);
                    continue;
                }
            };

            let magic_cmd = gen_magic_packet();
            for _ in 0..10 {
                if let Err(err) = write_packet_with_timeout_tolerance(&mut *dev, &magic_cmd) {
                    if err.kind() == std::io::ErrorKind::TimedOut {
                        write_timeout_count = write_timeout_count.saturating_add(1);
                        if write_timeout_count % WRITE_TIMEOUT_LOG_EVERY == 1 {
                            thread_logln!(
                                "rf_link_service magic write timeout on {} (count={})",
                                args.dev_name,
                                write_timeout_count
                            );
                        }
                        continue;
                    }
                    thread_logln!("rf_link_service magic write failed: {}", err);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }

            let mut ui_status_text = format!("{} @ {} baud", args.dev_name, args.baudrate);
            let state = build_elrs_state(
                true,
                ui_status_text.clone(),
                &status_state,
                &ui_state,
                &args.dev_name,
            );
            elrs_feedback_tx.send(build_feedback(
                true,
                format!("{} @ {} baud", args.dev_name, args.baudrate),
                &status_state,
            ));
            if state != last_state {
                elrs_state_tx.send(state.clone());
                last_state = state;
            }

            loop {
                while let Some(msg) = mixer_out_rx.try_read() {
                    crsf_chn_values[0] = mixer_out_to_crsf(msg.aileron);
                    crsf_chn_values[1] = mixer_out_to_crsf(msg.elevator);
                    crsf_chn_values[2] = mixer_out_to_crsf(msg.thrust);
                    crsf_chn_values[3] = mixer_out_to_crsf(msg.direction);
                    let raw_packet = new_rc_channel_packet(&crsf_chn_values);
                    if let Err(err) =
                        write_packet_with_timeout_tolerance(&mut *dev, raw_packet.data())
                    {
                        if err.kind() == std::io::ErrorKind::TimedOut {
                            write_timeout_count = write_timeout_count.saturating_add(1);
                            if write_timeout_count % WRITE_TIMEOUT_LOG_EVERY == 1 {
                                thread_logln!(
                                    "rf_link_service write timeout on {} (count={})",
                                    args.dev_name,
                                    write_timeout_count
                                );
                            }
                            continue;
                        }
                        write_timeout_count = 0;
                        thread_logln!("rf_link_service write failed: {}", err);
                        break;
                    }
                    write_timeout_count = 0;
                }

                while let Some(cmd) = elrs_cmd_rx.try_read() {
                    ui_status_text = ui_state.handle_command(cmd);
                    let state = build_elrs_state(
                        true,
                        ui_status_text.clone(),
                        &status_state,
                        &ui_state,
                        &args.dev_name,
                    );
                    if state != last_state {
                        elrs_state_tx.send(state.clone());
                        last_state = state;
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
                        elrs_feedback_tx.send(build_feedback(
                            true,
                            "telemetry rx".to_string(),
                            &status_state,
                        ));
                        let state = build_elrs_state(
                            true,
                            ui_status_text.clone(),
                            &status_state,
                            &ui_state,
                            &args.dev_name,
                        );
                        if state != last_state {
                            elrs_state_tx.send(state.clone());
                            last_state = state;
                        }
                    }
                    Ok(_) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(err) => {
                        let state = build_elrs_state(
                            false,
                            format!("serial error: {}", err),
                            &status_state,
                            &ui_state,
                            &args.dev_name,
                        );
                        elrs_feedback_tx.send(build_feedback(
                            false,
                            format!("serial error: {}", err),
                            &status_state,
                        ));
                        if state != last_state {
                            elrs_state_tx.send(state.clone());
                            last_state = state;
                        }
                        thread_logln!("rf_link_service read failed: {}", err);
                        break;
                    }
                }

                if status_state.telemetry_is_stale() {
                    let state = build_elrs_state(
                        true,
                        ui_status_text.clone(),
                        &status_state,
                        &ui_state,
                        &args.dev_name,
                    );
                    if state != last_state {
                        elrs_state_tx.send(state.clone());
                        last_state = state;
                    }
                }

                std::thread::sleep(Duration::from_millis(2));
            }

            std::thread::sleep(REOPEN_DELAY);
        }
    }));
}

#[derive(Debug, Clone, Default)]
struct TelemetryStatusState {
    remote_battery_percent: Option<u8>,
    aircraft_battery_percent: Option<u8>,
    signal_strength_percent: Option<u8>,
    last_telemetry_at: Option<Instant>,
    last_telemetry_unix_secs: Option<u64>,
}

impl TelemetryStatusState {
    fn consume_frame(&mut self, frame: &[u8]) -> Option<SystemStatusMsg> {
        if frame.len() < 5 {
            return None;
        }

        let mut changed = false;

        match frame[2] {
            CRSF_FRAME_LINK_ID => {
                if let Some(rx_quality) = frame.get(5).copied() {
                    self.signal_strength_percent = Some(rx_quality.min(100));
                    changed = true;
                }
            }
            CRSF_FRAME_LINK_RX_ID => {
                if let Some(rx_rssi_percent) = frame.get(4).copied() {
                    self.signal_strength_percent = Some(rx_rssi_percent.min(100));
                    changed = true;
                }
            }
            CRSF_FRAME_BATTERY_ID => {
                if let Some(remaining) = frame.get(10).copied() {
                    self.aircraft_battery_percent = Some(remaining.min(100));
                    changed = true;
                }
            }
            _ => {}
        }

        if !changed {
            return None;
        }

        self.last_telemetry_at = Some(Instant::now());
        self.last_telemetry_unix_secs = Some(now_unix_secs());

        Some(SystemStatusMsg {
            remote_battery_percent: self.remote_battery_percent.unwrap_or(100),
            aircraft_battery_percent: self.aircraft_battery_percent.unwrap_or(100),
            signal_strength_percent: self.signal_strength_percent.unwrap_or(100),
            unix_time_secs: now_unix_secs(),
        })
    }

    fn telemetry_is_stale(&self) -> bool {
        match self.last_telemetry_at {
            Some(last) => Instant::now().saturating_duration_since(last) > TELEMETRY_STALE_AFTER,
            None => true,
        }
    }
}

fn build_elrs_state(
    connected: bool,
    status_text: String,
    telemetry: &TelemetryStatusState,
    ui_state: &ElrsUiState,
    dev_name: &str,
) -> ElrsStateMsg {
    let editor_active = ui_state.editor.is_some();
    let (editor_buffer, editor_cursor) = if let Some(editor) = ui_state.editor.as_ref() {
        (editor.as_string(), editor.cursor)
    } else {
        (String::new(), 0)
    };

    ElrsStateMsg {
        connected,
        busy: false,
        can_leave: !editor_active,
        path: dev_name.to_string(),
        editor_active,
        editor_label: if editor_active {
            ui_state.editor_label().to_string()
        } else {
            String::new()
        },
        editor_buffer,
        editor_cursor,
        module_name: "ELRS/CRSF".to_string(),
        device_name: if connected {
            "UART Connected".to_string()
        } else {
            "Disconnected".to_string()
        },
        version: "--".to_string(),
        packet_rate: "--".to_string(),
        telemetry_ratio: "--".to_string(),
        tx_power: format!("{}mW", ui_state.config.tx_power_mw),
        status_text,
        wifi_running: ui_state.config.wifi_manual_on,
        selected_idx: ui_state.selected_idx,
        params: vec![
            crate::messages::ElrsParamEntry {
                id: "wifi_manual".to_string(),
                label: "Manual WiFi".to_string(),
                value: if ui_state.config.wifi_manual_on {
                    "ON".to_string()
                } else {
                    "OFF".to_string()
                },
                selectable: true,
            },
            crate::messages::ElrsParamEntry {
                id: "bind_mode".to_string(),
                label: "Bind Mode".to_string(),
                value: if ui_state.config.bind_mode {
                    "ACTIVE".to_string()
                } else {
                    "IDLE".to_string()
                },
                selectable: true,
            },
            crate::messages::ElrsParamEntry {
                id: "tx_power".to_string(),
                label: "TX Power".to_string(),
                value: format!("{}mW", ui_state.config.tx_power_mw),
                selectable: true,
            },
            crate::messages::ElrsParamEntry {
                id: "bind_phrase".to_string(),
                label: "Bind Phrase".to_string(),
                value: if ui_state.config.bind_phrase.is_empty() {
                    "(empty)".to_string()
                } else {
                    ui_state.config.bind_phrase.clone()
                },
                selectable: true,
            },
            crate::messages::ElrsParamEntry {
                id: "signal".to_string(),
                label: "Signal".to_string(),
                value: telemetry
                    .signal_strength_percent
                    .map(|v| format!("{}%", v))
                    .unwrap_or_else(|| "--".to_string()),
                selectable: false,
            },
            crate::messages::ElrsParamEntry {
                id: "aircraft_battery".to_string(),
                label: "Aircraft Battery".to_string(),
                value: telemetry
                    .aircraft_battery_percent
                    .map(|v| format!("{}%", v))
                    .unwrap_or_else(|| "--".to_string()),
                selectable: false,
            },
            crate::messages::ElrsParamEntry {
                id: "telemetry_fresh".to_string(),
                label: "Telemetry Fresh".to_string(),
                value: if telemetry.telemetry_is_stale() {
                    "stale".to_string()
                } else {
                    "fresh".to_string()
                },
                selectable: false,
            },
        ],
    }
}

fn build_feedback(
    connected: bool,
    detail: String,
    telemetry: &TelemetryStatusState,
) -> ElrsFeedbackMsg {
    ElrsFeedbackMsg {
        connected,
        signal_strength_percent: telemetry.signal_strength_percent,
        aircraft_battery_percent: telemetry.aircraft_battery_percent,
        last_update_unix_secs: telemetry.last_telemetry_unix_secs,
        detail,
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
        check_frame_crc, extract_crsf_frames, shift_power_level, EditorState, TelemetryStatusState,
        CRSF_CRC, CRSF_FRAME_BATTERY_ID, CRSF_FRAME_LINK_ID, CRSF_SYNC,
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

    #[test]
    fn test_shift_power_level_clamped() {
        assert_eq!(shift_power_level(100, -1), 25);
        assert_eq!(shift_power_level(100, 1), 250);
        assert_eq!(shift_power_level(10, -1), 10);
        assert_eq!(shift_power_level(1000, 1), 1000);
        assert_eq!(shift_power_level(50, 1), 100);
    }

    #[test]
    fn test_editor_state_cycle_and_cursor() {
        let mut editor = EditorState::new("ab");
        editor.move_cursor(1);
        editor.cycle_char(1);
        assert_eq!(editor.cursor, 1);
        assert_eq!(editor.as_string(), "ac");
        editor.move_cursor(-5);
        assert_eq!(editor.cursor, 0);
    }
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("rf_link_service", rf_link_service_main);
    rpos::module::Module::register("elrs_tx", rf_link_service_main);
}

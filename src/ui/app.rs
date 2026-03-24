use std::time::Duration;

use rpos::{
    channel::Sender,
    msg::{get_new_rx_of_message, get_new_tx_of_message},
};

use crate::{
    config::store,
    messages::{
        ActiveModelMsg, AdcRawMsg, ElrsCommandMsg, ElrsStateMsg, SystemConfigMsg, SystemStatusMsg,
    },
    mixer::MixerOutMsg,
};

use super::{
    backend::LvglBackend,
    catalog::{app_at, page, PAGE_SPECS},
    input::UiInputEvent,
    model::{AppId, UiFrame, UiModelEntry, UiPage},
};

const UI_ACTIVE_ANIMATION_WINDOW: Duration = Duration::from_millis(280);
const UI_MAX_IDLE_SLEEP: Duration = Duration::from_millis(80);

pub struct UiApp {
    frame: UiFrame,
}

impl UiApp {
    fn update_field<T: PartialEq>(slot: &mut T, value: T) -> bool {
        if *slot == value {
            false
        } else {
            *slot = value;
            true
        }
    }

    pub fn new() -> Self {
        let mut app = Self {
            frame: UiFrame::default(),
        };
        app.reload_models();
        app
    }

    fn reload_models(&mut self) {
        if let Err(err) = store::ensure_default_layout() {
            super::debug_log(&format!("ensure_default_layout failed: {err}"));
            return;
        }

        let models = match store::list_models() {
            Ok(models) => models,
            Err(err) => {
                super::debug_log(&format!("list_models failed: {err}"));
                return;
            }
        };

        self.frame.model_entries = models
            .iter()
            .map(|model| UiModelEntry {
                id: model.id.clone(),
                name: model.name.clone(),
                protocol: model.output.protocol.display_name().to_string(),
            })
            .collect();

        let active_model_id = store::load_radio_config()
            .map(|radio| radio.active_model)
            .unwrap_or_default();

        self.frame.model_active_idx = self
            .frame
            .model_entries
            .iter()
            .position(|entry| entry.id == active_model_id)
            .unwrap_or(0);

        if self.frame.model_entries.is_empty() {
            self.frame.model_focus_idx = 0;
            self.frame.model_active_idx = 0;
        } else {
            self.frame.model_focus_idx = self
                .frame
                .model_focus_idx
                .min(self.frame.model_entries.len().saturating_sub(1));
            self.frame.model_active_idx = self
                .frame
                .model_active_idx
                .min(self.frame.model_entries.len().saturating_sub(1));
        }
    }

    fn publish_config(&self, config_tx: &Sender<SystemConfigMsg>) {
        config_tx.send(self.frame.config);
    }

    fn publish_active_model(&self, active_model_tx: &Sender<ActiveModelMsg>) {
        match store::load_active_model() {
            Ok(model) => active_model_tx.send(ActiveModelMsg { model }),
            Err(err) => super::debug_log(&format!("load_active_model failed: {err}")),
        }
    }

    fn normalize_selection(&mut self) {
        let p = page(self.frame.launcher_page);
        if self.frame.selected_row >= p.rows {
            self.frame.selected_row = p.rows.saturating_sub(1);
        }
        if self.frame.selected_col >= p.cols {
            self.frame.selected_col = p.cols.saturating_sub(1);
        }
        while app_at(
            self.frame.launcher_page,
            self.frame.selected_row,
            self.frame.selected_col,
        )
        .is_none()
            && self.frame.selected_col > 0
        {
            self.frame.selected_col -= 1;
        }
    }

    fn move_selection_vertical(&mut self, dr: isize) {
        let p = page(self.frame.launcher_page);
        let nr = self.frame.selected_row as isize + dr;
        if nr < 0 {
            return;
        }
        let nr = nr as usize;
        if nr >= p.rows {
            return;
        }

        if app_at(self.frame.launcher_page, nr, self.frame.selected_col).is_some() {
            self.frame.selected_row = nr;
        }
    }

    fn move_left(&mut self) {
        if self.frame.selected_col > 0 {
            self.frame.selected_col -= 1;
            return;
        }

        if self.frame.launcher_page > 0 {
            self.frame.launcher_page -= 1;
            let p = page(self.frame.launcher_page);
            self.frame.selected_col = p.cols.saturating_sub(1);
            self.normalize_selection();
        }
    }

    fn move_right(&mut self) {
        let p = page(self.frame.launcher_page);
        if self.frame.selected_col + 1 < p.cols
            && app_at(
                self.frame.launcher_page,
                self.frame.selected_row,
                self.frame.selected_col + 1,
            )
            .is_some()
        {
            self.frame.selected_col += 1;
            return;
        }

        if self.frame.launcher_page + 1 < PAGE_SPECS.len() {
            self.frame.launcher_page += 1;
            self.frame.selected_col = 0;
            self.normalize_selection();
        }
    }

    fn apply_event_in_app(
        &mut self,
        app: AppId,
        event: UiInputEvent,
        config_tx: &Sender<SystemConfigMsg>,
        active_model_tx: &Sender<ActiveModelMsg>,
        elrs_cmd_tx: &Sender<ElrsCommandMsg>,
    ) {
        match app {
            AppId::System => match event {
                UiInputEvent::Up => {
                    self.frame.config.backlight_percent = self
                        .frame
                        .config
                        .backlight_percent
                        .saturating_add(5)
                        .min(100);
                    self.publish_config(config_tx);
                }
                UiInputEvent::Down => {
                    self.frame.config.backlight_percent =
                        self.frame.config.backlight_percent.saturating_sub(5);
                    self.publish_config(config_tx);
                }
                UiInputEvent::Left => {
                    self.frame.config.sound_percent =
                        self.frame.config.sound_percent.saturating_sub(5);
                    self.publish_config(config_tx);
                }
                UiInputEvent::Right => {
                    self.frame.config.sound_percent =
                        self.frame.config.sound_percent.saturating_add(5).min(100);
                    self.publish_config(config_tx);
                }
                _ => {}
            },
            AppId::Models => match event {
                UiInputEvent::Up => {
                    self.frame.model_focus_idx = self.frame.model_focus_idx.saturating_sub(1);
                }
                UiInputEvent::Down => {
                    let max_idx = self.frame.model_entries.len().saturating_sub(1);
                    self.frame.model_focus_idx = (self.frame.model_focus_idx + 1).min(max_idx);
                }
                UiInputEvent::Open => {
                    if let Some(entry) = self.frame.model_entries.get(self.frame.model_focus_idx) {
                        match store::set_active_model(&entry.id) {
                            Ok(_) => {
                                self.frame.model_active_idx = self.frame.model_focus_idx;
                                self.publish_active_model(active_model_tx);
                            }
                            Err(err) => {
                                super::debug_log(&format!("set_active_model failed: {err}"));
                            }
                        }
                    }
                }
                _ => {}
            },
            AppId::Cloud => {
                if event == UiInputEvent::Open {
                    self.frame.cloud_connected = !self.frame.cloud_connected;
                    if self.frame.cloud_connected {
                        self.frame.cloud_last_sync_secs = self.frame.status.unix_time_secs;
                    }
                }
            }
            AppId::Scripts => match event {
                UiInputEvent::Back | UiInputEvent::PagePrev => {
                    elrs_cmd_tx.send(ElrsCommandMsg::Back)
                }
                UiInputEvent::Up => elrs_cmd_tx.send(ElrsCommandMsg::SelectPrev),
                UiInputEvent::Down => elrs_cmd_tx.send(ElrsCommandMsg::SelectNext),
                UiInputEvent::Left => elrs_cmd_tx.send(ElrsCommandMsg::ValueDec),
                UiInputEvent::Right => elrs_cmd_tx.send(ElrsCommandMsg::ValueInc),
                UiInputEvent::Open => elrs_cmd_tx.send(ElrsCommandMsg::Activate),
                UiInputEvent::PageNext => elrs_cmd_tx.send(ElrsCommandMsg::Refresh),
                _ => {}
            },
            _ => {}
        }
    }

    fn apply_event(
        &mut self,
        event: UiInputEvent,
        config_tx: &Sender<SystemConfigMsg>,
        active_model_tx: &Sender<ActiveModelMsg>,
        elrs_cmd_tx: &Sender<ElrsCommandMsg>,
    ) -> bool {
        match event {
            UiInputEvent::Quit => return false,
            UiInputEvent::Back => {
                if matches!(self.frame.page, UiPage::App(AppId::Scripts))
                    && !self.frame.elrs.can_leave
                {
                    self.apply_event_in_app(
                        AppId::Scripts,
                        event,
                        config_tx,
                        active_model_tx,
                        elrs_cmd_tx,
                    );
                } else {
                    self.frame.page = UiPage::Launcher;
                }
            }
            UiInputEvent::Open => {
                if self.frame.page == UiPage::Launcher {
                    if let Some(app) = app_at(
                        self.frame.launcher_page,
                        self.frame.selected_row,
                        self.frame.selected_col,
                    ) {
                        self.frame.page = UiPage::App(app);
                    }
                } else if let UiPage::App(app) = self.frame.page {
                    self.apply_event_in_app(app, event, config_tx, active_model_tx, elrs_cmd_tx);
                }
            }
            UiInputEvent::Left => {
                if self.frame.page == UiPage::Launcher {
                    self.move_left();
                } else if let UiPage::App(app) = self.frame.page {
                    self.apply_event_in_app(app, event, config_tx, active_model_tx, elrs_cmd_tx);
                }
            }
            UiInputEvent::Right => {
                if self.frame.page == UiPage::Launcher {
                    self.move_right();
                } else if let UiPage::App(app) = self.frame.page {
                    self.apply_event_in_app(app, event, config_tx, active_model_tx, elrs_cmd_tx);
                }
            }
            UiInputEvent::Up | UiInputEvent::Down => {
                if self.frame.page == UiPage::Launcher {
                    self.move_selection_vertical(if event == UiInputEvent::Up { -1 } else { 1 });
                } else if let UiPage::App(app) = self.frame.page {
                    self.apply_event_in_app(app, event, config_tx, active_model_tx, elrs_cmd_tx);
                }
            }
            UiInputEvent::PagePrev => {
                if self.frame.page == UiPage::Launcher && self.frame.launcher_page > 0 {
                    self.frame.launcher_page -= 1;
                    self.normalize_selection();
                }
            }
            UiInputEvent::PageNext => {
                if self.frame.page == UiPage::Launcher
                    && self.frame.launcher_page + 1 < PAGE_SPECS.len()
                {
                    self.frame.launcher_page += 1;
                    self.normalize_selection();
                }
            }
        }
        true
    }

    pub fn run(&mut self, backend: &mut dyn LvglBackend, fps: u32) {
        super::debug_log(&format!("UiApp::run start fps={fps}"));
        let mut status_rx = get_new_rx_of_message::<SystemStatusMsg>("system_status").unwrap();
        let mut config_rx = get_new_rx_of_message::<SystemConfigMsg>("system_config").unwrap();
        let mut adc_raw_rx = get_new_rx_of_message::<AdcRawMsg>("adc_raw").unwrap();
        let mut mixer_out_rx = get_new_rx_of_message::<MixerOutMsg>("mixer_out").unwrap();
        let mut elrs_rx = get_new_rx_of_message::<ElrsStateMsg>("elrs_state").unwrap();
        let config_tx = get_new_tx_of_message::<SystemConfigMsg>("system_config").unwrap();
        let active_model_tx = get_new_tx_of_message::<ActiveModelMsg>("active_model").unwrap();
        let elrs_cmd_tx = get_new_tx_of_message::<ElrsCommandMsg>("elrs_cmd").unwrap();

        let elrs_state_tx = get_new_tx_of_message::<ElrsStateMsg>("elrs_state").unwrap();
        elrs_state_tx.send(ElrsStateMsg::default());

        self.reload_models();
        self.publish_active_model(&active_model_tx);

        let frame_time = Duration::from_millis((1000 / fps.max(1)) as u64);
        let idle_sleep = UI_MAX_IDLE_SLEEP.min(frame_time.saturating_mul(2));
        let mut frame_idx: u64 = 0;
        let mut active_until = std::time::Instant::now() + UI_ACTIVE_ANIMATION_WINDOW;
        let mut last_render = std::time::Instant::now()
            .checked_sub(frame_time)
            .unwrap_or_else(std::time::Instant::now);

        backend.init();
        super::debug_log("backend.init done");

        loop {
            let mut dirty = false;

            while let Some(status) = status_rx.try_read() {
                dirty |= Self::update_field(&mut self.frame.status, status);
            }

            while let Some(cfg) = config_rx.try_read() {
                dirty |= Self::update_field(&mut self.frame.config, cfg);
            }

            while let Some(adc_raw) = adc_raw_rx.try_read() {
                dirty |= Self::update_field(&mut self.frame.adc_raw, adc_raw);
            }

            while let Some(mixer_out) = mixer_out_rx.try_read() {
                dirty |= Self::update_field(&mut self.frame.mixer_out, mixer_out);
            }

            while let Some(elrs) = elrs_rx.try_read() {
                dirty |= Self::update_field(&mut self.frame.elrs, elrs);
            }

            if self.frame.cloud_connected
                && self.frame.status.unix_time_secs
                    >= self.frame.cloud_last_sync_secs.saturating_add(5)
            {
                self.frame.cloud_last_sync_secs = self.frame.status.unix_time_secs;
                dirty = true;
            }

            while let Some(evt) = backend.poll_event() {
                if !self.apply_event(evt, &config_tx, &active_model_tx, &elrs_cmd_tx) {
                    backend.shutdown();
                    return;
                }
                dirty = true;
            }

            let now = std::time::Instant::now();
            if dirty {
                active_until = now + UI_ACTIVE_ANIMATION_WINDOW;
            }

            let should_render = dirty
                || now < active_until
                || now.saturating_duration_since(last_render) >= idle_sleep;

            if should_render {
                backend.render(&self.frame);
                last_render = std::time::Instant::now();
                frame_idx = frame_idx.saturating_add(1);
                if super::debug_enabled() && frame_idx % 120 == 0 {
                    super::debug_log(&format!(
                        "render heartbeat frame={} page={:?} launcher_page={} selection=({}, {}) active={}",
                        frame_idx,
                        self.frame.page,
                        self.frame.launcher_page,
                        self.frame.selected_row,
                        self.frame.selected_col,
                        now < active_until
                    ));
                }
            }

            let sleep_for = if std::time::Instant::now() < active_until {
                frame_time
            } else {
                idle_sleep
            };
            std::thread::sleep(sleep_for);
        }
    }
}

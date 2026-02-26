use std::time::Duration;

use rpos::msg::get_new_rx_of_message;

use crate::messages::{SystemConfigMsg, SystemStatusMsg};

use super::{
    backend::LvglBackend,
    catalog::{app_at, page, PAGE_SPECS},
    input::UiInputEvent,
    model::{UiFrame, UiPage},
};

pub struct UiApp {
    frame: UiFrame,
}

impl UiApp {
    pub fn new() -> Self {
        Self {
            frame: UiFrame::default(),
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

    fn apply_event(&mut self, event: UiInputEvent) -> bool {
        match event {
            UiInputEvent::Quit => return false,
            UiInputEvent::Back => self.frame.page = UiPage::Launcher,
            UiInputEvent::Open => {
                if self.frame.page == UiPage::Launcher {
                    if let Some(app) = app_at(
                        self.frame.launcher_page,
                        self.frame.selected_row,
                        self.frame.selected_col,
                    ) {
                        self.frame.page = UiPage::App(app);
                    }
                }
            }
            UiInputEvent::Left => {
                if self.frame.page == UiPage::Launcher {
                    self.move_left();
                }
            }
            UiInputEvent::Right => {
                if self.frame.page == UiPage::Launcher {
                    self.move_right();
                }
            }
            UiInputEvent::Up => {
                if self.frame.page == UiPage::Launcher {
                    self.move_selection_vertical(-1);
                }
            }
            UiInputEvent::Down => {
                if self.frame.page == UiPage::Launcher {
                    self.move_selection_vertical(1);
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

        let frame_time = Duration::from_millis((1000 / fps.max(1)) as u64);
        let mut frame_idx: u64 = 0;

        backend.init();
        super::debug_log("backend.init done");

        loop {
            if let Some(status) = status_rx.try_read() {
                self.frame.status = status;
                if super::debug_enabled() {
                    super::debug_log(&format!(
                        "status update: remote={} aircraft={} signal={} time={}",
                        self.frame.status.remote_battery_percent,
                        self.frame.status.aircraft_battery_percent,
                        self.frame.status.signal_strength_percent,
                        self.frame.status.unix_time_secs
                    ));
                }
            }

            if let Some(cfg) = config_rx.try_read() {
                self.frame.config = cfg;
                if super::debug_enabled() {
                    super::debug_log(&format!(
                        "config update: backlight={} sound={}",
                        self.frame.config.backlight_percent, self.frame.config.sound_percent
                    ));
                }
            }

            while let Some(evt) = backend.poll_event() {
                if super::debug_enabled() {
                    super::debug_log(&format!("input event: {:?}", evt));
                }
                if !self.apply_event(evt) {
                    super::debug_log("apply_event requested quit");
                    backend.shutdown();
                    return;
                }
            }

            backend.render(&self.frame);
            frame_idx = frame_idx.saturating_add(1);
            if super::debug_enabled() && frame_idx % 120 == 0 {
                super::debug_log(&format!(
                    "render heartbeat frame={} page={:?} launcher_page={} selection=({}, {})",
                    frame_idx,
                    self.frame.page,
                    self.frame.launcher_page,
                    self.frame.selected_row,
                    self.frame.selected_col
                ));
            }
            std::thread::sleep(frame_time);
        }
    }
}

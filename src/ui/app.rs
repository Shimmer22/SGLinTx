use std::time::Duration;

use rpos::msg::get_new_rx_of_message;

use crate::messages::{SystemConfigMsg, SystemStatusMsg};

use super::{
    backend::LvglBackend,
    input::UiInputEvent,
    model::{AppId, UiFrame, UiPage},
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

    fn app_index(app: AppId) -> usize {
        match app {
            AppId::System => 0,
            AppId::Control => 1,
            AppId::Models => 2,
            AppId::Cloud => 3,
        }
    }

    fn app_from_index(idx: usize) -> AppId {
        AppId::ALL[idx % AppId::ALL.len()]
    }

    fn apply_event(&mut self, event: UiInputEvent) -> bool {
        match event {
            UiInputEvent::Quit => return false,
            UiInputEvent::Back => self.frame.page = UiPage::Launcher,
            UiInputEvent::Open => {
                if self.frame.page == UiPage::Launcher {
                    self.frame.page = UiPage::App(self.frame.selected_app);
                }
            }
            UiInputEvent::Next => {
                if self.frame.page == UiPage::Launcher {
                    let idx = Self::app_index(self.frame.selected_app);
                    self.frame.selected_app = Self::app_from_index(idx + 1);
                }
            }
            UiInputEvent::Prev => {
                if self.frame.page == UiPage::Launcher {
                    let idx = Self::app_index(self.frame.selected_app);
                    let prev = (idx + AppId::ALL.len() - 1) % AppId::ALL.len();
                    self.frame.selected_app = Self::app_from_index(prev);
                }
            }
        }
        true
    }

    pub fn run(&mut self, backend: &mut dyn LvglBackend, fps: u32) {
        let mut status_rx = get_new_rx_of_message::<SystemStatusMsg>("system_status").unwrap();
        let mut config_rx = get_new_rx_of_message::<SystemConfigMsg>("system_config").unwrap();

        let frame_time = Duration::from_millis((1000 / fps.max(1)) as u64);

        backend.init();

        loop {
            if let Some(status) = status_rx.try_read() {
                self.frame.status = status;
            }

            if let Some(cfg) = config_rx.try_read() {
                self.frame.config = cfg;
            }

            while let Some(evt) = backend.poll_event() {
                if !self.apply_event(evt) {
                    backend.shutdown();
                    return;
                }
            }

            backend.render(&self.frame);
            std::thread::sleep(frame_time);
        }
    }
}

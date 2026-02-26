pub mod app;
pub mod backend;
pub mod catalog;
pub mod input;
pub mod model;

pub fn debug_enabled() -> bool {
    let ui_flag = std::env::var("LINTX_UI_DEBUG")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
        .unwrap_or(false);
    let common_flag = std::env::var("LINTX_DEBUG")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
        .unwrap_or(false);
    ui_flag || common_flag
}

pub fn debug_log(msg: &str) {
    if debug_enabled() {
        eprintln!("[lintx-ui] {msg}");
    }
}

use super::{apps, model::AppId};

pub use apps::{AppSpec, PageSpec, PAGE_SPECS};

pub fn app_spec(id: AppId) -> &'static AppSpec {
    apps::app_spec(id)
}

pub fn page(page_idx: usize) -> &'static PageSpec {
    apps::page(page_idx)
}

pub fn app_at(page_idx: usize, row: usize, col: usize) -> Option<AppId> {
    apps::app_at(page_idx, row, col)
}

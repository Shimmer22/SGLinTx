use super::model::AppId;

#[derive(Debug, Clone, Copy)]
pub struct AppSpec {
    pub id: AppId,
    pub title: &'static str,
    pub icon_text: &'static str,
    pub accent: (u8, u8, u8),
}

#[derive(Debug)]
pub struct PageSpec {
    pub id: usize,
    pub rows: usize,
    pub cols: usize,
    pub apps: &'static [AppId],
}

pub const APP_SPECS: [AppSpec; 8] = [
    AppSpec {
        id: AppId::System,
        title: "SYSTEM",
        icon_text: "SYS",
        accent: (73, 143, 255),
    },
    AppSpec {
        id: AppId::Control,
        title: "CONTROL",
        icon_text: "CTL",
        accent: (86, 214, 165),
    },
    AppSpec {
        id: AppId::Models,
        title: "MODELS",
        icon_text: "MOD",
        accent: (255, 181, 92),
    },
    AppSpec {
        id: AppId::Cloud,
        title: "CLOUD",
        icon_text: "NET",
        accent: (186, 135, 255),
    },
    AppSpec {
        id: AppId::Sensor,
        title: "SENSOR",
        icon_text: "SNS",
        accent: (100, 220, 255),
    },
    AppSpec {
        id: AppId::Trainer,
        title: "TRAINER",
        icon_text: "TRN",
        accent: (255, 123, 118),
    },
    AppSpec {
        id: AppId::Scripts,
        title: "SCRIPT",
        icon_text: "SCR",
        accent: (255, 216, 109),
    },
    AppSpec {
        id: AppId::About,
        title: "ABOUT",
        icon_text: "ABT",
        accent: (160, 196, 255),
    },
];

const PAGE0_APPS: [AppId; 4] = [AppId::System, AppId::Control, AppId::Models, AppId::Cloud];
const PAGE1_APPS: [AppId; 8] = [
    AppId::System,
    AppId::Control,
    AppId::Models,
    AppId::Cloud,
    AppId::Sensor,
    AppId::Trainer,
    AppId::Scripts,
    AppId::About,
];

pub static PAGE_SPECS: [PageSpec; 2] = [
    PageSpec {
        id: 0,
        rows: 1,
        cols: 4,
        apps: &PAGE0_APPS,
    },
    PageSpec {
        id: 1,
        rows: 2,
        cols: 4,
        apps: &PAGE1_APPS,
    },
];

pub fn app_spec(id: AppId) -> &'static AppSpec {
    APP_SPECS.iter().find(|x| x.id == id).unwrap()
}

pub fn page(page_idx: usize) -> &'static PageSpec {
    &PAGE_SPECS[page_idx % PAGE_SPECS.len()]
}

pub fn app_at(page_idx: usize, row: usize, col: usize) -> Option<AppId> {
    let p = page(page_idx);
    if row >= p.rows || col >= p.cols {
        return None;
    }
    p.apps.get(row * p.cols + col).copied()
}

use crate::ui::model::UiFrame;

pub fn battery_grade(v: u8) -> &'static str {
    match v {
        80..=100 => "GOOD",
        50..=79 => "OK",
        20..=49 => "LOW",
        _ => "CRITICAL",
    }
}

pub fn signal_grade(v: u8) -> &'static str {
    match v {
        75..=100 => "SOLID",
        45..=74 => "FAIR",
        20..=44 => "WEAK",
        _ => "LOST",
    }
}

pub fn format_channel_groups(channels: &[i16]) -> String {
    if channels.is_empty() {
        return "No input yet".to_string();
    }

    let mut lines = channels
        .chunks(4)
        .enumerate()
        .take(2)
        .map(|(group_idx, group)| {
            let start = group_idx * 4;
            group
                .iter()
                .enumerate()
                .map(|(offset, value)| format!("CH{}:{}", start + offset + 1, value))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>();

    if channels.len() > 8 {
        lines.push(format!("... +{} more channels", channels.len() - 8));
    }

    lines.join("\n")
}

pub fn elrs_list_lines(frame: &UiFrame) -> [String; 4] {
    let total = frame.elrs.params.len();
    if total == 0 {
        return [
            "No ELRS params available".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];
    }

    let selected = frame.elrs.selected_idx.min(total.saturating_sub(1));
    let start = selected.saturating_sub(1).min(total.saturating_sub(4));
    let mut lines = Vec::with_capacity(4);
    for idx in start..(start + 4).min(total) {
        let entry = &frame.elrs.params[idx];
        lines.push(format!(
            "{} {}: {}",
            if idx == selected { ">" } else { " " },
            entry.label,
            entry.value
        ));
    }

    while lines.len() < 4 {
        lines.push(String::new());
    }

    [
        lines[0].clone(),
        lines[1].clone(),
        lines[2].clone(),
        lines[3].clone(),
    ]
}

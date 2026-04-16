use std::time::Instant;

use crate::messages::{
    UiFeedbackMotion, UiFeedbackSeverity, UiFeedbackSlot, UiFeedbackTarget, UiInteractionFeedback,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFeedbackSnapshot {
    pub seq: u32,
    pub severity: UiFeedbackSeverity,
    pub target: UiFeedbackTarget,
    pub motion: UiFeedbackMotion,
    pub slot: UiFeedbackSlot,
    pub message: String,
    pub elapsed_ms: u32,
    pub ttl_ms: u32,
}

#[derive(Debug, Default)]
pub struct UiFeedbackController {
    active: Option<(UiInteractionFeedback, Instant)>,
}

impl UiFeedbackController {
    pub fn submit(&mut self, event: UiInteractionFeedback, now: Instant) {
        self.active = Some((event, now));
    }

    pub fn snapshot(&mut self, now: Instant) -> Option<UiFeedbackSnapshot> {
        let (event, started_at) = self.active.as_ref()?;
        let elapsed = now.saturating_duration_since(*started_at);
        if elapsed.as_millis() >= u128::from(event.ttl_ms) {
            self.active = None;
            return None;
        }

        Some(UiFeedbackSnapshot {
            seq: event.seq,
            severity: event.severity,
            target: event.target.clone(),
            motion: event.motion,
            slot: event.slot,
            message: event.message.clone(),
            elapsed_ms: elapsed.as_millis().min(u128::from(u32::MAX)) as u32,
            ttl_ms: event.ttl_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{UiFeedbackController, UiFeedbackSnapshot};
    use crate::messages::{
        UiFeedbackMotion, UiFeedbackSeverity, UiFeedbackSlot, UiFeedbackTarget,
        UiInteractionFeedback,
    };
    use std::time::Instant;

    #[test]
    fn test_controller_reports_active_feedback_before_expiry() {
        let now = Instant::now();
        let mut controller = UiFeedbackController::default();
        controller.submit(
            UiInteractionFeedback {
                seq: 7,
                severity: UiFeedbackSeverity::Error,
                target: UiFeedbackTarget::SelectedListRow,
                motion: UiFeedbackMotion::ShakeX,
                slot: UiFeedbackSlot::TopStatusBar,
                message: "WiFi unavailable".to_string(),
                ttl_ms: 700,
            },
            now,
        );

        let snapshot = controller
            .snapshot(now + std::time::Duration::from_millis(120))
            .expect("active feedback");

        assert_eq!(snapshot.seq, 7);
        assert_eq!(snapshot.severity, UiFeedbackSeverity::Error);
        assert_eq!(snapshot.target, UiFeedbackTarget::SelectedListRow);
        assert_eq!(snapshot.motion, UiFeedbackMotion::ShakeX);
        assert_eq!(snapshot.slot, UiFeedbackSlot::TopStatusBar);
        assert_eq!(snapshot.message, "WiFi unavailable");
        assert_eq!(snapshot.elapsed_ms, 120);
        assert_eq!(snapshot.ttl_ms, 700);
    }

    #[test]
    fn test_controller_expires_feedback_after_ttl() {
        let now = Instant::now();
        let mut controller = UiFeedbackController::default();
        controller.submit(
            UiInteractionFeedback {
                seq: 8,
                severity: UiFeedbackSeverity::Busy,
                target: UiFeedbackTarget::Page,
                motion: UiFeedbackMotion::Pulse,
                slot: UiFeedbackSlot::TopStatusBar,
                message: "Bind sent".to_string(),
                ttl_ms: 500,
            },
            now,
        );

        assert!(controller
            .snapshot(now + std::time::Duration::from_millis(501))
            .is_none());
    }

    #[test]
    fn test_snapshot_preserves_slot_and_target_details() {
        let snapshot = UiFeedbackSnapshot {
            seq: 9,
            severity: UiFeedbackSeverity::Success,
            target: UiFeedbackTarget::FieldId("wifi_manual".to_string()),
            motion: UiFeedbackMotion::Pulse,
            slot: UiFeedbackSlot::TopStatusBar,
            message: "WiFi start queued".to_string(),
            elapsed_ms: 42,
            ttl_ms: 850,
        };

        assert_eq!(snapshot.seq, 9);
        assert_eq!(
            snapshot.target,
            UiFeedbackTarget::FieldId("wifi_manual".to_string())
        );
        assert_eq!(snapshot.slot, UiFeedbackSlot::TopStatusBar);
    }
}

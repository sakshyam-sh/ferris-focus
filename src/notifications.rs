use crate::models::{FerrisStage, SessionType};

pub fn notify_level_up(new_level: u32, stage: FerrisStage) {
    let title = format!("🎉 Level Up! Level {}", new_level);
    let body = format!(
        "Your Ferris has hatched into a {}! {}",
        stage.label(),
        stage.emoji()
    );

    if let Err(e) = notify_rust::Notification::new()
        .summary(&title)
        .body(&body)
        .appname("Ferris Focus")
        .timeout(8000)
        .show()
    {
        eprintln!("Failed to send level-up notification: {}", e);
    }
}

pub fn notify_session_complete(session_type: SessionType, xp_earned: Option<u32>) {
    let (title, body) = match session_type {
        SessionType::Focus => {
            let xp_msg = xp_earned
                .map(|xp| format!(" +{} XP!", xp))
                .unwrap_or_default();
            (
                "🦀 Focus Complete!".to_string(),
                format!("Great work! Time for a break.{}", xp_msg),
            )
        }
        SessionType::ShortBreak => (
            "☕ Break Over!".to_string(),
            "Ready to focus again?".to_string(),
        ),
        SessionType::LongBreak => (
            "🎉 Long Break Over!".to_string(),
            "You've earned it! Ready to start a new cycle?".to_string(),
        ),
    };

    if let Err(e) = notify_rust::Notification::new()
        .summary(&title)
        .body(&body)
        .appname("Ferris Focus")
        .timeout(5000)
        .show()
    {
        eprintln!("Failed to send notification: {}", e);
    }
}

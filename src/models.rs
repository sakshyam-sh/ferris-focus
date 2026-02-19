use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Focus,
    ShortBreak,
    LongBreak,
}

impl SessionType {
    pub fn label(&self) -> &'static str {
        match self {
            SessionType::Focus => "FOCUS",
            SessionType::ShortBreak => "SHORT BREAK",
            SessionType::LongBreak => "LONG BREAK",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SessionType::Focus => "focus",
            SessionType::ShortBreak => "short_break",
            SessionType::LongBreak => "long_break",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "focus" => SessionType::Focus,
            "short_break" => SessionType::ShortBreak,
            "long_break" => SessionType::LongBreak,
            _ => SessionType::Focus,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_secs: u32,
    pub session_type: SessionType,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub total_xp: u32,
    pub level: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub last_session_date: Option<NaiveDate>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            total_xp: 0,
            level: 1,
            current_streak: 0,
            longest_streak: 0,
            last_session_date: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerrisStage {
    Egg,
    Hatchling,
    Junior,
    Senior,
    King,
}

impl FerrisStage {
    pub fn emoji(&self) -> &'static str {
        match self {
            FerrisStage::Egg => "🥚",
            FerrisStage::Hatchling => "🐣",
            FerrisStage::Junior => "🦀",
            FerrisStage::Senior => "⭐",
            FerrisStage::King => "👑",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FerrisStage::Egg => "Egg",
            FerrisStage::Hatchling => "Hatchling",
            FerrisStage::Junior => "Junior Crab",
            FerrisStage::Senior => "Senior Crab",
            FerrisStage::King => "King Crab",
        }
    }
}

pub const FOCUS_DURATION_SECS: u32 = 25 * 60;
pub const SHORT_BREAK_SECS: u32 = 5 * 60;
pub const LONG_BREAK_SECS: u32 = 15 * 60;
pub const SESSIONS_BEFORE_LONG_BREAK: u32 = 4;

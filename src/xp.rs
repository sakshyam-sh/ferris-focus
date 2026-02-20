use chrono::NaiveDate;

use crate::models::FerrisStage;

const BASE_XP: u32 = 100;
const STREAK_BONUS_PER_DAY: u32 = 10;
const MAX_STREAK_BONUS: u32 = 200;
const XP_PER_LEVEL: u32 = 500;

pub fn calculate_xp(current_streak: u32) -> u32 {
    let bonus = (current_streak * STREAK_BONUS_PER_DAY).min(MAX_STREAK_BONUS);
    BASE_XP + bonus
}

pub fn calculate_level(total_xp: u32) -> u32 {
    (total_xp / XP_PER_LEVEL) + 1
}

pub fn xp_for_next_level(level: u32) -> u32 {
    level * XP_PER_LEVEL
}

/// 0.0..1.0 progress within current level
pub fn level_progress(total_xp: u32) -> f32 {
    let level = calculate_level(total_xp);
    let xp_at_current_level = (level - 1) * XP_PER_LEVEL;
    let xp_in_level = total_xp - xp_at_current_level;
    xp_in_level as f32 / XP_PER_LEVEL as f32
}

pub fn ferris_stage(level: u32) -> FerrisStage {
    match level {
        1 => FerrisStage::Egg,
        2..=3 => FerrisStage::Hatchling,
        4..=6 => FerrisStage::Junior,
        7..=9 => FerrisStage::Senior,
        _ => FerrisStage::King,
    }
}

pub fn update_streak(
    last_session_date: Option<NaiveDate>,
    today: NaiveDate,
    current_streak: u32,
) -> u32 {
    match last_session_date {
        None => 1, // first session
        Some(last_date) => {
            let diff = (today - last_date).num_days();
            match diff {
                0 => current_streak.max(1), // same day
                1 => current_streak + 1,    // consecutive
                _ => 1,                     // missed, reset
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_base_xp() {
        assert_eq!(calculate_xp(0), 100);
    }

    #[test]
    fn test_streak_bonus() {
        assert_eq!(calculate_xp(5), 150); // 100 + 5*10
        assert_eq!(calculate_xp(10), 200); // 100 + 10*10
    }

    #[test]
    fn test_streak_bonus_cap() {
        assert_eq!(calculate_xp(50), 300); // 100 + min(500, 200) = 300
        assert_eq!(calculate_xp(100), 300); // same cap
    }

    #[test]
    fn test_level_calculation() {
        assert_eq!(calculate_level(0), 1);
        assert_eq!(calculate_level(499), 1);
        assert_eq!(calculate_level(500), 2);
        assert_eq!(calculate_level(999), 2);
        assert_eq!(calculate_level(1000), 3);
    }

    #[test]
    fn test_ferris_stages() {
        assert_eq!(ferris_stage(1), FerrisStage::Egg);
        assert_eq!(ferris_stage(2), FerrisStage::Hatchling);
        assert_eq!(ferris_stage(3), FerrisStage::Hatchling);
        assert_eq!(ferris_stage(4), FerrisStage::Junior);
        assert_eq!(ferris_stage(7), FerrisStage::Senior);
        assert_eq!(ferris_stage(10), FerrisStage::King);
        assert_eq!(ferris_stage(50), FerrisStage::King);
    }

    #[test]
    fn test_streak_continues() {
        let yesterday = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert_eq!(update_streak(Some(yesterday), today, 5), 6);
    }

    #[test]
    fn test_streak_resets() {
        let two_days_ago = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert_eq!(update_streak(Some(two_days_ago), today, 5), 1);
    }

    #[test]
    fn test_streak_same_day() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert_eq!(update_streak(Some(today), today, 3), 3);
    }

    #[test]
    fn test_first_session_streak() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert_eq!(update_streak(None, today, 0), 1);
    }

    #[test]
    fn test_level_progress() {
        assert!((level_progress(0) - 0.0).abs() < f32::EPSILON);
        assert!((level_progress(250) - 0.5).abs() < 0.01);
        assert!((level_progress(500) - 0.0).abs() < f32::EPSILON); // Level 2, 0 progress
        assert!((level_progress(750) - 0.5).abs() < 0.01);
    }
}

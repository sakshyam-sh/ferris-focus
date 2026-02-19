use chrono::NaiveDate;
use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

use crate::models::{Session, UserProfile};

/// db file path
fn db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferris-focus");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("ferris_focus.db")
}

/// open db + init tables
pub fn init_db() -> Result<Connection> {
    let conn = Connection::open(db_path())?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_secs INTEGER NOT NULL,
            session_type TEXT NOT NULL,
            completed BOOLEAN NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS user_profile (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            total_xp INTEGER NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            current_streak INTEGER NOT NULL DEFAULT 0,
            longest_streak INTEGER NOT NULL DEFAULT 0,
            last_session_date TEXT
        );

        INSERT OR IGNORE INTO user_profile (id, total_xp, level, current_streak, longest_streak, last_session_date)
        VALUES (1, 0, 1, 0, 0, NULL);
        ",
    )?;

    Ok(conn)
}

/// save session
pub fn save_session(conn: &Connection, session: &Session) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (started_at, completed_at, duration_secs, session_type, completed)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            session.started_at,
            session.completed_at,
            session.duration_secs,
            session.session_type.as_str(),
            session.completed,
        ],
    )?;
    Ok(())
}

/// load profile
pub fn get_profile(conn: &Connection) -> Result<UserProfile> {
    conn.query_row(
        "SELECT total_xp, level, current_streak, longest_streak, last_session_date FROM user_profile WHERE id = 1",
        [],
        |row| {
            let last_date_str: Option<String> = row.get(4)?;
            let last_session_date = last_date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
            Ok(UserProfile {
                total_xp: row.get(0)?,
                level: row.get(1)?,
                current_streak: row.get(2)?,
                longest_streak: row.get(3)?,
                last_session_date,
            })
        },
    )
}

/// save profile
pub fn update_profile(conn: &Connection, profile: &UserProfile) -> Result<()> {
    let last_date_str = profile.last_session_date.map(|d| d.format("%Y-%m-%d").to_string());
    conn.execute(
        "UPDATE user_profile SET total_xp = ?1, level = ?2, current_streak = ?3, longest_streak = ?4, last_session_date = ?5 WHERE id = 1",
        params![
            profile.total_xp,
            profile.level,
            profile.current_streak,
            profile.longest_streak,
            last_date_str,
        ],
    )?;
    Ok(())
}

/// today's completed focus count
pub fn get_today_session_count(conn: &Connection, today: &str) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE session_type = 'focus' AND completed = 1 AND started_at LIKE ?1",
        params![format!("{}%", today)],
        |row| row.get(0),
    )
}

/// sessions in range, grouped by date
pub fn get_sessions_in_range(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<Vec<(String, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT substr(started_at, 1, 10) as day, COUNT(*) as cnt
         FROM sessions
         WHERE session_type = 'focus' AND completed = 1
           AND substr(started_at, 1, 10) >= ?1
           AND substr(started_at, 1, 10) <= ?2
         GROUP BY day
         ORDER BY day",
    )?;

    let rows = stmt.query_map(params![start, end], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// total sessions + total focus secs
pub fn get_total_stats(conn: &Connection) -> Result<(u32, u32)> {
    conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(duration_secs), 0) FROM sessions WHERE session_type = 'focus' AND completed = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE sessions (
                id INTEGER PRIMARY KEY,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                duration_secs INTEGER NOT NULL,
                session_type TEXT NOT NULL,
                completed BOOLEAN NOT NULL DEFAULT 0
            );
            CREATE TABLE user_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                total_xp INTEGER NOT NULL DEFAULT 0,
                level INTEGER NOT NULL DEFAULT 1,
                current_streak INTEGER NOT NULL DEFAULT 0,
                longest_streak INTEGER NOT NULL DEFAULT 0,
                last_session_date TEXT
            );
            INSERT INTO user_profile (id, total_xp, level, current_streak, longest_streak, last_session_date)
            VALUES (1, 0, 1, 0, 0, NULL);
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_init_and_save() {
        let conn = in_memory_db();
        let session = Session {
            id: None,
            started_at: "2026-02-19T10:00:00".to_string(),
            completed_at: Some("2026-02-19T10:25:00".to_string()),
            duration_secs: 1500,
            session_type: SessionType::Focus,
            completed: true,
        };
        save_session(&conn, &session).unwrap();

        let count = get_today_session_count(&conn, "2026-02-19").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_profile_roundtrip() {
        let conn = in_memory_db();
        let mut profile = get_profile(&conn).unwrap();
        assert_eq!(profile.total_xp, 0);

        profile.total_xp = 500;
        profile.level = 2;
        profile.current_streak = 3;
        profile.last_session_date = NaiveDate::from_ymd_opt(2026, 2, 19);
        update_profile(&conn, &profile).unwrap();

        let loaded = get_profile(&conn).unwrap();
        assert_eq!(loaded.total_xp, 500);
        assert_eq!(loaded.level, 2);
        assert_eq!(loaded.current_streak, 3);
        assert_eq!(loaded.last_session_date, NaiveDate::from_ymd_opt(2026, 2, 19));
    }

    #[test]
    fn test_total_stats() {
        let conn = in_memory_db();
        let session = Session {
            id: None,
            started_at: "2026-02-19T10:00:00".to_string(),
            completed_at: Some("2026-02-19T10:25:00".to_string()),
            duration_secs: 1500,
            session_type: SessionType::Focus,
            completed: true,
        };
        save_session(&conn, &session).unwrap();
        save_session(&conn, &session).unwrap();

        let (count, total_secs) = get_total_stats(&conn).unwrap();
        assert_eq!(count, 2);
        assert_eq!(total_secs, 3000);
    }
}

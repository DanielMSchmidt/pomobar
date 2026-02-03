//! SQLite persistence layer for settings and session data.

use crate::models::{DailyStats, Session, Settings};
use chrono::{Local, NaiveDate};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Failed to create database directory")]
    DirectoryCreation,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Creates a new database connection, initializing tables if needed.
    pub fn new() -> Result<Self, DatabaseError> {
        let db_path = Self::db_path();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| DatabaseError::DirectoryCreation)?;
        }

        let conn = Connection::open(&db_path)?;
        Self::initialize_tables(&conn)?;

        Ok(Self { conn })
    }

    /// Creates an in-memory database (for testing).
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_tables(&conn)?;
        Ok(Self { conn })
    }

    fn initialize_tables(conn: &Connection) -> Result<(), DatabaseError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                completed_pomodoros INTEGER NOT NULL DEFAULT 0,
                total_focus_minutes INTEGER NOT NULL DEFAULT 0
            );
        "#,
        )?;
        Ok(())
    }

    fn db_path() -> PathBuf {
        ProjectDirs::from("com", "pomobar", "Pomobar")
            .map(|dirs| dirs.data_dir().join("pomobar.db"))
            .unwrap_or_else(|| PathBuf::from("pomobar.db"))
    }

    /// Loads settings from the database, returning defaults if not found.
    pub fn load_settings(&self) -> Result<Settings, DatabaseError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'config'",
                [],
                |row| row.get(0),
            )
            .ok();

        match json {
            Some(j) => Ok(serde_json::from_str(&j)?),
            None => Ok(Settings::default()),
        }
    }

    /// Saves settings to the database.
    pub fn save_settings(&self, settings: &Settings) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(settings)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('config', ?)",
            [&json],
        )?;
        Ok(())
    }

    /// Loads the session for today from the database.
    pub fn load_today_session(&self) -> Result<Session, DatabaseError> {
        let today = Local::now().date_naive();
        let stats = self.get_daily_stats(today)?;

        Ok(Session {
            pomodoros_completed_today: stats.completed_pomodoros,
            total_focus_mins_today: stats.total_focus_minutes,
            pomodoros_in_cycle: 0, // Reset cycle on app restart
            last_date: today,
        })
    }

    /// Saves the current session to the database.
    pub fn save_session(&self, session: &Session) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO daily_stats (date, completed_pomodoros, total_focus_minutes)
             VALUES (?, ?, ?)",
            params![
                session.last_date.to_string(),
                session.pomodoros_completed_today,
                session.total_focus_mins_today,
            ],
        )?;
        Ok(())
    }

    /// Gets daily statistics for a specific date.
    pub fn get_daily_stats(&self, date: NaiveDate) -> Result<DailyStats, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT date, completed_pomodoros, total_focus_minutes
             FROM daily_stats WHERE date = ?",
            [date.to_string()],
            |row| {
                Ok(DailyStats {
                    date,
                    completed_pomodoros: row.get(1)?,
                    total_focus_minutes: row.get(2)?,
                })
            },
        );

        match result {
            Ok(stats) => Ok(stats),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(DailyStats::new(date)),
            Err(e) => Err(e.into()),
        }
    }

    /// Resets the statistics for today.
    pub fn reset_today(&self) -> Result<(), DatabaseError> {
        let today = Local::now().date_naive();
        self.conn.execute(
            "DELETE FROM daily_stats WHERE date = ?",
            [today.to_string()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new_in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn test_settings_save_and_load() {
        let db = Database::new_in_memory().unwrap();

        // Default settings should be returned when nothing is saved
        let settings = db.load_settings().unwrap();
        assert_eq!(settings, Settings::default());

        // Save custom settings
        let custom_settings = Settings {
            pomodoro_mins: 30,
            short_break_mins: 10,
            long_break_mins: 20,
            pomodoros_for_long_break: 3,
            sound_enabled: false,
            notifications_enabled: true,
        };
        db.save_settings(&custom_settings).unwrap();

        // Load and verify
        let loaded = db.load_settings().unwrap();
        assert_eq!(loaded, custom_settings);
    }

    #[test]
    fn test_session_save_and_load() {
        let db = Database::new_in_memory().unwrap();

        // Create and save a session
        let today = Local::now().date_naive();
        let session = Session {
            pomodoros_completed_today: 5,
            total_focus_mins_today: 125,
            pomodoros_in_cycle: 2,
            last_date: today,
        };
        db.save_session(&session).unwrap();

        // Load and verify (note: pomodoros_in_cycle resets on load)
        let loaded = db.load_today_session().unwrap();
        assert_eq!(loaded.pomodoros_completed_today, 5);
        assert_eq!(loaded.total_focus_mins_today, 125);
        assert_eq!(loaded.pomodoros_in_cycle, 0); // Always 0 on load
        assert_eq!(loaded.last_date, today);
    }

    #[test]
    fn test_daily_stats_for_nonexistent_date() {
        let db = Database::new_in_memory().unwrap();

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let stats = db.get_daily_stats(date).unwrap();

        assert_eq!(stats.date, date);
        assert_eq!(stats.completed_pomodoros, 0);
        assert_eq!(stats.total_focus_minutes, 0);
    }

    #[test]
    fn test_reset_today() {
        let db = Database::new_in_memory().unwrap();

        // Save some data
        let today = Local::now().date_naive();
        let session = Session {
            pomodoros_completed_today: 5,
            total_focus_mins_today: 125,
            pomodoros_in_cycle: 2,
            last_date: today,
        };
        db.save_session(&session).unwrap();

        // Reset
        db.reset_today().unwrap();

        // Verify it's cleared
        let loaded = db.load_today_session().unwrap();
        assert_eq!(loaded.pomodoros_completed_today, 0);
        assert_eq!(loaded.total_focus_mins_today, 0);
    }

    #[test]
    fn test_settings_overwrite() {
        let db = Database::new_in_memory().unwrap();

        let settings1 = Settings {
            pomodoro_mins: 30,
            ..Settings::default()
        };
        db.save_settings(&settings1).unwrap();

        let settings2 = Settings {
            pomodoro_mins: 45,
            ..Settings::default()
        };
        db.save_settings(&settings2).unwrap();

        let loaded = db.load_settings().unwrap();
        assert_eq!(loaded.pomodoro_mins, 45);
    }
}

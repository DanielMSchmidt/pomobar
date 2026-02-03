//! Main application state and logic.

use crate::models::{Session, Settings, TimerState};
use crate::persistence::{Database, DatabaseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
}

/// Events that should trigger notifications/sounds on the main thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompletionEvent {
    PomodoroComplete { count: u32, is_long_break: bool },
    BreakComplete,
}

/// Main application state (without audio - audio is handled separately on main thread).
pub struct App {
    pub state: TimerState,
    pub settings: Settings,
    pub session: Session,
    pub db: Database,
}

impl App {
    /// Creates a new application instance.
    pub fn new() -> Result<Self, AppError> {
        let db = Database::new()?;
        let settings = db.load_settings()?;
        let session = db.load_today_session()?;

        Ok(Self {
            state: TimerState::Idle,
            settings,
            session,
            db,
        })
    }

    /// Creates a new app with a custom database (for testing).
    #[cfg(test)]
    pub fn new_with_db(db: Database) -> Result<Self, AppError> {
        let settings = db.load_settings()?;
        let session = db.load_today_session()?;

        Ok(Self {
            state: TimerState::Idle,
            settings,
            session,
            db,
        })
    }

    /// Starts a new pomodoro session.
    pub fn start_pomodoro(&mut self) {
        let total_secs = self.settings.pomodoro_mins * 60;
        self.state = TimerState::PomodoroActive {
            remaining_secs: total_secs,
            total_secs,
        };
    }

    /// Pauses the current pomodoro.
    pub fn pause(&mut self) {
        if let TimerState::PomodoroActive {
            remaining_secs,
            total_secs,
        } = self.state
        {
            self.state = TimerState::PomodoroPaused {
                remaining_secs,
                total_secs,
            };
        }
    }

    /// Resumes a paused pomodoro.
    pub fn resume(&mut self) {
        if let TimerState::PomodoroPaused {
            remaining_secs,
            total_secs,
        } = self.state
        {
            self.state = TimerState::PomodoroActive {
                remaining_secs,
                total_secs,
            };
        }
    }

    /// Stops the current timer and returns to idle.
    pub fn stop(&mut self) {
        self.state = TimerState::Idle;
    }

    /// Completes the current pomodoro early.
    /// Returns a completion event if the pomodoro was active.
    pub fn complete_early(&mut self) -> Option<CompletionEvent> {
        if matches!(self.state, TimerState::PomodoroActive { .. }) {
            Some(self.finish_pomodoro())
        } else {
            None
        }
    }

    /// Skips the current break.
    pub fn skip_break(&mut self) {
        if matches!(self.state, TimerState::BreakActive { .. }) {
            self.state = TimerState::BreakFinished;
        }
    }

    /// Advances the timer by one second.
    /// Returns (state_changed, optional_completion_event).
    pub fn tick(&mut self) -> (bool, Option<CompletionEvent>) {
        match &mut self.state {
            TimerState::PomodoroActive { remaining_secs, .. } => {
                if *remaining_secs > 0 {
                    *remaining_secs -= 1;
                    (true, None)
                } else {
                    let event = self.finish_pomodoro();
                    (true, Some(event))
                }
            }
            TimerState::BreakActive { remaining_secs, .. } => {
                if *remaining_secs > 0 {
                    *remaining_secs -= 1;
                    (true, None)
                } else {
                    self.finish_break();
                    (true, Some(CompletionEvent::BreakComplete))
                }
            }
            _ => (false, None),
        }
    }

    fn finish_pomodoro(&mut self) -> CompletionEvent {
        // Update session
        self.session.complete_pomodoro(self.settings.pomodoro_mins);
        let _ = self.db.save_session(&self.session);

        // Determine break type
        let is_long = self
            .session
            .is_long_break_due(self.settings.pomodoros_for_long_break);
        if is_long {
            self.session.reset_cycle();
        }

        let break_mins = if is_long {
            self.settings.long_break_mins
        } else {
            self.settings.short_break_mins
        };

        let total_secs = break_mins * 60;
        self.state = TimerState::BreakActive {
            is_long_break: is_long,
            remaining_secs: total_secs,
            total_secs,
        };

        CompletionEvent::PomodoroComplete {
            count: self.session.pomodoros_completed_today,
            is_long_break: is_long,
        }
    }

    fn finish_break(&mut self) {
        self.state = TimerState::BreakFinished;
    }

    /// Updates a setting and saves to database.
    pub fn update_setting<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut Settings),
    {
        updater(&mut self.settings);
        let _ = self.db.save_settings(&self.settings);
    }

    /// Resets today's statistics.
    pub fn reset_today(&mut self) {
        self.session.reset_today();
        let _ = self.db.reset_today();
    }

    /// Returns the long break duration in minutes.
    pub fn long_break_mins(&self) -> u32 {
        self.settings.long_break_mins
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Database;

    fn create_test_app() -> App {
        let db = Database::new_in_memory().unwrap();
        App::new_with_db(db).unwrap()
    }

    #[test]
    fn test_app_initial_state() {
        let app = create_test_app();
        assert!(app.state.is_idle());
        assert_eq!(app.settings, Settings::default());
    }

    #[test]
    fn test_start_pomodoro() {
        let mut app = create_test_app();
        app.start_pomodoro();

        assert!(matches!(app.state, TimerState::PomodoroActive { .. }));
        assert_eq!(app.state.remaining_secs(), Some(25 * 60));
    }

    #[test]
    fn test_pause_and_resume() {
        let mut app = create_test_app();
        app.start_pomodoro();

        // Tick a few times
        for _ in 0..10 {
            app.tick();
        }

        app.pause();
        assert!(app.state.is_paused());
        let remaining_before = app.state.remaining_secs().unwrap();

        // Tick shouldn't change time when paused
        app.tick();
        assert_eq!(app.state.remaining_secs().unwrap(), remaining_before);

        app.resume();
        assert!(matches!(app.state, TimerState::PomodoroActive { .. }));
    }

    #[test]
    fn test_stop() {
        let mut app = create_test_app();
        app.start_pomodoro();
        app.stop();

        assert!(app.state.is_idle());
        assert!(matches!(app.state, TimerState::Idle));
    }

    #[test]
    fn test_complete_early() {
        let mut app = create_test_app();
        app.start_pomodoro();
        let event = app.complete_early();

        // Should transition to break and return event
        assert!(app.state.is_break());
        assert_eq!(app.session.pomodoros_completed_today, 1);
        assert!(matches!(
            event,
            Some(CompletionEvent::PomodoroComplete { count: 1, .. })
        ));
    }

    #[test]
    fn test_skip_break() {
        let mut app = create_test_app();
        app.start_pomodoro();
        app.complete_early();

        assert!(app.state.is_break());

        app.skip_break();
        assert!(matches!(app.state, TimerState::BreakFinished));
    }

    #[test]
    fn test_pomodoro_completes_to_break() {
        let mut app = create_test_app();
        app.settings.pomodoro_mins = 1; // 1 minute for faster test

        app.start_pomodoro();

        // Tick through the entire pomodoro
        let mut completion_event = None;
        for _ in 0..61 {
            let (_, event) = app.tick();
            if event.is_some() {
                completion_event = event;
                break;
            }
        }

        assert!(app.state.is_break());
        assert_eq!(app.session.pomodoros_completed_today, 1);
        assert!(completion_event.is_some());
    }

    #[test]
    fn test_long_break_after_threshold() {
        let mut app = create_test_app();
        app.settings.pomodoros_for_long_break = 2;

        // Complete first pomodoro
        app.start_pomodoro();
        let event = app.complete_early();

        // Should be short break
        if let TimerState::BreakActive { is_long_break, .. } = &app.state {
            assert!(!is_long_break);
        } else {
            panic!("Expected BreakActive state");
        }
        assert!(matches!(
            event,
            Some(CompletionEvent::PomodoroComplete {
                is_long_break: false,
                ..
            })
        ));

        app.skip_break();
        app.start_pomodoro();
        let event = app.complete_early();

        // Should be long break now
        if let TimerState::BreakActive { is_long_break, .. } = &app.state {
            assert!(*is_long_break);
        } else {
            panic!("Expected BreakActive state");
        }
        assert!(matches!(
            event,
            Some(CompletionEvent::PomodoroComplete {
                is_long_break: true,
                ..
            })
        ));
    }

    #[test]
    fn test_update_setting() {
        let mut app = create_test_app();
        app.update_setting(|s| s.pomodoro_mins = 30);

        assert_eq!(app.settings.pomodoro_mins, 30);

        // Verify it was saved
        let loaded = app.db.load_settings().unwrap();
        assert_eq!(loaded.pomodoro_mins, 30);
    }

    #[test]
    fn test_reset_today() {
        let mut app = create_test_app();
        app.start_pomodoro();
        app.complete_early();

        assert_eq!(app.session.pomodoros_completed_today, 1);

        app.reset_today();

        assert_eq!(app.session.pomodoros_completed_today, 0);
        assert_eq!(app.session.total_focus_mins_today, 0);
    }

    #[test]
    fn test_tick_returns_correct_flags() {
        let mut app = create_test_app();

        // Idle state - no changes
        let (changed, event) = app.tick();
        assert!(!changed);
        assert!(event.is_none());

        // Active state - changes but not completed
        app.start_pomodoro();
        let (changed, event) = app.tick();
        assert!(changed);
        assert!(event.is_none());

        // Paused state - no changes
        app.pause();
        let (changed, event) = app.tick();
        assert!(!changed);
        assert!(event.is_none());
    }

    #[test]
    fn test_break_completion_event() {
        let mut app = create_test_app();
        app.settings.short_break_mins = 1; // 1 minute for faster test

        // Start and complete a pomodoro to get to break
        app.start_pomodoro();
        app.complete_early();

        assert!(app.state.is_break());

        // Tick through the break
        let mut break_event = None;
        for _ in 0..61 {
            let (_, event) = app.tick();
            if event.is_some() {
                break_event = event;
                break;
            }
        }

        assert!(matches!(app.state, TimerState::BreakFinished));
        assert!(matches!(break_event, Some(CompletionEvent::BreakComplete)));
    }
}

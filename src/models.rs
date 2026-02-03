//! Data models for the Pomobar application.

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

/// Timer state machine representing all possible states of the pomodoro timer.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TimerState {
    /// No active timer, ready to start a pomodoro.
    #[default]
    Idle,
    /// Pomodoro work session in progress.
    PomodoroActive {
        remaining_secs: u32,
        total_secs: u32,
    },
    /// Pomodoro paused by user.
    PomodoroPaused {
        remaining_secs: u32,
        total_secs: u32,
    },
    /// Break in progress (short or long).
    BreakActive {
        is_long_break: bool,
        remaining_secs: u32,
        total_secs: u32,
    },
    /// Break finished, waiting for user to start next pomodoro.
    BreakFinished,
}

impl TimerState {
    /// Returns true if the timer is in an idle state (Idle or BreakFinished).
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle | Self::BreakFinished)
    }

    /// Returns true if the timer is actively counting down.
    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::PomodoroActive { .. } | Self::BreakActive { .. })
    }

    /// Returns true if the timer is paused.
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::PomodoroPaused { .. })
    }

    /// Returns true if currently in a pomodoro session (active or paused).
    pub fn is_pomodoro(&self) -> bool {
        matches!(
            self,
            Self::PomodoroActive { .. } | Self::PomodoroPaused { .. }
        )
    }

    /// Returns true if currently on a break.
    pub fn is_break(&self) -> bool {
        matches!(self, Self::BreakActive { .. })
    }

    /// Returns the progress percentage (0.0 to 1.0) if a timer is active.
    pub fn progress_percent(&self) -> Option<f32> {
        match self {
            Self::PomodoroActive {
                remaining_secs,
                total_secs,
            }
            | Self::PomodoroPaused {
                remaining_secs,
                total_secs,
            }
            | Self::BreakActive {
                remaining_secs,
                total_secs,
                ..
            } => {
                if *total_secs == 0 {
                    return Some(1.0);
                }
                Some(1.0 - (*remaining_secs as f32 / *total_secs as f32))
            }
            _ => None,
        }
    }

    /// Returns the remaining seconds if a timer is active.
    #[cfg(test)]
    pub fn remaining_secs(&self) -> Option<u32> {
        match self {
            Self::PomodoroActive { remaining_secs, .. }
            | Self::PomodoroPaused { remaining_secs, .. }
            | Self::BreakActive { remaining_secs, .. } => Some(*remaining_secs),
            _ => None,
        }
    }

    /// Returns the total seconds if a timer is active.
    #[cfg(test)]
    pub fn total_secs(&self) -> Option<u32> {
        match self {
            Self::PomodoroActive { total_secs, .. }
            | Self::PomodoroPaused { total_secs, .. }
            | Self::BreakActive { total_secs, .. } => Some(*total_secs),
            _ => None,
        }
    }
}

/// User-configurable settings for the pomodoro timer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    /// Duration of a pomodoro work session in minutes.
    pub pomodoro_mins: u32,
    /// Duration of a short break in minutes.
    pub short_break_mins: u32,
    /// Duration of a long break in minutes.
    pub long_break_mins: u32,
    /// Number of pomodoros before a long break.
    pub pomodoros_for_long_break: u32,
    /// Whether to play sounds on timer completion.
    pub sound_enabled: bool,
    /// Whether to show system notifications.
    pub notifications_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pomodoro_mins: 25,
            short_break_mins: 5,
            long_break_mins: 15,
            pomodoros_for_long_break: 4,
            sound_enabled: true,
            notifications_enabled: true,
        }
    }
}

/// Session tracking for the current day.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// Number of pomodoros completed today.
    pub pomodoros_completed_today: u32,
    /// Total minutes of focus time today.
    pub total_focus_mins_today: u32,
    /// Number of pomodoros in current cycle (resets after long break).
    pub pomodoros_in_cycle: u32,
    /// The date these stats are for.
    pub last_date: NaiveDate,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            pomodoros_completed_today: 0,
            total_focus_mins_today: 0,
            pomodoros_in_cycle: 0,
            last_date: Local::now().date_naive(),
        }
    }
}

impl Session {
    /// Creates a new session for the given date.
    #[cfg(test)]
    pub fn new(date: NaiveDate) -> Self {
        Self {
            pomodoros_completed_today: 0,
            total_focus_mins_today: 0,
            pomodoros_in_cycle: 0,
            last_date: date,
        }
    }

    /// Checks if the day has rolled over and resets daily counts if so.
    pub fn check_day_rollover(&mut self) {
        let today = Local::now().date_naive();
        if self.last_date != today {
            self.pomodoros_completed_today = 0;
            self.total_focus_mins_today = 0;
            self.last_date = today;
        }
    }

    /// Records completion of a pomodoro.
    pub fn complete_pomodoro(&mut self, duration_mins: u32) {
        self.check_day_rollover();
        self.pomodoros_completed_today += 1;
        self.total_focus_mins_today += duration_mins;
        self.pomodoros_in_cycle += 1;
    }

    /// Returns true if a long break is due based on the threshold.
    pub fn is_long_break_due(&self, threshold: u32) -> bool {
        self.pomodoros_in_cycle >= threshold
    }

    /// Resets the current cycle counter (called after a long break).
    pub fn reset_cycle(&mut self) {
        self.pomodoros_in_cycle = 0;
    }

    /// Resets all counts for today.
    pub fn reset_today(&mut self) {
        self.pomodoros_completed_today = 0;
        self.total_focus_mins_today = 0;
        self.pomodoros_in_cycle = 0;
    }
}

/// Daily statistics for persistence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyStats {
    pub date: NaiveDate,
    pub completed_pomodoros: u32,
    pub total_focus_minutes: u32,
}

impl DailyStats {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            completed_pomodoros: 0,
            total_focus_minutes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_timer_state_idle() {
        let state = TimerState::Idle;
        assert!(state.is_idle());
        assert!(!state.is_active());
        assert!(!state.is_paused());
        assert!(!state.is_pomodoro());
        assert!(!state.is_break());
        assert_eq!(state.progress_percent(), None);
        assert_eq!(state.remaining_secs(), None);
    }

    #[test]
    fn test_timer_state_pomodoro_active() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 1200,
            total_secs: 1500,
        };
        assert!(!state.is_idle());
        assert!(state.is_active());
        assert!(!state.is_paused());
        assert!(state.is_pomodoro());
        assert!(!state.is_break());
        assert_eq!(state.remaining_secs(), Some(1200));
        assert_eq!(state.total_secs(), Some(1500));

        // Progress should be 20% (300 seconds elapsed out of 1500)
        let progress = state.progress_percent().unwrap();
        assert!((progress - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_timer_state_pomodoro_paused() {
        let state = TimerState::PomodoroPaused {
            remaining_secs: 600,
            total_secs: 1500,
        };
        assert!(!state.is_idle());
        assert!(!state.is_active());
        assert!(state.is_paused());
        assert!(state.is_pomodoro());
        assert!(!state.is_break());
        assert_eq!(state.remaining_secs(), Some(600));

        // Progress should be 60%
        let progress = state.progress_percent().unwrap();
        assert!((progress - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_timer_state_break_active() {
        let state = TimerState::BreakActive {
            is_long_break: false,
            remaining_secs: 150,
            total_secs: 300,
        };
        assert!(!state.is_idle());
        assert!(state.is_active());
        assert!(!state.is_paused());
        assert!(!state.is_pomodoro());
        assert!(state.is_break());
        assert_eq!(state.remaining_secs(), Some(150));

        // Progress should be 50%
        let progress = state.progress_percent().unwrap();
        assert!((progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_timer_state_break_finished() {
        let state = TimerState::BreakFinished;
        assert!(state.is_idle());
        assert!(!state.is_active());
        assert!(!state.is_paused());
        assert!(!state.is_pomodoro());
        assert!(!state.is_break());
    }

    #[test]
    fn test_timer_state_progress_at_zero() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 0,
            total_secs: 1500,
        };
        let progress = state.progress_percent().unwrap();
        assert!((progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_timer_state_progress_division_by_zero() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 0,
            total_secs: 0,
        };
        let progress = state.progress_percent().unwrap();
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.pomodoro_mins, 25);
        assert_eq!(settings.short_break_mins, 5);
        assert_eq!(settings.long_break_mins, 15);
        assert_eq!(settings.pomodoros_for_long_break, 4);
        assert!(settings.sound_enabled);
        assert!(settings.notifications_enabled);
    }

    #[test]
    fn test_session_default() {
        let session = Session::default();
        assert_eq!(session.pomodoros_completed_today, 0);
        assert_eq!(session.total_focus_mins_today, 0);
        assert_eq!(session.pomodoros_in_cycle, 0);
        assert_eq!(session.last_date, Local::now().date_naive());
    }

    #[test]
    fn test_session_complete_pomodoro() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let mut session = Session::new(date);

        // Move the date to today so check_day_rollover doesn't reset
        session.last_date = Local::now().date_naive();

        session.complete_pomodoro(25);
        assert_eq!(session.pomodoros_completed_today, 1);
        assert_eq!(session.total_focus_mins_today, 25);
        assert_eq!(session.pomodoros_in_cycle, 1);

        session.complete_pomodoro(25);
        assert_eq!(session.pomodoros_completed_today, 2);
        assert_eq!(session.total_focus_mins_today, 50);
        assert_eq!(session.pomodoros_in_cycle, 2);
    }

    #[test]
    fn test_session_long_break_due() {
        let mut session = Session::default();
        assert!(!session.is_long_break_due(4));

        session.pomodoros_in_cycle = 3;
        assert!(!session.is_long_break_due(4));

        session.pomodoros_in_cycle = 4;
        assert!(session.is_long_break_due(4));

        session.pomodoros_in_cycle = 5;
        assert!(session.is_long_break_due(4));
    }

    #[test]
    fn test_session_reset_cycle() {
        let mut session = Session::default();
        session.pomodoros_in_cycle = 4;
        session.reset_cycle();
        assert_eq!(session.pomodoros_in_cycle, 0);
    }

    #[test]
    fn test_session_reset_today() {
        let mut session = Session::default();
        session.pomodoros_completed_today = 5;
        session.total_focus_mins_today = 125;
        session.pomodoros_in_cycle = 3;

        session.reset_today();

        assert_eq!(session.pomodoros_completed_today, 0);
        assert_eq!(session.total_focus_mins_today, 0);
        assert_eq!(session.pomodoros_in_cycle, 0);
    }

    #[test]
    fn test_daily_stats_new() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let stats = DailyStats::new(date);
        assert_eq!(stats.date, date);
        assert_eq!(stats.completed_pomodoros, 0);
        assert_eq!(stats.total_focus_minutes, 0);
    }
}

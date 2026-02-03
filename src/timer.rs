//! Timer tick loop for updating pomodoro state.

use crate::app::{App, CompletionEvent};
use crate::models::TimerState;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Message sent from the timer thread to the main thread.
#[derive(Debug, Clone)]
pub enum TimerMessage {
    /// Timer state has changed, UI needs update.
    StateChanged { title: String },
    /// A timer completed, trigger notification/sound.
    Completed(CompletionEvent),
}

/// Runs the timer loop, ticking every second.
/// Sends messages to the main thread via the provided channel.
pub fn run_timer_loop(app: Arc<Mutex<App>>, tx: Sender<TimerMessage>) {
    loop {
        thread::sleep(Duration::from_secs(1));

        let message = {
            let mut app = app.lock().unwrap();

            // Check for day rollover
            app.session.check_day_rollover();

            // Tick the timer
            let (changed, completion) = app.tick();

            if let Some(event) = completion {
                // Send completion event
                let _ = tx.send(TimerMessage::Completed(event));
            }

            if changed {
                // Send state update
                let title = format_tray_title(&app.state);
                Some(TimerMessage::StateChanged { title })
            } else {
                None
            }
        };

        if let Some(msg) = message {
            let _ = tx.send(msg);
        }
    }
}

/// Formats the tray title based on current timer state.
pub fn format_tray_title(state: &TimerState) -> String {
    match state {
        TimerState::Idle | TimerState::BreakFinished => "🍅".to_string(),
        TimerState::PomodoroActive { remaining_secs, .. } => {
            format!("🍅 {:02}:{:02}", remaining_secs / 60, remaining_secs % 60)
        }
        TimerState::PomodoroPaused { remaining_secs, .. } => {
            format!("⏸ {:02}:{:02}", remaining_secs / 60, remaining_secs % 60)
        }
        TimerState::BreakActive { remaining_secs, .. } => {
            format!("☕ {:02}:{:02}", remaining_secs / 60, remaining_secs % 60)
        }
    }
}

/// Formats time in MM:SS format.
pub fn format_time(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tray_title_idle() {
        let state = TimerState::Idle;
        assert_eq!(format_tray_title(&state), "🍅");
    }

    #[test]
    fn test_format_tray_title_break_finished() {
        let state = TimerState::BreakFinished;
        assert_eq!(format_tray_title(&state), "🍅");
    }

    #[test]
    fn test_format_tray_title_pomodoro_active() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 1432,
            total_secs: 1500,
        };
        assert_eq!(format_tray_title(&state), "🍅 23:52");
    }

    #[test]
    fn test_format_tray_title_pomodoro_paused() {
        let state = TimerState::PomodoroPaused {
            remaining_secs: 600,
            total_secs: 1500,
        };
        assert_eq!(format_tray_title(&state), "⏸ 10:00");
    }

    #[test]
    fn test_format_tray_title_break_active() {
        let state = TimerState::BreakActive {
            is_long_break: false,
            remaining_secs: 272,
            total_secs: 300,
        };
        assert_eq!(format_tray_title(&state), "☕ 04:32");
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0), "00:00");
        assert_eq!(format_time(59), "00:59");
        assert_eq!(format_time(60), "01:00");
        assert_eq!(format_time(125), "02:05");
        assert_eq!(format_time(1500), "25:00");
        assert_eq!(format_time(3599), "59:59");
    }
}

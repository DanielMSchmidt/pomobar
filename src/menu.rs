//! Menu building and updating for the tray dropdown.

use crate::models::{Session, Settings, TimerState};
use crate::timer::format_time;
use muda::accelerator::Accelerator;
use muda::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use std::collections::HashMap;
use thiserror::Error;

// Menu item IDs as constants
pub const ID_STATUS: &str = "status";
pub const ID_PROGRESS: &str = "progress";
pub const ID_STATS: &str = "stats";
pub const ID_START: &str = "start";
pub const ID_PAUSE: &str = "pause";
pub const ID_RESUME: &str = "resume";
pub const ID_STOP: &str = "stop";
pub const ID_COMPLETE: &str = "complete";
pub const ID_SKIP_BREAK: &str = "skip_break";
pub const ID_SOUND_TOGGLE: &str = "sound_toggle";
pub const ID_NOTIF_TOGGLE: &str = "notif_toggle";
pub const ID_RESET_COUNT: &str = "reset_count";
pub const ID_QUIT: &str = "quit";

#[derive(Error, Debug)]
pub enum MenuError {
    #[error("Menu error: {0}")]
    Muda(#[from] muda::Error),
}

/// Holds references to menu items that need dynamic updates.
pub struct MenuItems {
    pub status: MenuItem,
    pub progress: MenuItem,
    pub stats: MenuItem,
    pub start: MenuItem,
    pub pause: MenuItem,
    pub resume: MenuItem,
    pub stop: MenuItem,
    pub complete: MenuItem,
    pub skip_break: MenuItem,
    pub sound_toggle: CheckMenuItem,
    pub notif_toggle: CheckMenuItem,
    pub pomo_checks: HashMap<u32, CheckMenuItem>,
    pub short_checks: HashMap<u32, CheckMenuItem>,
    pub long_checks: HashMap<u32, CheckMenuItem>,
    pub thresh_checks: HashMap<u32, CheckMenuItem>,
}

/// Builds the complete menu structure.
pub fn build_menu(
    state: &TimerState,
    session: &Session,
    settings: &Settings,
) -> Result<(Menu, MenuItems), MenuError> {
    let menu = Menu::new();

    // Status display (disabled, info only)
    let status = MenuItem::with_id(
        MenuId::new(ID_STATUS),
        format_status(state),
        false, // disabled
        None::<Accelerator>,
    );
    menu.append(&status)?;

    // Progress bar (ASCII)
    let progress = MenuItem::with_id(
        MenuId::new(ID_PROGRESS),
        format_progress(state),
        false,
        None::<Accelerator>,
    );
    menu.append(&progress)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Today's stats
    let stats = MenuItem::with_id(
        MenuId::new(ID_STATS),
        format_stats(session),
        false,
        None::<Accelerator>,
    );
    menu.append(&stats)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Control buttons
    let start = MenuItem::with_id(
        MenuId::new(ID_START),
        "▶  Start Pomodoro",
        state.is_idle(),
        None::<Accelerator>,
    );
    let pause = MenuItem::with_id(
        MenuId::new(ID_PAUSE),
        "⏸  Pause",
        matches!(state, TimerState::PomodoroActive { .. }),
        None::<Accelerator>,
    );
    let resume = MenuItem::with_id(
        MenuId::new(ID_RESUME),
        "▶  Resume",
        state.is_paused(),
        None::<Accelerator>,
    );
    let stop = MenuItem::with_id(
        MenuId::new(ID_STOP),
        "⏹  Stop",
        state.is_pomodoro(),
        None::<Accelerator>,
    );
    let complete = MenuItem::with_id(
        MenuId::new(ID_COMPLETE),
        "✓  Complete Early",
        matches!(state, TimerState::PomodoroActive { .. }),
        None::<Accelerator>,
    );
    let skip_break = MenuItem::with_id(
        MenuId::new(ID_SKIP_BREAK),
        "⏭  Skip Break",
        state.is_break(),
        None::<Accelerator>,
    );

    menu.append(&start)?;
    menu.append(&pause)?;
    menu.append(&resume)?;
    menu.append(&stop)?;
    menu.append(&complete)?;
    menu.append(&skip_break)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Settings submenu
    let (settings_menu, pomo_checks, short_checks, long_checks, thresh_checks, sound_toggle, notif_toggle) =
        build_settings_submenu(settings)?;
    menu.append(&settings_menu)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Quit
    let quit = MenuItem::with_id(MenuId::new(ID_QUIT), "Quit Pomobar", true, None::<Accelerator>);
    menu.append(&quit)?;

    let items = MenuItems {
        status,
        progress,
        stats,
        start,
        pause,
        resume,
        stop,
        complete,
        skip_break,
        sound_toggle,
        notif_toggle,
        pomo_checks,
        short_checks,
        long_checks,
        thresh_checks,
    };

    Ok((menu, items))
}

/// Type alias for the settings submenu result to avoid clippy complexity warning.
type SettingsSubmenuResult = (
    Submenu,
    HashMap<u32, CheckMenuItem>,
    HashMap<u32, CheckMenuItem>,
    HashMap<u32, CheckMenuItem>,
    HashMap<u32, CheckMenuItem>,
    CheckMenuItem,
    CheckMenuItem,
);

fn build_settings_submenu(settings: &Settings) -> Result<SettingsSubmenuResult, MenuError> {
    let submenu = Submenu::new("⚙  Settings", true);

    // Pomodoro duration submenu
    let pomo_sub = Submenu::new(format!("Pomodoro: {} min", settings.pomodoro_mins), true);
    let mut pomo_checks = HashMap::new();
    for mins in [15, 20, 25, 30, 45, 60] {
        let checked = mins == settings.pomodoro_mins;
        let item = CheckMenuItem::with_id(
            MenuId::new(format!("pomo_{}", mins)),
            format!("{} min", mins),
            true,
            checked,
            None::<Accelerator>,
        );
        pomo_sub.append(&item)?;
        pomo_checks.insert(mins, item);
    }
    submenu.append(&pomo_sub)?;

    // Short break submenu
    let short_sub = Submenu::new(
        format!("Short Break: {} min", settings.short_break_mins),
        true,
    );
    let mut short_checks = HashMap::new();
    for mins in [3, 5, 10, 15] {
        let checked = mins == settings.short_break_mins;
        let item = CheckMenuItem::with_id(
            MenuId::new(format!("short_{}", mins)),
            format!("{} min", mins),
            true,
            checked,
            None::<Accelerator>,
        );
        short_sub.append(&item)?;
        short_checks.insert(mins, item);
    }
    submenu.append(&short_sub)?;

    // Long break submenu
    let long_sub = Submenu::new(format!("Long Break: {} min", settings.long_break_mins), true);
    let mut long_checks = HashMap::new();
    for mins in [10, 15, 20, 30] {
        let checked = mins == settings.long_break_mins;
        let item = CheckMenuItem::with_id(
            MenuId::new(format!("long_{}", mins)),
            format!("{} min", mins),
            true,
            checked,
            None::<Accelerator>,
        );
        long_sub.append(&item)?;
        long_checks.insert(mins, item);
    }
    submenu.append(&long_sub)?;

    // Long break threshold submenu
    let thresh_sub = Submenu::new(
        format!(
            "Long Break After: {} pomodoros",
            settings.pomodoros_for_long_break
        ),
        true,
    );
    let mut thresh_checks = HashMap::new();
    for count in [2, 3, 4, 5, 6] {
        let checked = count == settings.pomodoros_for_long_break;
        let item = CheckMenuItem::with_id(
            MenuId::new(format!("thresh_{}", count)),
            format!("{} pomodoros", count),
            true,
            checked,
            None::<Accelerator>,
        );
        thresh_sub.append(&item)?;
        thresh_checks.insert(count, item);
    }
    submenu.append(&thresh_sub)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    // Toggle checkboxes
    let sound_toggle = CheckMenuItem::with_id(
        MenuId::new(ID_SOUND_TOGGLE),
        "Sound Enabled",
        true,
        settings.sound_enabled,
        None::<Accelerator>,
    );
    submenu.append(&sound_toggle)?;

    let notif_toggle = CheckMenuItem::with_id(
        MenuId::new(ID_NOTIF_TOGGLE),
        "Notifications Enabled",
        true,
        settings.notifications_enabled,
        None::<Accelerator>,
    );
    submenu.append(&notif_toggle)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    let reset = MenuItem::with_id(
        MenuId::new(ID_RESET_COUNT),
        "Reset Today's Count",
        true,
        None::<Accelerator>,
    );
    submenu.append(&reset)?;

    Ok((
        submenu,
        pomo_checks,
        short_checks,
        long_checks,
        thresh_checks,
        sound_toggle,
        notif_toggle,
    ))
}

/// Updates the menu items based on the current state.
pub fn update_menu_items(items: &MenuItems, state: &TimerState, session: &Session) {
    // Update text items
    items.status.set_text(format_status(state));
    items.progress.set_text(format_progress(state));
    items.stats.set_text(format_stats(session));

    // Update enabled states
    items.start.set_enabled(state.is_idle());
    items
        .pause
        .set_enabled(matches!(state, TimerState::PomodoroActive { .. }));
    items.resume.set_enabled(state.is_paused());
    items.stop.set_enabled(state.is_pomodoro());
    items
        .complete
        .set_enabled(matches!(state, TimerState::PomodoroActive { .. }));
    items.skip_break.set_enabled(state.is_break());
}

/// Formats the status line for the menu.
pub fn format_status(state: &TimerState) -> String {
    match state {
        TimerState::Idle => "Ready to focus".to_string(),
        TimerState::PomodoroActive { remaining_secs, .. } => {
            format!("⏱  {} remaining", format_time(*remaining_secs))
        }
        TimerState::PomodoroPaused { remaining_secs, .. } => {
            format!("⏸  {} (paused)", format_time(*remaining_secs))
        }
        TimerState::BreakActive {
            is_long_break,
            remaining_secs,
            ..
        } => {
            let kind = if *is_long_break {
                "Long break"
            } else {
                "Short break"
            };
            format!("☕  {} - {}", kind, format_time(*remaining_secs))
        }
        TimerState::BreakFinished => "Break complete - ready for next".to_string(),
    }
}

/// Formats the progress bar for the menu.
pub fn format_progress(state: &TimerState) -> String {
    match state.progress_percent() {
        Some(pct) => {
            let filled = (pct * 20.0).round() as usize;
            let empty = 20 - filled;
            format!(
                "{}{}  {}%",
                "█".repeat(filled),
                "░".repeat(empty),
                (pct * 100.0).round() as u32
            )
        }
        None => "░░░░░░░░░░░░░░░░░░░░  0%".to_string(),
    }
}

/// Formats the daily stats for the menu.
pub fn format_stats(session: &Session) -> String {
    let tomatoes = "🍅".repeat(session.pomodoros_completed_today.min(10) as usize);
    let extra = if session.pomodoros_completed_today > 10 {
        format!("+{}", session.pomodoros_completed_today - 10)
    } else {
        String::new()
    };

    if session.pomodoros_completed_today == 0 {
        "Today: —  0 (0 min)".to_string()
    } else {
        format!(
            "Today: {}{}  {} ({} min)",
            tomatoes, extra, session.pomodoros_completed_today, session.total_focus_mins_today
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_format_status_idle() {
        let state = TimerState::Idle;
        assert_eq!(format_status(&state), "Ready to focus");
    }

    #[test]
    fn test_format_status_pomodoro_active() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 1432,
            total_secs: 1500,
        };
        assert_eq!(format_status(&state), "⏱  23:52 remaining");
    }

    #[test]
    fn test_format_status_paused() {
        let state = TimerState::PomodoroPaused {
            remaining_secs: 600,
            total_secs: 1500,
        };
        assert_eq!(format_status(&state), "⏸  10:00 (paused)");
    }

    #[test]
    fn test_format_status_short_break() {
        let state = TimerState::BreakActive {
            is_long_break: false,
            remaining_secs: 180,
            total_secs: 300,
        };
        assert_eq!(format_status(&state), "☕  Short break - 03:00");
    }

    #[test]
    fn test_format_status_long_break() {
        let state = TimerState::BreakActive {
            is_long_break: true,
            remaining_secs: 600,
            total_secs: 900,
        };
        assert_eq!(format_status(&state), "☕  Long break - 10:00");
    }

    #[test]
    fn test_format_status_break_finished() {
        let state = TimerState::BreakFinished;
        assert_eq!(format_status(&state), "Break complete - ready for next");
    }

    #[test]
    fn test_format_progress_idle() {
        let state = TimerState::Idle;
        assert_eq!(format_progress(&state), "░░░░░░░░░░░░░░░░░░░░  0%");
    }

    #[test]
    fn test_format_progress_half() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 750,
            total_secs: 1500,
        };
        assert_eq!(format_progress(&state), "██████████░░░░░░░░░░  50%");
    }

    #[test]
    fn test_format_progress_complete() {
        let state = TimerState::PomodoroActive {
            remaining_secs: 0,
            total_secs: 1500,
        };
        assert_eq!(format_progress(&state), "████████████████████  100%");
    }

    #[test]
    fn test_format_stats_empty() {
        let session = Session {
            pomodoros_completed_today: 0,
            total_focus_mins_today: 0,
            pomodoros_in_cycle: 0,
            last_date: Local::now().date_naive(),
        };
        assert_eq!(format_stats(&session), "Today: —  0 (0 min)");
    }

    #[test]
    fn test_format_stats_with_pomodoros() {
        let session = Session {
            pomodoros_completed_today: 4,
            total_focus_mins_today: 100,
            pomodoros_in_cycle: 0,
            last_date: Local::now().date_naive(),
        };
        assert_eq!(format_stats(&session), "Today: 🍅🍅🍅🍅  4 (100 min)");
    }

    #[test]
    fn test_format_stats_many_pomodoros() {
        let session = Session {
            pomodoros_completed_today: 15,
            total_focus_mins_today: 375,
            pomodoros_in_cycle: 0,
            last_date: Local::now().date_naive(),
        };
        let result = format_stats(&session);
        assert!(result.contains("+5"));
        assert!(result.contains("15"));
        assert!(result.contains("375 min"));
    }
}

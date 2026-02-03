//! Menu event handling.

use crate::app::{App, CompletionEvent};
use crate::launch_agent;
use crate::menu::{
    MenuItems, ID_COMPLETE, ID_LOGIN_TOGGLE, ID_NOTIF_TOGGLE, ID_PAUSE, ID_QUIT, ID_RESET_COUNT,
    ID_RESUME, ID_SKIP_BREAK, ID_SOUND_TOGGLE, ID_START, ID_STOP,
};
use muda::MenuEvent;

/// Result of handling a menu event.
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    /// Event handled, continue running.
    Continue,
    /// User requested quit.
    Quit,
    /// State changed, menu needs update.
    StateChanged,
    /// Settings changed, menu needs rebuild.
    SettingsChanged,
    /// State changed with a completion event.
    StateChangedWithCompletion(CompletionEvent),
}

/// Handles a menu event and updates the app state accordingly.
pub fn handle_menu_event(app: &mut App, items: &MenuItems, event: MenuEvent) -> EventResult {
    let id = event.id().as_ref();

    match id {
        ID_START => {
            app.start_pomodoro();
            EventResult::StateChanged
        }
        ID_PAUSE => {
            app.pause();
            EventResult::StateChanged
        }
        ID_RESUME => {
            app.resume();
            EventResult::StateChanged
        }
        ID_STOP => {
            app.stop();
            EventResult::StateChanged
        }
        ID_COMPLETE => {
            if let Some(event) = app.complete_early() {
                EventResult::StateChangedWithCompletion(event)
            } else {
                EventResult::Continue
            }
        }
        ID_SKIP_BREAK => {
            app.skip_break();
            EventResult::StateChanged
        }
        ID_SOUND_TOGGLE => {
            app.update_setting(|s| s.sound_enabled = !s.sound_enabled);
            items.sound_toggle.set_checked(app.settings.sound_enabled);
            EventResult::Continue
        }
        ID_NOTIF_TOGGLE => {
            app.update_setting(|s| s.notifications_enabled = !s.notifications_enabled);
            items
                .notif_toggle
                .set_checked(app.settings.notifications_enabled);
            EventResult::Continue
        }
        ID_LOGIN_TOGGLE => {
            let new_state = !app.settings.launch_at_login;
            if launch_agent::set_enabled(new_state).is_ok() {
                app.update_setting(|s| s.launch_at_login = new_state);
                items.login_toggle.set_checked(new_state);
            } else {
                // Revert the checkbox if the operation failed
                items.login_toggle.set_checked(app.settings.launch_at_login);
            }
            EventResult::Continue
        }
        ID_RESET_COUNT => {
            app.reset_today();
            EventResult::StateChanged
        }
        ID_QUIT => EventResult::Quit,
        _ => {
            // Check for settings duration changes
            if let Some(result) = handle_duration_change(app, items, id) {
                return result;
            }
            EventResult::Continue
        }
    }
}

/// Handles duration setting changes from submenus.
fn handle_duration_change(app: &mut App, items: &MenuItems, id: &str) -> Option<EventResult> {
    // Pomodoro duration
    if let Some(mins_str) = id.strip_prefix("pomo_") {
        if let Ok(mins) = mins_str.parse::<u32>() {
            // Update checkmarks
            for (&m, check) in &items.pomo_checks {
                check.set_checked(m == mins);
            }
            app.update_setting(|s| s.pomodoro_mins = mins);
            return Some(EventResult::SettingsChanged);
        }
    }

    // Short break duration
    if let Some(mins_str) = id.strip_prefix("short_") {
        if let Ok(mins) = mins_str.parse::<u32>() {
            for (&m, check) in &items.short_checks {
                check.set_checked(m == mins);
            }
            app.update_setting(|s| s.short_break_mins = mins);
            return Some(EventResult::SettingsChanged);
        }
    }

    // Long break duration
    if let Some(mins_str) = id.strip_prefix("long_") {
        if let Ok(mins) = mins_str.parse::<u32>() {
            for (&m, check) in &items.long_checks {
                check.set_checked(m == mins);
            }
            app.update_setting(|s| s.long_break_mins = mins);
            return Some(EventResult::SettingsChanged);
        }
    }

    // Long break threshold
    if let Some(count_str) = id.strip_prefix("thresh_") {
        if let Ok(count) = count_str.parse::<u32>() {
            for (&c, check) in &items.thresh_checks {
                check.set_checked(c == count);
            }
            app.update_setting(|s| s.pomodoros_for_long_break = count);
            return Some(EventResult::SettingsChanged);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    // Event handling tests would require mocking the menu items
    // which is complex. The logic is tested through integration tests
    // and the app module tests.
}

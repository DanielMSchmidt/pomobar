//! macOS system notifications for timer events.

use notify_rust::Notification;
use std::thread;

/// Shows a notification when a pomodoro is completed.
/// Runs in a background thread to avoid blocking.
pub fn notify_pomodoro_complete(count: u32) {
    thread::spawn(move || {
        let body = if count == 1 {
            "Great work! You've completed 1 pomodoro today.\nTime for a break.".to_string()
        } else {
            format!(
                "Great work! You've completed {} pomodoros today.\nTime for a break.",
                count
            )
        };

        if let Err(e) = Notification::new()
            .summary("Pomodoro Complete! 🍅")
            .body(&body)
            .sound_name("default")
            .show()
        {
            eprintln!("Failed to show notification: {}", e);
        }
    });
}

/// Shows a notification when a break is completed.
/// Runs in a background thread to avoid blocking.
pub fn notify_break_complete() {
    thread::spawn(|| {
        if let Err(e) = Notification::new()
            .summary("Break Over! ☕")
            .body("Ready to start another pomodoro?")
            .sound_name("default")
            .show()
        {
            eprintln!("Failed to show notification: {}", e);
        }
    });
}

/// Shows a notification when a long break starts.
/// Runs in a background thread to avoid blocking.
pub fn notify_long_break_start(duration_mins: u32) {
    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .summary("Long Break Time! 🎉")
            .body(&format!(
                "You've earned a {} minute break. Great job staying focused!",
                duration_mins
            ))
            .sound_name("default")
            .show()
        {
            eprintln!("Failed to show notification: {}", e);
        }
    });
}

#[cfg(test)]
mod tests {
    // Note: Notification tests are tricky because they interact with the system
    // and may hang waiting for user interaction. They are ignored by default.
    // Run with `cargo test -- --ignored` to execute them.

    use super::*;

    #[test]
    #[ignore = "Requires system notification interaction"]
    fn test_pomodoro_notification_singular() {
        notify_pomodoro_complete(1);
    }

    #[test]
    #[ignore = "Requires system notification interaction"]
    fn test_pomodoro_notification_plural() {
        notify_pomodoro_complete(5);
    }

    #[test]
    #[ignore = "Requires system notification interaction"]
    fn test_break_notification() {
        notify_break_complete();
    }

    #[test]
    #[ignore = "Requires system notification interaction"]
    fn test_long_break_notification() {
        notify_long_break_start(15);
    }
}

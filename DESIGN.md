# Pomobar - macOS Menubar Pomodoro Timer

## Overview

A native macOS menubar application for managing pomodoro work sessions with text-based progress display, configurable timings, daily statistics, and audio/visual notifications.

## Technology Choice

### **Pure Rust** with `tray-icon` + `muda` crates

**Rationale:**
- Single language (Rust only) - easier to maintain
- Native macOS menu integration via `muda`
- Lightweight binary (~5MB)
- No webview overhead
- Direct access to system APIs

**Key crates:**
- `tray-icon` - System tray icon management
- `muda` - Native menu creation
- `tokio` - Async runtime for timer
- `rodio` - Audio playback
- `rusqlite` - Persistence
- `notify-rust` - macOS notifications
- `directories` - App data paths

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        macOS Menubar                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  🍅 (idle) | 🍅 15:32 (working) | ☕ 04:32 (break)       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │ click
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Native Menu (muda)                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  ⏱  23:45 remaining                                      │   │
│  │  ████████████░░░░░░░░  62%                                │   │
│  │  ───────────────────────────────────                      │   │
│  │  Today: 🍅🍅🍅🍅  4 pomodoros (100 min)                   │   │
│  │  ───────────────────────────────────                      │   │
│  │  ▶  Start Pomodoro                                        │   │
│  │  ⏸  Pause                                   (disabled)    │   │
│  │  ⏹  Stop                                    (disabled)    │   │
│  │  ✓  Complete Early                          (disabled)    │   │
│  │  ───────────────────────────────────                      │   │
│  │  ⚙  Settings ►  [submenu with durations]                  │   │
│  │  ───────────────────────────────────                      │   │
│  │  Quit Pomobar                                             │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Rust Backend                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Timer     │  │   State     │  │   Persistence           │  │
│  │   Loop      │◄─┤   Machine   │◄─┤   (SQLite)              │  │
│  │  (tokio)    │  │             │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│         │                │                     │                 │
│         ▼                ▼                     ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Menu        │  │ Notifier    │  │ Audio Player            │  │
│  │ Manager     │  │(notify-rust)│  │ (rodio)                 │  │
│  │ (muda)      │  │             │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Menu Display States

### Tray Icon Title (shown in menubar)

| State | Icon + Title |
|-------|--------------|
| Idle | `🍅` (just tomato) |
| Pomodoro Active | `🍅 23:45` (tomato + remaining time) |
| Pomodoro Paused | `⏸ 23:45` (pause icon + remaining time) |
| Break Active | `☕ 04:32` (coffee + remaining time) |
| Break Finished | `🍅` (just tomato) |

### Menu Content (updated every second when active)

```
┌────────────────────────────────────────┐
│  ⏱  23:45 remaining                    │  <- MenuItem (disabled, info only)
│  ████████████░░░░░░░░  62%             │  <- MenuItem (disabled, ASCII bar)
│  ──────────────────────                │
│  Today: 🍅🍅🍅🍅  4 (100 min)          │  <- MenuItem (disabled, info only)
│  ──────────────────────                │
│  ▶  Start Pomodoro                     │  <- Enabled when idle
│  ⏸  Pause                              │  <- Enabled when active
│  ▶  Resume                             │  <- Enabled when paused
│  ⏹  Stop                               │  <- Enabled when active/paused
│  ✓  Complete Early                     │  <- Enabled when pomodoro active
│  ⏭  Skip Break                         │  <- Enabled when on break
│  ──────────────────────                │
│  ⚙  Settings                        ►  │  <- Submenu
│  ──────────────────────                │
│  Quit Pomobar                          │  <- Always enabled
└────────────────────────────────────────┘
```

### Settings Submenu

```
┌────────────────────────────────────────┐
│  Pomodoro: 25 min                   ►  │  -> [15, 20, 25✓, 30, 45, 60]
│  Short Break: 5 min                 ►  │  -> [3, 5✓, 10, 15]
│  Long Break: 15 min                 ►  │  -> [10, 15✓, 20, 30]
│  Long Break After: 4 pomodoros      ►  │  -> [2, 3, 4✓, 5, 6]
│  ──────────────────────                │
│  ✓  Sound Enabled                      │  <- Checkbox
│  ✓  Notifications Enabled              │  <- Checkbox
│  ──────────────────────                │
│  Reset Today's Count                   │  <- Action
└────────────────────────────────────────┘
```

---

## State Machine

```
                    ┌──────────────────┐
                    │                  │
         ┌─────────►│      IDLE        │◄─────────┐
         │          │      (🍅)        │          │
         │          └────────┬─────────┘          │
         │                   │                    │
         │          start_pomodoro()              │
         │                   │                    │
         │                   ▼                    │
         │          ┌──────────────────┐          │
         │          │                  │          │
    stop()          │  POMODORO_ACTIVE │      stop()
         │          │   (🍅 MM:SS)     │          │
         │          └────────┬─────────┘          │
         │                   │                    │
         │          ┌────────┼────────┐           │
         │          │        │        │           │
         │       pause()  timer=0  complete()     │
         │          │        │        │           │
         │          ▼        │        │           │
         │   ┌────────────┐  │        │           │
         │   │  PAUSED    │  │        │           │
         │   │ (⏸ MM:SS)  │──┘        │           │
         │   └────────────┘           │           │
         │          │                 │           │
         │       resume()             │           │
         │          │                 │           │
         │          └────────►────────┘           │
         │                   │                    │
         │                   ▼                    │
         │          ┌──────────────────┐          │
         │          │   BREAK_ACTIVE   │          │
         │          │   (☕ MM:SS)     │          │
         │          └────────┬─────────┘          │
         │                   │                    │
         │          timer=0 / skip()              │
         │                   │                    │
         │                   ▼                    │
         │          ┌──────────────────┐          │
         └──────────│  BREAK_FINISHED  │──────────┘
                    │      (🍅)        │
                    └──────────────────┘
```

---

## Data Models

```rust
// src/models.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local, NaiveDate};

/// Timer state machine
#[derive(Debug, Clone, PartialEq)]
pub enum TimerState {
    Idle,
    PomodoroActive {
        remaining_secs: u32,
        total_secs: u32,
    },
    PomodoroPaused {
        remaining_secs: u32,
        total_secs: u32,
    },
    BreakActive {
        is_long_break: bool,
        remaining_secs: u32,
        total_secs: u32,
    },
    BreakFinished,
}

impl TimerState {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle | Self::BreakFinished)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::PomodoroActive { .. } | Self::BreakActive { .. })
    }

    pub fn progress_percent(&self) -> Option<f32> {
        match self {
            Self::PomodoroActive { remaining_secs, total_secs } |
            Self::PomodoroPaused { remaining_secs, total_secs } |
            Self::BreakActive { remaining_secs, total_secs, .. } => {
                Some(1.0 - (*remaining_secs as f32 / *total_secs as f32))
            }
            _ => None,
        }
    }

    pub fn remaining_secs(&self) -> Option<u32> {
        match self {
            Self::PomodoroActive { remaining_secs, .. } |
            Self::PomodoroPaused { remaining_secs, .. } |
            Self::BreakActive { remaining_secs, .. } => Some(*remaining_secs),
            _ => None,
        }
    }
}

/// User-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub pomodoro_mins: u32,           // default: 25
    pub short_break_mins: u32,        // default: 5
    pub long_break_mins: u32,         // default: 15
    pub pomodoros_for_long_break: u32, // default: 4
    pub sound_enabled: bool,          // default: true
    pub notifications_enabled: bool,  // default: true
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

/// Session tracking
#[derive(Debug, Clone, Default)]
pub struct Session {
    pub pomodoros_completed_today: u32,
    pub total_focus_mins_today: u32,
    pub pomodoros_in_cycle: u32,  // resets after long break
    pub last_date: NaiveDate,
}

impl Session {
    pub fn check_day_rollover(&mut self) {
        let today = Local::now().date_naive();
        if self.last_date != today {
            self.pomodoros_completed_today = 0;
            self.total_focus_mins_today = 0;
            self.last_date = today;
        }
    }

    pub fn complete_pomodoro(&mut self, duration_mins: u32) {
        self.check_day_rollover();
        self.pomodoros_completed_today += 1;
        self.total_focus_mins_today += duration_mins;
        self.pomodoros_in_cycle += 1;
    }

    pub fn is_long_break_due(&self, threshold: u32) -> bool {
        self.pomodoros_in_cycle >= threshold
    }

    pub fn reset_cycle(&mut self) {
        self.pomodoros_in_cycle = 0;
    }
}

/// Daily statistics for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: NaiveDate,
    pub completed_pomodoros: u32,
    pub total_focus_minutes: u32,
}
```

---

## File Structure

```
pomobar/
├── Cargo.toml
├── Cargo.lock
├── build.rs                    # Embed resources, set app metadata
├── resources/
│   ├── tomato.png              # Tray icon (22x22 @1x, 44x44 @2x)
│   ├── tomato@2x.png
│   └── chime.mp3               # Completion sound
├── src/
│   ├── main.rs                 # Entry point, event loop setup
│   ├── app.rs                  # Main application struct
│   ├── models.rs               # Data structures (above)
│   ├── timer.rs                # Timer state machine logic
│   ├── menu.rs                 # Menu building and updates
│   ├── tray.rs                 # Tray icon management
│   ├── audio.rs                # Sound playback
│   ├── notifications.rs        # macOS notifications
│   ├── persistence.rs          # SQLite operations
│   └── event.rs                # Menu event handlers
└── DESIGN.md
```

---

## Core Implementation

### Main Entry Point

```rust
// src/main.rs

use std::sync::{Arc, Mutex};
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use muda::{Menu, MenuEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod models;
mod timer;
mod menu;
mod tray;
mod audio;
mod notifications;
mod persistence;
mod event;

use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize app state
    let app = Arc::new(Mutex::new(App::new()?));

    // Create event loop (required for tray on macOS)
    let event_loop = EventLoop::new()?;

    // Build menu
    let menu = menu::build_menu(&app.lock().unwrap())?;

    // Create tray icon
    let icon = tray::load_icon()?;
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_icon(icon)
        .with_title("🍅")
        .with_tooltip("Pomobar")
        .build()?;

    // Spawn timer tick task
    let app_clone = Arc::clone(&app);
    std::thread::spawn(move || {
        timer::run_timer_loop(app_clone);
    });

    // Menu event channel
    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    // Run event loop
    event_loop.run(move |_event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);

        // Handle menu events
        if let Ok(event) = menu_channel.try_recv() {
            let mut app = app.lock().unwrap();
            event::handle_menu_event(&mut app, &menu, event);
        }

        // Handle tray events (icon click)
        if let Ok(event) = tray_channel.try_recv() {
            // Tray click handling if needed
        }
    })?;

    Ok(())
}
```

### Application State

```rust
// src/app.rs

use crate::models::{Settings, Session, TimerState};
use crate::persistence::Database;
use crate::audio::AudioPlayer;

pub struct App {
    pub state: TimerState,
    pub settings: Settings,
    pub session: Session,
    pub db: Database,
    pub audio: AudioPlayer,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db = Database::new()?;
        let settings = db.load_settings()?;
        let session = db.load_today_session()?;

        Ok(Self {
            state: TimerState::Idle,
            settings,
            session,
            db,
            audio: AudioPlayer::new()?,
        })
    }

    pub fn start_pomodoro(&mut self) {
        let total_secs = self.settings.pomodoro_mins * 60;
        self.state = TimerState::PomodoroActive {
            remaining_secs: total_secs,
            total_secs,
        };
    }

    pub fn pause(&mut self) {
        if let TimerState::PomodoroActive { remaining_secs, total_secs } = self.state {
            self.state = TimerState::PomodoroPaused { remaining_secs, total_secs };
        }
    }

    pub fn resume(&mut self) {
        if let TimerState::PomodoroPaused { remaining_secs, total_secs } = self.state {
            self.state = TimerState::PomodoroActive { remaining_secs, total_secs };
        }
    }

    pub fn stop(&mut self) {
        self.state = TimerState::Idle;
    }

    pub fn complete_early(&mut self) {
        if matches!(self.state, TimerState::PomodoroActive { .. }) {
            self.finish_pomodoro();
        }
    }

    pub fn skip_break(&mut self) {
        if matches!(self.state, TimerState::BreakActive { .. }) {
            self.state = TimerState::BreakFinished;
        }
    }

    pub fn tick(&mut self) -> bool {
        match &mut self.state {
            TimerState::PomodoroActive { remaining_secs, .. } => {
                if *remaining_secs > 0 {
                    *remaining_secs -= 1;
                    true
                } else {
                    self.finish_pomodoro();
                    true
                }
            }
            TimerState::BreakActive { remaining_secs, .. } => {
                if *remaining_secs > 0 {
                    *remaining_secs -= 1;
                    true
                } else {
                    self.finish_break();
                    true
                }
            }
            _ => false,
        }
    }

    fn finish_pomodoro(&mut self) {
        // Update session
        self.session.complete_pomodoro(self.settings.pomodoro_mins);
        self.db.save_session(&self.session).ok();

        // Play sound & notify
        if self.settings.sound_enabled {
            self.audio.play_chime();
        }
        if self.settings.notifications_enabled {
            crate::notifications::notify_pomodoro_complete(
                self.session.pomodoros_completed_today
            );
        }

        // Determine break type
        let is_long = self.session.is_long_break_due(self.settings.pomodoros_for_long_break);
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
    }

    fn finish_break(&mut self) {
        if self.settings.sound_enabled {
            self.audio.play_chime();
        }
        if self.settings.notifications_enabled {
            crate::notifications::notify_break_complete();
        }
        self.state = TimerState::BreakFinished;
    }
}
```

### Menu Building

```rust
// src/menu.rs

use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem};
use crate::app::App;
use crate::models::TimerState;

// Menu item IDs
pub const ID_START: &str = "start";
pub const ID_PAUSE: &str = "pause";
pub const ID_RESUME: &str = "resume";
pub const ID_STOP: &str = "stop";
pub const ID_COMPLETE: &str = "complete";
pub const ID_SKIP_BREAK: &str = "skip_break";
pub const ID_QUIT: &str = "quit";
pub const ID_SOUND_TOGGLE: &str = "sound_toggle";
pub const ID_NOTIF_TOGGLE: &str = "notif_toggle";
pub const ID_RESET_COUNT: &str = "reset_count";

pub fn build_menu(app: &App) -> Result<Menu, muda::Error> {
    let menu = Menu::new();

    // Status display (disabled, info only)
    let status_item = MenuItem::with_id(
        "status",
        format_status(&app.state),
        false, // disabled
        None::<&str>,
    );
    menu.append(&status_item)?;

    // Progress bar (ASCII)
    let progress_item = MenuItem::with_id(
        "progress",
        format_progress(&app.state),
        false,
        None::<&str>,
    );
    menu.append(&progress_item)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Today's stats
    let stats_item = MenuItem::with_id(
        "stats",
        format_stats(&app.session),
        false,
        None::<&str>,
    );
    menu.append(&stats_item)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Control buttons
    let start = MenuItem::with_id(ID_START, "▶  Start Pomodoro", true, None::<&str>);
    let pause = MenuItem::with_id(ID_PAUSE, "⏸  Pause", false, None::<&str>);
    let resume = MenuItem::with_id(ID_RESUME, "▶  Resume", false, None::<&str>);
    let stop = MenuItem::with_id(ID_STOP, "⏹  Stop", false, None::<&str>);
    let complete = MenuItem::with_id(ID_COMPLETE, "✓  Complete Early", false, None::<&str>);
    let skip = MenuItem::with_id(ID_SKIP_BREAK, "⏭  Skip Break", false, None::<&str>);

    menu.append(&start)?;
    menu.append(&pause)?;
    menu.append(&resume)?;
    menu.append(&stop)?;
    menu.append(&complete)?;
    menu.append(&skip)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Settings submenu
    let settings_menu = build_settings_submenu(app)?;
    menu.append(&settings_menu)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Quit
    let quit = MenuItem::with_id(ID_QUIT, "Quit Pomobar", true, None::<&str>);
    menu.append(&quit)?;

    // Update enabled states
    update_menu_state(&menu, &app.state);

    Ok(menu)
}

fn build_settings_submenu(app: &App) -> Result<Submenu, muda::Error> {
    let submenu = Submenu::new("⚙  Settings", true);

    // Pomodoro duration submenu
    let pomo_sub = Submenu::new(
        format!("Pomodoro: {} min", app.settings.pomodoro_mins),
        true,
    );
    for mins in [15, 20, 25, 30, 45, 60] {
        let checked = mins == app.settings.pomodoro_mins;
        let item = CheckMenuItem::with_id(
            format!("pomo_{}", mins),
            format!("{} min", mins),
            true,
            checked,
            None::<&str>,
        );
        pomo_sub.append(&item)?;
    }
    submenu.append(&pomo_sub)?;

    // Short break submenu
    let short_sub = Submenu::new(
        format!("Short Break: {} min", app.settings.short_break_mins),
        true,
    );
    for mins in [3, 5, 10, 15] {
        let checked = mins == app.settings.short_break_mins;
        let item = CheckMenuItem::with_id(
            format!("short_{}", mins),
            format!("{} min", mins),
            true,
            checked,
            None::<&str>,
        );
        short_sub.append(&item)?;
    }
    submenu.append(&short_sub)?;

    // Long break submenu
    let long_sub = Submenu::new(
        format!("Long Break: {} min", app.settings.long_break_mins),
        true,
    );
    for mins in [10, 15, 20, 30] {
        let checked = mins == app.settings.long_break_mins;
        let item = CheckMenuItem::with_id(
            format!("long_{}", mins),
            format!("{} min", mins),
            true,
            checked,
            None::<&str>,
        );
        long_sub.append(&item)?;
    }
    submenu.append(&long_sub)?;

    // Long break threshold submenu
    let thresh_sub = Submenu::new(
        format!("Long Break After: {} pomodoros", app.settings.pomodoros_for_long_break),
        true,
    );
    for count in [2, 3, 4, 5, 6] {
        let checked = count == app.settings.pomodoros_for_long_break;
        let item = CheckMenuItem::with_id(
            format!("thresh_{}", count),
            format!("{} pomodoros", count),
            true,
            checked,
            None::<&str>,
        );
        thresh_sub.append(&item)?;
    }
    submenu.append(&thresh_sub)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    // Toggle checkboxes
    let sound_item = CheckMenuItem::with_id(
        ID_SOUND_TOGGLE,
        "Sound Enabled",
        true,
        app.settings.sound_enabled,
        None::<&str>,
    );
    submenu.append(&sound_item)?;

    let notif_item = CheckMenuItem::with_id(
        ID_NOTIF_TOGGLE,
        "Notifications Enabled",
        true,
        app.settings.notifications_enabled,
        None::<&str>,
    );
    submenu.append(&notif_item)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    let reset = MenuItem::with_id(ID_RESET_COUNT, "Reset Today's Count", true, None::<&str>);
    submenu.append(&reset)?;

    Ok(submenu)
}

pub fn update_menu_state(menu: &Menu, state: &TimerState) {
    // Enable/disable items based on current state
    // This requires getting menu items by ID and calling set_enabled()
    // Implementation depends on muda's API for accessing items
}

fn format_status(state: &TimerState) -> String {
    match state {
        TimerState::Idle => "Ready to focus".to_string(),
        TimerState::PomodoroActive { remaining_secs, .. } => {
            format!("⏱  {} remaining", format_time(*remaining_secs))
        }
        TimerState::PomodoroPaused { remaining_secs, .. } => {
            format!("⏸  {} (paused)", format_time(*remaining_secs))
        }
        TimerState::BreakActive { is_long_break, remaining_secs, .. } => {
            let kind = if *is_long_break { "Long break" } else { "Short break" };
            format!("☕  {} - {}", kind, format_time(*remaining_secs))
        }
        TimerState::BreakFinished => "Break complete - ready for next".to_string(),
    }
}

fn format_progress(state: &TimerState) -> String {
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

fn format_stats(session: &crate::models::Session) -> String {
    let tomatoes = "🍅".repeat(session.pomodoros_completed_today as usize);
    format!(
        "Today: {}  {} ({} min)",
        if tomatoes.is_empty() { "—" } else { &tomatoes },
        session.pomodoros_completed_today,
        session.total_focus_mins_today
    )
}

fn format_time(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
```

### Timer Loop

```rust
// src/timer.rs

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use crate::app::App;
use crate::tray;

pub fn run_timer_loop(app: Arc<Mutex<App>>) {
    loop {
        thread::sleep(Duration::from_secs(1));

        let mut app = app.lock().unwrap();

        // Check for day rollover
        app.session.check_day_rollover();

        // Tick the timer
        if app.tick() {
            // Timer state changed, update UI
            // Note: Need to signal main thread to rebuild menu
            // This can be done via a channel or by rebuilding menu items in place
        }

        // Update tray title
        tray::update_tray_title(&app.state);
    }
}
```

### Tray Icon Management

```rust
// src/tray.rs

use tray_icon::Icon;
use crate::models::TimerState;

pub fn load_icon() -> Result<Icon, tray_icon::Error> {
    // Load icon from embedded bytes or file
    let icon_bytes = include_bytes!("../resources/tomato.png");
    Icon::from_rgba(/* decoded RGBA */)
}

pub fn update_tray_title(state: &TimerState) {
    let title = match state {
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
    };

    // Update tray title (requires tray handle)
    // tray.set_title(Some(&title));
}
```

### Audio Playback

```rust
// src/audio.rs

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;

pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, rodio::StreamError> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
        })
    }

    pub fn play_chime(&self) {
        let sound_data = include_bytes!("../resources/chime.mp3");
        let cursor = Cursor::new(sound_data);

        if let Ok(source) = Decoder::new(cursor) {
            if let Ok(sink) = Sink::try_new(&self.handle) {
                sink.append(source);
                sink.detach(); // Play in background
            }
        }
    }
}
```

### Notifications

```rust
// src/notifications.rs

use notify_rust::Notification;

pub fn notify_pomodoro_complete(count: u32) {
    Notification::new()
        .summary("Pomodoro Complete! 🍅")
        .body(&format!(
            "Great work! You've completed {} pomodoro(s) today.\nTime for a break.",
            count
        ))
        .sound_name("default")
        .show()
        .ok();
}

pub fn notify_break_complete() {
    Notification::new()
        .summary("Break Over!")
        .body("Ready to start another pomodoro?")
        .sound_name("default")
        .show()
        .ok();
}
```

### Persistence

```rust
// src/persistence.rs

use rusqlite::{Connection, params};
use directories::ProjectDirs;
use std::path::PathBuf;
use crate::models::{Settings, Session, DailyStats};
use chrono::{Local, NaiveDate};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_path = Self::db_path();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(db_path)?;

        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                completed_pomodoros INTEGER NOT NULL DEFAULT 0,
                total_focus_minutes INTEGER NOT NULL DEFAULT 0
            );
        "#)?;

        Ok(Self { conn })
    }

    fn db_path() -> PathBuf {
        ProjectDirs::from("com", "pomobar", "Pomobar")
            .map(|dirs| dirs.data_dir().join("pomobar.db"))
            .unwrap_or_else(|| PathBuf::from("pomobar.db"))
    }

    pub fn load_settings(&self) -> Result<Settings, rusqlite::Error> {
        let json: Option<String> = self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'config'",
                [],
                |row| row.get(0),
            )
            .ok();

        Ok(json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), rusqlite::Error> {
        let json = serde_json::to_string(settings).unwrap();
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('config', ?)",
            [&json],
        )?;
        Ok(())
    }

    pub fn load_today_session(&self) -> Result<Session, rusqlite::Error> {
        let today = Local::now().date_naive();
        let stats = self.get_daily_stats(today)?;

        Ok(Session {
            pomodoros_completed_today: stats.completed_pomodoros,
            total_focus_mins_today: stats.total_focus_minutes,
            pomodoros_in_cycle: 0, // Reset cycle on app restart
            last_date: today,
        })
    }

    pub fn save_session(&self, session: &Session) -> Result<(), rusqlite::Error> {
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

    fn get_daily_stats(&self, date: NaiveDate) -> Result<DailyStats, rusqlite::Error> {
        self.conn
            .query_row(
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
            )
            .or_else(|_| {
                Ok(DailyStats {
                    date,
                    completed_pomodoros: 0,
                    total_focus_minutes: 0,
                })
            })
    }
}
```

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "pomobar"
version = "0.1.0"
edition = "2021"

[dependencies]
# System tray
tray-icon = "0.14"
muda = "0.13"

# Event loop (required for tray on macOS)
winit = "0.29"

# Async runtime (optional, for timer - can use std::thread instead)
# tokio = { version = "1", features = ["rt", "time", "sync"] }

# Audio
rodio = "0.18"

# Notifications
notify-rust = "4"

# Persistence
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# App directories
directories = "5"

[build-dependencies]
# For embedding resources on macOS
embed-resource = "2"
```

---

## Standard Pomodoro Timings (Configurable)

| Setting | Default | Options |
|---------|---------|---------|
| Pomodoro | 25 min | 15, 20, 25, 30, 45, 60 |
| Short Break | 5 min | 3, 5, 10, 15 |
| Long Break | 15 min | 10, 15, 20, 30 |
| Pomodoros until Long Break | 4 | 2, 3, 4, 5, 6 |

---

## Implementation Order

### Phase 1: Skeleton
- [ ] Set up Cargo project with dependencies
- [ ] Basic tray icon with static menu (using `tray-icon` + `muda`)
- [ ] Event loop setup with `winit`

### Phase 2: Core Timer
- [ ] Implement `TimerState` enum and transitions
- [ ] Timer tick loop (1 second interval)
- [ ] Tray title updates (🍅, 🍅 23:45, ☕ 04:32)

### Phase 3: Menu Interaction
- [ ] Control buttons (Start, Pause, Resume, Stop, Complete, Skip)
- [ ] Dynamic enable/disable based on state
- [ ] Menu content updates (status line, progress bar, stats)

### Phase 4: Persistence
- [ ] SQLite database setup
- [ ] Settings save/load
- [ ] Daily stats tracking with day rollover

### Phase 5: Notifications & Audio
- [ ] Audio chime playback with `rodio`
- [ ] macOS notifications with `notify-rust`

### Phase 6: Settings
- [ ] Settings submenu with duration options
- [ ] Sound/notification toggles
- [ ] Reset today's count

### Phase 7: Polish
- [ ] Proper app icon (icns for macOS)
- [ ] Bundle as .app
- [ ] Test all edge cases

---

## Notes for Implementation Agent

1. **Menu rebuilding**: The `muda` crate may require rebuilding the entire menu to update text. Check if individual items can be updated in place.

2. **Thread safety**: The `App` state is wrapped in `Arc<Mutex<>>`. Be careful about lock contention between the timer thread and the main event loop.

3. **Tray title on macOS**: Use `tray.set_title()` to show text next to the icon. This is a macOS-specific feature.

4. **Icon format**: macOS tray icons should be 22x22 pixels at 1x scale (44x44 for @2x retina). Use template images (grayscale) for proper dark/light mode support.

5. **App bundle**: To run as a proper macOS app (required for notifications), create an `.app` bundle. Use `cargo-bundle` or manually create the structure.

6. **Sound file**: Include a short chime sound (MP3 or WAV). Keep it under 100KB. Many free options available online.

7. **Day rollover**: Call `session.check_day_rollover()` on every tick to handle midnight crossing correctly.

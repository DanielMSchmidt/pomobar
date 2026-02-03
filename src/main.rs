//! Pomobar - A native macOS menubar Pomodoro timer.
//!
//! This application provides a simple, distraction-free pomodoro timer
//! that lives in your menubar.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use muda::MenuEvent;
use tray_icon::{TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

mod app;
mod audio;
mod event;
mod menu;
mod models;
mod notifications;
mod persistence;
mod timer;

use app::{App, CompletionEvent};
use audio::AudioPlayer;
use event::EventResult;
use menu::MenuItems;
use timer::TimerMessage;

/// Application handler for the winit event loop.
struct Pomobar {
    app: Arc<Mutex<App>>,
    tray: Option<TrayIcon>,
    menu_items: Option<MenuItems>,
    timer_rx: Receiver<TimerMessage>,
    audio: Option<AudioPlayer>,
}

impl Pomobar {
    fn new(app: Arc<Mutex<App>>, tray: TrayIcon, timer_rx: Receiver<TimerMessage>) -> Self {
        // Audio is created on the main thread to avoid Send issues
        let audio = AudioPlayer::new().ok();

        Self {
            app,
            tray: Some(tray),
            menu_items: None,
            timer_rx,
            audio,
        }
    }

    fn set_menu_items(&mut self, items: MenuItems) {
        self.menu_items = Some(items);
    }

    fn update_menu(&self) {
        if let Some(ref items) = self.menu_items {
            let app = self.app.lock().unwrap();
            menu::update_menu_items(items, &app.state, &app.session);
        }
    }

    fn update_tray_title(&self, title: &str) {
        if let Some(ref tray) = self.tray {
            tray.set_title(Some(title));
        }
    }

    fn handle_completion(&self, event: CompletionEvent) {
        let app = self.app.lock().unwrap();

        // Play sound if enabled
        if app.settings.sound_enabled {
            if let Some(ref audio) = self.audio {
                audio.play_chime();
            }
        }

        // Show notification if enabled
        if app.settings.notifications_enabled {
            match event {
                CompletionEvent::PomodoroComplete {
                    count,
                    is_long_break,
                } => {
                    if is_long_break {
                        notifications::notify_long_break_start(app.long_break_mins());
                    } else {
                        notifications::notify_pomodoro_complete(count);
                    }
                }
                CompletionEvent::BreakComplete => {
                    notifications::notify_break_complete();
                }
            }
        }
    }

    fn process_timer_messages(&mut self) {
        // Process all pending timer messages
        while let Ok(msg) = self.timer_rx.try_recv() {
            match msg {
                TimerMessage::StateChanged { title, state: _ } => {
                    self.update_tray_title(&title);
                    self.update_menu();
                }
                TimerMessage::Completed(event) => {
                    self.handle_completion(event);
                }
            }
        }
    }

    fn process_menu_events(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(ref items) = self.menu_items {
                let result = {
                    let mut app = self.app.lock().unwrap();
                    event::handle_menu_event(&mut app, items, event)
                };

                match result {
                    EventResult::Quit => {
                        event_loop.exit();
                    }
                    EventResult::StateChanged | EventResult::SettingsChanged => {
                        self.update_menu();
                        // Update tray title
                        let app = self.app.lock().unwrap();
                        let title = timer::format_tray_title(&app.state);
                        drop(app); // Release lock before updating tray
                        self.update_tray_title(&title);
                    }
                    EventResult::StateChangedWithCompletion(completion_event) => {
                        self.update_menu();
                        // Update tray title
                        let app = self.app.lock().unwrap();
                        let title = timer::format_tray_title(&app.state);
                        drop(app); // Release lock before handling completion
                        self.update_tray_title(&title);
                        self.handle_completion(completion_event);
                    }
                    EventResult::Continue => {}
                }
            }
        }
    }
}

impl ApplicationHandler for Pomobar {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Nothing to do on resume for a tray-only app
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
        // No window events for a tray-only app
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Set a short poll interval to check for events
        event_loop.set_control_flow(ControlFlow::Poll);

        // Process timer messages from the background thread
        self.process_timer_messages();

        // Process menu events
        self.process_menu_events(event_loop);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize app state
    let app = Arc::new(Mutex::new(App::new()?));

    // Create event loop (required for tray on macOS)
    let event_loop = EventLoop::new()?;

    // Build menu
    let (built_menu, menu_items) = {
        let app_lock = app.lock().unwrap();
        menu::build_menu(&app_lock.state, &app_lock.session, &app_lock.settings)?
    };

    // Create tray icon (no icon image, just use title text on macOS)
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(built_menu))
        .with_title("🍅")
        .with_tooltip("Pomobar - Pomodoro Timer")
        .build()?;

    // Create channel for timer messages
    let (tx, rx) = mpsc::channel();

    // Spawn timer tick thread
    let app_clone = Arc::clone(&app);
    thread::spawn(move || {
        timer::run_timer_loop(app_clone, tx);
    });

    // Create application handler
    let mut pomobar = Pomobar::new(Arc::clone(&app), tray, rx);
    pomobar.set_menu_items(menu_items);

    // Run event loop
    event_loop.run_app(&mut pomobar)?;

    Ok(())
}

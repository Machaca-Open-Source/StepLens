use rdev::{Event, EventType as RdevEventType, Key, listen};
use std::sync::mpsc;
use std::thread;
use crate::models::EventType as AppEventType;

#[derive(Debug, Clone)]
pub struct InputEvent {
    pub event_type: AppEventType,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub key: Option<String>,
    pub window_title: Option<String>,
}

pub struct InputHook {
    sender: mpsc::Sender<InputEvent>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl InputHook {
    pub fn new() -> (Self, mpsc::Receiver<InputEvent>) {
        let (sender, receiver) = mpsc::channel();
        
        let sender_clone = sender.clone();
        let thread_handle = thread::spawn(move || {
            eprintln!("DEBUG: Hook thread started, setting up listener...");
            eprintln!("DEBUG: Note - on Wayland, rdev may have limited support for global hooks");
            eprintln!("DEBUG: Try clicking/typing OUTSIDE the application window to test");
            
            let mut event_count = 0;
            let callback = move |event: Event| {
                event_count += 1;
                // Only log non-MouseMove events to reduce spam
                if !matches!(event.event_type, RdevEventType::MouseMove { .. }) {
                    eprintln!("DEBUG: rdev event #{} received: {:?}", event_count, event.event_type);
                }
                let input_event = Self::process_rdev_event(event);
                if let Some(input_event) = input_event {
                    eprintln!("DEBUG: Sending input event to channel: {:?}", input_event.event_type);
                    if let Err(e) = sender_clone.send(input_event) {
                        eprintln!("DEBUG: Failed to send event: {:?}", e);
                    } else {
                        eprintln!("DEBUG: Event sent successfully");
                    }
                }
            };

            eprintln!("DEBUG: Starting rdev::listen()...");
            if let Err(error) = listen(callback) {
                eprintln!("ERROR: rdev::listen() failed: {:?}", error);
                eprintln!("ERROR: On Wayland, you may need to run with XWayland or use X11 session");
            } else {
                eprintln!("DEBUG: rdev::listen() exited (shouldn't happen)");
            }
        });

        (
            Self {
                sender,
                _thread: Some(thread_handle),
            },
            receiver,
        )
    }

    fn process_rdev_event(event: Event) -> Option<InputEvent> {
        eprintln!("DEBUG: process_rdev_event called with: {:?}", event.event_type);
        match event.event_type {
            RdevEventType::KeyPress(key) => {
                eprintln!("DEBUG: KeyPress detected: {:?}", key);
                let key_str = Self::key_to_string(key);
                let window_title = Self::get_active_window_title();
                
                Some(InputEvent {
                    event_type: AppEventType::KeyPress,
                    x: None,
                    y: None,
                    key: Some(key_str),
                    window_title,
                })
            }
            RdevEventType::ButtonPress(button) => {
                eprintln!("DEBUG: ButtonPress detected: {:?}", button);
                let button_str = format!("{:?}", button);
                let window_title = Self::get_active_window_title();
                // rdev doesn't provide position in Event, we'll get it from system
                // For now, leave as None - can be enhanced with platform-specific code
                Some(InputEvent {
                    event_type: AppEventType::MouseClick { button: button_str },
                    x: None, // TODO: Get from system API
                    y: None, // TODO: Get from system API  
                    key: None,
                    window_title,
                })
            }
            RdevEventType::MouseMove { x, y } => {
                // Log occasionally to avoid spam
                eprintln!("DEBUG: MouseMove filtered out: ({}, {})", x, y);
                None
            },
            RdevEventType::KeyRelease(_) => {
                eprintln!("DEBUG: KeyRelease filtered out");
                None
            },
            RdevEventType::ButtonRelease(_) => {
                eprintln!("DEBUG: ButtonRelease filtered out");
                None
            },
            RdevEventType::Wheel { .. } => {
                eprintln!("DEBUG: Wheel filtered out");
                None
            },
        }
    }

    fn key_to_string(key: Key) -> String {
        match key {
            Key::Space => "Space".to_string(),
            Key::Return => "Enter".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Escape => "Escape".to_string(),
            Key::LeftArrow => "Left Arrow".to_string(),
            Key::RightArrow => "Right Arrow".to_string(),
            Key::UpArrow => "Up Arrow".to_string(),
            Key::DownArrow => "Down Arrow".to_string(),
            Key::Delete => "Delete".to_string(),
            Key::Home => "Home".to_string(),
            Key::End => "End".to_string(),
            Key::PageUp => "Page Up".to_string(),
            Key::PageDown => "Page Down".to_string(),
            Key::ControlLeft => "Ctrl".to_string(),
            Key::ControlRight => "Ctrl".to_string(),
            Key::Alt => "Alt".to_string(),
            Key::ShiftLeft => "Shift".to_string(),
            Key::ShiftRight => "Shift".to_string(),
            Key::MetaLeft => "Meta".to_string(),
            Key::MetaRight => "Meta".to_string(),
            Key::F1 => "F1".to_string(),
            Key::F2 => "F2".to_string(),
            Key::F3 => "F3".to_string(),
            Key::F4 => "F4".to_string(),
            Key::F5 => "F5".to_string(),
            Key::F6 => "F6".to_string(),
            Key::F7 => "F7".to_string(),
            Key::F8 => "F8".to_string(),
            Key::F9 => "F9".to_string(),
            Key::F10 => "F10".to_string(),
            Key::F11 => "F11".to_string(),
            Key::F12 => "F12".to_string(),
            Key::CapsLock => "Caps Lock".to_string(),
            Key::Unknown(c) => format!("Unknown({})", c),
            _ => format!("{:?}", key),
        }
    }

    fn get_active_window_title() -> Option<String> {
        // This is a simplified version - on Linux you might need
        // to use xdotool or similar. For now, return None.
        // On Windows/macOS, you'd need platform-specific code.
        #[cfg(target_os = "linux")]
        {
            // Try to get window title using xdotool if available
            if let Ok(output) = std::process::Command::new("xdotool")
                .arg("getactivewindow")
                .arg("getwindowname")
                .output()
            {
                if output.status.success() {
                    return String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                }
            }
        }
        
        None
    }
}


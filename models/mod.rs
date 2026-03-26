use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub timestamp: i64,
    pub title: String,
    pub description: String,
    pub screenshot_path: PathBuf,
    pub event_type: EventType,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub key: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    MouseClick { button: String },
    KeyPress,
    WindowFocus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub steps: Vec<Step>,
}

impl Recording {
    pub fn new(name: String) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        let id = format!("recording_{}", timestamp);
        
        Self {
            id,
            name,
            created_at: timestamp,
            steps: Vec::new(),
        }
    }
}

impl Step {
    pub fn new(
        event_type: EventType,
        screenshot_path: PathBuf,
        x: Option<i32>,
        y: Option<i32>,
        key: Option<String>,
        window_title: Option<String>,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        let id = format!("step_{}", timestamp);
        
        let title = match &event_type {
            EventType::MouseClick { button } => {
                if let (Some(x), Some(y)) = (x, y) {
                    format!("Click {} at ({}, {})", button, x, y)
                } else {
                    format!("Click {}", button)
                }
            }
            EventType::KeyPress => {
                if let Some(k) = &key {
                    format!("Key pressed: {}", k)
                } else {
                    "Key pressed".to_string()
                }
            }
            EventType::WindowFocus => {
                if let Some(title) = &window_title {
                    format!("Window focused: {}", title)
                } else {
                    "Window focused".to_string()
                }
            }
        };

        // Generate default description based on event type
        let description = match &event_type {
            EventType::MouseClick { button } => {
                if let (Some(x), Some(y)) = (x, y) {
                    format!("User clicked {} button at coordinates ({}, {}).", button, x, y)
                } else {
                    format!("User clicked {} button.", button)
                }
            }
            EventType::KeyPress => {
                if let Some(k) = &key {
                    format!("User pressed the '{}' key.", k)
                } else {
                    "User pressed a key.".to_string()
                }
            }
            EventType::WindowFocus => {
                if let Some(title) = &window_title {
                    format!("User switched focus to the window: '{}'.", title)
                } else {
                    "User switched window focus.".to_string()
                }
            }
        };

        Self {
            id,
            timestamp,
            title,
            description,
            screenshot_path,
            event_type,
            x,
            y,
            key,
            window_title,
        }
    }
}


use anyhow::Result;
use crate::models::Recording;
use std::path::PathBuf;

pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?
            .join("capturer");

        std::fs::create_dir_all(&data_dir)?;

        Ok(Self { data_dir })
    }

    pub fn get_recordings_dir(&self) -> PathBuf {
        self.data_dir.join("recordings")
    }

    pub fn get_screenshots_dir(&self) -> PathBuf {
        self.data_dir.join("screenshots")
    }

    pub fn save_recording(&self, recording: &Recording) -> Result<PathBuf> {
        let recordings_dir = self.get_recordings_dir();
        std::fs::create_dir_all(&recordings_dir)?;

        let filepath = recordings_dir.join(format!("{}.json", recording.id));
        let json = serde_json::to_string_pretty(recording)?;
        std::fs::write(&filepath, json)?;

        Ok(filepath)
    }

    pub fn load_recording(&self, recording_id: &str) -> Result<Recording> {
        let filepath = self.get_recordings_dir().join(format!("{}.json", recording_id));
        let json = std::fs::read_to_string(&filepath)?;
        let recording: Recording = serde_json::from_str(&json)?;

        Ok(recording)
    }

    pub fn list_recordings(&self) -> Result<Vec<Recording>> {
        let recordings_dir = self.get_recordings_dir();
        
        if !recordings_dir.exists() {
            return Ok(Vec::new());
        }

        let mut recordings = Vec::new();

        for entry in std::fs::read_dir(&recordings_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(recording) = serde_json::from_str::<Recording>(&json) {
                        recordings.push(recording);
                    }
                }
            }
        }

        recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(recordings)
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let filepath = self.get_recordings_dir().join(format!("{}.json", recording_id));
        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }
        Ok(())
    }
}


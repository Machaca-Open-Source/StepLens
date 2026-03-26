use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use base64::Engine;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    images: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct OllamaResponseChunk {
    response: String,
    done: Option<bool>,
}

#[derive(Clone)]
pub struct LLMClient {
    base_url: String,
    model: String,
}

impl LLMClient {
    pub fn new(model: String) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn generate_instruction(
        &self,
        screenshot_path: &std::path::Path,
        context: &str,
    ) -> Result<String> {
        // Read and encode screenshot to base64
        let image_data = fs::read(screenshot_path)?;
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_data);
        
        let prompt = format!(
            "You are a helpful assistant that generates step-by-step instructions for software tutorials. \
            Based on this screenshot and the context below, write a clear, concise instruction for what \
            the user should do in this step. Be specific and actionable.\n\n\
            Context: {}\n\n\
            Generate a single sentence instruction:",
            context
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            images: Some(vec![base64_image]),
        };

        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", self.base_url);
        
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "LLM API error: {}",
                response.status()
            ));
        }

        // For non-streaming, Ollama still returns line-delimited JSON
        let text = response.text().await?;
        let mut full_response = String::new();
        
        for line in text.lines() {
            if let Ok(chunk_response) = serde_json::from_str::<OllamaResponseChunk>(line) {
                full_response.push_str(&chunk_response.response);
                if chunk_response.done == Some(true) {
                    break;
                }
            }
        }
        
        if full_response.is_empty() {
            // Fallback: try parsing as single JSON object
            if let Ok(chunk_response) = serde_json::from_str::<OllamaResponseChunk>(&text) {
                full_response = chunk_response.response;
            }
        }
        
        Ok(full_response.trim().to_string())
    }

    pub fn is_available(&self) -> bool {
        // Simple check if Ollama is running
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::new();
            let url = format!("{}/api/tags", self.base_url);
            client.get(&url).send().await.is_ok()
        })
    }
}


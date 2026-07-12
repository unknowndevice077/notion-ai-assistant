use serde::Serialize;
use std::process::{Command, Stdio};

pub const OLLAMA_BASE_URL: &str = "http://localhost:11434";
pub const OLLAMA_DUMMY_KEY: &str = "ollama-local";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub pulled_models: Vec<String>,
}

pub async fn check_status() -> OllamaStatus {
    let installed = Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let client = reqwest::Client::new();
    let tags_resp = client.get(format!("{OLLAMA_BASE_URL}/api/tags")).send().await;

    let mut running = false;
    let mut pulled_models = Vec::new();

    if let Ok(resp) = tags_resp {
        if resp.status().is_success() {
            running = true;
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json["models"].as_array() {
                    for m in models {
                        if let Some(name) = m["name"].as_str() {
                            pulled_models.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    OllamaStatus { installed, running, pulled_models }
}

pub fn pull_model(model: &str) -> Result<(), String> {
    Command::new("ollama")
        .arg("pull")
        .arg(model)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Couldn't start 'ollama pull {model}': {e}. Is Ollama installed and on PATH?"))?;
    Ok(())
}
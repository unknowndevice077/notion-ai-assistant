use crate::ollama::OllamaStatus;
use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub source: String,
    pub status: String,
    pub detail: String,
    pub recommended: bool,
}

struct LocalModelSpec { id: &'static str, label: &'static str, min_ram_gb: u32 }
struct CloudModelSpec { id: &'static str, label: &'static str }

pub const DEFAULT_MODEL_ID: &str = "llama3.2:3b";
pub const LEGACY_HEAVY_MODEL_ID: &str = "deepseek-coder-v2:16b";

const LOCAL_CATALOG: [LocalModelSpec; 12] = [
    LocalModelSpec { id: "deepseek-coder-v2:16b", label: "DeepSeek Coder V2 16B — bundled default", min_ram_gb: 8 },
    LocalModelSpec { id: "llama3.2:1b",     label: "Llama 3.2 1B — ultra-light, lowest RAM",  min_ram_gb: 4 },
    LocalModelSpec { id: "llama3.2:3b",     label: "Llama 3.2 3B — light, low RAM",           min_ram_gb: 4 },
    LocalModelSpec { id: "phi3:mini",       label: "Phi-3 Mini 3.8B — light, low RAM",        min_ram_gb: 4 },
    LocalModelSpec { id: "gemma2:2b",       label: "Gemma 2 2B — light, low RAM",             min_ram_gb: 4 },
    LocalModelSpec { id: "deepseek-r1:1.5b", label: "DeepSeek R1 1.5B — fastest, lowest RAM", min_ram_gb: 4 },
    LocalModelSpec { id: "mistral:7b",      label: "Mistral 7B — balanced, general-purpose",  min_ram_gb: 8 },
    LocalModelSpec { id: "deepseek-r1:7b",  label: "DeepSeek R1 7B — balanced",                min_ram_gb: 8 },
    LocalModelSpec { id: "deepseek-r1:8b",  label: "DeepSeek R1 8B — balanced (Llama-based)",  min_ram_gb: 8 },
    LocalModelSpec { id: "gemma2:9b",       label: "Gemma 2 9B — balanced",                    min_ram_gb: 8 },
    LocalModelSpec { id: "deepseek-r1:14b", label: "DeepSeek R1 14B — stronger reasoning",     min_ram_gb: 16 },
    LocalModelSpec { id: "qwen2.5:14b",     label: "Qwen 2.5 14B",                             min_ram_gb: 16 },
];

const CLOUD_CATALOG: [CloudModelSpec; 3] = [
    CloudModelSpec { id: "deepseek/deepseek-v4-flash", label: "DeepSeek V4 Flash (cloud, low-cost)" },
    CloudModelSpec { id: "qwen/qwen-2.5-72b-instruct", label: "Qwen 2.5 72B Instruct (cloud)" },
    CloudModelSpec { id: "openai/gpt-4o-mini", label: "GPT-4o mini (cloud)" },
];

fn total_ram_gb() -> f64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory() as f64 / 1_073_741_824.0
}

fn recommend_local_model_id_for_ram(ram_gb: f64) -> String {
    if ram_gb < 4.0 {
        return "llama3.2:1b".to_string();
    }
    DEFAULT_MODEL_ID.to_string()
}

pub fn recommended_local_model_id() -> String {
    let ram_gb = total_ram_gb();
    recommend_local_model_id_for_ram(ram_gb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommends_smaller_models_on_low_ram() {
        assert_eq!(recommend_local_model_id_for_ram(3.0), "llama3.2:1b");
        assert_eq!(recommend_local_model_id_for_ram(5.5), DEFAULT_MODEL_ID);
        assert_eq!(recommend_local_model_id_for_ram(12.0), DEFAULT_MODEL_ID);
    }
}

pub fn list_models(ollama: &OllamaStatus) -> Vec<ModelInfo> {
    let ram_gb = total_ram_gb();

    let mut models: Vec<ModelInfo> = LOCAL_CATALOG
        .iter()
        .map(|spec| {
            let compatible = ram_gb + 0.5 >= spec.min_ram_gb as f64;
            let (status, detail) = if !ollama.installed {
                ("not_installed".to_string(), "Ollama isn't installed on this machine yet.".to_string())
            } else if !compatible {
                ("insufficient_ram".to_string(), format!("Needs ~{}GB RAM — this machine has ~{:.0}GB.", spec.min_ram_gb, ram_gb))
            } else if ollama.pulled_models.iter().any(|m| m == spec.id) {
                ("ready".to_string(), "Downloaded and ready to use.".to_string())
            } else {
                ("pull_required".to_string(), format!("Needs ~{}GB RAM (you have ~{:.0}GB) — not downloaded yet.", spec.min_ram_gb, ram_gb))
            };
            ModelInfo {
                id: spec.id.to_string(),
                label: spec.label.to_string(),
                source: "local".to_string(),
                status,
                detail,
                recommended: spec.id == DEFAULT_MODEL_ID,
            }
        })
        .collect();

    models.extend(CLOUD_CATALOG.iter().map(|spec| ModelInfo {
        id: spec.id.to_string(),
        label: spec.label.to_string(),
        source: "cloud".to_string(),
        status: "available".to_string(),
        detail: "Runs through the app's cloud AI connection — no download, no local compute.".to_string(),
        recommended: false,
    }));

    // The curated catalog above is a set of suggestions, not the full
    // picture — surface anything the user has actually pulled via Ollama
    // that isn't already one of those entries, so their real local setup
    // is always reflected accurately.
    for pulled in &ollama.pulled_models {
        let already_listed = LOCAL_CATALOG.iter().any(|spec| spec.id == pulled);
        if !already_listed {
            models.push(ModelInfo {
                id: pulled.clone(),
                label: pulled.clone(),
                source: "local".to_string(),
                status: "ready".to_string(),
                detail: "Installed locally via Ollama (not in the curated list).".to_string(),
                recommended: false,
            });
        }
    }

    models
}
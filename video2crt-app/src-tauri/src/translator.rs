//! Cloud translation glue for Minimax.
//!
//! Responsibilities:
//! - Fetch `GET /v1/models` from `https://api.minimax.io/v1` using the
//!   saved API key (Bearer token).
//! - Filter the response down to "language model" entries suitable for
//!   subtitle translation (drop vision / TTS / legacy video models).
//! - When the network call fails or no key is configured, return a
//!   hard-coded fallback list per HANDOFF-newtask-app.md so the UI dropdown
//!   is never empty.

use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.minimax.io/v1";
const REQUEST_TIMEOUT_SECS: u64 = 8;

/// One entry in the dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub group: String, // "language" | "video" | "speech" | "other"
    pub is_default: bool,
}

/// Return the model list. Always non-empty (falls back to a hard-coded list).
pub async fn list_models() -> Vec<ModelInfo> {
    // Try the live endpoint first; on any error (no key, network, parse),
    // drop to the hard-coded list.
    match fetch_live().await {
        Ok(mut live) if !live.is_empty() => {
            // Ensure the canonical default is always present even if the
            // remote omits it.
            if !live.iter().any(|m| m.is_default) {
                if let Some(m) = live.first_mut() {
                    m.is_default = true;
                }
            }
            live
        }
        _ => hard_coded(),
    }
}

/// Hard-coded fallback list per HANDOFF-newtask-app.md.
fn hard_coded() -> Vec<ModelInfo> {
    vec![
        // Language models (used for cloud translation)
        ModelInfo {
            id: "MiniMax-M3".into(),
            label: "M3 (latest, 1M ctx) — default".into(),
            group: "language".into(),
            is_default: true,
        },
        ModelInfo {
            id: "MiniMax-M2.7".into(),
            label: "M2.7".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2.7-highspeed".into(),
            label: "M2.7 highspeed".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2.5".into(),
            label: "M2.5 (legacy)".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2.5-highspeed".into(),
            label: "M2.5 highspeed (legacy)".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2.1".into(),
            label: "M2.1 (legacy)".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2.1-highspeed".into(),
            label: "M2.1 highspeed (legacy)".into(),
            group: "language".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-M2".into(),
            label: "M2 (legacy)".into(),
            group: "language".into(),
            is_default: false,
        },
        // Video models (informational; not used for translation)
        ModelInfo {
            id: "MiniMax-H3".into(),
            label: "H3".into(),
            group: "video".into(),
            is_default: false,
        },
        ModelInfo {
            id: "MiniMax-H3-Max".into(),
            label: "H3 Max".into(),
            group: "video".into(),
            is_default: false,
        },
        ModelInfo {
            id: "Hailuo-2.3".into(),
            label: "Hailuo 2.3 (legacy)".into(),
            group: "video".into(),
            is_default: false,
        },
        ModelInfo {
            id: "Hailuo-2.3Fast".into(),
            label: "Hailuo 2.3 Fast (legacy)".into(),
            group: "video".into(),
            is_default: false,
        },
        ModelInfo {
            id: "Hailuo-02".into(),
            label: "Hailuo 02 (legacy)".into(),
            group: "video".into(),
            is_default: false,
        },
        // Speech models (TTS; not used)
        ModelInfo {
            id: "speech-2.8-hd".into(),
            label: "Speech 2.8 HD".into(),
            group: "speech".into(),
            is_default: false,
        },
        ModelInfo {
            id: "speech-2.8-turbo".into(),
            label: "Speech 2.8 Turbo".into(),
            group: "speech".into(),
            is_default: false,
        },
        ModelInfo {
            id: "speech-2.6-hd".into(),
            label: "Speech 2.6 HD (legacy)".into(),
            group: "speech".into(),
            is_default: false,
        },
        ModelInfo {
            id: "speech-2.6-turbo".into(),
            label: "Speech 2.6 Turbo (legacy)".into(),
            group: "speech".into(),
            is_default: false,
        },
    ]
}

#[derive(Debug, Deserialize)]
struct LiveModel {
    id: String,
    #[serde(default)]
    object: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelsResp {
    data: Vec<LiveModel>,
}

async fn fetch_live() -> anyhow::Result<Vec<ModelInfo>> {
    let key = match super::settings::get_api_key() {
        Some(k) if !k.is_empty() => k,
        _ => anyhow::bail!("no api key"),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;
    let resp = client
        .get(format!("{BASE_URL}/models"))
        .bearer_auth(&key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("models endpoint returned {status}");
    }
    let body: ModelsResp = resp.json().await?;
    Ok(body
        .data
        .into_iter()
        .map(|m| {
            let group = classify(&m.id);
            ModelInfo {
                id: m.id.clone(),
                label: m.id.clone(),
                group,
                is_default: false,
            }
        })
        .collect())
}

fn classify(id: &str) -> String {
    let lower = id.to_lowercase();
    if lower.contains("speech") || lower.contains("tts") {
        "speech".into()
    } else if lower.contains("h3")
        || lower.starts_with("hailuo")
        || lower.contains("video")
    {
        "video".into()
    } else {
        "language".into()
    }
}
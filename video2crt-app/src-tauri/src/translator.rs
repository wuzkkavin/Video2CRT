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
    } else if lower.contains("h3") || lower.starts_with("hailuo") || lower.contains("video") {
        "video".into()
    } else {
        "language".into()
    }
}

// ---------- Translation runtime ------------------------------------------

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// Translate `text` from its source language (Whisper ASR writes
/// whatever the speaker used) into Traditional Chinese via the
/// configured Minimax model. Returns the translated string on
/// success, or an error if the API key is missing, the HTTP call
/// fails, the response body cannot be parsed, or the model returns
/// an empty `choices[0].message.content`.
pub async fn translate_text(text: &str, model: &str) -> anyhow::Result<String> {
    let key = match super::settings::get_api_key() {
        Some(k) if !k.is_empty() => k,
        _ => anyhow::bail!("no api key configured"),
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS * 4))
        .build()?;
    let sys_prompt = "You are a subtitle translator. Translate the user's text into Traditional Chinese (zh-Hant). Output ONLY the translation, no quotes, no explanation, no leading labels. Preserve line breaks if any.";
    let req = ChatRequest {
        model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: sys_prompt,
            },
            ChatMessage {
                role: "user",
                content: text,
            },
        ],
        temperature: 0.2,
    };
    let resp = client
        .post(format!("{BASE_URL}/chat/completions"))
        .bearer_auth(&key)
        .header("Accept", "application/json")
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("translate endpoint returned {status}: {body}");
    }
    let parsed: ChatResponse = resp.json().await?;
    let content = parsed
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();
    clean_subtitle_translation(&content, text)
}

/// A subtitle may contain only the translated line. Some reasoning-capable
/// models emit an internal `<think>...</think>` block despite the prompt; it
/// must never reach an SRT or become visible in the video.
fn clean_subtitle_translation(raw: &str, source: &str) -> anyhow::Result<String> {
    let mut output = raw.trim().to_string();
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find("<think>") else { break };
        let Some(end_relative) = lower[start + 7..].find("</think>") else {
            anyhow::bail!("翻譯服務回傳未結束的思考內容，已拒絕燒錄字幕");
        };
        let end = start + 7 + end_relative + "</think>".len();
        output.replace_range(start..end, "");
    }
    let output = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if output.is_empty() {
        anyhow::bail!("翻譯服務未回傳可用的繁中字幕");
    }
    let max_length = (source.chars().count() * 6 + 40).max(80);
    if output.chars().count() > max_length {
        anyhow::bail!("翻譯結果異常過長，已拒絕燒錄字幕");
    }
    if !output.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
        anyhow::bail!("翻譯服務未回傳繁體中文字幕");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::clean_subtitle_translation;

    #[test]
    fn thought_blocks_are_never_returned_as_subtitles() {
        assert_eq!(
            clean_subtitle_translation(
                "<think>Explain the translation at length.</think> 拜託！",
                "C'mon!",
            )
            .unwrap(),
            "拜託！"
        );
    }

    #[test]
    fn giant_explanations_are_rejected() {
        assert!(clean_subtitle_translation(&"說明".repeat(100), "短句").is_err());
    }
}

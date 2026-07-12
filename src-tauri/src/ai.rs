use crate::models::ContentItem;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("provider returned an error: {0}")]
    Provider(String),
    #[error("could not parse the model's response as content JSON: {0}")]
    Parse(String),
}

impl AiError {
    pub fn should_retry_with_safe_model(&self) -> bool {
        match self {
            AiError::Provider(message) => {
                let lower = message.to_lowercase();
                lower.contains("bad_alloc")
                    || lower.contains("failed to allocate")
                    || lower.contains("llama-server process has terminated")
                    || lower.contains("exit status 1")
                    || lower.contains("out of memory")
            }
            _ => false,
        }
    }
}

pub struct OpenAiCompatibleProvider {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

const SYSTEM_PROMPT: &str = r#"You are a content generation assistant embedded in a desktop app connected to a client's Notion workspace. You will be given an instruction, which may include a business name and business context. You will also sometimes be given EXISTING CONTENT for a business that you are being asked to edit or add to — in that case, stay consistent with its tone and only produce what the instruction asks for. Respond ONLY with a single JSON object, no prose, no markdown fences, matching exactly this shape:

{
  "items": [
    { "type": "headline", "title": "short hook, 3-8 words", "text": "the longer ready-to-publish body/description" },
    { "type": "subline", "text": "..." },
    { "type": "quote", "title": "short attribution or theme", "text": "the quote itself" },
    { "type": "tip", "title": "short hook, 3-8 words", "text": "the longer ready-to-publish body/description" }
  ]
}

Do NOT include "calendar" items — the app schedules those deterministically from the headlines/quotes/tips you return, so never emit them yourself.

"headline", "quote", and "tip" items MUST always include both "title" (short) and "text" (longer body) — never omit "title" for these types. "subline" items only need "text". Every item must be directly relevant to the business name and business context given in the instruction — never write generic, business-agnostic filler. Keep text concise and ready to publish as-is."#;

impl OpenAiCompatibleProvider {
    pub async fn ping(&self) -> Result<String, AiError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "messages": [
                    { "role": "user", "content": "Reply with exactly one word: OK" }
                ],
                "max_tokens": 5
            }))
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AiError::Provider(format!("{status}: {body_text}")));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| AiError::Parse(format!("invalid response envelope: {e}")))?;

        body.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|m| m.get("message"))
            .and_then(|con| con.get("content"))
            .and_then(|s| s.as_str())
            .map(str::to_string)
            .ok_or_else(|| AiError::Parse("missing choices[0].message.content".into()))
    }

    pub async fn generate(&self, user_prompt: &str) -> Result<Vec<ContentItem>, AiError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "messages": [
                    { "role": "system", "content": SYSTEM_PROMPT },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.8,
                // Raised from 4096: a 10-day preset asking for 5 headlines +
                // 5 quotes + 5 tips per day is 150 items, each needing a
                // title + text field. At ~50-80 tokens/item that's roughly
                // 9,000-12,000 tokens of pure JSON content — well past the
                // old 4096 cap, which is what was silently truncating the
                // response mid-JSON and leaving only a handful of items
                // after parsing. This is a mitigation, not a full fix — see
                // commands.rs for the real fix (per-day generation calls
                // instead of one giant request).
                "max_tokens": 16000
            }))
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AiError::Provider(format!("{status}: {body_text}")));
        }

        let body: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| AiError::Parse(format!("invalid response envelope: {e}")))?;

        let content = body.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|m| m.get("message"))
            .and_then(|con| con.get("content"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| AiError::Parse("missing choices[0].message.content".into()))?;

        parse_content_json(content)
    }
}

fn parse_content_json(raw: &str) -> Result<Vec<ContentItem>, AiError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(default)]
        items: Vec<ContentItem>,
    }

    // Some local models (especially smaller ones, under a bigger request)
    // don't reliably emit one JSON object — they produce several JSON
    // objects concatenated back-to-back instead. Stream-parse every JSON
    // value present in the response and merge all their "items" arrays,
    // rather than failing outright on the first object boundary.
    let stream = serde_json::Deserializer::from_str(cleaned).into_iter::<Wrapper>();
    let mut all_items = Vec::new();
    let mut parsed_any = false;

    for value in stream {
        match value {
            Ok(wrapper) => {
                parsed_any = true;
                all_items.extend(wrapper.items);
            }
            Err(e) => {
                if !parsed_any {
                    return Err(AiError::Parse(format!("{e}. Raw content: {cleaned}")));
                }
                break; // got at least one good chunk — stop at the first bad one
            }
        }
    }

    if all_items.is_empty() {
        return Err(AiError::Parse(format!("model returned no content items. Raw content: {cleaned}")));
    }

    Ok(all_items)
}
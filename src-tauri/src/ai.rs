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

#[derive(Debug, serde::Deserialize)]
pub struct PageDesign {
    pub emoji: String,
    pub tagline: String,
    pub unsplash_query: String,
}

const PAGE_DESIGN_SYSTEM_PROMPT: &str = r#"You design the visual identity for a Notion content hub page for a business. Given a business name and context, respond ONLY with a single JSON object, no prose, no markdown fences, matching exactly this shape:
{
  "emoji": "one single emoji that genuinely fits this specific business",
  "tagline": "one short punchy sentence, under 12 words, capturing what this business actually does",
  "unsplash_query": "a 2-4 word photo search query specific to this business (e.g. 'cozy coffee shop interior', not generic like 'business')"
}"#;

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

"headline", "quote", and "tip" items MUST always include both "title" (short) and "text" (longer body) — never omit "title" for these types. "subline" items only need "text". Every item must be directly relevant to the business name and business context given in the instruction — never write generic, business-agnostic filler. Keep text concise and ready to publish as-is. Never include an unescaped double-quote character (") inside any "title" or "text" value — if you need to show a quotation within a quote, use single quotes (') instead, e.g. 'like this', not "like this"."#;

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

    pub async fn design_page(&self, business_name: &str, business_context: &str) -> Result<PageDesign, AiError> {
        let user_prompt = format!(
            "Business name: {business_name}\nBusiness context: {}",
            if business_context.trim().is_empty() { "(none given)" } else { business_context }
        );

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "messages": [
                    { "role": "system", "content": PAGE_DESIGN_SYSTEM_PROMPT },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.7,
                "max_tokens": 200
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
        let content = body
            .get("choices").and_then(|c| c.get(0)).and_then(|m| m.get("message")).and_then(|c| c.get("content"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| AiError::Parse("missing choices[0].message.content".into()))?;

        let cleaned = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
        serde_json::from_str::<PageDesign>(cleaned).map_err(|e| AiError::Parse(format!("{e}. Raw content: {cleaned}")))
    }
}

fn parse_content_json(raw: &str) -> Result<Vec<ContentItem>, AiError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match try_parse_items(cleaned) {
        Ok(items) => Ok(items),
        Err(first_err) => {
            // The model likely embedded a literal, unescaped `"` inside a
            // string value (e.g. quoting something within a "quote" item) —
            // exactly the failure mode this was added to fix. Try to
            // repair it heuristically and re-parse once before giving up.
            let repaired = repair_unescaped_quotes(cleaned);
            try_parse_items(&repaired).map_err(|_| first_err)
        }
    }
}

fn try_parse_items(cleaned: &str) -> Result<Vec<ContentItem>, AiError> {
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

/// Best-effort repair for a common small-model mistake: writing a literal
/// `"` inside a string value (e.g. quoting something within a "quote" item)
/// without escaping it as `\"`. Scans character-by-character, tracking
/// whether we're inside a JSON string; when a `"` appears mid-string and
/// isn't immediately followed (past whitespace) by a JSON structural
/// character (`,` `}` `]` `:` or end-of-input), it's treated as a literal
/// quote and escaped instead of closing the string. Not bulletproof for
/// every pathological case, but resolves the exact failure mode seen in
/// practice: a model quoting an attribution inline, e.g.
/// `"text": "...is now." – Chinese Proverb"`.
fn repair_unescaped_quotes(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len() + 16);
    let mut in_string = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if in_string {
            if c == '\\' && i + 1 < chars.len() {
                out.push(c);
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let next = chars.get(j).copied();
                let closes_string = matches!(next, Some(',') | Some('}') | Some(']') | Some(':') | None);
                if closes_string {
                    out.push(c);
                    in_string = false;
                } else {
                    out.push('\\');
                    out.push('"');
                }
                i += 1;
                continue;
            }
            out.push(c);
            i += 1;
        } else {
            out.push(c);
            if c == '"' {
                in_string = true;
            }
            i += 1;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_unescaped_quote_with_trailing_attribution() {
        let broken = r#"{ "items": [ { "type": "quote", "title": "Timeless Wisdom", "text": "The best time to plant a tree was 20 years ago. The second best time is now." – Chinese Proverb" } ] }"#;
        let repaired = repair_unescaped_quotes(broken);
        let parsed: serde_json::Value = serde_json::from_str(&repaired).expect("repaired JSON should parse");
        let text = parsed["items"][0]["text"].as_str().unwrap();
        assert!(text.contains("Chinese Proverb"));
    }

    #[test]
    fn leaves_well_formed_json_unchanged_in_effect() {
        let good = r#"{ "items": [ { "type": "tip", "title": "Water Early", "text": "Water in the morning." } ] }"#;
        let items = try_parse_items(good).expect("well-formed JSON should parse directly");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "Water in the morning.");
    }
}
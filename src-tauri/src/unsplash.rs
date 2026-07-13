use serde_json::Value;

const UNSPLASH_API_BASE: &str = "https://api.unsplash.com";

/// Baked-in build-time key (optional), used only if the user hasn't set
/// their own key in Settings. `UNSPLASH_ACCESS_KEY=xxxxx cargo build`.
fn built_in_key() -> String {
    option_env!("UNSPLASH_ACCESS_KEY").unwrap_or("").to_string()
}

/// Fetches one relevant photo URL for use as a Notion page cover, keyed off
/// whatever the business name/industry was (the AI's own `unsplash_query`
/// from design_page, e.g. "cozy coffee shop interior" — not a generic
/// placeholder). `user_key`, if the user has set one in Settings, is tried
/// first; otherwise falls back to a build-time key if one was baked in.
/// If neither is available, returns None and the page simply gets no
/// cover — a plain page rather than a random/unrelated stock photo.
pub async fn fetch_cover_image(query: &str, user_key: Option<&str>) -> Option<String> {
    let key = match user_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => built_in_key(),
    };
    if key.is_empty() {
        return None;
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{UNSPLASH_API_BASE}/search/photos"))
        .query(&[("query", query), ("per_page", "1"), ("orientation", "landscape")])
        .header("Authorization", format!("Client-ID {key}"))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: Value = resp.json().await.ok()?;
    body.get("results")?
        .get(0)?
        .get("urls")?
        .get("regular")?
        .as_str()
        .map(str::to_string)
}
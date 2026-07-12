use serde_json::Value;

const UNSPLASH_API_BASE: &str = "https://api.unsplash.com";

/// Baked in at build time, same pattern as BUILT_IN_OPENROUTER_KEY:
/// UNSPLASH_ACCESS_KEY=xxxxx cargo build
/// Get a free key at https://unsplash.com/developers (Demo apps get 50
/// requests/hour, which is plenty for "one cover image per new client").
fn access_key() -> String {
    option_env!("UNSPLASH_ACCESS_KEY").unwrap_or("").to_string()
}

/// Fetches one relevant photo URL for use as a Notion page cover, keyed off
/// whatever the business name/industry was. Returns None — never an error —
/// if the key isn't configured or nothing matched, since a missing cover
/// image should never block creating the actual content hub.
pub async fn fetch_cover_image(query: &str) -> Option<String> {
    let key = access_key();
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
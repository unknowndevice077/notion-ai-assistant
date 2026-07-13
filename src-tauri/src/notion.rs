use serde_json::{json, Value};
use thiserror::Error;

const NOTION_API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

#[derive(Debug, Error)]
pub enum NotionError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("notion api error ({status}): {body}")]
    Api { status: u16, body: String },
}

pub struct NotionClient {
    token: String,
    client: reqwest::Client,
}

pub const TEMPLATE_DATABASES: [&str; 5] = ["Headlines", "Sublines", "Quotes", "Useful Tips", "Content Calendar"];

fn icon_for_database(title: &str) -> &'static str {
    match title {
        "Headlines" => "📰",
        "Sublines" => "✍️",
        "Quotes" => "💬",
        "Useful Tips" => "💡",
        "Content Calendar" => "📅",
        _ => "🗂️",
    }
}

impl NotionClient {
    pub fn new(token: String) -> Self {
        Self { token, client: reqwest::Client::new() }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", self.token).parse().unwrap());
        headers.insert("Notion-Version", NOTION_VERSION.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }

    async fn handle(&self, resp: reqwest::Response) -> Result<Value, NotionError> {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(NotionError::Api { status, body });
        }
        Ok(serde_json::from_str(&body).unwrap_or(Value::Null))
    }

    pub async fn verify_token(&self) -> Result<(), NotionError> {
        let resp = self.client.get(format!("{NOTION_API_BASE}/users/me")).headers(self.headers()).send().await?;
        self.handle(resp).await.map(|_| ())
    }

    /// Checks whether a page still exists and is shared with the
    /// integration. Returns Ok(false) specifically on a 404 (deleted,
    /// archived-and-purged, or unshared) so callers can self-heal by
    /// recreating it, instead of surfacing a permanent error every time.
    /// Any other error (network, auth, etc.) is propagated as-is.
    pub async fn page_exists(&self, page_id: &str) -> Result<bool, NotionError> {
        let resp = self.client.get(format!("{NOTION_API_BASE}/pages/{page_id}")).headers(self.headers()).send().await?;
        let status = resp.status().as_u16();
        if status == 404 {
            return Ok(false);
        }
        let body = resp.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(NotionError::Api { status, body });
        }
        // A page that's been trashed still 200s but comes back with
        // "in_trash": true / "archived": true — treat that as gone too.
        let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
        let trashed = value.get("in_trash").and_then(|v| v.as_bool()).unwrap_or(false)
            || value.get("archived").and_then(|v| v.as_bool()).unwrap_or(false);
        Ok(!trashed)
    }

    pub fn is_not_found(err: &NotionError) -> bool {
        matches!(err, NotionError::Api { status: 404, .. })
    }

    pub async fn find_accessible_page(&self) -> Result<Option<String>, NotionError> {
        let resp = self
            .client
            .post(format!("{NOTION_API_BASE}/search"))
            .headers(self.headers())
            .json(&json!({ "filter": { "property": "object", "value": "page" }, "page_size": 1 }))
            .send()
            .await?;
        let body = self.handle(resp).await?;
        Ok(body.get("results").and_then(|r| r.get(0)).and_then(|r| r.get("id")).and_then(|id| id.as_str()).map(str::to_string))
    }

    /// `cover_image_url`, if present (see unsplash.rs), sets the page's cover
    /// photo — this is the "beautiful, aesthetic" piece: an image genuinely
    /// relevant to the business, not a random placeholder.
    pub async fn create_content_hub(
        &self,
        parent_page_id: &str,
        business_name: &str,
        business_context: Option<&str>,
        icon_emoji: &str,
        tagline: Option<&str>,
        cover_image_url: Option<&str>,
    ) -> Result<String, NotionError> {
        let mut children = Vec::new();

        if let Some(tag) = tagline {
            if !tag.trim().is_empty() {
                children.push(json!({
                    "object": "block",
                    "type": "heading_2",
                    "heading_2": { "rich_text": [{ "type": "text", "text": { "content": tag.trim() } }] }
                }));
            }
        }

        if let Some(ctx) = business_context {
            if !ctx.trim().is_empty() {
                children.push(json!({
                    "object": "block",
                    "type": "callout",
                    "callout": {
                        "icon": { "type": "emoji", "emoji": "🏷️" },
                        "color": "gray_background",
                        "rich_text": [{ "type": "text", "text": { "content": format!("About this business: {}", ctx.trim()) } }]
                    }
                }));
            }
        }

        children.push(json!({ "object": "block", "type": "divider", "divider": {} }));

        let mut payload = json!({
            "parent": { "page_id": parent_page_id },
            "icon": { "type": "emoji", "emoji": icon_emoji },
            "properties": { "title": { "title": [{ "text": { "content": business_name } }] } },
            "children": children
        });

        if let Some(url) = cover_image_url {
            payload["cover"] = json!({ "type": "external", "external": { "url": url } });
        }

        let resp = self.client.post(format!("{NOTION_API_BASE}/pages")).headers(self.headers()).json(&payload).send().await?;
        let body = self.handle(resp).await?;
        Ok(body.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string())
    }

    pub async fn create_database(&self, hub_page_id: &str, title: &str, is_calendar: bool) -> Result<String, NotionError> {
        let mut properties = serde_json::Map::new();
        properties.insert("Name".to_string(), json!({ "title": {} }));

        if is_calendar {
            properties.insert("Date".to_string(), json!({ "date": {} }));
            properties.insert("Content Type".to_string(), json!({
                "select": { "options": [
                    { "name": "Headline", "color": "blue" },
                    { "name": "Subline", "color": "purple" },
                    { "name": "Quote", "color": "pink" },
                    { "name": "Tip", "color": "yellow" }
                ] }
            }));
            properties.insert("Status".to_string(), json!({
                "select": { "options": [
                    { "name": "Draft", "color": "gray" },
                    { "name": "Pushed", "color": "green" },
                    { "name": "Archived", "color": "red" }
                ] }
            }));
            properties.insert("Platform".to_string(), json!({ "rich_text": {} }));
        } else {
    // "Text" removed — it was just re-storing the same title already
    // shown in "Name", which is what caused the duplicate-looking rows.
    properties.insert("Description".to_string(), json!({ "rich_text": {} }));
}

        let resp = self
            .client
            .post(format!("{NOTION_API_BASE}/databases"))
            .headers(self.headers())
            .json(&json!({
                "parent": { "type": "page_id", "page_id": hub_page_id },
                "icon": { "type": "emoji", "emoji": icon_for_database(title) },
                "title": [{ "type": "text", "text": { "content": title } }],
                "properties": properties
            }))
            .send()
            .await?;
        let body = self.handle(resp).await?;
        Ok(body.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string())
    }

    pub async fn add_database_row(&self, database_id: &str, title: &str, description: &str) -> Result<String, NotionError> {
    let resp = self
        .client
        .post(format!("{NOTION_API_BASE}/pages"))
        .headers(self.headers())
        .json(&json!({
            "parent": { "database_id": database_id },
            "properties": {
                "Name": { "title": [{ "text": { "content": truncate(title, 200) } }] },
                "Description": { "rich_text": [{ "text": { "content": description } }] }
            }
        }))
        .send()
        .await?;
    let body = self.handle(resp).await?;
    Ok(body.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string())
}

    pub async fn archive_page(&self, page_id: &str) -> Result<(), NotionError> {
        let resp = self
            .client
            .patch(format!("{NOTION_API_BASE}/pages/{page_id}"))
            .headers(self.headers())
            .json(&json!({ "archived": true }))
            .send()
            .await?;
        self.handle(resp).await.map(|_| ())
    }

    pub async fn list_child_databases(&self, hub_page_id: &str) -> Result<std::collections::HashMap<String, String>, NotionError> {
        let mut map = std::collections::HashMap::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut url = format!("{NOTION_API_BASE}/blocks/{hub_page_id}/children?page_size=50");
            if let Some(c) = &cursor {
                url.push_str(&format!("&start_cursor={c}"));
            }
            let resp = self.client.get(url).headers(self.headers()).send().await?;
            let body = self.handle(resp).await?;

            if let Some(results) = body.get("results").and_then(|r| r.as_array()) {
                for block in results {
                    if block.get("type").and_then(|t| t.as_str()) == Some("child_database") {
                        let id = block.get("id").and_then(|i| i.as_str());
                        let title = block.get("child_database").and_then(|cd| cd.get("title")).and_then(|t| t.as_str());
                        if let (Some(id_str), Some(title_str)) = (id, title) {
                            map.insert(title_str.to_string(), id_str.to_string());
                        }
                    }
                }
            }

            if body.get("has_more").and_then(|h| h.as_bool()).unwrap_or(false) {
                cursor = body.get("next_cursor").and_then(|n| n.as_str()).map(str::to_string);
            } else {
                break;
            }
        }

        Ok(map)
    }

    pub async fn add_calendar_row(
        &self,
        database_id: &str,
        text: &str,
        date: Option<&str>,
        platform: Option<&str>,
        content_type: Option<&str>,
    ) -> Result<String, NotionError> {
        let mut properties = serde_json::Map::new();
        properties.insert("Name".to_string(), json!({ "title": [{ "text": { "content": truncate(text, 200) } }] }));
        properties.insert("Status".to_string(), json!({ "select": { "name": "Pushed" } }));
        if let Some(d) = date {
            properties.insert("Date".to_string(), json!({ "date": { "start": d } }));
        }
        if let Some(p) = platform {
            properties.insert("Platform".to_string(), json!({ "rich_text": [{ "text": { "content": p } }] }));
        }
        if let Some(ct) = content_type {
            // Matches the options declared in create_database: Headline/
            // Subline/Quote/Tip.
            let label = match ct {
                "headline" => "Headline",
                "subline" => "Subline",
                "quote" => "Quote",
                "tip" => "Tip",
                other => other,
            };
            properties.insert("Content Type".to_string(), json!({ "select": { "name": label } }));
        }

        let resp = self
            .client
            .post(format!("{NOTION_API_BASE}/pages"))
            .headers(self.headers())
            .json(&json!({ "parent": { "database_id": database_id }, "properties": properties }))
            .send()
            .await?;
        let body = self.handle(resp).await?;
        Ok(body.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max.min(s.len())]) }
}
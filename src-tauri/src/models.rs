use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSettings {
    pub notion_connected: bool,
    pub notion_parent_page_id: Option<String>,
    pub ai_provider: String, // "local" | "byo"
    pub selected_model_id: String,
    pub byo_model: String,
    pub byo_key_set: bool,
    /// Whether the user has saved their own Unsplash API key in Settings.
    /// If false (and no build-time key was baked in), business pages
    /// simply get no cover image instead of a random stock photo.
    pub unsplash_key_set: bool,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            notion_connected: false,
            notion_parent_page_id: None,
            ai_provider: "local".to_string(),
            selected_model_id: crate::model_registry::recommended_local_model_id(),
            byo_model: "deepseek/deepseek-v4-flash".to_string(),
            byo_key_set: false,
            unsplash_key_set: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub ai_provider: Option<String>,
    pub selected_model_id: Option<String>,
    pub byo_model: Option<String>,
    pub byo_api_key: Option<String>,
    pub unsplash_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub prompt_template: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    #[serde(rename = "type")]
    pub item_type: String,
    /// Short hook/headline for the item. Populated for headline/subline/
    /// quote/tip items; calendar items reuse the title of the source item
    /// they were derived from. None only if a model response omitted it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The body/description text — the longer, ready-to-publish content.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// For "calendar" items only: which type of content ("headline" /
    /// "quote" / "tip") this calendar entry was derived from, so it can be
    /// tagged with the right "Content Type" select value in Notion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// A business/client the user has generated a Notion content hub for.
/// One child page under the workspace's root parent page per business —
/// this is what lets us (a) recognize and reuse the same page across
/// prompts instead of creating duplicates, and (b) detect when the user
/// has deleted that page in Notion (404) and transparently recreate it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Business {
    pub id: String,
    pub name: String,
    pub context: String,
    pub hub_page_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBatch {
    pub id: String,
    pub created_at: String,
    pub status: String,
    pub items: Vec<ContentItem>,
    /// The real reason the push to Notion failed, if it did — replaces
    /// guessing at a cause on the frontend. None when the push succeeded
    /// (or was never attempted because Notion isn't connected at all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_error: Option<String>,
}
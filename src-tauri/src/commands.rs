use crate::ai::{AiError, OpenAiCompatibleProvider};
use crate::db::{self, Db};
use crate::models::{Business, ContentBatch, ContentItem, Preset, SettingsPatch, WorkspaceSettings};
use crate::notion::{NotionClient, TEMPLATE_DATABASES};
use chrono::{Duration, Local, Utc};
use keyring::Entry;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "content-prompter";
const NOTION_TOKEN_KEY: &str = "notion_token";
const BYO_API_KEY: &str = "byo_api_key";

fn built_in_openrouter_key() -> String {
    option_env!("BUILT_IN_OPENROUTER_KEY").unwrap_or("").to_string()
}

fn keyring_get(key: &str) -> Option<String> {
    Entry::new(KEYRING_SERVICE, key).ok()?.get_password().ok()
}

fn keyring_set(key: &str, value: &str) -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, key).map_err(|e| e.to_string())?.set_password(value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(db: State<Db>) -> WorkspaceSettings {
    let conn = db.inner().0.lock().unwrap();
    db::load_settings(&conn)
}

#[tauri::command]
pub fn save_settings(db: State<Db>, patch: SettingsPatch) -> Result<WorkspaceSettings, String> {
    let conn = db.inner().0.lock().unwrap();
    if let Some(v) = &patch.ai_provider {
        db::set_setting(&conn, "ai_provider", v).map_err(|e| e.to_string())?;
    }
    if let Some(v) = &patch.selected_model_id {
        db::set_setting(&conn, "selected_model_id", v).map_err(|e| e.to_string())?;
    }
    if let Some(v) = &patch.byo_model {
        db::set_setting(&conn, "byo_model", v).map_err(|e| e.to_string())?;
    }
    if let Some(v) = &patch.byo_api_key {
        if !v.is_empty() {
            keyring_set(BYO_API_KEY, v)?;
            db::set_setting(&conn, "byo_key_set", "true").map_err(|e| e.to_string())?;
        }
    }
    Ok(db::load_settings(&conn))
}

#[tauri::command]
pub async fn get_available_models() -> Vec<crate::model_registry::ModelInfo> {
    let status = crate::ollama::check_status().await;
    crate::model_registry::list_models(&status)
}

#[tauri::command]
pub async fn get_ollama_status() -> crate::ollama::OllamaStatus {
    crate::ollama::check_status().await
}

#[tauri::command]
pub fn pull_ollama_model(model_id: String) -> Result<serde_json::Value, String> {
    crate::ollama::pull_model(&model_id)?;
    Ok(serde_json::json!({ "ok": true, "message": format!("Downloading \"{model_id}\" in the background — check back in a bit.") }))
}

#[tauri::command]
pub async fn connect_notion(db: State<'_, Db>, integration_token: String) -> Result<serde_json::Value, String> {
    let client = NotionClient::new(integration_token.clone());
    client.verify_token().await.map_err(|e| format!("Couldn't verify that token with Notion: {e}"))?;
    keyring_set(NOTION_TOKEN_KEY, &integration_token)?;
    let conn = db.inner().0.lock().unwrap();
    db::set_setting(&conn, "notion_connected", "true").map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "message": "Connected." }))
}

#[tauri::command]
pub async fn test_notion_connection(_db: State<'_, Db>) -> Result<serde_json::Value, String> {
    let token = match keyring_get(NOTION_TOKEN_KEY) {
        Some(t) => t,
        None => return Ok(serde_json::json!({ "ok": false, "message": "Notion isn't connected yet." })),
    };
    let client = NotionClient::new(token);
    match client.verify_token().await {
        Ok(_) => Ok(serde_json::json!({ "ok": true, "message": "Notion connection is working." })),
        Err(e) => Ok(serde_json::json!({ "ok": false, "message": format!("Notion check failed: {e}") })),
    }
}

#[tauri::command]
pub async fn test_ai_connection(db: State<'_, Db>) -> Result<serde_json::Value, String> {
    let provider = match active_provider(db.inner()).await {
        Ok(p) => p,
        Err(e) => return Ok(serde_json::json!({ "ok": false, "message": e })),
    };
    match provider.ping().await {
        Ok(reply) => Ok(serde_json::json!({ "ok": true, "message": format!("AI agent responded ({}): \"{}\"", provider.model, reply.trim()) })),
        Err(e) => Ok(serde_json::json!({ "ok": false, "message": format!("AI check failed: {e}") })),
    }
}

#[tauri::command]
pub fn get_presets(db: State<Db>) -> Result<Vec<Preset>, String> {
    let conn = db.inner().0.lock().unwrap();
    db::list_presets(&conn).map_err(|e| e.to_string())
}

fn resolve_local_model(settings: &WorkspaceSettings, status: &crate::ollama::OllamaStatus) -> Result<String, String> {
    let requested = settings.selected_model_id.trim();

    if requested.is_empty() {
        let recommended = crate::model_registry::recommended_local_model_id();
        return Ok(recommended);
    }

    let is_legacy_heavy = requested == crate::model_registry::LEGACY_HEAVY_MODEL_ID
        || requested.starts_with("deepseek-coder-v2")
        || requested.starts_with("deepseek-r1:14b")
        || requested.starts_with("qwen2.5:14b");

    if is_legacy_heavy {
        let fallback = crate::model_registry::recommended_local_model_id();
        if status.pulled_models.iter().any(|m| m == &fallback) {
            return Ok(fallback);
        }
    }

    if status.pulled_models.iter().any(|m| m == requested) {
        return Ok(requested.to_string());
    }

    Err(format!(
        "The selected local model \"{requested}\" isn't downloaded yet. Open Settings → AI agent and download it, or choose a different downloaded model."
    ))
}

async fn active_provider(db: &Db) -> Result<OpenAiCompatibleProvider, String> {
    let settings = {
        let conn = db.0.lock().unwrap();
        db::load_settings(&conn)
    };

    match settings.ai_provider.as_str() {
        "byo" => {
            let key = keyring_get(BYO_API_KEY).ok_or("No API key saved yet — add one in Settings.".to_string())?;
            Ok(OpenAiCompatibleProvider { base_url: "https://openrouter.ai/api/v1".to_string(), api_key: key, model: settings.byo_model })
        }
        "cloud" => {
            let key = built_in_openrouter_key();
            if key.is_empty() {
                return Err("Built-in cloud AI key isn't configured in this build. Use a local model, or your own key in Settings.".to_string());
            }
            Ok(OpenAiCompatibleProvider { base_url: "https://openrouter.ai/api/v1".to_string(), api_key: key, model: settings.selected_model_id })
        }
        _ => {
            let status = crate::ollama::check_status().await;
            if !status.installed {
                return Err("Ollama isn't installed. Install it from https://ollama.com/download, then pick a model in Settings → AI agent.".to_string());
            }

            let model = resolve_local_model(&settings, &status)?;

            Ok(OpenAiCompatibleProvider {
                base_url: format!("{}/v1", crate::ollama::OLLAMA_BASE_URL),
                api_key: crate::ollama::OLLAMA_DUMMY_KEY.to_string(),
                model,
            })
        }
    }
}

async fn generate_with_safe_fallback(provider: &OpenAiCompatibleProvider, prompt: &str) -> Result<Vec<ContentItem>, AiError> {
    match provider.generate(prompt).await {
        Ok(items) => Ok(items),
        Err(err) if err.should_retry_with_safe_model() => {
            let fallback_model = crate::model_registry::recommended_local_model_id();
            let fallback_provider = OpenAiCompatibleProvider {
                base_url: provider.base_url.clone(),
                api_key: provider.api_key.clone(),
                model: fallback_model,
            };
            fallback_provider.generate(prompt).await
        }
        Err(err) => Err(err),
    }
}

fn new_batch(items: Vec<ContentItem>) -> ContentBatch {
    ContentBatch { id: Uuid::new_v4().to_string(), created_at: Utc::now().to_rfc3339(), status: "draft".to_string(), items, push_error: None }
}

/// Rebuilds calendar entries deterministically instead of trusting the
/// model to get counts and dates right. Given how many days were asked
/// for and the exact per-day headline/quote/tip counts, this buckets the
/// model's headline/quote/tip items into days in order and emits one
/// calendar entry per item, dated to its day — so a "5 days × 5
/// headlines/5 quotes/5 tips" request always produces a calendar with
/// exactly that many entries, spread exactly that way across days,
/// regardless of what the model itself produced for a "calendar" section
/// (which is ignored / never requested from the model at all now).
fn normalize_calendar_entries(items: &mut Vec<ContentItem>, days: usize, headlines_per_day: usize, quotes_per_day: usize, tips_per_day: usize) {
    items.retain(|i| i.item_type != "calendar");

    let headlines: Vec<ContentItem> = items.iter().filter(|i| i.item_type == "headline").cloned().collect();
    let quotes: Vec<ContentItem> = items.iter().filter(|i| i.item_type == "quote").cloned().collect();
    let tips: Vec<ContentItem> = items.iter().filter(|i| i.item_type == "tip").cloned().collect();

    if headlines.is_empty() && quotes.is_empty() && tips.is_empty() {
        return;
    }

    let today = Local::now().date_naive();
    let days = days.max(1);
    let mut calendar_entries = Vec::new();

    let mut bucket = |source: &[ContentItem], per_day: usize, content_type: &str| {
        if per_day == 0 {
            return;
        }
        for day in 0..days {
            let date = (today + Duration::days(day as i64)).format("%Y-%m-%d").to_string();
            let start = day * per_day;
            let end = ((day + 1) * per_day).min(source.len());
            if start >= source.len() {
                break;
            }
            for item in &source[start..end] {
                calendar_entries.push(ContentItem {
                    item_type: "calendar".to_string(),
                    title: item.title.clone(),
                    text: item.text.clone(),
                    date: Some(date.clone()),
                    platform: None,
                    content_type: Some(content_type.to_string()),
                });
            }
        }
    };

    bucket(&headlines, headlines_per_day, "headline");
    bucket(&quotes, quotes_per_day, "quote");
    bucket(&tips, tips_per_day, "tip");

    items.extend(calendar_entries);
}

/// Resolves the workspace's ROOT parent page — the single page the user
/// manually shared with the integration in Notion. Every business hub page
/// is created as a *child* of this page, one child page per business, so
/// businesses are distinct, recognizable pages rather than one shared bucket.
///
/// Self-healing: if the cached root page ID no longer exists (the user
/// deleted/unshared it — the exact "somehow I keep getting 404 after I
/// delete a page" symptom), the stale ID is cleared and re-discovered
/// instead of failing forever.
async fn ensure_root_parent(db: &Db, client: &NotionClient) -> Result<String, String> {
    let existing = {
        let conn = db.0.lock().unwrap();
        db::load_settings(&conn).notion_parent_page_id
    };

    if let Some(id) = existing {
        match client.page_exists(&id).await {
            Ok(true) => return Ok(id),
            Ok(false) => {
                // Stale — the page was deleted or unshared. Clear it and
                // fall through to re-discovery below.
                let conn = db.0.lock().unwrap();
                db::delete_setting(&conn, "notion_parent_page_id").map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    let parent_page_id = client
        .find_accessible_page()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No shared page found. In Notion, share a page with your Content Bot integration first.".to_string())?;

    let conn = db.0.lock().unwrap();
    db::set_setting(&conn, "notion_parent_page_id", &parent_page_id).map_err(|e| e.to_string())?;
    Ok(parent_page_id)
}

/// Finds or creates the per-business hub page (a child page of the root
/// parent) and makes sure its five template databases exist. Reused across
/// prompts for the same business name — and self-heals if the user deleted
/// the hub page (or an individual database inside it) in Notion, instead of
/// surfacing a permanent 404.
async fn ensure_business_hub(
    db: &Db,
    client: &NotionClient,
    business_name: &str,
    business_context: &str,
) -> Result<(Business, TemplateDatabaseIds), String> {
    let root_parent_id = ensure_root_parent(db, client).await?;

    let existing_business = {
        let conn = db.0.lock().unwrap();
        db::get_business_by_name(&conn, business_name)
    };

    let hub_id = if let Some(b) = &existing_business {
        match client.page_exists(&b.hub_page_id).await {
            Ok(true) => Some(b.hub_page_id.clone()),
            Ok(false) => None, // deleted in Notion — recreate below
            Err(e) => return Err(e.to_string()),
        }
    } else {
        None
    };

    let (hub_id, is_new) = match hub_id {
        Some(id) => (id, false),
        None => {
            let cover_query = if business_context.trim().is_empty() {
                format!("{business_name} business")
            } else {
                format!("{business_name} {business_context}")
            };
            let cover_url = crate::unsplash::fetch_cover_image(&cover_query).await;
            let id = client
                .create_content_hub(&root_parent_id, business_name, Some(business_context), cover_url.as_deref())
                .await
                .map_err(|e| e.to_string())?;
            (id, true)
        }
    };

    let business = Business {
        id: existing_business.as_ref().map(|b| b.id.clone()).unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: business_name.to_string(),
        context: business_context.to_string(),
        hub_page_id: hub_id.clone(),
        created_at: existing_business.map(|b| b.created_at).unwrap_or_else(|| Utc::now().to_rfc3339()),
    };
    {
        let conn = db.0.lock().unwrap();
        db::upsert_business(&conn, &business).map_err(|e| e.to_string())?;
    }

    let db_ids = if is_new {
        let mut ids = std::collections::HashMap::new();
        for db_title in TEMPLATE_DATABASES.iter() {
            let is_calendar = *db_title == "Content Calendar";
            let id = client.create_database(&hub_id, db_title, is_calendar).await.map_err(|e| e.to_string())?;
            ids.insert(db_title.to_string(), id);
        }
        template_ids_from_map(ids)?
    } else {
        resolve_template_database_ids(client, &hub_id).await?
    };

    Ok((business, db_ids))
}

struct TemplateDatabaseIds { headlines: String, sublines: String, quotes: String, tips: String, calendar: String }

fn template_ids_from_map(map: std::collections::HashMap<String, String>) -> Result<TemplateDatabaseIds, String> {
    let get = |title: &str| -> Result<String, String> {
        map.get(title).cloned().ok_or(format!("Couldn't find the \"{title}\" database in Notion."))
    };
    Ok(TemplateDatabaseIds { headlines: get("Headlines")?, sublines: get("Sublines")?, quotes: get("Quotes")?, tips: get("Useful Tips")?, calendar: get("Content Calendar")? })
}

/// Recreates any of the five template databases that are missing under the
/// hub page (e.g. the user deleted just one database, not the whole page),
/// then returns the full, current set of IDs.
async fn resolve_template_database_ids(client: &NotionClient, hub_id: &str) -> Result<TemplateDatabaseIds, String> {
    let mut map = client.list_child_databases(hub_id).await.map_err(|e| e.to_string())?;

    for db_title in TEMPLATE_DATABASES.iter() {
        if !map.contains_key(*db_title) {
            let is_calendar = *db_title == "Content Calendar";
            let id = client.create_database(hub_id, db_title, is_calendar).await.map_err(|e| e.to_string())?;
            map.insert(db_title.to_string(), id);
        }
    }

    template_ids_from_map(map)
}

/// How to bucket generated items into the calendar. `days` × each
/// per-day count is the exact expected total for that content type.
#[derive(Clone, Copy)]
struct CalendarShape { days: usize, headlines_per_day: usize, quotes_per_day: usize, tips_per_day: usize }

impl Default for CalendarShape {
    // Free-form (non-preset) prompts: everything lands on a single day,
    // sized to however many items actually came back — same behavior as
    // before per-day counts existed.
    fn default() -> Self {
        Self { days: 1, headlines_per_day: usize::MAX, quotes_per_day: usize::MAX, tips_per_day: usize::MAX }
    }
}

async fn execute_and_push(
    db: &Db,
    prompt: String,
    business_name: String,
    business_context: String,
    shape: CalendarShape,
) -> Result<ContentBatch, String> {
    let provider = active_provider(db).await?;
    let mut items = generate_with_safe_fallback(&provider, &prompt).await.map_err(|e| e.to_string())?;

    normalize_calendar_entries(&mut items, shape.days, shape.headlines_per_day, shape.quotes_per_day, shape.tips_per_day);

    let mut batch = new_batch(items);

    // Every failure branch below sets push_error to the REAL reason instead
    // of being silently thrown away — this is what used to make "Notion
    // unavailable" show up even when Notion was demonstrably connected.
    let push_error: Option<String> = match keyring_get(NOTION_TOKEN_KEY) {
        None => Some("Notion isn't connected. Go to Settings and connect it, then try again.".to_string()),
        Some(token) => {
            let client = NotionClient::new(token);
            match ensure_business_hub(db, &client, &business_name, &business_context).await {
                Err(e) => Some(format!("Couldn't set up the Notion page for \"{business_name}\": {e}")),
                Ok((_business, db_ids)) => {
                    let mut page_ids = Vec::new();
                    let mut first_row_error: Option<String> = None;

                    for item in &batch.items {
                        let title = item.title.as_deref().unwrap_or(&item.text);
                        let result = match item.item_type.as_str() {
                            "headline" => client.add_database_row(&db_ids.headlines, title, &item.text).await,
                            "subline" => client.add_database_row(&db_ids.sublines, &item.text, &item.text).await,
                            "quote" => client.add_database_row(&db_ids.quotes, title, &item.text).await,
                            "tip" => client.add_database_row(&db_ids.tips, title, &item.text).await,
                            "calendar" => {
                                client
                                    .add_calendar_row(&db_ids.calendar, title, item.date.as_deref(), item.platform.as_deref(), item.content_type.as_deref())
                                    .await
                            }
                            _ => continue,
                        };
                        match result {
                            Ok(id) => page_ids.push(id),
                            Err(e) => {
                                if first_row_error.is_none() {
                                    first_row_error = Some(e.to_string());
                                }
                            }
                        }
                    }

                    if page_ids.is_empty() && first_row_error.is_some() {
                        first_row_error.map(|e| format!("Notion rejected every item: {e}"))
                    } else {
                        batch.status = "pushed".to_string();
                        let conn = db.0.lock().unwrap();
                        db::save_pushed_pages(&conn, &batch.id, &page_ids).map_err(|e| e.to_string())?;
                        // Partial success: some rows landed, some didn't — still worth surfacing.
                        first_row_error.map(|e| format!("Most items were pushed, but some failed: {e}"))
                    }
                }
            }
        }
    };

    batch.push_error = push_error;

    let conn = db.0.lock().unwrap();
    db::save_batch(&conn, &batch).map_err(|e| e.to_string())?;
    Ok(batch)
}

#[tauri::command]
pub async fn generate_content(db: State<'_, Db>, prompt: String) -> Result<ContentBatch, String> {
    execute_and_push(db.inner(), prompt, "General".to_string(), String::new(), CalendarShape::default()).await
}

#[tauri::command]
pub async fn run_preset(db: State<'_, Db>, preset_id: String, field_values: HashMap<String, String>) -> Result<ContentBatch, String> {
    let template = {
        let conn = db.inner().0.lock().unwrap();
        let presets = db::list_presets(&conn).map_err(|e| e.to_string())?;
        presets.into_iter().find(|p| p.id == preset_id).ok_or("Unknown preset.".to_string())?.prompt_template
    };

    let business_name = field_values.get("Business Name").cloned().unwrap_or_else(|| "the business".to_string());
    let business_context = field_values.get("Business Context").cloned().unwrap_or_default();
    let amount = field_values.get("Amount of content generation").cloned().unwrap_or_else(|| "10".to_string());
    let headlines = field_values.get("Headlines").cloned().unwrap_or_else(|| "5".to_string());
    let quotes = field_values.get("Quotes").cloned().unwrap_or_else(|| "5".to_string());
    let tips = field_values.get("Tips").cloned().unwrap_or_else(|| "5".to_string());

    // "Amount of content" on the Business preset is DAYS worth of content —
    // headlines/quotes/tips fields are the exact per-day counts, so total
    // generated = days × each.
    let days: usize = amount.trim().parse().unwrap_or(1).max(1);
    let headlines_per_day: usize = headlines.trim().parse().unwrap_or(5);
    let quotes_per_day: usize = quotes.trim().parse().unwrap_or(5);
    let tips_per_day: usize = tips.trim().parse().unwrap_or(5);

    let business_context_clause = if business_context.trim().is_empty() {
        String::new()
    } else {
        format!("Business context — what this business is about: {}. ", business_context.trim())
    };

    let prompt = template
        .replace("{business_name}", &business_name)
        .replace("{business_context}", &business_context_clause)
        .replace("{amount}", &days.to_string())
        .replace("{headlines}", &headlines_per_day.to_string())
        .replace("{quotes}", &quotes_per_day.to_string())
        .replace("{tips}", &tips_per_day.to_string());

    let shape = CalendarShape { days, headlines_per_day, quotes_per_day, tips_per_day };
    execute_and_push(db.inner(), prompt, business_name, business_context, shape).await
}

/// Lists all businesses/child pages the app knows about, for the "edit an
/// existing business page" dropdown.
#[tauri::command]
pub fn list_businesses(db: State<Db>) -> Result<Vec<Business>, String> {
    let conn = db.inner().0.lock().unwrap();
    db::list_businesses(&conn).map_err(|e| e.to_string())
}

/// Prompts the AI scoped to a single, already-existing business page —
/// this never creates a new business/page. If the business was removed
/// from the app's local list, this fails rather than silently creating a
/// new one, so a selection always maps to exactly one page.
#[tauri::command]
pub async fn edit_business(db: State<'_, Db>, business_id: String, prompt: String) -> Result<ContentBatch, String> {
    let business = {
        let conn = db.inner().0.lock().unwrap();
        db::get_business(&conn, &business_id).ok_or("That business page is no longer in the list — refresh and pick again.".to_string())?
    };

    let scoped_prompt = format!(
        "This request is ONLY about the existing business \"{}\". Business context: {}. Instruction: {}",
        business.name,
        if business.context.trim().is_empty() { "(none given)" } else { business.context.trim() },
        prompt
    );

    execute_and_push(db.inner(), scoped_prompt, business.name, business.context, CalendarShape::default()).await
}

#[tauri::command]
pub async fn discard_notion_push(db: State<'_, Db>, batch_id: String) -> Result<serde_json::Value, String> {
    let token = keyring_get(NOTION_TOKEN_KEY).ok_or("Notion isn't connected.".to_string())?;
    let page_ids = {
        let conn = db.inner().0.lock().unwrap();
        db::get_pushed_pages(&conn, &batch_id).map_err(|e| e.to_string())?
    };

    let client = NotionClient::new(token);
    for page_id in &page_ids {
        let _ = client.archive_page(page_id).await;
    }

    let conn = db.inner().0.lock().unwrap();
    db::clear_pushed_pages(&conn, &batch_id).map_err(|e| e.to_string())?;
    if let Some(mut batch) = db::get_batch(&conn, &batch_id) {
        batch.status = "draft".to_string();
        db::save_batch(&conn, &batch).map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({ "ok": true, "message": "Removed from Notion." }))
}

#[tauri::command]
pub fn list_batches(db: State<Db>) -> Result<Vec<ContentBatch>, String> {
    let conn = db.inner().0.lock().unwrap();
    db::list_batches(&conn).map_err(|e| e.to_string())
}
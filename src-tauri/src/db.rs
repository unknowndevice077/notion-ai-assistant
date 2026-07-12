use crate::models::{Business, ContentBatch, Preset, WorkspaceSettings};
use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

// Plain sized array — no `&[...]` outer type, so no unsizing coercion is
// needed anywhere, and each field-list literal gets an explicit
// `as &[&str]` cast, since Rust never coerces array-to-slice through a
// tuple field automatically.
const BUILT_IN_PRESETS: [(&str, &str, &str, &[&str]); 1] = [(
    "business",
    "Business",
    "Create {amount} DAYS worth of content for {business_name}, one calendar day at a time starting \
     from {start_date} (day 1 = {start_date}, the next day is day 2, and so on through day {amount}). \
     {business_context}For EACH of the \
     {amount} days, you MUST include EXACTLY {headlines} headline items, EXACTLY {quotes} quote \
     items, and EXACTLY {tips} tip items for that day — no more, no fewer per day. That means the \
     total count across all days is {amount} × {headlines} headlines, {amount} × {quotes} quotes, \
     and {amount} × {tips} tips. Every headline, quote, and tip item MUST have both a short \
     \"title\" (a hook, 3-8 words) and a longer \"text\" (a ready-to-publish description/body), \
     and both must be directly relevant to {business_name} and the business context given above — \
     do not write generic filler. Also include a matching subline for each headline (subline items \
     only need \"text\", no title needed). Do not include calendar items — those are generated \
     separately. Do not stop early, do not summarize, and do not omit any items even if the full \
     list is long.",
    &["Business Name", "Business Context", "Start Date", "Amount of content generation", "Headlines", "Quotes", "Tips"] as &[&str],
)];

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS presets (
            id             TEXT PRIMARY KEY,
            label          TEXT NOT NULL,
            prompt_template TEXT NOT NULL,
            fields_json    TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS content_batches (
            id         TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            status     TEXT NOT NULL,
            items_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pushed_pages (
            batch_id TEXT NOT NULL,
            page_id  TEXT NOT NULL
        );

        -- One row per business/client content hub (child page under the
        -- workspace's root parent page in Notion). Looked up by name so
        -- repeat prompts for the same business land on the same page
        -- instead of creating duplicates, and so we can detect + recreate
        -- a page the user deleted out from under us in Notion.
        CREATE TABLE IF NOT EXISTS businesses (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL UNIQUE,
            context      TEXT NOT NULL DEFAULT '',
            hub_page_id  TEXT NOT NULL,
            created_at   TEXT NOT NULL
        );
        ",
    )?;

    conn.execute("DELETE FROM presets", [])?;

    for (id, label, template, fields) in BUILT_IN_PRESETS.iter() {
        let fields_json = serde_json::to_string(fields).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO presets (id, label, prompt_template, fields_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, label, template, fields_json],
        )?;
    }

    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| row.get(0))
        .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}

pub fn load_settings(conn: &Connection) -> WorkspaceSettings {
    let mut s = WorkspaceSettings::default();
    if let Some(v) = get_setting(conn, "notion_connected") {
        s.notion_connected = v == "true";
    }
    s.notion_parent_page_id = get_setting(conn, "notion_parent_page_id");
    if let Some(v) = get_setting(conn, "ai_provider") {
        s.ai_provider = v;
    }
    if let Some(v) = get_setting(conn, "selected_model_id") {
        s.selected_model_id = v;
    }
    if let Some(v) = get_setting(conn, "byo_model") {
        s.byo_model = v;
    }
    if let Some(v) = get_setting(conn, "byo_key_set") {
        s.byo_key_set = v == "true";
    }
    s
}

pub fn list_presets(conn: &Connection) -> rusqlite::Result<Vec<Preset>> {
    let mut stmt = conn.prepare("SELECT id, label, prompt_template, fields_json FROM presets")?;
    let rows = stmt.query_map([], |row| {
        let fields_json: String = row.get(3)?;
        let fields: Vec<String> = serde_json::from_str(&fields_json).unwrap_or_default();
        Ok(Preset {
            id: row.get(0)?,
            label: row.get(1)?,
            prompt_template: row.get(2)?,
            fields,
        })
    })?;
    rows.collect()
}

pub fn save_batch(conn: &Connection, batch: &ContentBatch) -> rusqlite::Result<()> {
    let items_json = serde_json::to_string(&batch.items).unwrap();
    conn.execute(
        "INSERT INTO content_batches (id, created_at, status, items_json)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET status = excluded.status, items_json = excluded.items_json",
        params![batch.id, batch.created_at, batch.status, items_json],
    )?;
    Ok(())
}

pub fn get_batch(conn: &Connection, id: &str) -> Option<ContentBatch> {
    conn.query_row(
        "SELECT id, created_at, status, items_json FROM content_batches WHERE id = ?1",
        params![id],
        |row| {
            let items_json: String = row.get(3)?;
            let items = serde_json::from_str(&items_json).unwrap_or_default();
            Ok(ContentBatch {
                id: row.get(0)?,
                created_at: row.get(1)?,
                status: row.get(2)?,
                items,
                push_error: None,
            })
        },
    )
    .ok()
}

pub fn list_batches(conn: &Connection) -> rusqlite::Result<Vec<ContentBatch>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, status, items_json FROM content_batches ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let items_json: String = row.get(3)?;
        let items = serde_json::from_str(&items_json).unwrap_or_default();
        Ok(ContentBatch {
            id: row.get(0)?,
            created_at: row.get(1)?,
            status: row.get(2)?,
            items,
            push_error: None,
        })
    })?;
    rows.collect()
}

pub fn save_pushed_pages(conn: &Connection, batch_id: &str, page_ids: &[String]) -> rusqlite::Result<()> {
    for page_id in page_ids {
        conn.execute(
            "INSERT INTO pushed_pages (batch_id, page_id) VALUES (?1, ?2)",
            params![batch_id, page_id],
        )?;
    }
    Ok(())
}

pub fn get_pushed_pages(conn: &Connection, batch_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT page_id FROM pushed_pages WHERE batch_id = ?1")?;
    let rows = stmt.query_map(params![batch_id], |row| row.get(0))?;
    rows.collect()
}

pub fn clear_pushed_pages(conn: &Connection, batch_id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM pushed_pages WHERE batch_id = ?1", params![batch_id])?;
    Ok(())
}

/// Case-insensitive lookup — "Acme Studio" and "acme studio" should be the
/// same business, not two duplicate Notion pages.
pub fn get_business_by_name(conn: &Connection, name: &str) -> Option<Business> {
    conn.query_row(
        "SELECT id, name, context, hub_page_id, created_at FROM businesses WHERE lower(name) = lower(?1)",
        params![name],
        |row| {
            Ok(Business {
                id: row.get(0)?,
                name: row.get(1)?,
                context: row.get(2)?,
                hub_page_id: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .ok()
}

pub fn get_business(conn: &Connection, id: &str) -> Option<Business> {
    conn.query_row(
        "SELECT id, name, context, hub_page_id, created_at FROM businesses WHERE id = ?1",
        params![id],
        |row| {
            Ok(Business {
                id: row.get(0)?,
                name: row.get(1)?,
                context: row.get(2)?,
                hub_page_id: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .ok()
}

pub fn list_businesses(conn: &Connection) -> rusqlite::Result<Vec<Business>> {
    let mut stmt = conn.prepare("SELECT id, name, context, hub_page_id, created_at FROM businesses ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Business {
            id: row.get(0)?,
            name: row.get(1)?,
            context: row.get(2)?,
            hub_page_id: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

/// Insert a brand-new business row, or — if one already exists under this
/// name — update its hub_page_id (used when we detect the old page was
/// deleted in Notion and recreate it) and refresh the context.
pub fn upsert_business(conn: &Connection, business: &Business) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO businesses (id, name, context, hub_page_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(name) DO UPDATE SET hub_page_id = excluded.hub_page_id, context = excluded.context",
        params![business.id, business.name, business.context, business.hub_page_id, business.created_at],
    )?;
    Ok(())
}

pub fn delete_business(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM businesses WHERE id = ?1", params![id])?;
    Ok(())
}
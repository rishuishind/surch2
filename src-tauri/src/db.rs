use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::PathBuf;
use crate::models::SearchResultItem;

fn get_db_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    path.push("surch2");
    fs::create_dir_all(&path).unwrap_or_default();
    path.push("data.db");
    path
}

pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path();
    let conn = Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY,
            content TEXT NOT NULL UNIQUE,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Create an index on timestamp for fast sorting
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp ON clipboard_history(timestamp DESC)",
        [],
    )?;

    Ok(conn)
}

pub fn add_clipboard_entry(content: &str) -> Result<()> {
    // Basic validation to avoid inserting tiny or irrelevant things
    if content.trim().is_empty() {
        return Ok(());
    }

    let conn = Connection::open(get_db_path())?;
    
    // Insert, or update timestamp if it already exists (UNIQUE constraint on content)
    conn.execute(
        "INSERT INTO clipboard_history (content, timestamp) 
         VALUES (?1, CURRENT_TIMESTAMP) 
         ON CONFLICT(content) DO UPDATE SET timestamp = CURRENT_TIMESTAMP",
        params![content],
    )?;

    Ok(())
}

pub fn get_clipboard_history(query: &str, limit: usize) -> Result<Vec<SearchResultItem>> {
    let conn = Connection::open(get_db_path())?;
    let limit_i64 = limit as i64;
    let mut results = Vec::new();
    
    if query.trim().is_empty() {
        let mut stmt = conn.prepare(
            "SELECT id, content FROM clipboard_history 
             ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let items_iter = stmt.query_map(params![limit_i64], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((id, content))
        })?;
        for item in items_iter {
            if let Ok((id, content)) = item {
                results.push(make_clipboard_item(id, content));
            }
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, content FROM clipboard_history 
             WHERE content LIKE ?1 
             ORDER BY timestamp DESC LIMIT ?2"
        )?;
        let like_query = format!("%{}%", query);
        let items_iter = stmt.query_map(params![like_query, limit_i64], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((id, content))
        })?;
        for item in items_iter {
            if let Ok((id, content)) = item {
                results.push(make_clipboard_item(id, content));
            }
        }
    }

    Ok(results)
}

fn make_clipboard_item(id: i64, content: String) -> SearchResultItem {
    let mut title = content.clone();
    title.retain(|c| c != '\n' && c != '\r');
    if title.len() > 60 {
        title.truncate(60);
        title.push_str("...");
    }

    SearchResultItem {
        id: format!("cb-{}", id),
        title,
        subtitle: Some("Clipboard History".to_string()),
        icon: None,
        icon_data: None, // Could add a clipboard SVG here
        item_type: "clipboard".to_string(),
        action_data: content, // The full content to be pasted
        score: 0,
    }
}

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

    // Snippets table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snippets (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            keyword TEXT UNIQUE
        )",
        [],
    )?;

    // Seed some example snippets if the table is empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM snippets", [], |row| row.get(0))?;
    if count == 0 {
        let default_snippets = vec![
            ("My Email", "hello@example.com", "@@mail"),
            ("Shrug", "¯\\_(ツ)_/¯", "@@shrug"),
            ("Meeting Link", "https://meet.google.com/abc-defg-hij", "@@meet"),
        ];
        
        for (title, content, keyword) in default_snippets {
            conn.execute(
                "INSERT INTO snippets (title, content, keyword) VALUES (?1, ?2, ?3)",
                params![title, content, keyword],
            )?;
        }
    }

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

pub fn get_all_snippets() -> Result<Vec<SearchResultItem>> {
    let conn = Connection::open(get_db_path())?;
    let mut stmt = conn.prepare("SELECT id, title, content, keyword FROM snippets")?;
    
    let items_iter = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        let keyword: Option<String> = row.get(3).ok(); // ok() because keyword is UNIQUE, could be NULL if we allowed it, but it's bound as TEXT. wait, it's TEXT UNIQUE.
        Ok((id, title, content, keyword))
    })?;

    let mut results = Vec::new();
    for item in items_iter {
        if let Ok((id, title, content, keyword)) = item {
            let subtitle = if let Some(kw) = keyword {
                format!("Snippet • Keyword: {}", kw)
            } else {
                "Snippet".to_string()
            };

            results.push(SearchResultItem {
                id: format!("snp-{}", id),
                title,
                subtitle: Some(subtitle),
                icon: None,
                icon_data: None, // Frontend can render a snippet icon
                item_type: "snippet".to_string(),
                action_data: content,
                score: 0,
            });
        }
    }

    Ok(results)
}

pub fn insert_snippet(title: &str, content: &str, keyword: Option<&str>) -> Result<()> {
    let conn = Connection::open(get_db_path())?;
    conn.execute(
        "INSERT INTO snippets (title, content, keyword) VALUES (?1, ?2, ?3)",
        params![title, content, keyword],
    )?;
    Ok(())
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

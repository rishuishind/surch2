use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,      // Predefined icon name or CSS class
    pub icon_data: Option<String>, // Base64 data if available
    pub item_type: String,         // "app", "system", "clipboard", "math"
    pub action_data: String,       // Data needed for execution (e.g. exec command, or clipboard text)
    #[serde(skip)]
    pub score: i64,                // Used internally for sorting
}

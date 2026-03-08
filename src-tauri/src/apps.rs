use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub icon_data: Option<String>,
    pub description: Option<String>,
    pub categories: Option<String>,
    pub desktop_file: String,
}

/// Directories to scan for .desktop files
fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    // User-local applications
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in data_dirs.split(':') {
            let app_dir = PathBuf::from(dir).join("applications");
            if !dirs.contains(&app_dir) {
                dirs.push(app_dir);
            }
        }
    }

    dirs
}

/// Parse a .desktop file into an AppEntry
fn parse_desktop_file(path: &PathBuf) -> Option<AppEntry> {
    // freedesktop_entry_parser::parse_entry takes a file path
    let entry = freedesktop_entry_parser::parse_entry(path.to_str()?).ok()?;

    let section = entry.section("Desktop Entry");

    let name = section.attr("Name")?.to_string();
    let exec_raw = section.attr("Exec")?.to_string();

    // Skip entries that are hidden or shouldn't show in menus
    if let Some(no_display) = section.attr("NoDisplay") {
        if no_display.to_lowercase() == "true" {
            return None;
        }
    }
    if let Some(hidden) = section.attr("Hidden") {
        if hidden.to_lowercase() == "true" {
            return None;
        }
    }

    // Only include "Application" type entries
    if let Some(entry_type) = section.attr("Type") {
        if entry_type != "Application" {
            return None;
        }
    }

    // Clean exec command: remove field codes like %f, %u, %F, %U, etc.
    let exec = exec_raw
        .replace("%f", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%U", "")
        .replace("%d", "")
        .replace("%D", "")
        .replace("%n", "")
        .replace("%N", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .trim()
        .to_string();

    let icon = section.attr("Icon").map(|s| s.to_string());
    let description = section.attr("Comment").map(|s| s.to_string());
    let categories = section.attr("Categories").map(|s| s.to_string());

    let icon_data = icon.as_ref().and_then(|icon_name| resolve_icon(icon_name));

    Some(AppEntry {
        name,
        exec,
        icon,
        icon_data,
        description,
        categories,
        desktop_file: path.to_string_lossy().to_string(),
    })
}

/// Resolve an icon name to a base64-encoded PNG data URL
fn resolve_icon(icon_name: &str) -> Option<String> {
    // If it's already an absolute path
    if icon_name.starts_with('/') {
        return read_icon_to_base64(icon_name);
    }

    // Search common icon theme directories
    let icon_dirs = vec![
        "/usr/share/icons/hicolor",
        "/usr/share/pixmaps",
        "/usr/share/icons",
    ];

    let sizes = vec![
        "48x48", "64x64", "128x128", "256x256", "scalable", "32x32", "24x24",
    ];
    let categories = vec!["apps", "mimetypes", "devices", "actions", "categories"];
    let extensions = vec!["png", "svg", "xpm"];

    for dir in &icon_dirs {
        for size in &sizes {
            for cat in &categories {
                for ext in &extensions {
                    let path = format!("{}/{}/{}/{}.{}", dir, size, cat, icon_name, ext);
                    if let Some(data) = read_icon_to_base64(&path) {
                        return Some(data);
                    }
                }
            }
        }
        // Also check pixmaps directly
        for ext in &extensions {
            let path = format!("{}/{}.{}", dir, icon_name, ext);
            if let Some(data) = read_icon_to_base64(&path) {
                return Some(data);
            }
        }
    }

    // Check snap icon paths
    let snap_icon = format!(
        "/snap/{}/current/meta/gui/icon.png",
        icon_name.replace("snap.", "").split('_').next().unwrap_or(icon_name)
    );
    if let Some(data) = read_icon_to_base64(&snap_icon) {
        return Some(data);
    }

    None
}

/// Read an icon file and convert to base64 data URL
fn read_icon_to_base64(path: &str) -> Option<String> {
    let data = fs::read(path).ok()?;
    let mime = if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".xpm") {
        "image/x-xpixmap"
    } else {
        "image/png"
    };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
    Some(format!("data:{};base64,{}", mime, b64))
}

/// Scan all .desktop files and return deduplicated AppEntry list
pub fn scan_applications() -> Vec<AppEntry> {
    let mut seen: HashMap<String, AppEntry> = HashMap::new();

    for dir in desktop_dirs() {
        if !dir.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "desktop") {
                    if let Some(app) = parse_desktop_file(&path) {
                        // Deduplicate by name (keep first occurrence)
                        seen.entry(app.name.clone()).or_insert(app);
                    }
                }
            }
        }
    }

    let mut apps: Vec<AppEntry> = seen.into_values().collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

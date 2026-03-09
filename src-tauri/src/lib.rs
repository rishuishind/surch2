mod apps;
mod db;
mod clipboard;
mod i3;
mod models;
mod system;

use apps::scan_applications;
use clipboard::start_clipboard_monitor;
use db::{get_all_snippets, get_clipboard_history, init_db, insert_snippet};
use i3::get_i3_items;
use models::SearchResultItem;
use system::get_system_commands;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;
use std::sync::Mutex;
use tauri::Manager;

/// Cached list of searchable items (apps, system commands, etc.)
struct AppState {
    items: Mutex<Vec<SearchResultItem>>,
}

#[tauri::command]
fn search_items(query: &str, state: tauri::State<'_, AppState>) -> Vec<SearchResultItem> {
    let items = state.items.lock().unwrap();

    if query.is_empty() {
        // Return first 20 items when no query
        return items.iter().take(20).cloned().collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, SearchResultItem)> = items
        .iter()
        .filter_map(|item| {
            let title_score = matcher.fuzzy_match(&item.title, query).unwrap_or(0);
            let subtitle_score = item
                .subtitle
                .as_ref()
                .and_then(|d| matcher.fuzzy_match(d, query))
                .unwrap_or(0);

            // Weight ranking: apps > snippets > system > i3
            let weight = if item.item_type == "app" {
                3
            } else if item.item_type == "snippet" {
                2
            } else {
                1
            };
            let total = title_score * weight + subtitle_score;
            
            if total > 0 {
                Some((total, item.clone()))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(20).map(|(_, item)| item).collect()
}

#[tauri::command]
fn search_clipboard_history(query: &str) -> Vec<SearchResultItem> {
    // Return up to 50 items for the dedicated clipboard view
    get_clipboard_history(query, 50).unwrap_or_default()
}

#[tauri::command]
fn execute_item(item: SearchResultItem) -> Result<(), String> {
    if item.item_type == "app" || item.item_type == "system" {
        // ... (existing launch logic)
        let parts: Vec<&str> = item.action_data.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        Command::new(parts[0])
            .args(&parts[1..])
            .spawn()
            .map_err(|e| format!("Failed to launch: {}", e))?;
    } else if item.item_type == "clipboard" || item.item_type == "snippet" {
        // Paste it!
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(item.action_data);
        }
    } else if item.item_type == "i3_command" {
        if let Ok(mut conn) = i3ipc::I3Connection::connect() {
            let _ = conn.run_command(&item.action_data);
        }
    }
    
    Ok(())
}

#[tauri::command]
fn refresh_items(state: tauri::State<'_, AppState>) -> usize {
    let mut new_items = scan_applications();
    new_items.extend(get_system_commands());
    if let Ok(snippets) = get_all_snippets() {
        new_items.extend(snippets);
    }
    new_items.extend(get_i3_items());
    let count = new_items.len();
    *state.items.lock().unwrap() = new_items;
    count
}

#[tauri::command]
fn save_snippet(title: String, content: String, keyword: Option<String>) -> Result<(), String> {
    insert_snippet(&title, &content, keyword.as_deref())
        .map_err(|e| format!("Failed to save snippet: {}", e))
}

#[tauri::command]
fn evaluate_math(expression: &str) -> Result<String, String> {
    let expr = expression.trim();

    if let Ok(result) = eval_simple_math(expr) {
        Ok(format_number(result))
    } else {
        Err("Invalid expression".to_string())
    }
}

/// Known function names for math evaluation
const MATH_FUNCTIONS: &[&str] = &["sqrt", "sin", "cos", "tan", "log", "ln", "abs", "ceil", "floor", "round"];

fn eval_simple_math(expr: &str) -> Result<f64, String> {
    let expr = expr.replace(' ', "");

    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    // ---- STEP 1: Handle math functions FIRST (before generic parentheses) ----
    let lower = expr.to_lowercase();

    // Constants
    if lower == "pi" {
        return Ok(std::f64::consts::PI);
    }
    if lower == "e" && expr.len() == 1 {
        return Ok(std::f64::consts::E);
    }

    // Check for function calls: func(...)
    for func_name in MATH_FUNCTIONS {
        if lower.starts_with(&format!("{}(", func_name)) {
            // Find the matching closing parenthesis for this function
            if let Some(close) = find_matching_paren(&expr, func_name.len()) {
                let inner = &expr[func_name.len() + 1..close];
                let val = eval_simple_math(inner)?;
                let func_result = apply_function(func_name, val)?;

                // If there's more expression after the function, continue evaluating
                let remaining = &expr[close + 1..];
                if remaining.is_empty() {
                    return Ok(func_result);
                }
                // Rebuild expression with the function result substituted
                let new_expr = format!("{}{}", format_number(func_result), remaining);
                return eval_simple_math(&new_expr);
            }
        }
    }

    // ---- STEP 2: Handle generic parentheses ----
    if let Some(start) = expr.rfind('(') {
        if let Some(end_rel) = expr[start..].find(')') {
            let end = start + end_rel;
            let inner = &expr[start + 1..end];
            let inner_result = eval_simple_math(inner)?;
            let new_expr = format!(
                "{}{}{}",
                &expr[..start],
                format_number(inner_result),
                &expr[end + 1..]
            );
            return eval_simple_math(&new_expr);
        }
    }

    // ---- STEP 3: Addition/Subtraction (lowest precedence, right to left scan) ----
    let chars: Vec<char> = expr.chars().collect();
    let mut depth = 0i32;
    let mut last_add_sub = None;

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => depth += 1,
            '(' => depth -= 1,
            '+' | '-' if depth == 0 && i > 0 => {
                let prev = chars[i - 1];
                if prev != '*' && prev != '/' && prev != '+' && prev != '-' && prev != '^' {
                    last_add_sub = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    if let Some(pos) = last_add_sub {
        let left = eval_simple_math(&expr[..pos])?;
        let right = eval_simple_math(&expr[pos + 1..])?;
        return match chars[pos] {
            '+' => Ok(left + right),
            '-' => Ok(left - right),
            _ => unreachable!(),
        };
    }

    // ---- STEP 4: Multiplication/Division/Modulo ----
    depth = 0;
    let mut last_mul_div = None;
    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => depth += 1,
            '(' => depth -= 1,
            '*' | '/' | '%' if depth == 0 => {
                last_mul_div = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = last_mul_div {
        let left = eval_simple_math(&expr[..pos])?;
        let right = eval_simple_math(&expr[pos + 1..])?;
        return match chars[pos] {
            '*' => Ok(left * right),
            '/' => {
                if right == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(left / right)
                }
            }
            '%' => Ok(left % right),
            _ => unreachable!(),
        };
    }

    // ---- STEP 5: Power (^) ----
    if let Some(pos) = expr.rfind('^') {
        if pos > 0 {
            let left = eval_simple_math(&expr[..pos])?;
            let right = eval_simple_math(&expr[pos + 1..])?;
            return Ok(left.powf(right));
        }
    }

    // ---- STEP 6: Parse as number ----
    expr.parse::<f64>()
        .map_err(|_| format!("Cannot parse: {}", expr))
}

/// Find the closing parenthesis that matches the opening at `open_pos`
fn find_matching_paren(expr: &str, open_pos: usize) -> Option<usize> {
    let chars: Vec<char> = expr.chars().collect();
    if open_pos >= chars.len() || chars[open_pos] != '(' {
        return None;
    }
    let mut depth = 0;
    for i in open_pos..chars.len() {
        match chars[i] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Apply a math function to a value
fn apply_function(name: &str, val: f64) -> Result<f64, String> {
    match name {
        "sqrt" => Ok(val.sqrt()),
        "sin" => Ok(val.sin()),
        "cos" => Ok(val.cos()),
        "tan" => Ok(val.tan()),
        "log" => Ok(val.log10()),
        "ln" => Ok(val.ln()),
        "abs" => Ok(val.abs()),
        "ceil" => Ok(val.ceil()),
        "floor" => Ok(val.floor()),
        "round" => Ok(val.round()),
        _ => Err(format!("Unknown function: {}", name)),
    }
}

fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{:.10}", n)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Socket path for IPC toggle commands
fn socket_path() -> String {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/surch2.sock", runtime_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let sock_path = socket_path();
    
    let is_daemon_start = args.iter().any(|a| a == "daemon");
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if cmd == "toggle" || cmd == "show" || cmd == "hide" {
        if let Ok(mut stream) = UnixStream::connect(&sock_path) {
            let _ = writeln!(stream, "{}", cmd);
        } else {
            eprintln!("Surch2 is not running in the background.");
        }
        std::process::exit(0);
    } else if !is_daemon_start {
        // If they just typed `surch2`, and it's already running, toggle it and exit!
        if let Ok(mut stream) = UnixStream::connect(&sock_path) {
            let _ = writeln!(stream, "toggle");
            std::process::exit(0);
        }
    }

    // Initialize Database
    if let Err(e) = init_db() {
        eprintln!("[Surch2] Failed to initialize database: {}", e);
    }

    // Start Clipboard Monitor Thread
    start_clipboard_monitor();

    // Pre-scan items
    let mut items = scan_applications();
    items.extend(get_system_commands());
    if let Ok(snippets) = get_all_snippets() {
        items.extend(snippets);
    }
    items.extend(get_i3_items());
    println!("[Surch2] Indexed {} searchable items", items.len());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            items: Mutex::new(items),
        })
        .setup(move |app| {
            let main_window = app.get_webview_window("main").unwrap();

            if !is_daemon_start {
                let _ = main_window.show();
                let _ = main_window.set_focus();
            }

            // ---- Global Shortcut: Alt+Space ----
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

                let window_clone = main_window.clone();
                match app.global_shortcut().on_shortcut("alt+space", move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Ok(visible) = window_clone.is_visible() {
                            if visible {
                                let _ = window_clone.hide();
                            } else {
                                let _ = window_clone.show();
                                let _ = window_clone.set_focus();
                            }
                        }
                    }
                }) {
                    Ok(_) => println!("[Surch2] ✓ Global shortcut registered: Alt+Space"),
                    Err(e) => {
                        eprintln!("[Surch2] ✗ Could not register Alt+Space: {}", e);
                        eprintln!("[Surch2]   Fix: gsettings set org.gnome.desktop.wm.keybindings activate-window-menu \"['']\"");
                    }
                }
            }

            // ---- IPC Socket (secondary toggle, for scripts/i3 bindings) ----
            {
                let sock_path = socket_path();
                let _ = std::fs::remove_file(&sock_path);

                match UnixListener::bind(&sock_path) {
                    Ok(listener) => {
                        println!("[Surch2] ✓ IPC socket: {}", sock_path);

                        let window_clone = main_window.clone();
                        std::thread::spawn(move || {
                            for stream in listener.incoming() {
                                if let Ok(stream) = stream {
                                    let reader = BufReader::new(stream);
                                    for line in reader.lines().flatten() {
                                        let cmd = line.trim().to_lowercase();
                                        match cmd.as_str() {
                                            "toggle" => {
                                                if let Ok(visible) = window_clone.is_visible() {
                                                    if visible {
                                                        let _ = window_clone.hide();
                                                    } else {
                                                        let _ = window_clone.show();
                                                        let _ = window_clone.set_focus();
                                                    }
                                                }
                                            }
                                            "show" => {
                                                let _ = window_clone.show();
                                                let _ = window_clone.set_focus();
                                            }
                                            "hide" => {
                                                let _ = window_clone.hide();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[Surch2] ✗ IPC socket failed: {}", e);
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_items,
            search_clipboard_history,
            save_snippet,
            execute_item,
            refresh_items,
            evaluate_math
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

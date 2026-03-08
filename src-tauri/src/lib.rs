mod apps;

use apps::{scan_applications, AppEntry};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::process::Command;
use std::sync::Mutex;
use tauri::Manager;

/// Cached list of applications
struct AppState {
    apps: Mutex<Vec<AppEntry>>,
}

#[tauri::command]
fn search_apps(query: &str, state: tauri::State<'_, AppState>) -> Vec<AppEntry> {
    let apps = state.apps.lock().unwrap();

    if query.is_empty() {
        // Return first 20 apps when no query
        return apps.iter().take(20).cloned().collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &AppEntry)> = apps
        .iter()
        .filter_map(|app| {
            let name_score = matcher.fuzzy_match(&app.name, query).unwrap_or(0);
            let desc_score = app
                .description
                .as_ref()
                .and_then(|d| matcher.fuzzy_match(d, query))
                .unwrap_or(0);
            let cat_score = app
                .categories
                .as_ref()
                .and_then(|c| matcher.fuzzy_match(c, query))
                .unwrap_or(0);

            let total = name_score * 3 + desc_score + cat_score;
            if total > 0 {
                Some((total, app))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(20).map(|(_, app)| app.clone()).collect()
}

#[tauri::command]
fn launch_app(exec: &str) -> Result<(), String> {
    // Split exec into command and args
    let parts: Vec<&str> = exec.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    Command::new(parts[0])
        .args(&parts[1..])
        .spawn()
        .map_err(|e| format!("Failed to launch: {}", e))?;

    Ok(())
}

#[tauri::command]
fn refresh_apps(state: tauri::State<'_, AppState>) -> usize {
    let new_apps = scan_applications();
    let count = new_apps.len();
    *state.apps.lock().unwrap() = new_apps;
    count
}

#[tauri::command]
fn evaluate_math(expression: &str) -> Result<String, String> {
    // Basic math evaluation using a simple approach
    // For now, we'll use a simple parser; later we can add a full math engine
    let expr = expression.trim();

    // Try to parse simple arithmetic
    if let Ok(result) = eval_simple_math(expr) {
        Ok(format_number(result))
    } else {
        Err("Invalid expression".to_string())
    }
}

fn eval_simple_math(expr: &str) -> Result<f64, String> {
    // Remove spaces and handle basic operations
    let expr = expr.replace(' ', "");

    // Handle parentheses recursively
    if let Some(start) = expr.rfind('(') {
        if let Some(end) = expr[start..].find(')') {
            let inner = &expr[start + 1..start + end];
            let inner_result = eval_simple_math(inner)?;
            let new_expr = format!(
                "{}{}{}",
                &expr[..start],
                inner_result,
                &expr[start + end + 1..]
            );
            return eval_simple_math(&new_expr);
        }
    }

    // Handle addition and subtraction (lowest precedence)
    // Find the last + or - that's not at the start (for negative numbers)
    let mut depth = 0;
    let mut last_add_sub = None;
    let chars: Vec<char> = expr.chars().collect();

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => depth += 1,
            '(' => depth -= 1,
            '+' | '-' if depth == 0 && i > 0 => {
                // Make sure it's not part of a number (e.g., after * or /)
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

    // Handle multiplication and division
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

    // Handle power (^)
    if let Some(pos) = expr.find('^') {
        let left = eval_simple_math(&expr[..pos])?;
        let right = eval_simple_math(&expr[pos + 1..])?;
        return Ok(left.powf(right));
    }

    // Handle common math functions
    let lower = expr.to_lowercase();
    if lower == "pi" {
        return Ok(std::f64::consts::PI);
    }
    if lower == "e" {
        return Ok(std::f64::consts::E);
    }
    if let Some(inner) = lower.strip_prefix("sqrt(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.sqrt());
    }
    if let Some(inner) = lower.strip_prefix("sin(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.sin());
    }
    if let Some(inner) = lower.strip_prefix("cos(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.cos());
    }
    if let Some(inner) = lower.strip_prefix("tan(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.tan());
    }
    if let Some(inner) = lower.strip_prefix("log(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.log10());
    }
    if let Some(inner) = lower.strip_prefix("ln(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.ln());
    }
    if let Some(inner) = lower.strip_prefix("abs(").and_then(|s| s.strip_suffix(')')) {
        let val = eval_simple_math(inner)?;
        return Ok(val.abs());
    }

    // Parse as number
    expr.parse::<f64>()
        .map_err(|_| format!("Cannot parse: {}", expr))
}

fn format_number(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{:.10}", n)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Pre-scan applications
    let apps = scan_applications();
    println!("[Surch2] Indexed {} applications", apps.len());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            apps: Mutex::new(apps),
        })
        .setup(|app| {
            // Register global shortcut to toggle the window
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            let main_window = app.get_webview_window("main").unwrap();

            // Toggle visibility on Alt+Space (not Super+Space which i3 uses)
            let window_clone = main_window.clone();
            match app.global_shortcut().on_shortcut("alt+space", move |_app, _shortcut, _event| {
                if let Ok(visible) = window_clone.is_visible() {
                    if visible {
                        let _ = window_clone.hide();
                    } else {
                        let _ = window_clone.show();
                        let _ = window_clone.set_focus();
                    }
                }
            }) {
                Ok(_) => println!("[Surch2] Global shortcut registered: Alt+Space"),
                Err(e) => {
                    eprintln!("[Surch2] Warning: Could not register Alt+Space shortcut: {}", e);
                    eprintln!("[Surch2] You can still use the app, just launch it manually.");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_apps,
            launch_app,
            refresh_apps,
            evaluate_math
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

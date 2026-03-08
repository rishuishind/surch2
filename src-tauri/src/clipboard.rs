use arboard::Clipboard;
use std::thread;
use std::time::Duration;
use crate::db::add_clipboard_entry;

pub fn start_clipboard_monitor() {
    thread::spawn(|| {
        let mut last_content = String::new();
        
        loop {
            if let Ok(mut clipboard) = Clipboard::new() {
                if let Ok(current_content) = clipboard.get_text() {
                    let trimmed = current_content.trim().to_string();
                    
                    // Only process completely new content
                    if !trimmed.is_empty() && trimmed != last_content {
                        last_content = trimmed.clone();
                        
                        // Save to database
                        let _ = add_clipboard_entry(&trimmed);
                    }
                }
            }
            // Poll every 1 second
            thread::sleep(Duration::from_millis(1000));
        }
    });
}

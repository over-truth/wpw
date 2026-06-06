use std::thread;
use std::time::Duration;

/// Copy text to clipboard and clear after 30 seconds.
pub fn copy_and_clear(text: &str, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {e}"))?;
    
    clipboard.set_text(text.to_string())
        .map_err(|e| format!("Failed to copy to clipboard: {e}"))?;
    
    println!("{} copied to clipboard. Will clear in 30 seconds.", label);
    
    // Spawn a thread to clear clipboard after 30 seconds
    let text = text.to_string();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(30));
        if let Ok(mut cb) = arboard::Clipboard::new() {
            // Only clear if clipboard still contains our text
            if let Ok(current) = cb.get_text() {
                if current == text {
                    let _ = cb.set_text(String::new());
                }
            }
        }
    });
    
    Ok(())
}

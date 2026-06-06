use rand::Rng;

#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub use_upper: bool,
    pub use_lower: bool,
    pub use_digits: bool,
    pub symbols: Option<String>,  // None = no symbols, Some(chars) = use these symbols
    pub exclude: String,          // characters to exclude
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 20,
            use_upper: true,
            use_lower: true,
            use_digits: true,
            symbols: Some("!@#$%^&*".to_string()),
            exclude: String::new(),
        }
    }
}

/// Generate a random password based on options.
pub fn generate_password(options: &PasswordOptions) -> String {
    let mut charset = String::new();
    
    if options.use_upper {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if options.use_lower {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if options.use_digits {
        charset.push_str("0123456789");
    }
    if let Some(ref symbols) = options.symbols {
        charset.push_str(symbols);
    }
    
    // Remove excluded characters
    for ch in options.exclude.chars() {
        charset = charset.replace(ch, "");
    }
    
    if charset.is_empty() {
        // Fallback to lowercase if everything excluded
        charset = "abcdefghijklmnopqrstuvwxyz".to_string();
    }
    
    let charset_bytes = charset.as_bytes();
    let mut rng = rand::thread_rng();
    
    (0..options.length)
        .map(|_| {
            let idx = rng.gen_range(0..charset_bytes.len());
            charset_bytes[idx] as char
        })
        .collect()
}

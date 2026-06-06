pub fn run(
    length: usize,
    no_upper: bool,
    no_lower: bool,
    no_digits: bool,
    no_symbols: bool,
    symbols: Option<String>,
    exclude: Option<String>,
    count: usize,
    passphrase: bool,
    words: usize,
    separator: char,
    capitalize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if passphrase {
        let opts = wpw_core::generator::PassphraseOptions {
            word_count: words,
            separator,
            capitalize,
        };
        for _ in 0..count {
            println!("{}", wpw_core::generator::generate_passphrase(&opts));
        }
    } else {
        let opts = wpw_core::generator::PasswordOptions {
            length,
            use_upper: !no_upper,
            use_lower: !no_lower,
            use_digits: !no_digits,
            symbols: if no_symbols { None } else { symbols.or(Some("!@#$%^&*".to_string())) },
            exclude: exclude.unwrap_or_default(),
        };
        for _ in 0..count {
            println!("{}", wpw_core::generator::generate_password(&opts));
        }
    }
    Ok(())
}

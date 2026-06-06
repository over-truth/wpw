use std::io::{self, Write};

/// Prompt user for a password (hidden input).
pub fn prompt_password(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    rpassword::read_password()
}

/// Prompt user for a password with confirmation.
pub fn prompt_password_confirm() -> io::Result<String> {
    let pw = prompt_password("Enter master password: ")?;
    let confirm = prompt_password("Confirm master password: ")?;
    if pw != confirm {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Passwords do not match"));
    }
    Ok(pw)
}

/// Prompt with yes/no confirmation.
pub fn confirm(prompt: &str) -> io::Result<bool> {
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase() == "y")
}

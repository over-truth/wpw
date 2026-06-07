use std::io::{self, BufRead, Write};

/// Prompt user for a password (hidden input).
pub fn prompt_password(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    rpassword::read_password()
}

/// Read a single line from stdin without echo prompt. Used for `--password-stdin`
/// so the password never appears in `ps aux`.
pub fn read_password_stdin() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    // Trim trailing newline only; preserve any leading/internal whitespace the user typed.
    if line.ends_with('\n') { line.pop(); }
    if line.ends_with('\r') { line.pop(); }
    Ok(line)
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

/// Prompt with yes/no confirmation. Accepts y, yes (case-insensitive); anything else is No.
pub fn confirm(prompt: &str) -> io::Result<bool> {
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

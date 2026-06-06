mod commands;
mod session;
mod tty;
mod clipboard;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wpw", version, about = "WPW Password Manager")]
struct Cli {
    /// Override default vault path
    #[arg(long, global = true)]
    vault: Option<String>,
    
    /// Override default config file path
    #[arg(long, global = true)]
    config: Option<String>,
    
    /// Disable color output
    #[arg(long, global = true)]
    no_color: bool,
    
    /// Only output results
    #[arg(long, global = true)]
    quiet: bool,
    
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault
    Init {
        /// Override vault path
        #[arg(long)]
        vault: Option<String>,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Unlock the vault
    Unlock {
        /// Session timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
        /// Override vault path
        #[arg(long)]
        vault: Option<String>,
    },
    /// Lock the vault
    Lock,
    /// Show vault status
    Status,
    /// Add a new entry
    Add {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
        /// Generate a random password
        #[arg(long)]
        generate: bool,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        tag: Vec<String>,
        /// TOTP secret (Base32)
        #[arg(long)]
        totp: Option<String>,
    },
    /// Get an entry
    Get {
        /// Entry ID or title
        id: String,
        /// Specific field to retrieve
        #[arg(long)]
        field: Option<String>,
        /// Copy to clipboard
        #[arg(long)]
        copy: bool,
        /// Show password in terminal
        #[arg(long)]
        show: bool,
    },
    /// List entries
    List {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Edit an entry
    Edit {
        /// Entry ID or title
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        generate: bool,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Delete an entry
    Delete {
        /// Entry ID or title
        id: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Generate a random password
    Generate {
        #[arg(long, default_value = "20")]
        length: usize,
        #[arg(long)]
        no_upper: bool,
        #[arg(long)]
        no_lower: bool,
        #[arg(long)]
        no_digits: bool,
        #[arg(long)]
        no_symbols: bool,
        #[arg(long)]
        symbols: Option<String>,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long, default_value = "1")]
        count: usize,
        /// Use passphrase mode
        #[arg(long)]
        passphrase: bool,
        #[arg(long, default_value = "5")]
        words: usize,
        #[arg(long, default_value = "-")]
        separator: String,
        #[arg(long)]
        capitalize: bool,
    },
    /// Show TOTP code
    Totp {
        /// Entry ID or title
        id: String,
        #[arg(long)]
        copy: bool,
    },
    /// Show password history
    History {
        /// Entry ID or title
        id: String,
    },
    /// Restore a password from history
    Restore {
        /// Entry ID or title
        id: String,
        /// Unix timestamp of the password to restore
        #[arg(long)]
        at: i64,
    },
    /// Export vault
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        output: Option<String>,
        /// Include password history
        #[arg(long)]
        include_history: bool,
    },
    /// Import entries
    Import {
        #[arg(long, default_value = "json")]
        format: String,
        /// File to import
        file: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value
    Set { key: String, value: String },
    /// Get a configuration value
    Get { key: String },
}

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command {
        Commands::Init { ref vault } => commands::init::run(&cli, vault.as_deref()),
        Commands::Config { ref action } => match action {
            ConfigAction::Set { ref key, ref value } => commands::config::run_set(&cli, key, value),
            ConfigAction::Get { ref key } => commands::config::run_get(&cli, key),
        },
        Commands::Unlock { timeout, ref vault } => commands::unlock::run(&cli, timeout, vault.as_deref()),
        Commands::Lock => commands::lock::run(&cli),
        Commands::Status => commands::status::run(&cli),
        Commands::Add { ref title, ref url, ref username, ref password, generate, ref notes, ref tag, ref totp } => {
            commands::add::run(&cli, title.clone(), url.clone(), username.clone(), password.clone(), generate, notes.clone(), tag.clone(), totp.clone())
        }
        Commands::Get { ref id, ref field, copy, show } => commands::get::run(&cli, id, field.as_deref(), copy, show),
        Commands::List { ref tag, ref url, ref format } => commands::list::run(&cli, tag.as_deref(), url.as_deref(), format),
        Commands::Edit { ref id, ref title, ref url, ref username, ref password, generate, ref notes } => {
            commands::edit::run(&cli, id, title.clone(), url.clone(), username.clone(), password.clone(), generate, notes.clone())
        }
        Commands::Delete { ref id, yes } => commands::delete::run(&cli, id, yes),
        Commands::Generate { length, no_upper, no_lower, no_digits, no_symbols, ref symbols, ref exclude, count, passphrase, words, ref separator, capitalize } => {
            commands::generate::run(length, no_upper, no_lower, no_digits, no_symbols, symbols.clone(), exclude.clone(), count, passphrase, words, separator.chars().next().unwrap_or('-'), capitalize)
        }
        Commands::Totp { ref id, copy } => commands::totp::run(&cli, id, copy),
        Commands::History { ref id } => commands::history::run(&cli, id),
        Commands::Restore { ref id, at } => commands::restore::run(&cli, id, at),
        Commands::Export { ref format, ref output, include_history } => commands::export::run(&cli, format, output.as_deref(), include_history),
        Commands::Import { ref format, ref file } => commands::import::run(&cli, format, file),
    };
    
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

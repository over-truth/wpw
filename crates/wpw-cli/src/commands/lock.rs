use crate::{Cli, session};

pub fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    session::clear_session();
    if !cli.quiet {
        println!("Vault locked.");
    }
    Ok(())
}

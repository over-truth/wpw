use crate::{session, Cli};

pub fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    session::clear_session();
    if !cli.quiet {
        println!("Vault locked.");
    }
    Ok(())
}

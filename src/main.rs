mod config;
mod handlers;
mod cli;
mod wasm;

use crate::cli::Cli;
use crate::config::load_config;
use crate::handlers::{create_authorization, send_msg};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;
    
    match &cli.command {
        cli::Commands::CreateAuthorization { key_moniker, pool } => {
            create_authorization(key_moniker, pool, &config)?;
        }
        cli::Commands::SendMsg { key_moniker, msg } => {
            send_msg(key_moniker, msg, &config)?;
        }
    }
    Ok(())
}

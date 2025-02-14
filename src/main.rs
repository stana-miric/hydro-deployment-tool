mod cli;
mod config;
mod handlers;
mod helpers;
mod wasm;

use crate::cli::Cli;
use crate::config::load_config;
use crate::handlers::{create_authorization, execute_authorization, fund_program, tick_processor};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;

    match &cli.command {
        cli::Commands::CreateAuthorization { label, pool } => {
            create_authorization(label, pool, &config)?;
        }
        cli::Commands::ExecuteAuthorization { label } => {
            execute_authorization(label, &config)?;
        }
        cli::Commands::FundProgram { destination, funds } => {
            fund_program(destination, funds, &config)?;
        }
        cli::Commands::TickProcessor => {
            tick_processor(&config)?;
        }
    }
    Ok(())
}

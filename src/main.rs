mod authorization;
mod cli;
mod config;
mod handlers;
mod helpers;
mod node_cmd;
mod wasm;

use crate::cli::Cli;
use crate::config::load_config;
use crate::handlers::{create_program, execute_program, tick_processor};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;

    match &cli.command {
        cli::Commands::CreateProgram {
            label_prefix,
            pools,
        } => {
            create_program(label_prefix, pools, &config)?;
        }
        cli::Commands::ExecuteProgram {
            auth_contract_address,
            action,
        } => {
            execute_program(auth_contract_address, action.clone(), &config)?;
        }
        cli::Commands::TickProcessor {
            processor_contract_address,
        } => {
            tick_processor(processor_contract_address, &config)?;
        }
    }
    Ok(())
}

use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "liquidity-deployment-tool")]
#[command(about = "CLI tool to interact with Valence Authorization contract", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a program with a list of pools and a label prefix (label sufix will be deploy or withdraw)
    CreateProgram {
        label_prefix: String,
        pools: Vec<PoolInfo>,
    },
    /// Execute a program using the authorization contract address and the action (deploy or withdraw)
    ExecuteProgram {
        auth_contract_address: String,
        action: ProgramAction,
    },
    /// Tick the processor contract with the given address
    TickProcessor { processor_contract_address: String },
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum ProgramAction {
    Deploy,
    Withdraw,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PoolInfo {
    pub address: String,
    pub amount_a: u128,
    pub amount_b: u128,
    pub denom_a: String,
    pub denom_b: String,
}

impl FromStr for PoolInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 5 {
            return Err(
                "Invalid format. Expected: address,amount_a,amount_b,denom_a,denom_b".to_string(),
            );
        }
        Ok(PoolInfo {
            address: parts[0].to_string(),
            amount_a: parts[1]
                .parse()
                .map_err(|_| "Invalid amount_a format".to_string())?,
            amount_b: parts[2]
                .parse()
                .map_err(|_| "Invalid amount_b format".to_string())?,
            denom_a: parts[3].to_string(),
            denom_b: parts[4].to_string(),
        })
    }
}

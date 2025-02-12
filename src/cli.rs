use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "valence-tool")]
#[command(about = "CLI tool to interact with Valence Authorization contract", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create an authorization message
    CreateAuthorization {
        key_moniker: String,
        pool: PoolInfo,
    },
    /// Send a message to the Valence Authorization contract
    SendMsg {
        key_moniker: String,
        msg: String,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct PoolInfo {
    pub address: String,
    pub amount: u128,
    pub denom_a: String,
    pub denom_b: String,
}

// Implement FromStr for PoolInfo
impl FromStr for PoolInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 4 {
            return Err("Invalid format. Expected: address,amount,denom_a,denom_b".to_string());
        }
        Ok(PoolInfo {
            address: parts[0].to_string(),
            amount: parts[1]
                .parse()
                .map_err(|_| "Invalid amount format".to_string())?,
            denom_a: parts[2].to_string(),
            denom_b: parts[3].to_string(),
        })
    }
}

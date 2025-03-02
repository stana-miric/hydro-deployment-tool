use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use valence_astroport_utils::{astroport_cw20_lp_token, astroport_native_lp_token, PoolType};

#[derive(Parser)]
#[command(name = "liquidity-deployment-tool")]
#[command(about = "CLI tool to interact with Valence Authorization contract")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a program with a list of pools and a label prefix
    CreateProgram {
        #[arg(
            long,
            help = "Label prefix for the program (suffix will be deploy/withdraw)"
        )]
        label_prefix: String,

        #[arg(long, help = "Pool information in the format 'address,amount_a,amount_b,denom_a,denom_b'", value_parser = parse_pool)]
        pools: Vec<PoolInfo>,
    },

    /// Execute a program using the authorization contract address and the action (deploy or withdraw)
    ExecuteProgram {
        #[arg(long, help = "Authorization contract address")]
        auth_contract_address: String,

        #[arg(value_enum, long, help = "Action to perform (deploy or withdraw)")]
        action: ProgramAction,
    },

    /// Tick the processor contract with the given address
    TickProcessor {
        #[arg(long, help = "Processor contract address")]
        processor_contract_address: String,
    },
}

#[derive(ValueEnum, Debug, Clone)]
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
    pub pool_type: PoolType,
}

fn parse_pool(s: &str) -> Result<PoolInfo, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 6 {
        return Err(
            "Invalid format. Expected: address,amount_a,amount_b,denom_a,denom_b,pool_type"
                .to_string(),
        );
    }

    let pool_type = match parts[5] {
        "xyk_cw20" => PoolType::Cw20LpToken(astroport_cw20_lp_token::PairType::Xyk {}),
        "stable_cw20" => PoolType::Cw20LpToken(astroport_cw20_lp_token::PairType::Stable {}),
        "custom_cw20" => PoolType::Cw20LpToken(astroport_cw20_lp_token::PairType::Custom(
            parts[5].to_string(),
        )),
        "xyk_native" => PoolType::NativeLpToken(astroport_native_lp_token::PairType::Xyk {}),
        "stable_native" => PoolType::NativeLpToken(astroport_native_lp_token::PairType::Stable {}),
        "custom_native" => PoolType::NativeLpToken(astroport_native_lp_token::PairType::Custom(
            parts[5].to_string(),
        )),
        _ => return Err("Invalid pool_type format".to_string()),
    };

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
        pool_type,
    })
}

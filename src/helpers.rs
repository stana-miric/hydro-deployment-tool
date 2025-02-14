use crate::config::Config;
use crate::wasm::execute_wasm_contract;
use crate::{cli::PoolInfo, wasm::instantiate_wasm_contract};
use anyhow::{anyhow, Error, Result};
use cosmwasm_std::Decimal;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use valence_account_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_astroport_lper::msg::{
    LibraryConfig as AstroLperLibraryConfig, LiquidityProviderConfig,
};
use valence_library_utils::{
    denoms::UncheckedDenom, liquidity_utils::AssetData, LibraryAccountType,
};
use valence_splitter_library::msg::LibraryConfig as SpliterLibraryConfig;

pub fn create_base_account(config: &Config) -> Result<String> {
    let acc_instantiate_msg = InstantiateMsg {
        admin: config.liquidity_deployer_address.to_string(),
        approved_libraries: vec![],
    };

    let contract_address = instantiate_wasm_contract(
        config.base_account_code_id,
        &serde_json::to_string(&acc_instantiate_msg)?,
        config,
        "base_account",
    )?;

    Ok(contract_address)
}

pub fn instantiate_splitter_library(
    config: &Config,
    pool: &PoolInfo,
    input_addr: &String,
    output_addr: &String,
) -> Result<String> {
    let split_lib_instantiate_msg =
        valence_library_utils::msg::InstantiateMsg::<SpliterLibraryConfig> {
            owner: config.liquidity_deployer_address.to_string(),
            processor: config.processor_contract_address.to_string(),
            config: valence_splitter_library::msg::LibraryConfig {
                input_addr: LibraryAccountType::Addr(input_addr.to_string()),
                splits: vec![
                    valence_splitter_library::msg::UncheckedSplitConfig {
                        denom: UncheckedDenom::Native(pool.denom_a.to_string()),
                        account: LibraryAccountType::Addr(output_addr.to_string()),
                        amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                            Decimal::percent(100),
                        ),
                    },
                    valence_splitter_library::msg::UncheckedSplitConfig {
                        denom: UncheckedDenom::Native(pool.denom_b.to_string()),
                        account: LibraryAccountType::Addr(output_addr.to_string()),
                        amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                            Decimal::percent(100),
                        ),
                    },
                ],
            },
        };

    let contract_address = instantiate_wasm_contract(
        config.spliter_code_id,
        &serde_json::to_string(&split_lib_instantiate_msg)?,
        config,
        "splitter",
    )?;
    Ok(contract_address)
}

pub fn instantiate_astro_lper_library(
    config: &Config,
    pool: &PoolInfo,
    input_addr: &String,
    output_addr: &String,
) -> Result<String> {
    let astro_lper_instantiate_msg =
        valence_library_utils::msg::InstantiateMsg::<AstroLperLibraryConfig> {
            owner: config.liquidity_deployer_address.to_string(),
            processor: config.processor_contract_address.to_string(),
            config: valence_astroport_lper::msg::LibraryConfig {
                input_addr: LibraryAccountType::Addr(input_addr.to_string()),
                output_addr: LibraryAccountType::Addr(output_addr.to_string()),
                pool_addr: pool.address.to_string(),
                lp_config: LiquidityProviderConfig {
                    pool_type: valence_astroport_utils::PoolType::Cw20LpToken(
                        valence_astroport_utils::astroport_cw20_lp_token::PairType::Xyk {},
                    ),
                    asset_data: AssetData {
                        asset1: pool.denom_a.to_string(),
                        asset2: pool.denom_b.to_string(),
                    },
                    max_spread: None,
                },
            },
        };

    let contract_address = instantiate_wasm_contract(
        config.astro_lper_code_id,
        &serde_json::to_string(&astro_lper_instantiate_msg)?,
        config,
        "astro_lper",
    )?;
    Ok(contract_address)
}

pub fn approve_library(config: &Config, account: &String, library_address: &String) -> Result<()> {
    let create_authorization_msg = &ExecuteMsg::ApproveLibrary {
        library: library_address.to_string(),
    };
    execute_wasm_contract(
        account,
        &serde_json::to_string(&create_authorization_msg)?,
        config,
    )?;

    Ok(())
}

pub fn build_tx_flags(config: &Config) -> String {
    format!(
        "--from={} --gas auto --gas-adjustment {} --gas-prices {} --chain-id={} \
        --keyring-backend=test --output=json --home {} --node {} -y",
        config.liquidity_deployer_moniker,
        config.gas_adjustment,
        config.gas_price,
        config.chain_id,
        config.home,
        config.node_rpc
    )
}

pub fn run_command(cmd: &str) -> Result<String, Error> {
    println!("Running command: {}", cmd);

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command failed with status: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // Wait a few seconds for transaction to be indexed
    sleep(Duration::from_secs(5));
    Ok(stdout)
}

use anyhow::{Context, Result};
use std::env; // Import Context from anyhow for more descriptive errors

#[derive(Debug, Clone)]
pub struct Config {
    pub auth_contract_address: String,
    pub processor_contract_address: String,
    pub base_account_code_id: u64,
    pub spliter_code_id: u64,
    pub astro_lper_code_id: u64,
    pub liquidity_deployer_address: String, // admin address on neutron that will be liquidity deployer
    pub liquidity_deployer_moniker: String, // moniker that belongs to liquidity deployer key
    pub node_rpc: String,
    pub node_binary: String,
    pub chain_id: String,
    pub home: String,
    pub gas_price: String,
    pub gas_adjustment: String,
}

pub fn load_config() -> Result<Config> {
    let base_account_code_id_str = env::var("LD_TOOL_BASE_ACCOUNT_CODE_ID")
        .context("LD_TOOL_BASE_ACCOUNT_CODE_ID environment variable is required")?;
    let spliter_code_id_str = env::var("LD_TOOL_SPLITER_CODE_ID")
        .context("LD_TOOL_SPLITER_CODE_ID environment variable is required")?;
    let astro_lper_code_id_str = env::var("LD_TOOL_ASTRO_LPER_CODE_ID")
        .context("LD_TOOL_ASTRO_LPER_CODE_ID environment variable is required")?;

    Ok(Config {
        auth_contract_address: env::var("LD_TOOL_AUTHORIZATION_CONTRACT_ADDRESS")
            .context("LD_TOOL_AUTHORIZATION_CONTRACT_ADDRESS environment variable is required")?,
        processor_contract_address: env::var("LD_TOOL_PROCESSOR_CONTRACT_ADDRESS")
            .context("LD_TOOL_PROCESSOR_CONTRACT_ADDRESS environment variable is required")?,
        base_account_code_id: base_account_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_BASE_ACCOUNT_CODE_ID")?,
        spliter_code_id: spliter_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_SPLITER_CODE_ID")?,
        astro_lper_code_id: astro_lper_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_ASTRO_LPER_CODE_ID")?,
        liquidity_deployer_address: env::var("LD_TOOL_LIQUIDITY_DEPLOYER_ADDRESS")
            .context("LD_TOOL_LIQUIDITY_DEPLOYER_ADDRESS environment variable is required")?,
        liquidity_deployer_moniker: env::var("LD_TOOL_LIQUIDITY_DEPLOYER_MONIKER")
            .context("LD_TOOL_LIQUIDITY_DEPLOYER_MONIKER environment variable is required")?,
        node_rpc: env::var("LD_TOOL_NEUTRON_NODE_RPC")
            .context("LD_TOOL_NEUTRON_NODE_RPC environment variable is required")?,
        node_binary: env::var("LD_TOOL_NEUTRON_NODE_BINARY")
            .context("LD_TOOL_NEUTRON_NODE_BINARY environment variable is required")?,
        chain_id: env::var("LD_TOOL_NEUTRON_CHAIN_ID")
            .context("LD_TOOL_NEUTRON_CHAIN_ID environment variable is required")?,
        home: env::var("LD_TOOL_HOME_DIR")
            .context("LD_TOOL_HOME_DIR environment variable is required")?,
        gas_adjustment: env::var("LD_TOOL_GAS_ADJUSTMENT")
            .context("LD_TOOL_GAS_ADJUSTMENT environment variable is required")?,
        gas_price: env::var("LD_TOOL_GAS_PRICE")
            .context("LD_TOOL_GAS_PRICE environment variable is required")?,
    })
}

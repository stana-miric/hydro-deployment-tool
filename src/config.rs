use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub base_account_code_id: u64,
    pub spliter_code_id: u64,
    pub astro_lper_code_id: u64,
    pub astro_withdraw_code_id: u64,
    pub authorization_code_id: u64,
    pub processor_code_id: u64,
    pub tool_operator_address: String,
    pub tool_operator_moniker: String,
    pub neutron_dao_committee_address: String,
    // flags related to communication with neutron node
    pub neutron_rpc: String,
    pub neutron_binary: String,
    pub neutron_chain_id: String,
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
    let astro_withdraw_code_id_str = env::var("LD_TOOL_ASTRO_WITHDRAW_CODE_ID")
        .context("LD_TOOL_ASTRO_WITHDRAW_CODE_ID environment variable is required")?;
    let authorization_code_id_str = env::var("LD_TOOL_AUTHORIZATION_CODE_ID")
        .context("LD_TOOL_AUTHORIZATION_CODE_ID environment variable is required")?;
    let processor_code_id_str = env::var("LD_TOOL_PROCESSOR_CODE_ID")
        .context("LD_TOOL_PROCESSOR_CODE_ID environment variable is required")?;

    Ok(Config {
        base_account_code_id: base_account_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_BASE_ACCOUNT_CODE_ID")?,
        spliter_code_id: spliter_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_SPLITER_CODE_ID")?,
        astro_lper_code_id: astro_lper_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_ASTRO_LPER_CODE_ID")?,
        astro_withdraw_code_id: astro_withdraw_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_ASTRO_WITHDRAW_CODE_ID")?,
        authorization_code_id: authorization_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_AUTHORIZATION_CODE_ID")?,
        processor_code_id: processor_code_id_str
            .parse()
            .context("Failed to parse LD_TOOL_PROCESSOR_CODE_ID")?,
        tool_operator_address: env::var("LD_TOOL_OPERATOR_ADDRESS")
            .context("LD_TOOL_OPERATOR_ADDRESS environment variable is required")?,
        tool_operator_moniker: env::var("LD_TOOL_OPERATOR_MONIKER")
            .context("LD_TOOL_OPERATOR_MONIKER environment variable is required")?,
        neutron_dao_committee_address: env::var("LD_TOOL_DAO_COMMITTEE_ADDRESS")
            .context("LD_TOOL_DAO_COMMITTEE_ADDRESS environment variable is required")?,
        neutron_rpc: env::var("LD_TOOL_NEUTRON_NODE_RPC")
            .context("LD_TOOL_NEUTRON_NODE_RPC environment variable is required")?,
        neutron_binary: env::var("LD_TOOL_NEUTRON_NODE_BINARY")
            .context("LD_TOOL_NEUTRON_NODE_BINARY environment variable is required")?,
        neutron_chain_id: env::var("LD_TOOL_NEUTRON_CHAIN_ID")
            .context("LD_TOOL_NEUTRON_CHAIN_ID environment variable is required")?,
        home: env::var("LD_TOOL_HOME_DIR")
            .context("LD_TOOL_HOME_DIR environment variable is required")?,
        gas_adjustment: env::var("LD_TOOL_GAS_ADJUSTMENT")
            .context("LD_TOOL_GAS_ADJUSTMENT environment variable is required")?,
        gas_price: env::var("LD_TOOL_GAS_PRICE")
            .context("LD_TOOL_GAS_PRICE environment variable is required")?,
    })
}

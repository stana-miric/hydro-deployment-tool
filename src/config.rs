use std::env;
use anyhow::{Result, Context}; // Import Context from anyhow for more descriptive errors

#[derive(Debug, Clone)]
pub struct Config {
    pub auth_contract_address: String,
    pub processor_contract_address: String,
    pub base_account_code_id: u64,
    pub spliter_code_id: u64,
    pub astro_lper_code_id: u64,
    pub neutron_admin_address: String,
    pub node_rpc: String,
    pub chain_id: String,
    pub home: String,
}

pub fn load_config() -> Result<Config> {
    let base_account_code_id_str = env::var("BASE_ACCOUNT_CODE_ID")
        .context("BASE_ACCOUNT_CODE_ID environment variable is required")?;  // Context for better error handling
    let spliter_code_id_str = env::var("SPLITER_A_CODE_ID")
        .context("SPLITER_A_CODE_ID environment variable is required")?;  // Context for better error handling
    let astro_lper_code_id_str = env::var("ASTRO_LPER_CODE_ID")
        .context("ASTRO_LPER_CODE_ID environment variable is required")?;  // Context for better error handling

    Ok(Config {
        auth_contract_address: env::var("AUTHORIZATION_CONTRACT_ADDRESS")
            .context("AUTHORIZATION_CONTRACT_ADDRESS environment variable is required")?,
        processor_contract_address: env::var("PROCESSOR_CONTRACT_ADDRESS")
            .context("PROCESSOR_CONTRACT_ADDRESS environment variable is required")?,
        base_account_code_id: base_account_code_id_str
            .parse()
            .context("Failed to parse BASE_ACCOUNT_CODE_ID")?,
        spliter_code_id: spliter_code_id_str
            .parse()
            .context("Failed to parse SPLITER_A_CODE_ID")?,
        astro_lper_code_id: astro_lper_code_id_str
            .parse()
            .context("Failed to parse ASTRO_LPER_CODE_ID")?,
        neutron_admin_address: env::var("NEUTRON_ADMIN_ADDRESS")
            .context("NEUTRON_ADMIN_ADDRESS environment variable is required")?,
        node_rpc: env::var("NEUTRON_NODE_RPC")
            .context("NEUTRON_NODE_RPC environment variable is required")?,
        chain_id: env::var("NEUTRON_CHAIN_ID")
            .context("NEUTRON_CHAIN_ID environment variable is required")?,
        home: env::var("HOME_DIR")
            .context("HOME_DIR environment variable is required")?,
    })
}

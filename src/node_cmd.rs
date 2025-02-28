use crate::config::Config;
use anyhow::{anyhow, Error};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn build_tx_flags(config: &Config) -> String {
    format!(
        "--from={} --gas auto --gas-adjustment {} --gas-prices {} --chain-id={} \
        --keyring-backend=test --output=json --home {} --node {} -y",
        config.tool_operator_moniker,
        config.gas_adjustment,
        config.gas_price,
        config.neutron_chain_id,
        config.home,
        config.neutron_rpc
    )
}

pub fn build_query_flags(config: &Config) -> String {
    format!(
        "--chain-id={} --node {} --output=json",
        config.neutron_chain_id, config.neutron_rpc
    )
}

pub fn build_wasm_instantiate_flags(config: &Config, label: &str) -> String {
    format!(
        "--admin={} --label={}",
        config.neutron_dao_committee_address, label
    )
}

pub fn run_command(cmd: &str) -> Result<String, Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        println!("Running command failed: {}", cmd);
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

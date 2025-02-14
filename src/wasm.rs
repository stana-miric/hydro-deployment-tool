use crate::config::Config;
use crate::helpers::build_tx_flags;
use crate::helpers::run_command;
use anyhow::{anyhow, Error};
use serde_json::Value;

pub fn execute_wasm_contract(
    contract_address: &str,
    msg: &str,
    config: &Config,
) -> Result<(), Error> {
    println!(
        "Executing CosmWasm message: {:?} on contract {}",
        msg, contract_address
    );

    let flags = build_tx_flags(config);
    let cmd = format!(
        "{} tx wasm execute {} '{}' {}",
        config.node_binary, contract_address, msg, flags
    );

    run_command(&cmd)?;
    Ok(())
}

pub fn instantiate_wasm_contract(
    code_id: u64,
    msg: &str,
    config: &Config,
    label: &str,
) -> Result<String, Error> {
    println!("Instantiating CosmWasm contract with code_id: {}", code_id);

    let flags = build_tx_flags(config);
    let init_flags = build_wasm_instantiate_flags(config, label);
    let cmd = format!(
        "{} tx wasm instantiate {} '{}' {} {}",
        config.node_binary, code_id, msg, init_flags, flags
    );

    let output = run_command(&cmd)?;

    // Extract txhash from JSON output
    let tx_output: Value = serde_json::from_str(&output)?;
    let tx_hash = tx_output["txhash"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to extract txhash"))?;

    // Query the transaction
    let query_cmd = format!(
        "{} q tx {} --node {} --output json",
        config.node_binary, tx_hash, config.node_rpc
    );
    let tx_response = run_command(&query_cmd)?;

    // Parse JSON response
    let tx_data: Value = serde_json::from_str(&tx_response)?;

    // Find contract address in events
    let contract_address = tx_data["events"]
        .as_array()
        .ok_or_else(|| anyhow!("Failed to parse transaction events"))?
        .iter()
        .flat_map(|event| event["attributes"].as_array())
        .flatten()
        .find_map(|attr| {
            if attr["key"] == "_contract_address" {
                attr["value"].as_str().map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Contract address not found in transaction events"))?;

    println!("Contract instantiated at: {}", contract_address);

    Ok(contract_address)
}

fn build_wasm_instantiate_flags(config: &Config, label: &str) -> String {
    format!(
        "--admin={} --label={}",
        config.liquidity_deployer_address, label
    )
}

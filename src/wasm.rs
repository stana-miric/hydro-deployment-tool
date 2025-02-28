use crate::authorization::{Authorization, AuthorizationsResponse};
use crate::config::Config;
use crate::node_cmd::{
    build_query_flags, build_tx_flags, build_wasm_instantiate_flags, run_command,
};
use anyhow::{anyhow, Error};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read};

pub fn execute_wasm_contract(
    contract_address: &str,
    msg: &str,
    config: &Config,
) -> Result<(), Error> {
    let flags = build_tx_flags(config);
    let cmd = format!(
        "{} tx wasm execute {} '{}' {}",
        config.neutron_binary, contract_address, msg, flags
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
    instantiate_wasm_contract_internal(code_id, msg, config, label, None)
}

pub fn instantiate2_wasm_contract(
    code_id: u64,
    msg: &str,
    config: &Config,
    label: &str,
    salt: &str,
) -> Result<String, Error> {
    instantiate_wasm_contract_internal(code_id, msg, config, label, Some(salt))
}

// Internal function that handles both instantiations
fn instantiate_wasm_contract_internal(
    code_id: u64,
    msg: &str,
    config: &Config,
    label: &str,
    salt: Option<&str>,
) -> Result<String, Error> {
    let flags = build_tx_flags(config);
    let init_flags = build_wasm_instantiate_flags(config, label);

    let cmd = match salt {
        Some(s) => format!(
            "{} tx wasm instantiate2 {} '{}' {} {} {}",
            config.neutron_binary, code_id, msg, s, init_flags, flags
        ),
        None => format!(
            "{} tx wasm instantiate {} '{}' {} {}",
            config.neutron_binary, code_id, msg, init_flags, flags
        ),
    };

    let output = run_command(&cmd)?;

    let tx_output: Value = serde_json::from_str(&output)?;
    let tx_hash = tx_output["txhash"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to extract txhash"))?;

    // Query the transaction by tx hash
    let query_cmd = format!(
        "{} q tx {} --node {} --output json",
        config.neutron_binary, tx_hash, config.neutron_rpc
    );
    let tx_response = run_command(&query_cmd)?;

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

    Ok(contract_address)
}

pub fn get_code_hash(config: &Config, code_id: u64) -> Result<String, Error> {
    let temp_filename = format!("/tmp/wasm_code_{}.wasm", code_id);

    let flags = build_query_flags(config);
    let cmd = format!(
        "{} q wasm code {} {} {}",
        config.neutron_binary, code_id, temp_filename, flags
    );

    run_command(&cmd)?;

    let mut file = File::open(&temp_filename)?;
    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)?;

    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let hash_result = hasher.finalize();
    let hash_hex = format!("{:x}", hash_result);

    std::fs::remove_file(&temp_filename)?;

    Ok(hash_hex)
}

pub fn get_authorizations(
    config: &Config,
    auth_contract_address: &str,
) -> Result<Vec<Authorization>, Error> {
    let flags = build_query_flags(config);
    let query_msg = format!(
        r#"{{
            "authorizations": {{
                "start_after": null,
                "limit": 100
            }}
        }}"#
    );

    let cmd = format!(
        "{} q wasm contract-state smart {} '{}' {}",
        config.neutron_binary, auth_contract_address, query_msg, flags
    );

    let output = run_command(&cmd)?;
    let authorizations_response: AuthorizationsResponse = serde_json::from_str(&output)?;

    // Return the parsed authorizations
    Ok(authorizations_response.data)
}

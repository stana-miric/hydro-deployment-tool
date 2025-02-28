use crate::authorization::Authorization;
use crate::cli::{PoolInfo, ProgramAction};
use crate::config::Config;
use crate::wasm::{
    execute_wasm_contract, get_authorizations, get_code_hash, instantiate2_wasm_contract,
    instantiate_wasm_contract,
};
use anyhow::{Error, Result};
use bech32::{encode, primitives::decode::CheckedHrpstring, Bech32, Hrp};
use chrono::Utc;
use cosmwasm_std::{instantiate2_address, Addr, CanonicalAddr, HexBinary};
use cw_ownable;
use serde_json::Value;
use std::collections::HashMap;
use valence_account_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_astroport_lper::msg::{
    LibraryConfig as AstroLperLibraryConfig, LiquidityProviderConfig,
};
use valence_astroport_withdrawer::msg::LibraryConfig as AstroWithdrawerLibraryConfig;
use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo, Subroutine},
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::{
    denoms::UncheckedDenom, liquidity_utils::AssetData, LibraryAccountType,
};
use valence_splitter_library::msg::LibraryConfig as SpliterLibraryConfig;

const NEUTRON_BECH32_PREFIX: &str = "neutron";
const DELIMITER: &str = "_";
const DEPLOY: &str = "deploy";
const WITHDRAW: &str = "withdraw";

pub fn create_base_account(config: &Config) -> Result<String> {
    let acc_instantiate_msg = InstantiateMsg {
        admin: config.tool_operator_address.to_string(), // once the program is created we will transfer the ownership to dao comittee
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

pub fn create_output_accounts(
    config: &Config,
    pools: &Vec<PoolInfo>,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut split_output_accounts = Vec::new();
    let mut liquidity_output_accounts = Vec::new();
    let mut withdrawal_accounts = Vec::new();

    for _ in 0..pools.len() {
        split_output_accounts.push(create_base_account(config)?.to_string());
        liquidity_output_accounts.push(create_base_account(config)?.to_string());
        withdrawal_accounts.push(create_base_account(config)?.to_string());
    }

    Ok((
        split_output_accounts,
        liquidity_output_accounts,
        withdrawal_accounts,
    ))
}

pub fn instantiate_splitter_library(
    config: &Config,
    pools: &Vec<PoolInfo>,
    input_addr: &String,
    output_addrs: &Vec<String>,
    processor_addr: &String,
) -> Result<String> {
    let splits: Vec<_> = pools
        .iter()
        .zip(output_addrs.iter())
        .flat_map(|(pool, output_addr)| {
            vec![
                valence_splitter_library::msg::UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(pool.denom_a.to_string()),
                    account: LibraryAccountType::Addr(output_addr.to_string()),
                    amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                        pool.amount_a.into(),
                    ),
                },
                valence_splitter_library::msg::UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(pool.denom_b.to_string()),
                    account: LibraryAccountType::Addr(output_addr.to_string()),
                    amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                        pool.amount_b.into(),
                    ),
                },
            ]
        })
        .collect();

    let split_lib_instantiate_msg =
        valence_library_utils::msg::InstantiateMsg::<SpliterLibraryConfig> {
            owner: config.neutron_dao_committee_address.to_string(),
            processor: processor_addr.to_string(),
            config: valence_splitter_library::msg::LibraryConfig {
                input_addr: LibraryAccountType::Addr(input_addr.to_string()),
                splits,
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

pub fn instantiate_and_approve_astroport_libraries(
    config: &Config,
    pools: &Vec<PoolInfo>,
    split_output_accounts: &Vec<String>,
    liquidity_output_accounts: &Vec<String>,
    processor_address: &String,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut astroport_lper_lib_addresses = Vec::new();
    let mut astroport_withdraw_lib_addresses = Vec::new();

    for ((pool, split_output_account), liquidity_output_account) in pools
        .iter()
        .zip(split_output_accounts)
        .zip(liquidity_output_accounts)
    {
        let astroport_lper_lib_address = instantiate_astro_lper_library(
            config,
            pool,
            split_output_account,
            liquidity_output_account,
            processor_address,
        )?;
        approve_library(config, split_output_account, &astroport_lper_lib_address)?;
        approve_library(
            config,
            liquidity_output_account,
            &astroport_lper_lib_address,
        )?;
        astroport_lper_lib_addresses.push(astroport_lper_lib_address);

        let astroport_withdraw_lib_address = instantiate_astro_withdraw_library(
            config,
            pool,
            liquidity_output_account,
            processor_address,
        )?;
        approve_library(
            config,
            liquidity_output_account,
            &astroport_withdraw_lib_address,
        )?;
        astroport_withdraw_lib_addresses.push(astroport_withdraw_lib_address);
    }

    Ok((
        astroport_lper_lib_addresses,
        astroport_withdraw_lib_addresses,
    ))
}

fn instantiate_astro_lper_library(
    config: &Config,
    pool: &PoolInfo,
    input_addr: &String,
    output_addr: &String,
    processor_addr: &String,
) -> Result<String> {
    let astro_lper_instantiate_msg =
        valence_library_utils::msg::InstantiateMsg::<AstroLperLibraryConfig> {
            owner: config.neutron_dao_committee_address.to_string(),
            processor: processor_addr.to_string(),
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

fn instantiate_astro_withdraw_library(
    config: &Config,
    pool: &PoolInfo,
    input_addr: &String,
    processor_addr: &String,
) -> Result<String> {
    let astro_withdraw_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        AstroWithdrawerLibraryConfig,
    > {
        owner: config.neutron_dao_committee_address.to_string(),
        processor: processor_addr.to_string(),
        config: valence_astroport_withdrawer::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_addr.to_string()),
            output_addr: LibraryAccountType::Addr(config.neutron_dao_committee_address.to_string()),
            pool_addr: pool.address.to_string(),
            withdrawer_config: valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
                pool_type: valence_astroport_utils::PoolType::Cw20LpToken(
                    valence_astroport_utils::astroport_cw20_lp_token::PairType::Xyk {},
                ),
                asset_data: AssetData {
                    asset1: pool.denom_a.to_string(),
                    asset2: pool.denom_b.to_string(),
                },
            },
        },
    };

    let contract_address = instantiate_wasm_contract(
        config.astro_withdraw_code_id,
        &serde_json::to_string(&astro_withdraw_instantiate_msg)?,
        config,
        "astro_lper",
    )?;
    Ok(contract_address)
}

pub fn instantiate_authorization_and_processor(config: &Config) -> Result<(String, String)> {
    // predict authorization address
    let authorization_salt = generate_salt();
    let code_hash = get_code_hash(config, config.authorization_code_id)?;
    let predicted_auth_address = predict_contract_address(
        &config.tool_operator_address,
        &authorization_salt,
        &code_hash,
    )?;

    // init processor
    let mut processor_instantiate_msg = HashMap::new();
    processor_instantiate_msg.insert("authorization_contract", predicted_auth_address.to_string());

    let processor_address = instantiate_wasm_contract(
        config.processor_code_id,
        &serde_json::to_string(&processor_instantiate_msg)?,
        config,
        "processor",
    )?;

    // init authorization
    let mut authorization_instantiate_msg = HashMap::new();
    authorization_instantiate_msg.insert(
        "owner",
        Value::String(config.tool_operator_address.to_string()),
    );
    authorization_instantiate_msg.insert("processor", Value::String(processor_address.to_string()));
    authorization_instantiate_msg.insert("sub_owners", serde_json::to_value(Vec::<String>::new())?);

    let authorization_address = instantiate2_wasm_contract(
        config.authorization_code_id,
        &serde_json::to_string(&authorization_instantiate_msg)?,
        config,
        "authorization",
        &authorization_salt,
    )?;

    println!("Authorization Address: {}", authorization_address);
    println!("Processor Address: {}", processor_address);

    Ok((authorization_address, processor_address))
}

/// Generates a hex-encoded timestamp to use as the salt
fn generate_salt() -> String {
    let timestamp = Utc::now().timestamp(); // Get the current Unix timestamp
    format!("{:x}", timestamp) // Convert to hex string
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

pub fn transfer_accounts_ownership(
    config: &Config,
    account_addresses: &[String],
    new_owner_addr: &String,
) -> Result<()> {
    for account_address in account_addresses {
        let update_acc_ownership_msg = valence_account_utils::msg::ExecuteMsg::UpdateOwnership(
            cw_ownable::Action::TransferOwnership {
                new_owner: new_owner_addr.to_string(),
                expiry: None,
            },
        );

        execute_wasm_contract(
            account_address,
            &serde_json::to_string(&update_acc_ownership_msg)?,
            config,
        )?;
    }

    Ok(())
}

pub fn get_filtered_authorizations(
    auth_contract_address: &str,
    action: ProgramAction,
    config: &Config,
) -> Result<Vec<Authorization>, Error> {
    let label_suffix = match action {
        ProgramAction::Deploy => DEPLOY,
        ProgramAction::Withdraw => WITHDRAW,
    };

    let authorizations: Vec<Authorization> = get_authorizations(config, auth_contract_address)?;

    // Filter by label
    let filtered_authorizations = authorizations
        .into_iter()
        .filter(|auth| auth.label.ends_with(label_suffix))
        .collect();

    Ok(filtered_authorizations)
}

pub fn build_deploy_subroutine(
    split_lib_address: &String,
    astroport_lper_lib_addresses: &Vec<String>,
) -> Subroutine {
    let mut deploy_subroutine_builder = AtomicSubroutineBuilder::new();

    deploy_subroutine_builder = deploy_subroutine_builder.with_function(
        AtomicFunctionBuilder::new()
            .with_contract_address(LibraryAccountType::Addr(split_lib_address.to_string()))
            .with_message_details(MessageDetails {
                message_type: MessageType::CosmwasmExecuteMsg,
                message: Message {
                    name: "process_function".to_string(),
                    params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                        "process_function".to_string(),
                        "split".to_string(),
                    ])]),
                },
            })
            .build(),
    );

    for astroport_lper_lib_address in astroport_lper_lib_addresses {
        deploy_subroutine_builder = deploy_subroutine_builder.with_function(
            AtomicFunctionBuilder::new()
                .with_contract_address(LibraryAccountType::Addr(
                    astroport_lper_lib_address.to_string(),
                ))
                .with_message_details(MessageDetails {
                    message_type: MessageType::CosmwasmExecuteMsg,
                    message: Message {
                        name: "process_function".to_string(),
                        params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                            "process_function".to_string(),
                            "provide_double_sided_liquidity".to_string(),
                        ])]),
                    },
                })
                .build(),
        );
    }

    deploy_subroutine_builder.build()
}

pub fn build_withdraw_subroutine(astroport_withdraw_lib_addresses: &Vec<String>) -> Subroutine {
    let mut withdraw_subroutine_builder = AtomicSubroutineBuilder::new();

    for astroport_withdraw_lib_address in astroport_withdraw_lib_addresses {
        withdraw_subroutine_builder = withdraw_subroutine_builder.with_function(
            AtomicFunctionBuilder::new()
                .with_contract_address(LibraryAccountType::Addr(
                    astroport_withdraw_lib_address.to_string(),
                ))
                .with_message_details(MessageDetails {
                    message_type: MessageType::CosmwasmExecuteMsg,
                    message: Message {
                        name: "process_function".to_string(),
                        params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                            "process_function".to_string(),
                            "withdraw_liquidity".to_string(),
                        ])]),
                    },
                })
                .build(),
        );
    }

    withdraw_subroutine_builder.build()
}

pub fn create_and_execute_authorization(
    authorization_address: &String,
    deploy_subroutine: Subroutine,
    withdraw_subroutine: Subroutine,
    config: &Config,
    label_prefix: &String,
) -> Result<()> {
    let deploy_authorization = AuthorizationBuilder::new()
        .with_label(&format!("{}{}{}", label_prefix, DELIMITER, DEPLOY))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![config.tool_operator_address.to_string()]),
        ))
        .with_subroutine(deploy_subroutine)
        .build();

    let withdraw_authorization = AuthorizationBuilder::new()
        .with_label(&format!("{}{}{}", label_prefix, DELIMITER, WITHDRAW))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![config.tool_operator_address.to_string()]),
        ))
        .with_subroutine(withdraw_subroutine)
        .build();

    let create_authorization_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
            authorizations: vec![deploy_authorization, withdraw_authorization],
        },
    );

    execute_wasm_contract(
        authorization_address,
        &serde_json::to_string(&create_authorization_msg)?,
        config,
    )?;

    Ok(())
}

pub fn transfer_ownership(
    config: &Config,
    authorization_address: &String,
    input_account: &String,
    split_output_accounts: &Vec<String>,
    liquidity_output_accounts: &Vec<String>,
    withdrawal_accounts: &Vec<String>,
) -> Result<()> {
    // Transfer ownership of the authorization contract
    let update_auth_ownership_msg = valence_authorization_utils::msg::ExecuteMsg::UpdateOwnership(
        cw_ownable::Action::TransferOwnership {
            new_owner: config.neutron_dao_committee_address.to_string(),
            expiry: None,
        },
    );

    execute_wasm_contract(
        authorization_address,
        &serde_json::to_string(&update_auth_ownership_msg)?,
        config,
    )?;

    // Collect all accounts
    let mut all_accounts = vec![input_account.clone()];
    all_accounts.extend_from_slice(&split_output_accounts);
    all_accounts.extend_from_slice(&liquidity_output_accounts);
    all_accounts.extend_from_slice(&withdrawal_accounts);

    // Transfer ownership of all accounts
    transfer_accounts_ownership(config, &all_accounts, &config.neutron_dao_committee_address)?;

    println!("Ownership change needs to be accepted for the following accounts:");
    for acc in &all_accounts {
        println!("{}", acc);
    }

    Ok(())
}

/// Computes the predicted contract address based on CosmWasm's derivation formula.
pub fn predict_contract_address(
    creator: &str,
    salt: &str,
    code_hash: &str,
) -> Result<String, Error> {
    let creator_canonical = addr_canonicalize(creator)?;
    let code_hash_bytes = HexBinary::from_hex(code_hash).unwrap();
    let salt_bytes = HexBinary::from_hex(salt).unwrap();

    // Call CosmWasm's instantiate2_address to get the predicted address
    let predicted_address =
        instantiate2_address(&code_hash_bytes, &creator_canonical, &salt_bytes)?;

    // Convert canonical address back to human-readable address
    let addr = addr_humanize(&predicted_address)?;

    // Return the address as a string
    Ok(addr.into_string())
}
/// Converts a Bech32 address to canonical format.
fn addr_canonicalize(input: &str) -> Result<CanonicalAddr, Error> {
    let hrp_str =
        CheckedHrpstring::new::<Bech32>(input).map_err(|_| Error::msg("Error decoding bech32"))?;

    // Ensure the Bech32 prefix is correct
    if !hrp_str
        .hrp()
        .as_bytes()
        .eq_ignore_ascii_case(NEUTRON_BECH32_PREFIX.as_bytes())
    {
        return Err(Error::msg("Wrong bech32 prefix").into());
    }

    // Collect bytes from the decoded Bech32 address
    let bytes: Vec<u8> = hrp_str.byte_iter().collect();

    // Validate the address length
    validate_length(&bytes)?;

    // Return the address as a CanonicalAddr
    Ok(bytes.into())
}

/// Converts a canonical address back to a human-readable address.
fn addr_humanize(canonical: &CanonicalAddr) -> Result<Addr, Error> {
    // Validate the canonical address length
    validate_length(canonical.as_ref())?;

    // Parse the Bech32 prefix
    let prefix =
        Hrp::parse(NEUTRON_BECH32_PREFIX).map_err(|_| Error::msg("Invalid bech32 prefix"))?;

    // Encode the canonical address back to Bech32 format
    encode::<Bech32>(prefix, canonical.as_slice())
        .map(Addr::unchecked)
        .map_err(|_| Error::msg("Bech32 encoding error"))
}

/// Basic validation for the number of bytes in a canonical address
fn validate_length(bytes: &[u8]) -> Result<()> {
    match bytes.len() {
        1..=255 => Ok(()),
        _ => Err(Error::msg("Invalid canonical address length")),
    }
}

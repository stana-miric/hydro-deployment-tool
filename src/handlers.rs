use crate::authorization::create_execute_messages_for_authorization;
use crate::cli::{PoolInfo, ProgramAction};
use crate::config::Config;
use crate::helpers::{
    approve_library, create_base_account, get_filtered_authorizations,
    instantiate_astro_lper_library, instantiate_astro_withdraw_library,
    instantiate_authorization_and_processor, instantiate_splitter_library,
    transfer_accounts_ownership,
};
use crate::wasm::execute_wasm_contract;
use anyhow::Result;
use cw_ownable;
use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::LibraryAccountType;
use valence_processor_utils;

pub const DELIMITER: &str = "_";
pub const DEPLOY: &str = "deploy";
pub const WITHDRAW: &str = "withdraw";

pub fn create_program(label_prefix: &String, pools: &Vec<PoolInfo>, config: &Config) -> Result<()> {
    println!("Creating program with label {}", label_prefix);

    // Deploy authorization and processor
    let (authorization_address, processor_address) =
        instantiate_authorization_and_processor(&config)?;
    println!("Authorization Address: {}", authorization_address);
    println!("Processor Address: {}", processor_address);

    // Create input account
    let input_account = create_base_account(config)?;
    println!("Input Account Address: {}", input_account);

    // Create multiple output accounts for splitter, liquidity deployment and withdrawal
    let mut split_output_accounts = Vec::new();
    let mut liquidity_output_accounts = Vec::new();
    let mut withdrawal_accounts = Vec::new();

    for _ in 0..pools.len() {
        split_output_accounts.push(create_base_account(config)?.to_string());
        liquidity_output_accounts.push(create_base_account(config)?.to_string());
        withdrawal_accounts.push(create_base_account(config)?.to_string());
    }

    // Instantiate splitter library
    let split_lib_address = instantiate_splitter_library(
        config,
        pools,
        &input_account,
        &split_output_accounts,
        &processor_address,
    )?;

    // Approve splitter library for input and output accounts
    approve_library(config, &input_account, &split_lib_address)?;
    for account in &split_output_accounts {
        approve_library(config, account, &split_lib_address)?;
    }

    // Instantiate Astroport LPer and Astroport Withdrawal libraries and approve them per pool
    let mut astroport_lper_lib_addresses = Vec::new();
    let mut astroport_withdraw_lib_addresses = Vec::new();
    for ((pool, split_output_account), liquidity_output_account) in pools
        .iter()
        .zip(&split_output_accounts)
        .zip(&liquidity_output_accounts)
    {
        // deploy library
        let astroport_lper_lib_address = instantiate_astro_lper_library(
            config,
            pool,
            split_output_account,
            liquidity_output_account,
            &processor_address,
        )?;
        approve_library(config, split_output_account, &astroport_lper_lib_address)?;
        approve_library(
            config,
            liquidity_output_account,
            &astroport_lper_lib_address,
        )?;
        astroport_lper_lib_addresses.push(astroport_lper_lib_address);

        // withdraw library
        let astroport_withdraw_lib_address = instantiate_astro_withdraw_library(
            config,
            pool,
            liquidity_output_account,
            &processor_address,
        )?;
        approve_library(
            config,
            liquidity_output_account,
            &astroport_withdraw_lib_address,
        )?;
        astroport_withdraw_lib_addresses.push(astroport_withdraw_lib_address);
    }

    // Create deployment subroutine
    let mut deploy_subroutine_builder = AtomicSubroutineBuilder::new();

    // Add the splitter function
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

    // Add all LPER library functions
    for astroport_lper_lib_address in &astroport_lper_lib_addresses {
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

    // Create a single authorization with all functions
    let deploy_authorization = AuthorizationBuilder::new()
        .with_label(&format!("{}{}{}", label_prefix, DELIMITER, DEPLOY))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![config.tool_operator_address.to_string()]),
        ))
        .with_subroutine(deploy_subroutine_builder.build())
        .build();

    // Create withdraw subroutine
    let mut withdraw_subroutine_builder = AtomicSubroutineBuilder::new();

    // Add all withdraw library functions
    for astroport_withdraw_lib_address in &astroport_withdraw_lib_addresses {
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

    // Create a single withdraw authorization with all functions
    let withdraw_authorization = AuthorizationBuilder::new()
        .with_label(&format!("{}{}{}", label_prefix, DELIMITER, WITHDRAW))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![config.tool_operator_address.to_string()]),
        ))
        .with_subroutine(withdraw_subroutine_builder.build())
        .build();

    let create_authorization_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
            authorizations: vec![deploy_authorization, withdraw_authorization],
        },
    );

    // Create authorizations
    execute_wasm_contract(
        &authorization_address,
        &serde_json::to_string(&create_authorization_msg)?,
        config,
    )?;

    // Transfer ownership of authorization contract
    let update_auth_ownership_msg = valence_authorization_utils::msg::ExecuteMsg::UpdateOwnership(
        cw_ownable::Action::TransferOwnership {
            new_owner: config.neutron_dao_committee_address.to_string(),
            expiry: None,
        },
    );

    execute_wasm_contract(
        &authorization_address,
        &serde_json::to_string(&update_auth_ownership_msg)?,
        config,
    )?;

    // Transfer ownership of the valence accounts
    let mut all_accounts = Vec::new();
    all_accounts.push(input_account.clone());
    all_accounts.extend_from_slice(&split_output_accounts);
    all_accounts.extend_from_slice(&liquidity_output_accounts);
    all_accounts.extend_from_slice(&withdrawal_accounts);

    transfer_accounts_ownership(
        &config,
        &all_accounts,
        &config.neutron_dao_committee_address,
    )?;

    println!("Ownership change needs to be accepted for the following accounts:");
    for acc in &all_accounts {
        println!("{}", acc);
    }

    Ok(())
}

pub fn execute_program(
    auth_contract_address: &str,
    action: ProgramAction,
    config: &Config,
) -> Result<()> {
    // Get the filtered authorizations based on action type
    let authorizations = get_filtered_authorizations(auth_contract_address, action, config)?;

    // Process each authorization and create messages
    for authorization in authorizations {
        let messages = create_execute_messages_for_authorization(&authorization)?;

        // Create SendMsgs
        let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                label: authorization.label.clone(),
                messages,
                ttl: None,
            },
        );

        // Execute contract call
        execute_wasm_contract(
            auth_contract_address,
            &serde_json::to_string(&send_msg)?,
            config,
        )?;
    }

    Ok(())
}

pub fn tick_processor(processor_contract_address: &String, config: &Config) -> Result<()> {
    let tick_msg = valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_processor_utils::msg::PermissionlessMsg::Tick {},
    );

    execute_wasm_contract(
        &processor_contract_address,
        &serde_json::to_string(&tick_msg)?,
        config,
    )?;

    Ok(())
}

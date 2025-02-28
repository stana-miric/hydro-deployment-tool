use crate::authorization::create_execute_messages_for_authorization;
use crate::cli::{PoolInfo, ProgramAction};
use crate::config::Config;
use crate::helpers::{
    approve_library, build_deploy_subroutine, build_withdraw_subroutine,
    create_and_execute_authorization, create_base_account, create_output_accounts,
    get_filtered_authorizations, instantiate_and_approve_astroport_libraries,
    instantiate_authorization_and_processor, instantiate_splitter_library, transfer_ownership,
};
use crate::wasm::execute_wasm_contract;
use anyhow::Result;
use valence_processor_utils;

pub fn create_program(label_prefix: &String, pools: &Vec<PoolInfo>, config: &Config) -> Result<()> {
    println!("Creating program with label {} ...", label_prefix);

    // Deploy authorization and processor
    let (authorization_address, processor_address) =
        instantiate_authorization_and_processor(&config)?;

    // Create input account
    let input_account = create_base_account(config)?;
    println!("Input Account Address: {}", input_account);

    let (split_output_accounts, liquidity_output_accounts, withdrawal_accounts) =
        create_output_accounts(&config, &pools)?;

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
    let (astroport_lper_lib_addresses, astroport_withdraw_lib_addresses) =
        instantiate_and_approve_astroport_libraries(
            &config,
            &pools,
            &split_output_accounts,
            &liquidity_output_accounts,
            &processor_address,
        )?;

    // Create deployment subroutines
    let deploy_subroutine =
        build_deploy_subroutine(&split_lib_address, &astroport_lper_lib_addresses);
    let withdraw_subroutine = build_withdraw_subroutine(&astroport_withdraw_lib_addresses);

    // Create Authorization Messages and Execute
    create_and_execute_authorization(
        &authorization_address,
        deploy_subroutine,
        withdraw_subroutine,
        &config,
        label_prefix,
    )?;

    // Transfer Ownership Athorization Contract and of Valence Accounts
    transfer_ownership(
        &config,
        &authorization_address,
        &input_account,
        &split_output_accounts,
        &liquidity_output_accounts,
        &withdrawal_accounts,
    )?;

    println!("Deployment completed successfully!");
    Ok(())
}

pub fn execute_program(
    auth_contract_address: &str,
    action: ProgramAction,
    config: &Config,
) -> Result<()> {
    println!(
        "Executing program for contract {} ...",
        auth_contract_address
    );
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
    println!(
        "Ticking the processor on address {} ...",
        processor_contract_address
    );
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

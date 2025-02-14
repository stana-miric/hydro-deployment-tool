use crate::cli::PoolInfo;
use crate::config::Config;
use crate::helpers::{
    approve_library, build_tx_flags, create_base_account, instantiate_astro_lper_library,
    instantiate_splitter_library, run_command,
};
use crate::wasm::execute_wasm_contract;
use anyhow::Result;
use cosmwasm_std::{to_json_vec, Binary};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_library_utils::LibraryAccountType;
use valence_processor_utils;
use valence_splitter_library::msg::FunctionMsgs;

pub fn create_authorization(label: &String, pool: &PoolInfo, config: &Config) -> Result<()> {
    println!(
        "Creating authorization: contract_address={} for Pool: {}, Amount: {}, Denoms: {}/{}",
        config.auth_contract_address, pool.address, pool.amount, pool.denom_a, pool.denom_b
    );

    // Create input and output accounts
    let input_account = create_base_account(config)?;
    let split_output_account = create_base_account(config)?;
    let liquidity_account = create_base_account(config)?;

    // Instantiate libraries
    let split_lib_address =
        instantiate_splitter_library(config, pool, &input_account, &split_output_account)?;
    let astroport_lper_lib_address =
        instantiate_astro_lper_library(config, pool, &split_output_account, &liquidity_account)?;

    // Add library approvals
    approve_library(config, &input_account, &split_lib_address)?;
    approve_library(config, &split_output_account, &split_lib_address)?;
    approve_library(config, &split_output_account, &astroport_lper_lib_address)?;
    approve_library(config, &liquidity_account, &astroport_lper_lib_address)?;

    // Create authorization
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label(label)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            split_lib_address.to_string(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(
                                    vec!["process_function".to_string(), "split".to_string()],
                                )]),
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            astroport_lper_lib_address.to_string(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(
                                    vec![
                                        "process_function".to_string(),
                                        "provide_double_sided_liquidity".to_string(),
                                    ],
                                )]),
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    let create_authorization_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    // Execute contract call
    execute_wasm_contract(
        &config.auth_contract_address,
        &serde_json::to_string(&create_authorization_msg)?,
        config,
    )?;

    Ok(())
}

pub fn execute_authorization(label: &String, config: &Config) -> Result<()> {
    println!(
        "Sending message to {} : {}",
        config.auth_contract_address, label
    );

    // Sending split message to authorization contract
    let split_bin = Binary::from(
        to_json_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                FunctionMsgs::Split {},
            ),
        )
        .unwrap(),
    );
    let split_msg = ProcessorMessage::CosmwasmExecuteMsg { msg: split_bin };

    let astro_lper_bin = Binary::from(
        to_json_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                    expected_pool_ratio_range: None,
                },
            ),
        )
        .unwrap(),
    );
    let astro_lper_msg = ProcessorMessage::CosmwasmExecuteMsg {
        msg: astro_lper_bin,
    };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: label.to_string(),
            messages: vec![split_msg, astro_lper_msg],
            ttl: None,
        },
    );

    // Execute contract call
    execute_wasm_contract(
        &config.auth_contract_address,
        &serde_json::to_string(&send_msg)?,
        config,
    )?;

    Ok(())
}

pub fn tick_processor(config: &Config) -> Result<()> {
    let tick_msg = valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_processor_utils::msg::PermissionlessMsg::Tick {},
    );

    execute_wasm_contract(
        &config.processor_contract_address,
        &serde_json::to_string(&tick_msg)?,
        config,
    )?;

    Ok(())
}

pub fn fund_program(destination: &str, funds: &str, config: &Config) -> Result<()> {
    let flags = build_tx_flags(config);
    let cmd = format!(
        "{} tx bank send {} {} {} {}",
        config.node_binary, config.liquidity_deployer_address, destination, funds, flags
    );

    run_command(&cmd)?;
    Ok(())
}

use crate::{cli::PoolInfo, wasm::instantiate_wasm_contract};
use crate::config::Config;
use crate::wasm::execute_wasm_contract;
use anyhow::Result;
use cosmwasm_std::{to_json_vec, Binary, Decimal};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_library_utils::{
    liquidity_utils::AssetData,
    LibraryAccountType,
    denoms::UncheckedDenom,
};
use valence_account_utils::msg::{InstantiateMsg,ExecuteMsg};
use valence_splitter_library::msg::{FunctionMsgs, LibraryConfig as SpliterLibraryConfig};
use valence_astroport_lper::msg::{LiquidityProviderConfig, LibraryConfig as AstroLperLibraryConfig};

pub fn create_authorization(
    key_moniker: &String,
    pool: &PoolInfo,
    config: &Config,
) -> Result<()> {
    println!(
        "Creating authorization: key_moniker={}, contract_address={} for Pool: {}, Amount: {}, Denoms: {}/{}",
        key_moniker, config.auth_contract_address, pool.address, pool.amount, pool.denom_a, pool.denom_b
    );
    
    // Create input and output accounts
    let input_account = create_base_account(config)?;
    let split_output_account = create_base_account(config)?;
    let liquidity_account = create_base_account(config)?;
    
    // Instantiate libraries
    let split_lib_address = instantiate_splitter_library(config, pool, &input_account,&split_output_account)?;
    let astroport_lper_lib_address = instantiate_astro_lper_library(config, pool, &split_output_account, &liquidity_account)?;

    // Add library approvals
    approve_library(config, &input_account, &split_lib_address)?;
    approve_library(config, &split_output_account, &split_lib_address)?;
    approve_library(config, &split_output_account, &astroport_lper_lib_address)?;
    approve_library(config, &liquidity_account, &astroport_lper_lib_address)?;

    // Create authorization
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(split_lib_address.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: Some(vec![
                                    ParamRestriction::MustBeIncluded(vec![
                                        "process_function".to_string(),
                                        "split".to_string(),
                                    ]),
                                ]),
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(astroport_lper_lib_address.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: Some(vec![
                                    ParamRestriction::MustBeIncluded(vec![
                                        "process_function".to_string(),
                                        "provide_double_sided_liquidity".to_string(),
                                    ]),
                                ]),
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
    execute_wasm_contract(&config.auth_contract_address, &serde_json::to_string(&create_authorization_msg)?, config)?;

    Ok(())
}

pub fn send_msg(key_moniker: &String, msg: &String, config: &Config) -> Result<()> {
    println!(
        "Sending message to {} using key_moniker {}: {}",
        config.auth_contract_address, key_moniker, msg
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
    let astro_lper_msg = ProcessorMessage::CosmwasmExecuteMsg { msg: astro_lper_bin };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![split_msg, astro_lper_msg],
            ttl: None,
        },
    );

    // Execute contract call
    execute_wasm_contract(&config.auth_contract_address, &serde_json::to_string(&send_msg)?, config)?;

    Ok(())
}

fn create_base_account(config: &Config) -> Result<String> {
    let acc_instantiate_msg = InstantiateMsg {
        admin: config.neutron_admin_address.to_string(), // TODO: user provided address to be used
        approved_libraries: vec![],
    };

    let contract_address = instantiate_wasm_contract(config.base_account_code_id,  &serde_json::to_string(&acc_instantiate_msg)?, config)?;
    Ok(contract_address)
}

fn instantiate_splitter_library(config: &Config, pool: &PoolInfo, input_addr: &String, output_addr: &String) -> Result<String> {
    let split_lib_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<SpliterLibraryConfig> {
        owner: config.neutron_admin_address.to_string(),
        processor: config.processor_contract_address.to_string(),
        config: valence_splitter_library::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_addr.clone()),
            splits: vec![
                valence_splitter_library::msg::UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(pool.denom_a.clone()),
                    account: LibraryAccountType::Addr(output_addr.clone()),
                    amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                        Decimal::percent(100),
                    ),
                },
                valence_splitter_library::msg::UncheckedSplitConfig {
                    denom:  UncheckedDenom::Native(pool.denom_b.clone()),
                    account: LibraryAccountType::Addr(output_addr.clone()),
                    amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                        Decimal::percent(100),
                    ),
                },
            ],
        },
    };

    let contract_address = instantiate_wasm_contract(config.astro_lper_code_id,  &serde_json::to_string(&split_lib_instantiate_msg)?, config)?;
    Ok(contract_address)
}

fn instantiate_astro_lper_library(config: &Config, pool: &PoolInfo, input_addr: &String, output_addr: &String) -> Result<String> {
    let astro_lper_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<AstroLperLibraryConfig> {
        owner: config.neutron_admin_address.to_string(),
        processor: config.processor_contract_address.to_string(),
        config: valence_astroport_lper::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_addr.clone()),
            output_addr: LibraryAccountType::Addr(output_addr.clone()),
            pool_addr: pool.address.clone(),
            lp_config: LiquidityProviderConfig {
                pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                    valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
                ),
                asset_data: AssetData {
                    asset1: pool.denom_a.to_string(),
                    asset2: pool.denom_b.to_string(),
                },
                max_spread: None,
            },
        },
    };

    let contract_address = instantiate_wasm_contract(config.astro_lper_code_id,  &serde_json::to_string(&astro_lper_instantiate_msg)?, config)?;
    Ok(contract_address)
}

fn approve_library(config: &Config, account: &String, library_address: &String) -> Result<()> {
    let create_authorization_msg=&ExecuteMsg::ApproveLibrary { library: library_address.clone() };
    execute_wasm_contract(account, &serde_json::to_string(&create_authorization_msg)?, config)?;
    
    Ok(())
}

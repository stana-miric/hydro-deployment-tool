use anyhow::{Error, Result};
use cosmwasm_std::{to_json_vec, Binary};
use serde::Deserialize;
use valence_astroport_lper;
use valence_authorization_utils::msg::ProcessorMessage;
use valence_library_utils::msg::ExecuteMsg;
use valence_splitter_library;

#[derive(Debug, Deserialize)]
pub struct AuthorizationsResponse {
    pub data: Vec<Authorization>,
}

#[derive(Debug, Deserialize)]
pub struct Authorization {
    pub label: String,
    pub subroutine: Subroutine,
}

#[derive(Debug, Deserialize)]
pub struct Subroutine {
    pub atomic: AtomicSubroutine,
}

#[derive(Debug, Deserialize)]
pub struct AtomicSubroutine {
    pub functions: Vec<Function>,
}

#[derive(Debug, Deserialize)]
pub struct Function {
    pub message_details: MessageDetails,
}

#[derive(Debug, Deserialize)]
pub struct MessageDetails {
    pub message: serde_json::Value,
}

fn get_functions_identifiers(authorization: &Authorization) -> Result<Vec<String>, Error> {
    let mut function_identifiers = Vec::new();

    for function in &authorization.subroutine.atomic.functions {
        if let Some(name) = function
            .message_details
            .message
            .get("name")
            .and_then(|v| v.as_str())
        {
            if name == "process_function" {
                let params_restrictions: Vec<serde_json::Value> = function
                    .message_details
                    .message
                    .get("params_restrictions")
                    .and_then(|v| v.as_array())
                    .unwrap_or(&Vec::new())
                    .to_vec();

                for restriction in params_restrictions {
                    if let Some(must_be_included) = restriction
                        .get("must_be_included")
                        .and_then(|v| v.as_array())
                    {
                        for param in must_be_included {
                            if let Some(param_str) = param.as_str() {
                                // Push the relevant function identifiers to the result list
                                match param_str {
                                    "split" => function_identifiers.push("split".to_string()),
                                    "provide_double_sided_liquidity" => function_identifiers
                                        .push("provide_double_sided_liquidity".to_string()),
                                    "withdraw_liquidity" => {
                                        function_identifiers.push("withdraw_liquidity".to_string())
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(function_identifiers)
}

pub fn create_execute_messages_for_authorization(
    authorization: &Authorization,
) -> Result<Vec<ProcessorMessage>, Error> {
    let function_identifiers = get_functions_identifiers(authorization)?;

    let mut messages = Vec::new();

    // Create message for each identifier
    for identifier in function_identifiers {
        match identifier.as_str() {
            "split" => {
                // Create message for "split"
                let split_bin = Binary::from(
                    to_json_vec(&ExecuteMsg::<_, ()>::ProcessFunction(
                        valence_splitter_library::msg::FunctionMsgs::Split {},
                    ))
                    .unwrap(),
                );
                let split_msg = ProcessorMessage::CosmwasmExecuteMsg { msg: split_bin };
                messages.push(split_msg);
            }
            "provide_double_sided_liquidity" => {
                // Create message for "provide_double_sided_liquidity"
                let astro_lper_bin = Binary::from(
                    to_json_vec(&ExecuteMsg::<_, ()>::ProcessFunction(
                        valence_astroport_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                            expected_pool_ratio_range: None,
                        },
                    ))
                    .unwrap(),
                );
                let astro_lper_msg = ProcessorMessage::CosmwasmExecuteMsg {
                    msg: astro_lper_bin,
                };
                messages.push(astro_lper_msg);
            }
            "withdraw_liquidity" => {
                // Create message for "withdraw_liquidity"
                let withdraw_bin = Binary::from(
                    to_json_vec(&ExecuteMsg::<_, ()>::ProcessFunction(
                        valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                            expected_pool_ratio_range: None,
                        },
                    ))
                    .unwrap(),
                );
                let withdraw_msg = ProcessorMessage::CosmwasmExecuteMsg { msg: withdraw_bin };
                messages.push(withdraw_msg);
            }
            _ => {}
        }
    }

    Ok(messages)
}

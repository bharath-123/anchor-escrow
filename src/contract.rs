#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg};
use cw2::set_contract_version;
use moneymarket::market::ExecuteMsg as MoneyMarketExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, PaymentValueResponse, QueryMsg, StateResponse};
use crate::state::{EscrowStatus, State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:anchor-escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        seller: deps.api.addr_validate(msg.seller.as_str())?,
        buyer: deps.api.addr_validate(msg.buyer.as_str())?,
        arbiter: info.sender,
        escrow_status: EscrowStatus::AwaitingPayment,
        payment_denom: "uusd".to_string(),
        buyer_payment: Coin::new(0_u128, "uusd".to_string()),
        anchor_market_contract: deps.api.addr_validate(msg.anchor_market_contract.as_str())?,
        anchor_aust_contract: deps.api.addr_validate(msg.anchor_aust_contract.as_str())?
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => deposit(deps, info, env),
        ExecuteMsg::ConfirmDelivery {} => confirm_delivery(deps, info, env),
    }
}

pub fn deposit(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // only allow buyer to deposit
    if info.sender != state.buyer {
        return Err(ContractError::Unauthorized {});
    }

    // validate the funds
    if info.funds.is_empty() {
        return Err(ContractError::NoFundsSent {});
    }

    if info.funds.len() > 1 {
        return Err(ContractError::MultipleCoinsSent {});
    }

    let payment = info.funds[0].clone();
    if payment.denom.ne(&state.payment_denom) {
        return Err(ContractError::WrongDenomSent {});
    }

    // deposit the ust into anchor

    // store the deposit into the state
    // change the escrow status to awaiting delivery
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.escrow_status = EscrowStatus::AwaitingDelivery;
        state.buyer_payment = payment.clone();
        Ok(state)
    })?;

    Ok(Response::new().add_message(WasmMsg::Execute {
        contract_addr: state.anchor_market_contract.to_string(),
        msg: to_binary(&MoneyMarketExecuteMsg::DepositStable {})?,
        funds: vec![payment]
    }))
}

pub fn confirm_delivery(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    // only allow buyer to call this

    // change the escrow state to delivered

    // query the total amount in anchor

    // pay the initial paid amount in state to the seller

    // send the remaining interest to the buyer.

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetPaymentValue {} => to_binary(&query_payment_value(deps)?)
    }
}

fn query_payment_value(deps: Deps) -> StdResult<PaymentValueResponse> {
    let state = STATE.load(deps.storage)?;

    Ok(PaymentValueResponse {
        payment_value_res: None
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use moneymarket::market::ExecuteMsg as MoneyMarketExecuteMsg;
use moneymarket::querier::compute_tax;

use crate::error::ContractError;
use crate::helpers::{get_aust_balance, get_taxed_coin, get_ust_deposited};
use crate::msg::{
    ExecuteMsg, InstantiateMsg, PaymentValueQueryResponse, PaymentValueResponse, QueryMsg,
    StateResponse,
};
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
        anchor_market_contract: deps
            .api
            .addr_validate(msg.anchor_market_contract.as_str())?,
        anchor_aust_contract: deps.api.addr_validate(msg.anchor_aust_contract.as_str())?,
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
        ExecuteMsg::ClaimDeposits {} => claim_deposits(deps, info, env),
        ExecuteMsg::ConfirmDelivery {} => confirm_delivery(deps, info, env),
    }
}

pub fn deposit(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // an escrow which has been delivered is not usable
    if matches!(state.escrow_status, EscrowStatus::Delivered) {
        return Err(ContractError::AlreadyDelivered {});
    }

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

    let taxed_coin = get_taxed_coin(deps.as_ref(), state.payment_denom.clone(), &payment)?;

    // store the deposit into the state
    // change the escrow status to awaiting delivery
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.escrow_status = EscrowStatus::AwaitingDelivery;
        state.buyer_payment = taxed_coin.clone();
        Ok(state)
    })?;

    // deposit the ust into anchor
    Ok(Response::new().add_message(WasmMsg::Execute {
        contract_addr: state.anchor_market_contract.to_string(),
        msg: to_binary(&MoneyMarketExecuteMsg::DepositStable {})?,
        funds: vec![taxed_coin],
    }))
}

pub fn claim_deposits(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // an escrow which has been delivered is not usable
    if matches!(state.escrow_status, EscrowStatus::Delivered) {
        return Err(ContractError::AlreadyDelivered {});
    }

    // called by the arbiter
    if info.sender != state.arbiter {
        return Err(ContractError::Unauthorized {});
    }

    // check the aust balance of the contract
    let aust_balance = get_aust_balance(deps.querier, &env, &state.anchor_aust_contract)?;

    // send the aust to the market contract to redeem the stables back
    Ok(Response::new().add_message(WasmMsg::Execute {
        contract_addr: state.anchor_aust_contract.to_string(),
        msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
            contract: state.anchor_market_contract.to_string(),
            amount: aust_balance,
            msg: to_binary(&moneymarket::market::Cw20HookMsg::RedeemStable {})?,
        })?,
        funds: vec![],
    }))
}

pub fn confirm_delivery(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // an escrow which has been delivered is not usable
    if matches!(state.escrow_status, EscrowStatus::Delivered) {
        return Err(ContractError::AlreadyDelivered {});
    }

    // only allow buyer to call this
    if info.sender != state.buyer {
        return Err(ContractError::Unauthorized {});
    }

    let mut msgs: Vec<BankMsg> = vec![];

    let total_balance = deps.querier.query_balance(
        env.contract.address.to_string(),
        state.payment_denom.clone(),
    )?;
    // assert if there is enough ust in the system.
    // if there are any losses from anchor we need to pump in the money to make up
    // for the losses as the contract needs to fulfilled
    if total_balance.amount.ne(&state.buyer_payment.amount) {
        return Err(ContractError::FundsNotReceivedFromAnchor {});
    }
    // the remaining accrued interest will be sent back to the buyer.
    let accrued_interest: Uint128 = total_balance
        .amount
        .checked_sub(state.buyer_payment.amount)
        .unwrap_or(Uint128::zero());

    let taxed_buyer_payment = get_taxed_coin(
        deps.as_ref(),
        state.payment_denom.clone(),
        &state.buyer_payment,
    )?;
    msgs.push(BankMsg::Send {
        to_address: state.seller.to_string(),
        amount: vec![taxed_buyer_payment],
    });

    let taxed_interest_payment = get_taxed_coin(
        deps.as_ref(),
        state.payment_denom.clone(),
        &Coin::new(accrued_interest.u128(), state.payment_denom),
    )?;
    msgs.push(BankMsg::Send {
        to_address: state.buyer.to_string(),
        amount: vec![taxed_interest_payment],
    });

    // change the escrow state to delivered
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.escrow_status = EscrowStatus::Delivered;
        Ok(state)
    })?;

    Ok(Response::new().add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetPaymentValue {} => to_binary(&query_payment_value(deps, env)?),
    }
}

fn query_payment_value(deps: Deps, env: Env) -> StdResult<PaymentValueResponse> {
    let state = STATE.load(deps.storage)?;
    let aust_amount = get_aust_balance(deps.querier, &env, &state.anchor_aust_contract)?;
    let ust_amount = get_ust_deposited(
        deps.querier,
        &env,
        &state.anchor_aust_contract,
        &state.anchor_market_contract,
    )?;

    Ok(PaymentValueResponse {
        payment_value_res: Some(PaymentValueQueryResponse {
            aust_amount,
            ust_amount,
        }),
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

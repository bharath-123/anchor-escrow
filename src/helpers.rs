use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Addr, Coin, Deps, Env, Fraction, QuerierWrapper, StdResult, Uint128};
use cw20::BalanceResponse;
use moneymarket::market::EpochStateResponse;
use moneymarket::querier::compute_tax;
use terra_cosmwasm::TerraQuerier;

pub fn get_ust_deposited(
    querier: QuerierWrapper,
    env: &Env,
    aust_token_contract: &Addr,
    money_market_contract: &Addr,
) -> StdResult<Uint256> {
    let aust_balance = get_aust_balance(querier, env, aust_token_contract)?;
    let market_epoch_state: EpochStateResponse = querier.query_wasm_smart(
        money_market_contract.to_string(),
        &moneymarket::market::QueryMsg::EpochState {
            block_height: None,
            distributed_interest: None,
        },
    )?;

    Ok(Uint256::from(aust_balance) * market_epoch_state.exchange_rate)
}

pub fn get_aust_balance(
    querier: QuerierWrapper,
    env: &Env,
    aust_token_contract: &Addr,
) -> StdResult<Uint128> {
    let aust_balance: BalanceResponse = querier.query_wasm_smart(
        aust_token_contract.to_string(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    Ok(aust_balance.balance)
}

pub fn get_taxed_coin(deps: Deps, denom: String, coin: &Coin) -> StdResult<Coin> {
    let computed_tax = Uint128::from(compute_tax(deps, coin)?);

    Ok(Coin::new(
        coin.amount.checked_sub(computed_tax).unwrap().u128(),
        denom.clone(),
    ))
}

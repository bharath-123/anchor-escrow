use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub seller: Addr,
    pub buyer: Addr,
    pub arbiter: Addr,

    pub escrow_status: EscrowStatus,
    pub payment_denom: String,

    // this is the initial buyer payment. we need this to separate out
    // the actual pay from the anchor interest
    pub buyer_payment: Coin,

    pub anchor_market_contract: Addr,
    pub anchor_aust_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum EscrowStatus {
    AwaitingPayment,
    AwaitingDelivery,
    Delivered,
}

pub const STATE: Item<State> = Item::new("state");

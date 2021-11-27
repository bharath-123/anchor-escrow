use crate::state::State;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub buyer: String,
    pub seller: String,
    pub anchor_market_contract: String,
    pub anchor_aust_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Called by buyer, to deposit some funds
    Deposit {},
    // Called by arbiter to claim the deposits stored in anchor back to the smart contract
    // We burn the aust we receive after a deposit. This has to be called inorder to successfully
    // complete the payments
    ClaimDeposits {},
    // Called by buyer after delivery has been done to transfer funds to seller
    ConfirmDelivery {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetState {},
    // returns the value of the payment from anchor
    GetPaymentValue {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PaymentValueQueryResponse {
    // amount of aust minted for the contract
    pub aust_amount: Uint128,
    // amount of ust based on the aust exchange rate
    pub ust_amount: Uint256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub state: State,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PaymentValueResponse {
    pub payment_value_res: Option<PaymentValueQueryResponse>,
}

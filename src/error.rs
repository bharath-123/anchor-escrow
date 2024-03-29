use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No funds sent")]
    NoFundsSent {},

    #[error("Multiple coins sent")]
    MultipleCoinsSent {},

    #[error("Wrong denom sent")]
    WrongDenomSent {},

    #[error("Escrow has been delivered")]
    AlreadyDelivered {},

    #[error("Funds have not been fetched from anchor")]
    FundsNotReceivedFromAnchor {},
}

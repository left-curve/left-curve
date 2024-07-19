//! The bank contract is one of the two "core" contracts required by Grug,
//! meaning contracts that provide core functionalities of the chain, the other
//! being the tax man, which levies transaction fees.
//!
//! The bank contract MUST implement the following two entry points:
//!
//! ```ignore
//! #[grug::export]
//! fn transfer<E>(ctx: TransferCtx, msg: TransferMsg) -> Result<Response, E>;
//!
//! #[grug::export]
//! fn query_bank<E>(ctx: QueryCtx, msg: BankQuery) -> Result<BankQueryResponse, E>;
//! ```
//!
//! All contract MUST implement a `receive` entry point as below. When someone
//! sends a contract coins, the recipient contract is informed of this transfer
//! via this entry point.
//!
//! ```ignore
//! #[grug::export]
//! fn receive<E>(ctx: ReceiveCtx) -> Result<Response, E>;
//! ```
//!
//! Some use cases where this may be useful:
//! - The deveoper wishes to prevent users from sending funds to a contract.
//!   To do this, simply throw an error in the `receive` entry point.
//! - For a "wrapped token" contract, mint the wrapped token on receipt.
//! - Forward the funds to another account.

use {
    crate::{Addr, Coin, Coins},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

/// The execute message that the host provides the bank contract during the
/// `bank_execute` function call.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BankMsg {
    pub from: Addr,
    pub to: Addr,
    pub coins: Coins,
}

/// The query message that the host provides the bank contract during the
/// `bank_query` function call.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BankQuery {
    Balance {
        address: Addr,
        denom: String,
    },
    Balances {
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    Supply {
        denom: String,
    },
    Supplies {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

/// The query response that the bank contract must return during the `bank_query`
/// function call.
///
/// The response MUST match the query. For example, if the host queries
/// `BankQuery::Balance`, the contract must return `BankQueryResponse::Balance`.
/// Returning a different `BankQueryResponse` variant can cause the host to
/// panic and the chain halted.
///
/// This said, we don't consider this a security vulnerability, because bank is
/// a _privileged contract_ that must be approved by governance.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BankQueryResponse {
    Balance(Coin),
    Balances(Coins),
    Supply(Coin),
    Supplies(Coins),
}

impl BankQueryResponse {
    pub fn as_balance(self) -> Coin {
        let BankQueryResponse::Balance(coin) = self else {
            panic!("BankQueryResponse is not Balance");
        };
        coin
    }

    pub fn as_balances(self) -> Coins {
        let BankQueryResponse::Balances(coins) = self else {
            panic!("BankQueryResponse is not Balances");
        };
        coins
    }

    pub fn as_supply(self) -> Coin {
        let BankQueryResponse::Supply(coin) = self else {
            panic!("BankQueryResponse is not Supply");
        };
        coin
    }

    pub fn as_supplies(self) -> Coins {
        let BankQueryResponse::Supplies(coins) = self else {
            panic!("BankQueryResponse is not Supplies");
        };
        coins
    }
}

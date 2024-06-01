//! The bank contract is one of the two "core" contracts required by Grug,
//! meaning contracts that provide core functionalities of the chain, the other
//! being the tax man, which levies transaction fees.
//!
//! The bank contract MUST implement the following two entry points:
//!
//! ```ignore
//! #[grug_export]
//! fn transfer<E>(ctx: TransferCtx, msg: TransferMsg) -> Result<Response, E>;
//!
//! #[grug_export]
//! fn query_bank<E>(ctx: QueryCtx, msg: BankQuery) -> Result<BankQueryResponse, E>;
//! ```
//!
//! All contract MUST implement a `receive` entry point as below. When someone
//! sends a contract coins, the recipient contract is informed of this transfer
//! via this entry point.
//!
//! ```ignore
//! #[grug_export]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TransferMsg {
    pub from: Addr,
    pub to: Addr,
    pub coins: Coins,
}

// Note: The bank contract MUST return query response that matches exactly the
// request. E.g. if the request is BankQuery::Balance, the response must be
// BankQueryResponse::Balance. It cannot be any other enum variant. Otherwise
// the chain may panic and halt.
//
// We consider it safe to make this assumption, because bank is a "core"
// contract, meaning it's not something that anyone can permissionless upload.
// It is set by the developer at chain genesis, and only only updatable by
// governance. We assume that the developer and governance have exercised
// caution when creating their own custom bank contracts.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BankQueryMsg {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

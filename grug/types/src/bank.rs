use {
    crate::{
        Addr, Coin, Coins, QueryBalanceRequest, QueryBalancesRequest, QuerySuppliesRequest,
        QuerySupplyRequest,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    paste::paste,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

/// The execute message that the host provides the bank contract during the
/// `bank_execute` function call.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct BankMsg {
    pub from: Addr,
    pub transfers: BTreeMap<Addr, Coins>,
}

/// The query message that the host provides the bank contract during the
/// `bank_query` function call.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BankQuery {
    Balance(QueryBalanceRequest),
    Balances(QueryBalancesRequest),
    Supply(QuerySupplyRequest),
    Supplies(QuerySuppliesRequest),
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
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BankQueryResponse {
    Balance(Coin),
    Balances(Coins),
    Supply(Coin),
    Supplies(Coins),
}

macro_rules! generate_downcast {
    ($id:ident => $ret:ty) => {
        paste! {
            pub fn [<as_$id:snake>](self) -> $ret {
                match self {
                    BankQueryResponse::$id(value) => value,
                    _ => panic!("BankQueryResponse is not {}", stringify!($id)),
                }
            }
        }
    };
    ($($id:ident => $ret:ty),+ $(,)?) => {
        $(
            generate_downcast!($id => $ret);
        )+
    };
}

impl BankQueryResponse {
    generate_downcast! {
        Balance  => Coin,
        Balances => Coins,
        Supply   => Coin,
        Supplies => Coins,
    }
}

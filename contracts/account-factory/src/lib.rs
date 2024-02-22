//! In CWD, all accounts are smart contracts. There is no such thing as private
//! key accounts or externally-owned accounts (EOAs) as in other executaion
//! frameworks. This makes accounts fully programmable, which opens the potential
//! for advanced features and UX (e.g. alternative authentication methods, margin
//! accounts) but also creates a few challenges:
//!
//! 1. Account addresses are not tied to private keys. There needs to be a way
//!    for wallet apps to determininistically derive account addresses from the
//!    authentication method(s) they use.
//! 2. If using private/public key pairs for authentication, the way key pair
//!    may own more than one accounts. There needs to be a place for wallet apps
//!    to query what accounts a key pair controls.
//! 3. Compared to EOAs, there is one extra step for user onboarding: the user
//!    needs to deploy their account contract, which involves sending an on-chain
//!    transaction. This creates a chicken-and-egg problem - how the user can
//!    send a transaction to create their account, when they don't have an account
//!    in the first place?
//! 4. Continue from the previous problem- who will pay the transaction fee?
//!
//! Our solution to these is this **account factory** contract. During onboarding,
//! once the user has created a key pair off-chain, they will submit a transaction,
//! for which the sender is the account factory. The transaction will invoke a
//! `register_account` method on the account factory, which instantiates an account
//! contract for the user. This solves the aforementioned problems:
//!
//! - For problem (1), account factory uses known formula for choosing the salt
//!   when instantiating accounts:
//!
//!   ```plain
//!   salt := sha256(public_key_type | public_key_bytes | serial)
//!   ```
//!
//!   Where `serial` is an integer number. The first account to ever be created
//!   gets serial number `0`, the next one gets `1`, so on.
//!
//!   With this, the wallet can deterministically derive the account address as:
//!
//!   ```plain
//!   address := sha256(account_factory_address | code_hash | salt)
//!   ```
//!
//! - For problem (2), account factory maintains a registry of accounts, indexed
//!   by the tuple `(public_key, serial)` which the wallet app can query.
//!
//! - For problem (3), the account creation transaction will have the account
//!   factory contract as the sender. The credential will be a signature signed
//!   by the private key that will control the account. Account factory will
//!   verify the signature in the `before_tx` entry point. The message to be
//!   signed is:
//!
//!   ```plain
//!   msg := sha256(b"register_account" | chain_id | serial)
//!   ```
//!
//!   Note that the chain ID is needed to prevent the transaction being replayed
//!   on other chains.
//!
//! - For problem (4), this is a harder challenge... My current best idea is that
//!   the account creation transaction will be free, while the account contract
//!   is to be programmed such that the first time it receives a fund deposit
//!   (bridging from another chain, or withdrawing from a CEX) it will transfer
//!   a fixed amount to the factory as an "account creation fee". This solution
//!   however does not prevent spamming account creations. We need to think a
//!   bit more on this.

#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use {
    anyhow::bail,
    cw_account::PublicKey,
    cw_std::{
        cw_serde, from_json, to_json, Addr, BeforeTxCtx, Binary, Bound, Coins, ExecuteCtx, Hash,
        InstantiateCtx, Map, MapKey, Message, Order, QueryCtx, Response, StdResult, Tx,
    },
    sha2::{Digest, Sha256},
};

pub const SERIALS: Map<&PublicKey, u32> = Map::new("s");
pub const ACCOUNTS: Map<(&PublicKey, u32), Addr> = Map::new("a");

pub const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a new account with the given public key.
    RegisterAccount {
        code_hash: Hash,
        public_key: PublicKey,
    },
}

#[cw_serde]
pub enum QueryMsg {
    /// Get the serial number of a public key. If you register a new account
    /// with the public key, this is the serial number that will be used.
    /// Returns: u32
    Serial {
        public_key: PublicKey,
    },
    /// Enumerate serial numbers of all public keys.
    /// Returns: Vec<SerialsResponseItem>
    Serials {
        start_after: Option<PublicKey>,
        limit: Option<u32>,
    },
    /// Get the type and address of an account given its public key and serial number.
    /// Returns: Addr
    Account {
        public_key: PublicKey,
        serial: u32,
    },
    /// Enumerate all accounts that is owned by the given public key.
    /// Returns: Vec<AccountsResponseItem>
    Accounts {
        public_key: PublicKey,
        start_after: Option<u32>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct SerialsResponseItem {
    pub public_key: PublicKey,
    pub serial: u32,
}

#[cw_serde]
pub struct AccountsResponseItem {
    pub serial: u32,
    pub address: Addr,
}

/// Create the (hashed) sign bytes that the signer needs to sign.
pub fn make_message(chain_id: &str, serial: u32) -> Binary {
    let mut hasher = Sha256::new();
    hasher.update(chain_id.as_bytes());
    hasher.update(&serial.to_be_bytes());
    hasher.finalize().to_vec().into()
}

/// Create the salt taht will be used for instantiating the contract.
pub fn make_salt(public_key: &PublicKey, serial: u32) -> Binary {
    let mut hasher = Sha256::new();
    for raw_key in public_key.raw_keys() {
        hasher.update(raw_key);
    }
    hasher.update(&serial.to_be_bytes());
    hasher.finalize().to_vec().into()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(_ctx: InstantiateCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn before_tx(ctx: BeforeTxCtx, tx: Tx) -> anyhow::Result<Response> {
    if tx.msgs.len() != 0 {
        bail!("transaction must contain exactly one message, got {}", tx.msgs.len());
    }

    let Message::Execute { contract, msg, funds } = &tx.msgs[0] else {
        bail!("message is not execute");
    };

    if contract != ctx.contract {
        bail!("contract address is not the account factory");
    }

    if !funds.is_empty() {
        bail!("do not send funds when creating contracts");
    }

    let ExecuteMsg::RegisterAccount { public_key, .. } = from_json(&msg)?;
    let serial = SERIALS.may_load(ctx.store, &public_key)?.unwrap_or(0);
    let msg_hash = make_message(&ctx.chain_id, serial);
    match public_key {
        PublicKey::Secp256k1(pk) => {
            ctx.secp256k1_verify(msg_hash, tx.credential, pk)?;
        },
        PublicKey::Secp256r1(pk) => {
            ctx.secp256r1_verify(msg_hash, tx.credential, pk)?;
        },
    }

    Ok(Response::new().add_attribute("method", "before_tx"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::RegisterAccount {
            code_hash,
            public_key,
        } => register_account(ctx, code_hash, public_key),
    }
}

pub fn register_account(
    ctx: ExecuteCtx,
    code_hash: Hash,
    public_key: PublicKey,
) -> StdResult<Response> {
    let serial = SERIALS.may_load(ctx.store, &public_key)?.unwrap_or(0);
    let salt = make_salt(&public_key, serial);
    let address = Addr::compute(&ctx.contract, &code_hash, &salt);

    SERIALS.save(ctx.store, &public_key, &(serial + 1))?;
    ACCOUNTS.save(ctx.store, (&public_key, serial), &address)?;

    Ok(Response::new()
        .add_attribute("method", "register_account")
        .add_attribute("serial", serial)
        .add_attribute("address", &address)
        .add_message(Message::Instantiate {
            code_hash,
            msg: to_json(&cw_account::InstantiateMsg {
                public_key,
            })?,
            salt,
            funds: Coins::new_empty(),
            admin: Some(address),
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Serial {
            public_key,
        } => to_json(&query_serial(ctx, public_key)?),
        QueryMsg::Serials {
            start_after,
            limit,
        } => to_json(&query_serials(ctx, start_after, limit)?),
        QueryMsg::Account {
            public_key,
            serial,
        } => to_json(&query_account(ctx, public_key, serial)?),
        QueryMsg::Accounts {
            public_key,
            start_after,
            limit,
        } => to_json(&query_accounts(ctx, public_key, start_after, limit)?),
    }
}

pub fn query_serial(ctx: QueryCtx, public_key: PublicKey) -> StdResult<u32> {
    SERIALS.may_load(ctx.store, &public_key).map(|opt| opt.unwrap_or(0))
}

pub fn query_serials(
    ctx: QueryCtx,
    start_after: Option<PublicKey>,
    limit: Option<u32>,
) -> StdResult<Vec<SerialsResponseItem>> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    SERIALS
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (public_key, serial) = item?;
            Ok(SerialsResponseItem {
                public_key,
                serial,
            })
        })
        .collect()
}

pub fn query_account(ctx: QueryCtx, public_key: PublicKey, serial: u32) -> StdResult<Addr> {
    ACCOUNTS.load(ctx.store, (&public_key, serial))
}

pub fn query_accounts(
    ctx: QueryCtx,
    public_key: PublicKey,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<Vec<AccountsResponseItem>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    ACCOUNTS
        .prefix(&public_key)
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (serial, address) = item?;
            Ok(AccountsResponseItem {
                serial,
                address,
            })
        })
        .collect()
}

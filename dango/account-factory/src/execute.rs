use {
    crate::{
        ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, DEPOSITS, KEYS, KEYS_BY_USER, NEXT_ACCOUNT_INDEX,
        USERS_BY_KEY,
    },
    anyhow::{bail, ensure},
    dango_types::{
        account::{self, multi, single},
        account_factory::{
            Account, AccountParams, AccountType, ExecuteMsg, InstantiateMsg, NewUserSalt, Salt,
            Username,
        },
        auth::Key,
        config::IBC_TRANSFER_KEY,
    },
    grug::{
        Addr, AuthCtx, AuthMode, AuthResponse, Coins, Hash160, Inner, JsonDeExt, Message,
        MutableCtx, Order, Response, StdResult, Storage, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    // Save the code hashes associated with the account types.
    for (account_type, code_hash) in &msg.code_hashes {
        CODE_HASHES.save(ctx.storage, *account_type, code_hash)?;
    }

    for (key_hash, key) in &msg.keys {
        KEYS.save(ctx.storage, *key_hash, key)?;
    }

    let instantiate_msgs = msg
        .users
        .into_iter()
        .map(|(username, key_hash)| {
            onboard_new_user(
                ctx.storage,
                ctx.contract,
                username,
                msg.keys.get(&key_hash).cloned(),
                key_hash,
                false,
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new().add_messages(instantiate_msgs))
}

// A new user who wishes to be onboarded must first make an initial deposit,
// then send a transaction with the account factory as sender, that contains
// exactly one message, to execute the factory itself with `Execute::RegisterUser`.
// This transaction does not need to include any metadata or credential.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, mut tx: Tx) -> anyhow::Result<AuthResponse> {
    let mut msgs = tx.msgs.iter();

    let (Some(Message::Execute { contract, msg, .. }), None) = (msgs.next(), msgs.next()) else {
        bail!("transaction must contain exactly one message");
    };

    let Ok(ExecuteMsg::RegisterUser { .. }) = msg.clone().deserialize_json() else {
        bail!("the execute message must be registering user");
    };

    ensure!(
        contract == ctx.contract,
        "the contract being executed must be the factory itself"
    );

    ensure!(
        tx.data.is_null() && tx.credential.is_null(),
        "unexpected transaction metadata or credential"
    );

    // Whereas normally during `CheckTx`, only `authenticate` and `withhold_fee`
    // are performed, here we also execute the register user message.
    //
    // This is to prevent a spamming attack, where the attacker spams txs that
    // contain an invalid register user message (i.e. the username already
    // exists, or a deposit doesn't exist).
    //
    // If we don't ensure the message is valid using `CheckTx`, the transaction
    // will make it into the mempool, and fail during `FinalizeBlock`, consuming
    // the node's computing power at no cost to the attacker, since the factory
    // is exempt from gas fees.
    //
    // The easy way to prevent this is to simply execute this message during
    // `CheckTx`. If it's invalid, check fails, and the tx is rejected from
    // entering mempool.
    let maybe_msg = if ctx.mode == AuthMode::Check {
        // We already asserted that `tx.msgs` contains exactly one message,
        // so safe to unwrap here.
        Some(tx.msgs.pop().unwrap())
    } else {
        None
    };

    Ok(AuthResponse::new()
        .may_add_message(maybe_msg)
        .request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Deposit { recipient } => deposit(ctx, recipient),
        ExecuteMsg::RegisterUser {
            username,
            key,
            key_hash,
        } => register_user(ctx, username, key, key_hash),
        ExecuteMsg::RegisterAccount { params } => register_account(ctx, params),
        ExecuteMsg::ConfigureSafe { updates } => configure_safe(ctx, updates),
    }
}

fn deposit(ctx: MutableCtx, recipient: Addr) -> anyhow::Result<Response> {
    let ibc_transfer: Addr = ctx.querier.query_app_config(IBC_TRANSFER_KEY)?;

    ensure!(
        ctx.sender == ibc_transfer,
        "only IBC transfer contract can make deposits"
    );

    // 1. If someone makes a depsoit twice, then we simply merge the deposits.
    // 2. We trust the IBC transfer contract is implemented correctly and won't
    // make empty deposits.
    DEPOSITS.update(ctx.storage, &recipient, |maybe_deposit| -> StdResult<_> {
        if let Some(mut existing_deposit) = maybe_deposit {
            for coin in ctx.funds {
                existing_deposit.insert(coin)?;
            }
            Ok(Some(existing_deposit))
        } else {
            Ok(Some(ctx.funds))
        }
    })?;

    Ok(Response::new())
}

fn register_user(
    ctx: MutableCtx,
    username: Username,
    key: Key,
    key_hash: Hash160,
) -> anyhow::Result<Response> {
    // The username must not already exist.
    // We ensure this by asserting there isn't any key already associated with
    // this username, since any existing username necessarily has at least one
    // key associated with it. (However, this key isn't necessarily index 1.)
    if KEYS_BY_USER
        .prefix(&username)
        .keys(ctx.storage, None, None, Order::Ascending)
        .next()
        .is_some()
    {
        bail!("username `{}` already exists", username);
    }

    // Save the key.
    // If the key exists (already used by another account), they must match.
    KEYS.update(ctx.storage, key_hash, |maybe_key| {
        if let Some(existing_key) = maybe_key {
            ensure!(key == existing_key, "reusing an existing key but mismatch");
        }
        Ok(Some(key))
    })?;

    Ok(Response::new().add_message(onboard_new_user(
        ctx.storage,
        ctx.contract,
        username,
        None,
        key_hash,
        true,
    )?))
}

// Onboarding a new user involves saving an initial key, and intantiate an
// initial account, under the username.
fn onboard_new_user(
    storage: &mut dyn Storage,
    factory: Addr,
    username: Username,
    key: Option<Key>,
    key_hash: Hash160,
    must_have_deposit: bool,
) -> StdResult<Message> {
    // A new user's 1st account is always a spot account.
    let code_hash = CODE_HASHES.load(storage, AccountType::Spot)?;

    // Associate the key with the user.
    KEYS_BY_USER.insert(storage, (&username, key_hash))?;
    USERS_BY_KEY.insert(storage, (key_hash, &username))?;

    // Increment the global account index, predict its address, and save the
    // account info under the username.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(storage)?;

    let key = if let Some(key) = key {
        key
    } else {
        KEYS.load(storage, key_hash)?
    };

    let salt = NewUserSalt {
        username: &username,
        key,
        key_hash,
    }
    .into_bytes();

    let address = Addr::compute(factory, code_hash, &salt);

    let funds = if must_have_deposit {
        DEPOSITS.take(storage, &address)?
    } else {
        Coins::new()
    };

    let account = Account {
        index,
        params: AccountParams::Spot(single::Params {
            owner: username.clone(),
        }),
    };

    ACCOUNTS.save(storage, address, &account)?;
    ACCOUNTS_BY_USER.insert(storage, (&username, address))?;

    // Create the message to instantiate this account.
    Message::instantiate(
        code_hash,
        &account::InstantiateMsg {},
        salt,
        funds,
        Some(factory),
    )
}

fn register_account(ctx: MutableCtx, params: AccountParams) -> anyhow::Result<Response> {
    // Basic validations of the account.
    // - For single signature accounts (spot and margin), one can only register
    //   accounts for themself. They cannot register account for another user.
    // - For multisig accounts (Safe), ensure voting threshold isn't greater
    //   than total voting power.
    match &params {
        AccountParams::Spot(params) | AccountParams::Margin(params) => {
            ensure!(
                ACCOUNTS_BY_USER.has(ctx.storage, (&params.owner, ctx.sender)),
                "can't register account for another user"
            );
        },
        AccountParams::Safe(params) => {
            ensure!(
                params.threshold.into_inner() <= params.total_power(),
                "threshold can't be greater than total power"
            );
        },
    }

    // Increment the global account index. This is used in the salt for deriving
    // the account address.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(ctx.storage)?;
    let salt = Salt { index }.into_bytes();

    // Find the code hash based on the account type.
    let code_hash = CODE_HASHES.load(ctx.storage, params.ty())?;

    // Derive the account address.
    let address = Addr::compute(ctx.contract, code_hash, &salt);

    // Save the account info.
    let account = Account { index, params };
    ACCOUNTS.save(ctx.storage, address, &account)?;

    // Save the account ownership info.
    match &account.params {
        AccountParams::Spot(params) | AccountParams::Margin(params) => {
            ACCOUNTS_BY_USER.insert(ctx.storage, (&params.owner, address))?;
        },
        AccountParams::Safe(params) => {
            for member in params.members.keys() {
                ACCOUNTS_BY_USER.insert(ctx.storage, (member, address))?;
            }
        },
    }

    Ok(Response::new().add_message(Message::instantiate(
        code_hash,
        &account::InstantiateMsg {},
        salt,
        ctx.funds,
        Some(ctx.contract),
    )?))
}

fn configure_safe(ctx: MutableCtx, updates: multi::ParamUpdates) -> anyhow::Result<Response> {
    for member in updates.members.add().keys() {
        ACCOUNTS_BY_USER.insert(ctx.storage, (member, ctx.sender))?;
    }

    for member in updates.members.remove() {
        ACCOUNTS_BY_USER.remove(ctx.storage, (member, ctx.sender));
    }

    ACCOUNTS.update(ctx.storage, ctx.sender, |maybe_account| {
        let mut account = maybe_account.unwrap();

        match &mut account.params {
            AccountParams::Safe(params) => {
                params.apply_updates(updates);

                ensure!(
                    params.threshold.into_inner() <= params.total_power(),
                    "threshold can't be greater than total power"
                );
            },
            _ => bail!("account isn't a Safe"),
        }

        Ok(Some(account))
    })?;

    Ok(Response::new())
}

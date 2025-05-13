use {
    crate::{
        ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, KEYS, MINIMUM_DEPOSIT, NEXT_ACCOUNT_INDEX,
        USERNAMES_BY_KEY,
    },
    anyhow::{bail, ensure},
    dango_auth::{VerifyData, verify_signature},
    dango_types::{
        account::{self, single},
        account_factory::{
            Account, AccountDisowned, AccountOwned, AccountParamUpdates, AccountParams,
            AccountRegistered, AccountType, ExecuteMsg, InstantiateMsg, KeyDisowned, KeyOwned,
            NewUserSalt, RegisterUserData, Salt, UserRegistered, Username,
        },
        auth::{Key, Signature},
    },
    grug::{
        Addr, AuthCtx, AuthMode, AuthResponse, Coins, EventBuilder, Hash256, Inner, JsonDeExt,
        Message, MsgExecute, MutableCtx, Op, Order, Response, StdResult, Storage, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    // Save the code hashes associated with the account types.
    for (account_type, code_hash) in &msg.code_hashes {
        CODE_HASHES.save(ctx.storage, *account_type, code_hash)?;
    }

    // During genesis:
    // 1. We use an incremental number, which should equal the account's index,
    //    as the secret.
    // 2. Minimum deposit is not required.
    let instantiate_data = msg
        .users
        .into_iter()
        .enumerate()
        .map(|(seed, (username, (key_hash, key)))| {
            KEYS.save(ctx.storage, (&username, key_hash), &key)?;
            USERNAMES_BY_KEY.insert(ctx.storage, (key_hash, &username))?;

            let (msg, user_registered, account_registered) = onboard_new_user(
                ctx.storage,
                ctx.contract,
                username,
                key,
                key_hash,
                seed as u32,
                Coins::default(),
            )?;

            Ok((msg, (user_registered, account_registered)))
        })
        .collect::<StdResult<Vec<_>>>()?;

    let (instantiate_msgs, (users_registered, accounts_registered)): (Vec<_>, (Vec<_>, Vec<_>)) =
        instantiate_data.into_iter().unzip();

    MINIMUM_DEPOSIT.save(ctx.storage, &msg.minimum_deposit)?;

    Response::new()
        .add_messages(instantiate_msgs)
        .add_events(users_registered)?
        .add_events(accounts_registered)
}

// A new user who wishes to be onboarded must first make an initial deposit,
// then send a transaction with the account factory as sender, that contains
// exactly one message, to execute the factory itself with `Execute::RegisterUser`.
// This transaction does not need to include any metadata or credential.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    let mut msgs = tx.msgs.iter();

    let (Some(Message::Execute(MsgExecute { contract, msg, .. })), None) =
        (msgs.next(), msgs.next())
    else {
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
        Some(tx.msgs.into_inner().pop().unwrap())
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
        ExecuteMsg::RegisterUser {
            username,
            key,
            key_hash,
            seed,
            signature,
        } => register_user(ctx, username, key, key_hash, seed, signature),
        ExecuteMsg::RegisterAccount { params } => register_account(ctx, params),
        ExecuteMsg::UpdateKey { key_hash, key } => update_key(ctx, key_hash, key),
        ExecuteMsg::UpdateAccount(updates) => update_account(ctx, updates),
    }
}

fn register_user(
    ctx: MutableCtx,
    username: Username,
    key: Key,
    key_hash: Hash256,
    seed: u32,
    signature: Signature,
) -> anyhow::Result<Response> {
    // Verify the signature is valid.
    verify_signature(
        ctx.api,
        key,
        signature,
        VerifyData::Onboard(RegisterUserData {
            username: username.clone(),
            chain_id: ctx.chain_id,
        }),
    )?;

    // The username must not already exist.
    // We ensure this by asserting there isn't any key already associated with
    // this username, since any existing username necessarily has at least one
    // key associated with it. (However, this key isn't necessarily index 1.)
    ensure!(
        KEYS.prefix(&username)
            .keys(ctx.storage, None, None, Order::Ascending)
            .next()
            .is_none(),
        "username `{username}` already exists"
    );

    // Save the key.
    KEYS.save(ctx.storage, (&username, key_hash), &key)?;
    USERNAMES_BY_KEY.insert(ctx.storage, (key_hash, &username))?;

    let minimum_deposit = MINIMUM_DEPOSIT.load(ctx.storage)?;

    let (msg, user_registered, account_registered) = onboard_new_user(
        ctx.storage,
        ctx.contract,
        username,
        key,
        key_hash,
        seed,
        minimum_deposit,
    )?;

    Ok(Response::new()
        .add_message(msg)
        .add_event(user_registered)?
        .add_event(account_registered)?)
}

/// Onboarding a new user involves saving the initial key, and instantiate an
/// initial account, under the username.
///
/// ## Returns
///
/// - The message to instantiate the account.
/// - The event indicating a new user has registered.
/// - The event indicating a new account has been created.
fn onboard_new_user(
    storage: &mut dyn Storage,
    factory: Addr,
    username: Username,
    key: Key,
    key_hash: Hash256,
    seed: u32,
    minimum_deposit: Coins,
) -> StdResult<(Message, UserRegistered, AccountRegistered)> {
    // A new user's 1st account is always a spot account.
    let code_hash = CODE_HASHES.load(storage, AccountType::Spot)?;

    // Increment the global account index, predict its address, and save the
    // account info under the username.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(storage)?;

    // Derive the account address.
    let salt = NewUserSalt {
        key,
        key_hash,
        seed,
    };
    let address = Addr::derive(factory, code_hash, &salt.to_bytes());

    let account = Account {
        index,
        params: AccountParams::Spot(single::Params::new(username.clone())),
    };

    ACCOUNTS.save(storage, address, &account)?;
    ACCOUNTS_BY_USER.insert(storage, (&username, address))?;

    Ok((
        Message::instantiate(
            code_hash,
            &account::spot::InstantiateMsg { minimum_deposit },
            salt,
            Some(format!("dango/account/{}/{}", AccountType::Spot, index)),
            Some(factory),
            Coins::default(),
        )?,
        UserRegistered {
            username,
            key,
            key_hash,
        },
        AccountRegistered {
            address,
            params: account.params,
            index,
        },
    ))
}

fn register_account(ctx: MutableCtx, params: AccountParams) -> anyhow::Result<Response> {
    // Basic validations of the account.
    // - For single signature accounts (spot and margin), one can only register
    //   accounts for themself. They cannot register account for another user.
    // - For multisig accounts, ensure voting threshold isn't greater than total
    //   voting power.
    match &params {
        AccountParams::Spot(params) | AccountParams::Margin(params) => {
            ensure!(
                ACCOUNTS_BY_USER.has(ctx.storage, (&params.owner, ctx.sender)),
                "can't register account for another user"
            );
        },
        AccountParams::Multi(params) => {
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
    let address = Addr::derive(ctx.contract, code_hash, &salt);

    // Save the account info.
    let account = Account { index, params };
    ACCOUNTS.save(ctx.storage, address, &account)?;

    // Save the account ownership info.
    match &account.params {
        AccountParams::Spot(params) | AccountParams::Margin(params) => {
            ACCOUNTS_BY_USER.insert(ctx.storage, (&params.owner, address))?;
        },
        AccountParams::Multi(params) => {
            for member in params.members.keys() {
                ACCOUNTS_BY_USER.insert(ctx.storage, (member, address))?;
            }
        },
    }

    Ok(Response::new()
        .add_message(Message::instantiate(
            code_hash,
            &account::spot::InstantiateMsg {
                minimum_deposit: Coins::default(),
            },
            salt,
            Some(format!("dango/account/{}/{}", account.params.ty(), index)),
            Some(ctx.contract),
            ctx.funds,
        )?)
        .add_events(match &account.params {
            AccountParams::Spot(params) | AccountParams::Margin(params) => {
                vec![AccountOwned {
                    username: params.owner.clone(),
                    address,
                }]
            },
            AccountParams::Multi(params) => params
                .members
                .keys()
                .map(|member| AccountOwned {
                    username: member.clone(),
                    address,
                })
                .collect(),
        })?
        .add_event(AccountRegistered {
            address,
            params: account.params,
            index,
        })?)
}

fn update_key(ctx: MutableCtx, key_hash: Hash256, key: Op<Key>) -> anyhow::Result<Response> {
    // The sender account must be a single signature account.
    // Find the username associated with the account.
    let username = match ACCOUNTS.load(ctx.storage, ctx.sender)? {
        Account {
            params: AccountParams::Spot(params) | AccountParams::Margin(params),
            ..
        } => params.owner,
        _ => bail!("sender is not a single signature account"),
    };

    match key {
        Op::Insert(key) => {
            KEYS.save(ctx.storage, (&username, key_hash), &key)?;
            USERNAMES_BY_KEY.insert(ctx.storage, (key_hash, &username))?;
        },
        Op::Delete => {
            KEYS.remove(ctx.storage, (&username, key_hash));
            USERNAMES_BY_KEY.remove(ctx.storage, (key_hash, &username));

            // Ensure the user hasn't removed every single key associated with
            // their username. There must be at least one remaining.
            ensure!(
                KEYS.prefix(&username)
                    .range(ctx.storage, None, None, Order::Ascending)
                    .next()
                    .is_some(),
                "can't delete the last key associated with username `{username}`"
            );
        },
    }

    Ok(Response::new()
        .may_add_event(if let Op::Insert(key) = key {
            Some(KeyOwned {
                username: username.clone(),
                key_hash,
                key,
            })
        } else {
            None
        })?
        .may_add_event(if let Op::Delete = key {
            Some(KeyDisowned {
                username: username.clone(),
                key_hash,
            })
        } else {
            None
        })?)
}

fn update_account(ctx: MutableCtx, updates: AccountParamUpdates) -> anyhow::Result<Response> {
    let mut account = ACCOUNTS.load(ctx.storage, ctx.sender)?;
    let mut events = EventBuilder::new();

    match (&mut account.params, updates) {
        (AccountParams::Multi(params), AccountParamUpdates::Multi(updates)) => {
            ensure!(
                params.threshold.into_inner() <= params.total_power(),
                "threshold can't be greater than total power"
            );

            for member in updates.members.add().keys() {
                ACCOUNTS_BY_USER.insert(ctx.storage, (member, ctx.sender))?;

                events.push(AccountOwned {
                    username: member.clone(),
                    address: ctx.sender,
                })?;
            }

            for member in updates.members.remove() {
                ACCOUNTS_BY_USER.remove(ctx.storage, (member, ctx.sender));

                events.push(AccountDisowned {
                    username: member.clone(),
                    address: ctx.sender,
                })?;
            }

            params.apply_updates(updates);

            ACCOUNTS.save(ctx.storage, ctx.sender, &account)?;
        },
        (params, updates) => {
            bail!(
                "account type ({}) and param update type ({}) don't match",
                params.ty(),
                updates.ty()
            );
        },
    }

    Ok(Response::new().add_events(events)?)
}

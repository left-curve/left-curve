use {
    crate::{
        ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, KEYS, NEXT_ACCOUNT_INDEX, NEXT_USER_INDEX,
        USER_INDEXES_BY_NAME, USER_NAMES_BY_INDEX, USERS_BY_KEY,
    },
    anyhow::{bail, ensure},
    dango_auth::{VerifyData, verify_signature},
    dango_types::{
        account::{self, single},
        account_factory::{
            Account, AccountDisowned, AccountOwned, AccountParamUpdates, AccountParams,
            AccountRegistered, AccountType, ExecuteMsg, InstantiateMsg, KeyDisowned, KeyOwned,
            NewUserSalt, RegisterUserData, Salt, UserIndex, UserRegistered, Username,
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
        .map(|user| {
            let (msg, user_registered, account_registered) = onboard_new_user(
                ctx.storage,
                ctx.contract,
                user.key,
                user.key_hash,
                user.seed,
                Coins::default(),
            )?;

            let user_index = user_registered.user_index;

            KEYS.save(ctx.storage, (user_index, user.key_hash), &user.key)?;
            USERS_BY_KEY.insert(ctx.storage, (user.key_hash, user_index))?;

            Ok((msg, (user_registered, account_registered)))
        })
        .collect::<StdResult<Vec<_>>>()?;

    let (instantiate_msgs, (users_registered, accounts_registered)): (Vec<_>, (Vec<_>, Vec<_>)) =
        instantiate_data.into_iter().unzip();

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
            key,
            key_hash,
            seed,
            signature,
        } => register_user(ctx, key, key_hash, seed, signature),
        ExecuteMsg::RegisterAccount { params } => register_account(ctx, params),
        ExecuteMsg::UpdateKey { key_hash, key } => update_key(ctx, key_hash, key),
        ExecuteMsg::UpdateAccount(updates) => update_account(ctx, updates),
        ExecuteMsg::UpdateUsername(username) => update_username(ctx, username),
    }
}

fn register_user(
    ctx: MutableCtx,
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
            chain_id: ctx.chain_id,
        }),
    )?;

    let (msg, user_registered, account_registered) =
        onboard_new_user(ctx.storage, ctx.contract, key, key_hash, seed, ctx.funds)?;

    // Save the key.
    KEYS.save(ctx.storage, (user_registered.user_index, key_hash), &key)?;
    USERS_BY_KEY.insert(ctx.storage, (key_hash, user_registered.user_index))?;

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
    key: Key,
    key_hash: Hash256,
    seed: u32,
    funds: Coins,
) -> StdResult<(Message, UserRegistered, AccountRegistered)> {
    // A new user's 1st account is always a spot account.
    let code_hash = CODE_HASHES.load(storage, AccountType::Spot)?;

    // Increment the global user index.
    let (user_index, _) = NEXT_USER_INDEX.increment(storage)?;

    // Increment the global account index.
    let (account_index, _) = NEXT_ACCOUNT_INDEX.increment(storage)?;

    // Derive the account address.
    let salt = NewUserSalt {
        key,
        key_hash,
        seed,
    };
    let address = Addr::derive(factory, code_hash, &salt.to_bytes());

    let account = Account {
        index: account_index,
        params: AccountParams::Spot(single::Params::new(user_index)),
    };

    ACCOUNTS.save(storage, address, &account)?;
    ACCOUNTS_BY_USER.insert(storage, (user_index, address))?;

    Ok((
        Message::instantiate(
            code_hash,
            &account::spot::InstantiateMsg {},
            salt,
            Some(format!(
                "dango/account/{}/{}",
                AccountType::Spot,
                account_index
            )),
            Some(factory),
            funds, // Foward the funds received to the account.
        )?,
        UserRegistered {
            user_index,
            key,
            key_hash,
        },
        AccountRegistered {
            account_index,
            address,
            params: account.params,
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
                ACCOUNTS_BY_USER.has(ctx.storage, (params.owner, ctx.sender)),
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
            ACCOUNTS_BY_USER.insert(ctx.storage, (params.owner, address))?;
        },
        AccountParams::Multi(params) => {
            for member in params.members.keys() {
                ACCOUNTS_BY_USER.insert(ctx.storage, (*member, address))?;
            }
        },
    }

    Ok(Response::new()
        .add_message(Message::instantiate(
            code_hash,
            &account::spot::InstantiateMsg {},
            salt,
            Some(format!("dango/account/{}/{}", account.params.ty(), index)),
            Some(ctx.contract),
            ctx.funds, // Forward the received funds to the account.
        )?)
        .add_events(match &account.params {
            AccountParams::Spot(params) | AccountParams::Margin(params) => {
                vec![AccountOwned {
                    user_index: params.owner,
                    address,
                }]
            },
            AccountParams::Multi(params) => params
                .members
                .keys()
                .map(|member| AccountOwned {
                    user_index: *member,
                    address,
                })
                .collect(),
        })?
        .add_event(AccountRegistered {
            account_index: index,
            address,
            params: account.params,
        })?)
}

fn update_key(ctx: MutableCtx, key_hash: Hash256, key: Op<Key>) -> anyhow::Result<Response> {
    let user_index = get_user_index_of_account_owner(ctx.storage, ctx.sender)?;

    match key {
        Op::Insert(key) => {
            // Ensure the key isn't already associated with the user index.
            ensure!(
                !KEYS
                    .prefix(user_index)
                    .values(ctx.storage, None, None, Order::Ascending)
                    .any(|v| v.is_ok_and(|k| k == key)),
                "key is already associated with user index {user_index}"
            );

            KEYS.save(ctx.storage, (user_index, key_hash), &key)?;
            USERS_BY_KEY.insert(ctx.storage, (key_hash, user_index))?;
        },
        Op::Delete => {
            KEYS.remove(ctx.storage, (user_index, key_hash));
            USERS_BY_KEY.remove(ctx.storage, (key_hash, user_index));

            // Ensure the user hasn't removed every single key associated with
            // their username. There must be at least one remaining.
            ensure!(
                KEYS.prefix(user_index)
                    .range(ctx.storage, None, None, Order::Ascending)
                    .next()
                    .is_some(),
                "can't delete the last key associated with user index {user_index}"
            );
        },
    }

    Ok(Response::new()
        .may_add_event(if let Op::Insert(key) = key {
            Some(KeyOwned {
                user_index,
                key_hash,
                key,
            })
        } else {
            None
        })?
        .may_add_event(if let Op::Delete = key {
            Some(KeyDisowned {
                user_index,
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
                ACCOUNTS_BY_USER.insert(ctx.storage, (*member, ctx.sender))?;

                events.push(AccountOwned {
                    user_index: *member,
                    address: ctx.sender,
                })?;
            }

            for member in updates.members.remove() {
                ACCOUNTS_BY_USER.remove(ctx.storage, (*member, ctx.sender));

                events.push(AccountDisowned {
                    user_index: *member,
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

fn update_username(ctx: MutableCtx, username: Username) -> anyhow::Result<Response> {
    let user_index = get_user_index_of_account_owner(ctx.storage, ctx.sender)?;

    ensure!(
        !USER_NAMES_BY_INDEX.has(ctx.storage, user_index),
        "a username is already associated with user index {user_index}"
    );

    ensure!(
        !USER_INDEXES_BY_NAME.has(ctx.storage, &username),
        "the username `{username}` is already associated with a user index"
    );

    USER_NAMES_BY_INDEX.save(ctx.storage, user_index, &username)?;
    USER_INDEXES_BY_NAME.save(ctx.storage, &username, &user_index)?;

    Ok(Response::new())
}

/// Given an account address,
/// - if it's a single-signature account, return the account owner's user index;
/// - if it's a multi-signature account, error.
fn get_user_index_of_account_owner(
    storage: &dyn Storage,
    address: Addr,
) -> anyhow::Result<UserIndex> {
    let account = ACCOUNTS.load(storage, address)?;
    match account.params {
        AccountParams::Spot(params) | AccountParams::Margin(params) => Ok(params.owner),
        _ => bail!("sender is not a single signature account"),
    }
}

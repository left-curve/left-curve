use {
    crate::{CODE_HASH, MAX_ACCOUNTS_PER_USER, NEXT_ACCOUNT_INDEX, NEXT_USER_INDEX, USERS},
    anyhow::{bail, ensure},
    dango_auth::{VerifyData, verify_signature},
    dango_types::{
        DangoQuerier, account,
        account_factory::{
            AccountOwned, AccountRegistered, ExecuteMsg, InstantiateMsg, KeyDisowned, KeyOwned,
            NewUserSalt, RegisterUserData, Salt, User, UserIndex, UserRegistered, Username,
            UsernameUpdated,
        },
        auth::{Key, Signature},
        perps,
    },
    grug::{
        Addr, AuthCtx, AuthMode, AuthResponse, Coins, Hash256, Inner, JsonDeExt, Message,
        MsgExecute, MutableCtx, Op, Response, StdResult, Storage, Tx, btree_map,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    // Save the code hashes associated with the Dango account contract.
    CODE_HASH.save(ctx.storage, &msg.account_code_hash)?;

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
            referrer,
        } => register_user(ctx, key, key_hash, seed, signature, referrer),
        ExecuteMsg::RegisterAccount {} => register_account(ctx),
        ExecuteMsg::UpdateKey { key_hash, key } => update_key(ctx, key_hash, key),
        ExecuteMsg::UpdateUsername(username) => update_username(ctx, username),
    }
}

fn register_user(
    ctx: MutableCtx,
    key: Key,
    key_hash: Hash256,
    seed: u32,
    signature: Signature,
    referrer: Option<UserIndex>,
) -> anyhow::Result<Response> {
    // Verify the signature is valid.
    // All registration parameters are bound in the signed data to prevent
    // front-running attacks.
    verify_signature(
        ctx.api,
        key,
        signature,
        VerifyData::Onboard(RegisterUserData {
            chain_id: ctx.chain_id,
            key,
            key_hash,
            seed,
            referrer,
        }),
    )?;

    let (msg, user_registered, account_registered) =
        onboard_new_user(ctx.storage, ctx.contract, key, key_hash, seed, ctx.funds)?;

    // If a referrer is provided, send a message to the perps contract to
    // register the referral relationship.
    let maybe_referral_msg = if let Some(referrer) = referrer {
        Some(Message::execute(
            ctx.querier.query_perps()?,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer,
                referee: user_registered.user_index,
            }),
            Coins::default(),
        )?)
    } else {
        None
    };

    Ok(Response::new()
        .add_message(msg)
        .may_add_message(maybe_referral_msg)
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
    // A new user's 1st account is always a single-signature account.
    let code_hash = CODE_HASH.load(storage)?;

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

    let user = User {
        index: user_index,
        name: Username::default_for_index(user_index),
        accounts: btree_map! { account_index => address },
        keys: btree_map! { key_hash => key },
    };

    USERS.save(storage, user_index, &user)?;

    Ok((
        Message::instantiate(
            code_hash,
            &account::InstantiateMsg {
                // A new user's first account is inactive by default.
                // An initial deposit is required to activate it.
                activate: false,
            },
            salt,
            Some(format!("dango/account/single/{account_index}")),
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
            owner: user_index,
        },
    ))
}

fn register_account(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Load the sender's user index.
    let user_index = USERS.idx.by_account.load_key(ctx.storage, ctx.sender)?;

    // Load the user's profile.
    let mut user = USERS.load(ctx.storage, user_index)?;

    // Ensure the sender has not already reached the maximum account count limit.
    ensure!(
        user.accounts.len() < MAX_ACCOUNTS_PER_USER,
        "user {user_index} has reached max account count",
    );

    // Increment the global account index. This is used in the salt for deriving
    // the account address.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(ctx.storage)?;
    let salt = Salt { index }.into_bytes();

    // Find the code hash of the account contract.
    let code_hash = CODE_HASH.load(ctx.storage)?;

    // Derive the account address.
    let address = Addr::derive(ctx.contract, code_hash, &salt);

    // Insert the account to the user's profile.
    user.accounts.insert(index, address);

    // Save the updated user profile.
    USERS.save(ctx.storage, user_index, &user)?;

    Ok(Response::new()
        .add_message(Message::instantiate(
            code_hash,
            &account::InstantiateMsg {
                // While a new user's first account is inactive by default and
                // requires an initial deposit to activate, all subsequent accounts
                // that the user creates are activated upon instantiation.
                activate: true,
            },
            salt,
            Some(format!("dango/account/single/{index}")),
            Some(ctx.contract),
            ctx.funds, // Forward the received funds to the account.
        )?)
        .add_event(AccountOwned {
            user_index,
            address,
        })?
        .add_event(AccountRegistered {
            account_index: index,
            address,
            owner: user_index,
        })?)
}

fn update_key(ctx: MutableCtx, key_hash: Hash256, key: Op<Key>) -> anyhow::Result<Response> {
    let user_index = USERS.idx.by_account.load_key(ctx.storage, ctx.sender)?;
    let mut user = USERS.load(ctx.storage, user_index)?;

    match key {
        Op::Insert(key) => {
            // Ensure the key isn't already associated with the user index.
            ensure!(
                user.keys.values().all(|k| *k != key),
                "key is already associated with user index {user_index}"
            );

            user.keys.insert(key_hash, key);
        },
        Op::Delete => {
            ensure!(
                user.keys.contains_key(&key_hash),
                "user {user_index} doesn't have a key with hash {key_hash}"
            );

            ensure!(
                user.keys.len() > 1,
                "can't delete the last key associated with user index {user_index}"
            );

            user.keys.remove(&key_hash);
        },
    }

    USERS.save(ctx.storage, user_index, &user)?;

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

fn update_username(ctx: MutableCtx, username: Username) -> anyhow::Result<Response> {
    let (user_index, mut user) = USERS.idx.by_account.load(ctx.storage, ctx.sender)?;

    ensure!(
        user.name.is_default(),
        "a custom username is already set for user {user_index}",
    );

    ensure!(
        !username.is_default(),
        "usernames matching 'user_N' are reserved",
    );

    ensure!(
        USERS
            .idx
            .by_name
            .may_load_key(ctx.storage, username.clone())?
            .is_none(),
        "the username `{username}` is already associated with a user index"
    );

    user.name = username.clone();

    USERS.save(ctx.storage, user_index, &user)?;

    Ok(Response::new().add_event(UsernameUpdated {
        user_index,
        username,
    })?)
}

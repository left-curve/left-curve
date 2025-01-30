use {
    crate::{ACCOUNTS, ACCOUNTS_BY_USER, CODE_HASHES, KEYS, NEXT_ACCOUNT_INDEX},
    anyhow::{bail, ensure},
    dango_types::{
        account::{self, multi, single},
        account_factory::{
            Account, AccountParams, AccountType, ExecuteMsg, InstantiateMsg, NewUserSalt, Salt,
            Username,
        },
        auth::Key,
        bank,
    },
    grug::{
        Addr, AuthCtx, AuthMode, AuthResponse, Coins, Hash256, Inner, JsonDeExt, Message,
        MsgExecute, MutableCtx, Op, Order, QuerierExt, Response, StdResult, Storage, Tx,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    // Save the code hashes associated with the account types.
    for (account_type, code_hash) in &msg.code_hashes {
        CODE_HASHES.save(ctx.storage, *account_type, code_hash)?;
    }

    let instantiate_msgs = msg
        .users
        .into_iter()
        .map(|(username, (key_hash, key))| {
            KEYS.save(ctx.storage, (&username, key_hash), &key)?;
            // claim msg can be ignored
            let (init_msg, _) =
                onboard_new_user(ctx.storage, ctx.contract, username, key, key_hash, None)?;

            Ok(init_msg)
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new().add_messages(instantiate_msgs))
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
        } => register_user(ctx, username, key, key_hash),
        ExecuteMsg::RegisterAccount { params } => register_account(ctx, params),
        ExecuteMsg::ConfigureKey { key_hash, key } => configure_key(ctx, key_hash, key),
        ExecuteMsg::ConfigureSafe { updates } => configure_safe(ctx, updates),
    }
}

fn register_user(
    ctx: MutableCtx,
    username: Username,
    key: Key,
    key_hash: Hash256,
) -> anyhow::Result<Response> {
    // The username must not already exist.
    // We ensure this by asserting there isn't any key already associated with
    // this username, since any existing username necessarily has at least one
    // key associated with it. (However, this key isn't necessarily index 1.)
    if KEYS
        .prefix(&username)
        .keys(ctx.storage, None, None, Order::Ascending)
        .next()
        .is_some()
    {
        bail!("username `{}` already exists", username);
    }

    // Save the key.
    KEYS.save(ctx.storage, (&username, key_hash), &key)?;

    let (init_msg, claim_msg) = onboard_new_user(
        ctx.storage,
        ctx.contract,
        username,
        key,
        key_hash,
        Some(ctx.querier.query_bank()?),
    )?;

    Ok(Response::new()
        .add_message(init_msg)
        .may_add_message(claim_msg))
}

// Onboarding a new user involves saving an initial key, and intantiate an
// initial account, under the username.
fn onboard_new_user(
    storage: &mut dyn Storage,
    factory: Addr,
    username: Username,
    key: Key,
    key_hash: Hash256,
    check_bank_deposit: Option<Addr>,
) -> StdResult<(Message, Option<Message>)> {
    // A new user's 1st account is always a spot account.
    let code_hash = CODE_HASHES.load(storage, AccountType::Spot)?;

    // Increment the global account index, predict its address, and save the
    // account info under the username.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(storage)?;

    let salt = NewUserSalt {
        username: &username,
        key,
        key_hash,
    }
    .into_bytes();

    let address = Addr::derive(factory, code_hash, &salt);

    let maybe_claim_msg = if let Some(bank_addr) = check_bank_deposit {
        Some(Message::execute(
            bank_addr,
            &bank::ExecuteMsg::ClaimPendingTransfer { addr: address },
            Coins::default(),
        )?)
    } else {
        None
    };

    let account = Account {
        index,
        params: AccountParams::Spot(single::Params::new(username.clone())),
    };

    ACCOUNTS.save(storage, address, &account)?;
    ACCOUNTS_BY_USER.insert(storage, (&username, address))?;

    // Create the message to instantiate this account.
    let init_msg = Message::instantiate(
        code_hash,
        &account::InstantiateMsg {},
        salt,
        Some(format!("dango/account/{}/{}", AccountType::Spot, index)),
        Some(factory),
        Coins::default(),
    )?;

    Ok((init_msg, maybe_claim_msg))
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
    let address = Addr::derive(ctx.contract, code_hash, &salt);

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
        Some(format!("dango/account/{}/{}", account.params.ty(), index)),
        Some(ctx.contract),
        ctx.funds,
    )?))
}

fn configure_key(ctx: MutableCtx, key_hash: Hash256, key: Op<Key>) -> anyhow::Result<Response> {
    let username = get_username_by_address(ctx.storage, ctx.sender)?;

    match key {
        Op::Insert(key) => KEYS.save(ctx.storage, (&username, key_hash), &key)?,
        Op::Delete => KEYS.remove(ctx.storage, (&username, key_hash)),
    }

    Ok(Response::new())
}

fn configure_safe(ctx: MutableCtx, updates: multi::ParamUpdates) -> anyhow::Result<Response> {
    for member in updates.members.add().keys() {
        ACCOUNTS_BY_USER.insert(ctx.storage, (member, ctx.sender))?;
    }

    for member in updates.members.remove() {
        ACCOUNTS_BY_USER.remove(ctx.storage, (member, ctx.sender));
    }

    ACCOUNTS.update(ctx.storage, ctx.sender, |mut account| {
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

        Ok(account)
    })?;

    Ok(Response::new())
}

fn get_username_by_address(storage: &dyn Storage, address: Addr) -> anyhow::Result<Username> {
    if let AccountParams::Margin(params) | AccountParams::Spot(params) =
        ACCOUNTS.load(storage, address)?.params
    {
        Ok(params.owner)
    } else {
        bail!("account isn't a Spot or Margin account");
    }
}

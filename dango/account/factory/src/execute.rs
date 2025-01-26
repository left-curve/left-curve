use {
    crate::{ACCOUNTS, ACCOUNTS_BY_USER, CONFIG, KEYS, NEXT_ACCOUNT_INDEX},
    anyhow::{bail, ensure},
    dango_types::{
        account::{self, multi, single},
        account_factory::{
            Account, AccountParams, AccountType, ExecuteMsg, InstantiateMsg, NewUserSalt, Salt,
            Username,
        },
        auth::Key,
    },
    grug::{
        Addr, Coins, Hash256, Inner, IsZero, Message, MutableCtx, Op, Order, Response, StdResult,
        Storage,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    // Each genesis user gets a spot account created.
    let code_hash = msg.config.code_hash_for(AccountType::Spot)?;
    let instantiate_msgs = msg
        .users
        .into_iter()
        .map(|(username, (key_hash, key))| {
            KEYS.save(ctx.storage, (&username, key_hash), &key)?;
            onboard_new_user(
                ctx.storage,
                ctx.contract,
                code_hash,
                username,
                key,
                key_hash,
                Coins::new(),
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new().add_messages(instantiate_msgs))
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
    let cfg = CONFIG.load(ctx.storage)?;

    // The initial deposit must contain at least one supported denom and no less
    // than the minimum amount.
    ensure!(
        ctx.funds.iter().any(|coin| {
            let min = cfg.minimum_deposits.amount_of(&coin.denom);
            min.is_non_zero() && *coin.amount >= min
        }),
        "insufficient deposit: {}, must be at least {}",
        ctx.funds,
        cfg.minimum_deposits
    );

    // The username must not already exist.
    // We ensure this by asserting there isn't any key already associated with
    // this username, since any existing username necessarily has at least one
    // key associated with it.
    ensure!(
        KEYS.prefix(&username)
            .keys(ctx.storage, None, None, Order::Ascending)
            .next()
            .is_none(),
        "username `{username}` already exists",
    );

    // Save the key.
    KEYS.save(ctx.storage, (&username, key_hash), &key)?;

    Ok(Response::new().add_message(onboard_new_user(
        ctx.storage,
        ctx.contract,
        cfg.code_hash_for(AccountType::Spot)?,
        username,
        key,
        key_hash,
        ctx.funds,
    )?))
}

// Onboarding a new user involves saving an initial key, and intantiate an
// initial account, under the username.
fn onboard_new_user(
    storage: &mut dyn Storage,
    factory: Addr,
    code_hash: Hash256,
    username: Username,
    key: Key,
    key_hash: Hash256,
    funds: Coins,
) -> StdResult<Message> {
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

    let account = Account {
        index,
        params: AccountParams::Spot(single::Params::new(username.clone())),
    };

    ACCOUNTS.save(storage, address, &account)?;
    ACCOUNTS_BY_USER.insert(storage, (&username, address))?;

    // Create the message to instantiate this account.
    Message::instantiate(
        code_hash,
        &account::InstantiateMsg {},
        salt,
        Some(format!("dango/account/{}/{}", AccountType::Spot, index)),
        Some(factory),
        funds,
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

    let cfg = CONFIG.load(ctx.storage)?;

    // Increment the global account index. This is used in the salt for deriving
    // the account address.
    let (index, _) = NEXT_ACCOUNT_INDEX.increment(ctx.storage)?;
    let salt = Salt { index }.into_bytes();

    // Find the code hash based on the account type.
    let code_hash = cfg.code_hash_for(params.ty())?;

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

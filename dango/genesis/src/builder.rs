use {
    crate::{Codes, Contracts, GenesisOption},
    dango_types::{
        account_factory::{self, AccountType},
        bank,
        config::{AppAddresses, AppConfig, Hyperlane},
        constants::dango,
        dex, gateway, oracle, taxman, vesting, warp,
    },
    grug::{
        Addr, Binary, Coins, Config, Duration, GENESIS_SENDER, GenesisState, Hash256, HashExt,
        IsZero, JsonSerExt, Message, Permission, Permissions, ResultExt, btree_map, btree_set,
        coins,
    },
    hyperlane_types::{isms, mailbox, va},
    serde::Serialize,
};

/// Create the Dango genesis state given a genesis config.
pub fn build_genesis<T>(
    codes: Codes<T>,
    opt: GenesisOption,
) -> anyhow::Result<(GenesisState, Contracts, Vec<Addr>)>
where
    T: Into<Binary>,
{
    let mut msgs = Vec::new();

    // Upload all the codes and compute code hashes.
    let account_factory_code_hash = upload(&mut msgs, codes.account_factory);
    let account_multi_code_hash = upload(&mut msgs, codes.account_multi);
    let account_spot_code_hash = upload(&mut msgs, codes.account_spot);
    let bank_code_hash = upload(&mut msgs, codes.bank);
    let dex_code_hash = upload(&mut msgs, codes.dex);
    let gateway_code_hash = upload(&mut msgs, codes.gateway);
    let hyperlane_ism_code_hash = upload(&mut msgs, codes.hyperlane.ism);
    let hyperlane_mailbox_code_hash = upload(&mut msgs, codes.hyperlane.mailbox);
    let hyperlane_va_code_hash = upload(&mut msgs, codes.hyperlane.va);
    let oracle_code_hash = upload(&mut msgs, codes.oracle);
    let taxman_code_hash = upload(&mut msgs, codes.taxman);
    let vesting_code_hash = upload(&mut msgs, codes.vesting);
    let warp_code_hash = upload(&mut msgs, codes.warp);

    // Instantiate account factory.
    let users = opt
        .account
        .genesis_users
        .iter()
        .map(|user| user.salt.clone())
        .collect();

    // Derive the account factory contract address.
    // This is needed for deriving the genesis account addresses.
    let account_factory = Addr::derive(
        GENESIS_SENDER,
        account_factory_code_hash,
        b"dango/account_factory",
    );

    // Derive the addresses of the genesis accounts.
    let addresses = opt
        .account
        .genesis_users
        .iter()
        .map(|user| {
            let salt = user.salt.to_bytes();
            Addr::derive(account_factory, account_spot_code_hash, &salt)
        })
        .collect::<Vec<_>>();

    // Genesis users starts from user index 0, so the owner's user index is the
    // same as the index in the `addresses` vector.
    let owner = addresses[opt.grug.owner_index as usize];

    let account_factory = instantiate(
        &mut msgs,
        account_factory_code_hash,
        &account_factory::InstantiateMsg {
            code_hashes: btree_map! {
                AccountType::Multi => account_multi_code_hash,
                AccountType::Spot  => account_spot_code_hash,
            },
            users,
        },
        "dango/account_factory",
        "dango/account_factory",
        owner,
    )?;

    // Derive the Hyperlane mailbox contract address.
    // This is needed for the hook and recipient contracts.
    let mailbox = Addr::derive(
        GENESIS_SENDER,
        hyperlane_mailbox_code_hash,
        b"hyperlane/mailbox",
    );

    // Instantiate Hyperlane message ID multisig ISM.
    let ism = instantiate(
        &mut msgs,
        hyperlane_ism_code_hash,
        &isms::multisig::InstantiateMsg {
            validator_sets: opt.hyperlane.ism_validator_sets,
        },
        "hyperlane/ism/multisig",
        "hyperlane/ism/multisig",
        owner,
    )?;

    // Instantiate Warp contract.
    let warp = instantiate(
        &mut msgs,
        warp_code_hash,
        &warp::InstantiateMsg { mailbox },
        "dango/warp",
        "dango/warp",
        owner,
    )?;

    // Instantiate Hyperlane mailbox. Ensure address is the same as the predicted.
    instantiate(
        &mut msgs,
        hyperlane_mailbox_code_hash,
        &mailbox::InstantiateMsg {
            config: mailbox::Config {
                local_domain: opt.hyperlane.local_domain,
                default_ism: ism,
            },
        },
        "hyperlane/mailbox",
        "hyperlane/mailbox",
        owner,
    )
    .should_succeed_and_equal(mailbox);

    // Instantiate Hyperlane validator announce.
    let va = instantiate(
        &mut msgs,
        hyperlane_va_code_hash,
        &va::InstantiateMsg {
            mailbox,
            announce_fee_per_byte: opt.hyperlane.va_announce_fee_per_byte,
        },
        "hyperlane/va",
        "hyperlane/va",
        owner,
    )?;

    // Instantiate the DEX contract.
    let dex = instantiate(
        &mut msgs,
        dex_code_hash,
        &dex::InstantiateMsg {
            pairs: opt.dex.pairs,
        },
        "dango/dex",
        "dango/dex",
        owner,
    )?;

    // Instantiate the gateway contract.
    let gateway = instantiate(
        &mut msgs,
        gateway_code_hash,
        &gateway::InstantiateMsg {
            routes: opt
                .gateway
                .warp_routes
                .into_iter()
                .map(|(part, remote)| (part, warp, remote))
                .collect(),
            rate_limits: opt.gateway.rate_limits,
            withdrawal_fees: opt.gateway.withdrawal_fees,
        },
        "dango/gateway",
        "dango/gateway",
        owner,
    )?;

    // Create the `balances` map needed for instantiating bank.
    let balances = opt
        .account
        .genesis_users
        .into_iter()
        .zip(&addresses)
        .filter_map(|(user, address)| {
            if user.dango_balance.is_non_zero() {
                Some((*address, coins! {
                    dango::DENOM.clone() => user.dango_balance,
                }))
            } else {
                None
            }
        })
        .collect();

    // Instantiate the bank contract.
    let bank = instantiate(
        &mut msgs,
        bank_code_hash,
        &bank::InstantiateMsg {
            balances,
            namespaces: btree_map! {
                dex::NAMESPACE.clone()     => dex,
                gateway::NAMESPACE.clone() => gateway,
            },
            metadatas: opt.bank.metadatas,
        },
        "dango/bank",
        "dango/bank",
        owner,
    )?;

    // Instantiate the taxman contract.
    let taxman = instantiate(
        &mut msgs,
        taxman_code_hash,
        &taxman::InstantiateMsg {
            config: opt.grug.fee_cfg,
        },
        "dango/taxman",
        "dango/taxman",
        owner,
    )?;

    // Instantiate the oracle contract.
    let oracle = instantiate(
        &mut msgs,
        oracle_code_hash,
        &oracle::InstantiateMsg {
            price_sources: opt.oracle.pyth_price_sources,
            trusted_signers: opt.oracle.pyth_trusted_signers,
        },
        "dango/oracle",
        "dango/oracle",
        owner,
    )?;

    // Instantiate the vesting contract.
    let vesting = instantiate(
        &mut msgs,
        vesting_code_hash,
        &vesting::InstantiateMsg {
            unlocking_cliff: opt.vesting.unlocking_cliff,
            unlocking_period: opt.vesting.unlocking_period,
        },
        "dango/vesting",
        "dango/vesting",
        owner,
    )?;

    let contracts = Contracts {
        account_factory,
        bank,
        dex,
        gateway,
        hyperlane: Hyperlane { ism, mailbox, va },
        oracle,
        taxman,
        vesting,
        warp,
    };

    let config = Config {
        owner,
        bank,
        taxman,
        cronjobs: btree_map! {
            dex => Duration::ZERO, // Important: DEX cronjob is to be invoked at end of every block.
            gateway => opt.gateway.rate_limit_refresh_period,
        },
        permissions: Permissions {
            upload: Permission::Nobody,
            instantiate: Permission::Somebodies(btree_set! { account_factory }),
        },
        max_orphan_age: opt.grug.max_orphan_age,
    };

    let app_config = AppConfig {
        addresses: AppAddresses {
            account_factory,
            dex,
            gateway,
            hyperlane: Hyperlane { ism, mailbox, va },
            oracle,
            taxman,
            warp,
        },
        minimum_deposit: opt.account.minimum_deposit,
        ..Default::default()
    };

    let genesis_state = GenesisState {
        config,
        msgs,
        app_config: app_config.to_json_value()?,
    };

    Ok((genesis_state, contracts, addresses))
}

fn upload<B>(msgs: &mut Vec<Message>, code: B) -> Hash256
where
    B: Into<Binary>,
{
    let code = code.into();
    let code_hash = code.hash256();

    msgs.push(Message::upload(code));

    code_hash
}

fn instantiate<M, S, L>(
    msgs: &mut Vec<Message>,
    code_hash: Hash256,
    msg: &M,
    salt: S,
    label: L,
    admin: Addr,
) -> anyhow::Result<Addr>
where
    M: Serialize,
    S: Into<Binary>,
    L: Into<String>,
{
    let salt = salt.into();
    let address = Addr::derive(GENESIS_SENDER, code_hash, &salt);

    msgs.push(Message::instantiate(
        code_hash,
        msg,
        salt,
        Some(label),
        Some(admin),
        Coins::new(),
    )?);

    Ok(address)
}

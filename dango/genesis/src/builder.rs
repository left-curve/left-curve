use {
    crate::{Addresses, Codes, Contracts, GenesisOption},
    dango_types::{
        account_factory::{self, AccountType, NewUserSalt},
        bank, bitcoin,
        config::{AppAddresses, AppConfig, Hyperlane},
        constants::dango,
        dex, gateway, lending, oracle, taxman, vesting, warp,
    },
    grug::{
        Addr, Binary, Coins, Config, Duration, GENESIS_SENDER, GenesisState, Hash256, HashExt,
        IsZero, JsonSerExt, Message, Permission, Permissions, ResultExt, StdResult, btree_map,
        btree_set, coins,
    },
    hyperlane_types::{isms, mailbox, va},
    serde::Serialize,
    std::collections::BTreeMap,
};

/// Create the Dango genesis state given a genesis config.
pub fn build_genesis<T>(
    codes: Codes<T>,
    opt: GenesisOption,
) -> anyhow::Result<(GenesisState, Contracts, Addresses)>
where
    T: Into<Binary>,
{
    let mut msgs = Vec::new();

    // Upload all the codes and compute code hashes.
    let account_factory_code_hash = upload(&mut msgs, codes.account_factory);
    let account_margin_code_hash = upload(&mut msgs, codes.account_margin);
    let account_multi_code_hash = upload(&mut msgs, codes.account_multi);
    let account_spot_code_hash = upload(&mut msgs, codes.account_spot);
    let bank_code_hash = upload(&mut msgs, codes.bank);
    let bitcoin_code_hash = upload(&mut msgs, codes.bitcoin);
    let dex_code_hash = upload(&mut msgs, codes.dex);
    let gateway_code_hash = upload(&mut msgs, codes.gateway);
    let hyperlane_ism_code_hash = upload(&mut msgs, codes.hyperlane.ism);
    let hyperlane_mailbox_code_hash = upload(&mut msgs, codes.hyperlane.mailbox);
    let hyperlane_va_code_hash = upload(&mut msgs, codes.hyperlane.va);
    let lending_code_hash = upload(&mut msgs, codes.lending);
    let oracle_code_hash = upload(&mut msgs, codes.oracle);
    let taxman_code_hash = upload(&mut msgs, codes.taxman);
    let vesting_code_hash = upload(&mut msgs, codes.vesting);
    let warp_code_hash = upload(&mut msgs, codes.warp);

    // Instantiate account factory.
    let users = opt
        .account
        .genesis_users
        .iter()
        .map(|(username, user)| (username.clone(), (user.key_hash, user.key)))
        .collect();

    let account_factory = instantiate(
        &mut msgs,
        account_factory_code_hash,
        &account_factory::InstantiateMsg {
            code_hashes: btree_map! {
                AccountType::Margin => account_margin_code_hash,
                AccountType::Multi  => account_multi_code_hash,
                AccountType::Spot   => account_spot_code_hash,
            },
            users,
            minimum_deposit: opt.account.minimum_deposit,
        },
        "dango/account_factory",
        "dango/account_factory",
    )?;

    // Derive the addresses of the genesis accounts that were just created.
    let addresses = opt
        .account
        .genesis_users
        .iter()
        .enumerate()
        .map(|(seed, (username, user))| {
            let salt = NewUserSalt {
                key: user.key,
                key_hash: user.key_hash,
                seed: seed as u32,
            }
            .to_bytes();
            let address = Addr::derive(account_factory, account_spot_code_hash, &salt);
            Ok((username.clone(), address))
        })
        .collect::<StdResult<BTreeMap<_, _>>>()?;

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
    )?;

    // Instantiate Warp contract.
    let warp = instantiate(
        &mut msgs,
        warp_code_hash,
        &warp::InstantiateMsg { mailbox },
        "dango/warp",
        "dango/warp",
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
    )?;

    // Instantiate the lending pool contract.
    let lending = instantiate(
        &mut msgs,
        lending_code_hash,
        &lending::InstantiateMsg {
            markets: opt.lending.markets,
        },
        "dango/lending",
        "dango/lending",
    )?;

    let bitcoin = instantiate(
        &mut msgs,
        bitcoin_code_hash,
        &bitcoin::InstantiateMsg {
            config: bitcoin::Config {
                network: opt.bitcoin.network,
                vault: opt.bitcoin.vault,
                multisig: opt.bitcoin.multisig,
                sats_per_vbyte: opt.bitcoin.sats_per_vbyte,
                fee_rate_updater: addresses
                    .get(&opt.bitcoin.fee_rate_updater)
                    .unwrap()
                    .clone(),
                minimum_deposit: opt.bitcoin.minimum_deposit,
                max_output_per_tx: opt.bitcoin.max_output_per_tx,
            },
        },
        "dango/bitcoin",
        "dango/bitcoin",
    )?;

    // Instantiate the gateway contract.
    let gateway = instantiate(
        &mut msgs,
        gateway_code_hash,
        &gateway::InstantiateMsg {
            routes: opt
                .gateway
                .routes
                .into_iter()
                .map(|(part, remote)| {
                    let contract = match remote {
                        gateway::Remote::Warp { .. } => warp,
                        gateway::Remote::Bitcoin => bitcoin,
                    };

                    (part, contract, remote)
                })
                .collect(),
            rate_limits: opt.gateway.rate_limits,
            withdrawal_fees: opt.gateway.withdrawal_fees,
        },
        "dango/gateway",
        "dango/gateway",
    )?;

    // Create the `balances` map needed for instantiating bank.
    let balances = opt
        .account
        .genesis_users
        .into_iter()
        .zip(&addresses)
        .filter_map(|((_, user), (_, address))| {
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
                lending::NAMESPACE.clone() => lending,
            },
            metadatas: opt.bank.metadatas,
        },
        "dango/bank",
        "dango/bank",
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
    )?;

    // Instantiate the oracle contract.
    let oracle = instantiate(
        &mut msgs,
        oracle_code_hash,
        &oracle::InstantiateMsg {
            price_sources: opt.oracle.pyth_price_sources,
            guardian_sets: opt.oracle.wormhole_guardian_sets,
        },
        "dango/oracle",
        "dango/oracle",
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
    )?;

    let contracts = Contracts {
        account_factory,
        bank,
        dex,
        gateway,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
        bitcoin,
    };

    let config = Config {
        owner: addresses.get(&opt.grug.owner_username).cloned().unwrap(),
        bank,
        taxman,
        cronjobs: btree_map! {
            dex => Duration::ZERO, // Important: DEX cronjob is to be invoked at end of every block.
            gateway => opt.gateway.rate_limit_refresh_period,
            bitcoin => opt.bitcoin.withdraw_timeout,
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
            lending,
            oracle,
            taxman,
            warp,
            bitcoin,
        },
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
        None,
        Coins::new(),
    )?);

    Ok(address)
}

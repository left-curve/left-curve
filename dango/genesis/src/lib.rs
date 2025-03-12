use {
    dango_types::{
        account_factory::{self, AccountType, NewUserSalt, Username},
        auth::Key,
        bank,
        config::{AppAddresses, AppConfig, Hyperlane},
        dex::{self, PairUpdate},
        lending::{self, InterestRateModel},
        oracle::{self, GuardianSet, GuardianSetIndex, PriceSource},
        taxman, vesting, warp,
    },
    grug::{
        btree_map, btree_set, Addr, Binary, Coin, Coins, Config, ContractBuilder, ContractWrapper,
        Denom, Duration, GenesisState, Hash256, HashExt, JsonSerExt, Message, Permission,
        Permissions, ResultExt, StdResult, GENESIS_SENDER,
    },
    hyperlane_types::{
        isms::{self, multisig::ValidatorSet},
        mailbox::{self, Domain},
        va, Addr32,
    },
    serde::Serialize,
    std::{collections::BTreeMap, fs, io, path::Path},
};

pub type GenesisUsers = BTreeMap<Username, GenesisUser>;

pub type Addresses = BTreeMap<Username, Addr>;

#[grug::derive(Serde)]
pub struct Contracts {
    pub account_factory: Addr,
    pub bank: Addr,
    pub dex: Addr,
    pub hyperlane: Hyperlane<Addr>,
    pub lending: Addr,
    pub oracle: Addr,
    pub taxman: Addr,
    pub vesting: Addr,
    pub warp: Addr,
}

#[derive(Clone, Copy)]
pub struct Codes<T> {
    pub account_factory: T,
    pub account_margin: T,
    pub account_multi: T,
    pub account_spot: T,
    pub bank: T,
    pub dex: T,
    pub hyperlane: Hyperlane<T>,
    pub lending: T,
    pub oracle: T,
    pub taxman: T,
    pub vesting: T,
    pub warp: T,
}

pub struct GenesisUser {
    pub key: Key,
    pub key_hash: Hash256,
    pub balances: Coins,
}

pub struct GenesisConfig<T> {
    /// Smart contract bytecodes.
    pub codes: Codes<T>,
    /// Initial users and their balances.
    /// For each genesis user will be created a spot account.
    pub users: BTreeMap<Username, GenesisUser>,
    /// The minimum deposit required to onboard a user.
    pub account_factory_minimum_deposit: Coins,
    /// A username whose genesis spot account is to be appointed as the owner.
    /// We expect to transfer ownership to a multisig account afterwards.
    pub owner: Username,
    /// Gas fee configuration.
    pub fee_cfg: taxman::Config,
    /// The maximum age a contract bytecode can remain orphaned (not used by any
    /// contract).
    /// Once this time is elapsed, the code is deleted and must be uploaded again.
    pub max_orphan_age: Duration,
    /// Metadata of tokens.
    pub metadatas: BTreeMap<Denom, bank::Metadata>,
    /// Initial Dango DEX trading pairs.
    pub pairs: Vec<PairUpdate>,
    /// Initial Dango lending markets.
    pub markets: BTreeMap<Denom, InterestRateModel>,
    /// Oracle price sources.
    pub price_sources: BTreeMap<Denom, PriceSource>,
    /// Cliff for Dango token unlocking.
    pub unlocking_cliff: Duration,
    /// Period for Dango token unlocking.
    pub unlocking_period: Duration,
    /// Wormhole guardian sets that will sign Pyth VAA messages.
    pub wormhole_guardian_sets: BTreeMap<GuardianSetIndex, GuardianSet>,
    /// Hyperlane domain ID of the local domain.
    pub hyperlane_local_domain: Domain,
    /// Hyperlane validator sets for remote domains.
    pub hyperlane_ism_validator_sets: BTreeMap<Domain, ValidatorSet>,
    /// Hyperlane validator announce fee rate.
    pub hyperlane_va_announce_fee_per_byte: Coin,
    /// Warp token transfer routes.
    pub warp_routes: BTreeMap<(Denom, Domain), Addr32>,
    // TODO: add margin account parameters (collateral powers and liquidation)
}

/// Create genesis contract codes for the Rust VM.
pub fn build_rust_codes() -> Codes<ContractWrapper> {
    let account_factory = ContractBuilder::new(Box::new(dango_account_factory::instantiate))
        .with_execute(Box::new(dango_account_factory::execute))
        .with_query(Box::new(dango_account_factory::query))
        .with_authenticate(Box::new(dango_account_factory::authenticate))
        .build();

    let account_margin = ContractBuilder::new(Box::new(dango_account_margin::instantiate))
        .with_execute(Box::new(dango_account_margin::execute))
        .with_authenticate(Box::new(dango_account_margin::authenticate))
        .with_backrun(Box::new(dango_account_margin::backrun))
        .with_receive(Box::new(dango_account_margin::receive))
        .with_query(Box::new(dango_account_margin::query))
        .build();

    let account_multi = ContractBuilder::new(Box::new(dango_account_multi::instantiate))
        .with_authenticate(Box::new(dango_account_multi::authenticate))
        .with_receive(Box::new(dango_account_multi::receive))
        .with_execute(Box::new(dango_account_multi::execute))
        .with_query(Box::new(dango_account_multi::query))
        .build();

    let account_spot = ContractBuilder::new(Box::new(dango_account_spot::instantiate))
        .with_authenticate(Box::new(dango_account_spot::authenticate))
        .with_receive(Box::new(dango_account_spot::receive))
        .with_query(Box::new(dango_account_spot::query))
        .with_reply(Box::new(dango_account_spot::reply))
        .build();

    let bank = ContractBuilder::new(Box::new(dango_bank::instantiate))
        .with_execute(Box::new(dango_bank::execute))
        .with_query(Box::new(dango_bank::query))
        .with_bank_execute(Box::new(dango_bank::bank_execute))
        .with_bank_query(Box::new(dango_bank::bank_query))
        .build();

    let dex = ContractBuilder::new(Box::new(dango_dex::instantiate))
        .with_execute(Box::new(dango_dex::execute))
        .with_cron_execute(Box::new(dango_dex::cron_execute))
        .with_query(Box::new(dango_dex::query))
        .build();

    let ism = ContractBuilder::new(Box::new(hyperlane_ism::instantiate))
        .with_execute(Box::new(hyperlane_ism::execute))
        .with_query(Box::new(hyperlane_ism::query))
        .build();

    let mailbox = ContractBuilder::new(Box::new(hyperlane_mailbox::instantiate))
        .with_execute(Box::new(hyperlane_mailbox::execute))
        .with_query(Box::new(hyperlane_mailbox::query))
        .build();

    let va = ContractBuilder::new(Box::new(hyperlane_va::instantiate))
        .with_execute(Box::new(hyperlane_va::execute))
        .with_query(Box::new(hyperlane_va::query))
        .build();

    let oracle = ContractBuilder::new(Box::new(dango_oracle::instantiate))
        .with_execute(Box::new(dango_oracle::execute))
        .with_authenticate(Box::new(dango_oracle::authenticate))
        .with_query(Box::new(dango_oracle::query))
        .build();

    let lending = ContractBuilder::new(Box::new(dango_lending::instantiate))
        .with_execute(Box::new(dango_lending::execute))
        .with_query(Box::new(dango_lending::query))
        .build();

    let taxman = ContractBuilder::new(Box::new(dango_taxman::instantiate))
        .with_execute(Box::new(dango_taxman::execute))
        .with_query(Box::new(dango_taxman::query))
        .with_withhold_fee(Box::new(dango_taxman::withhold_fee))
        .with_finalize_fee(Box::new(dango_taxman::finalize_fee))
        .build();

    let vesting = ContractBuilder::new(Box::new(dango_vesting::instantiate))
        .with_execute(Box::new(dango_vesting::execute))
        .with_query(Box::new(dango_vesting::query))
        .build();

    let warp = ContractBuilder::new(Box::new(dango_warp::instantiate))
        .with_execute(Box::new(dango_warp::execute))
        .with_query(Box::new(dango_warp::query))
        .with_cron_execute(Box::new(dango_warp::cron_execute))
        .build();

    Codes {
        account_factory,
        account_margin,
        account_multi,
        account_spot,
        bank,
        dex,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
    }
}

/// Create genesis contract codes from the Wasm VM.
///
/// This isn't used for production, as for mainnet we use the Rust VM for core
/// Dango contracts.
pub fn read_wasm_files(artifacts_dir: &Path) -> io::Result<Codes<Vec<u8>>> {
    let account_factory = fs::read(artifacts_dir.join("dango_account_factory.wasm"))?;
    let account_margin = fs::read(artifacts_dir.join("dango_account_margin.wasm"))?;
    let account_multi = fs::read(artifacts_dir.join("dango_account_multi.wasm"))?;
    let account_spot = fs::read(artifacts_dir.join("dango_account_spot.wasm"))?;
    let bank = fs::read(artifacts_dir.join("dango_bank.wasm"))?;
    let dex = fs::read(artifacts_dir.join("dango_dex.wasm"))?;
    let ism = fs::read(artifacts_dir.join("hyperlane_ism.wasm"))?;
    let mailbox = fs::read(artifacts_dir.join("hyperlane_mailbox.wasm"))?;
    let va = fs::read(artifacts_dir.join("hyperlane_va.wasm"))?;
    let lending = fs::read(artifacts_dir.join("dango_lending.wasm"))?;
    let oracle = fs::read(artifacts_dir.join("dango_oracle.wasm"))?;
    let taxman = fs::read(artifacts_dir.join("dango_taxman.wasm"))?;
    let vesting = fs::read(artifacts_dir.join("dango_vesting.wasm"))?;
    let warp = fs::read(artifacts_dir.join("hyperlane_warp.wasm"))?;

    Ok(Codes {
        account_factory,
        account_margin,
        account_multi,
        account_spot,
        bank,
        dex,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
    })
}

/// Create the Dango genesis state given a genesis config.
pub fn build_genesis<T>(
    GenesisConfig {
        codes,
        users: genesis_users,
        account_factory_minimum_deposit,
        owner,
        fee_cfg,
        max_orphan_age,
        metadatas,
        pairs,
        markets,
        price_sources,
        unlocking_cliff,
        unlocking_period,
        wormhole_guardian_sets,
        hyperlane_local_domain,
        hyperlane_ism_validator_sets,
        hyperlane_va_announce_fee_per_byte,
        // TODO: allow setting warp routes during instantiation
        warp_routes: _,
    }: GenesisConfig<T>,
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
    let dex_code_hash = upload(&mut msgs, codes.dex);
    let hyperlane_ism_code_hash = upload(&mut msgs, codes.hyperlane.ism);
    let hyperlane_mailbox_code_hash = upload(&mut msgs, codes.hyperlane.mailbox);
    let hyperlane_va_code_hash = upload(&mut msgs, codes.hyperlane.va);
    let lending_code_hash = upload(&mut msgs, codes.lending);
    let oracle_code_hash = upload(&mut msgs, codes.oracle);
    let taxman_code_hash = upload(&mut msgs, codes.taxman);
    let vesting_code_hash = upload(&mut msgs, codes.vesting);
    let warp_code_hash = upload(&mut msgs, codes.warp);

    // Instantiate account factory.
    let users = genesis_users
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
            minimum_deposit: account_factory_minimum_deposit,
        },
        "dango/account_factory",
        "dango/account_factory",
    )?;

    // Derive the addresses of the genesis accounts that were just created.
    let addresses = genesis_users
        .iter()
        .enumerate()
        .map(|(secret, (username, user))| {
            let salt = NewUserSalt {
                secret: secret as u32,
                key: user.key,
                key_hash: user.key_hash,
            }
            .into_bytes();
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
            validator_sets: hyperlane_ism_validator_sets,
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
                local_domain: hyperlane_local_domain,
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
            announce_fee_per_byte: hyperlane_va_announce_fee_per_byte,
        },
        "hyperlane/va",
        "hyperlane/va",
    )?;

    // Instantiate the DEX contract.
    let dex = instantiate(
        &mut msgs,
        dex_code_hash,
        &dex::InstantiateMsg { pairs },
        "dango/dex",
        "dango/dex",
    )?;

    // Instantiate the lending pool contract.
    let lending = instantiate(
        &mut msgs,
        lending_code_hash,
        &lending::InstantiateMsg { markets },
        "dango/lending",
        "dango/lending",
    )?;

    // Create the `balances` map needed for instantiating bank.
    let balances = genesis_users
        .into_iter()
        .zip(&addresses)
        .filter_map(|((_, user), (_, address))| {
            if user.balances.is_empty() {
                None
            } else {
                Some((*address, user.balances))
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
                lending::NAMESPACE.clone() => lending,
                warp::NAMESPACE.clone()    => warp,
            },
            metadatas,
        },
        "dango/bank",
        "dango/bank",
    )?;

    // Instantiate the taxman contract.
    let taxman = instantiate(
        &mut msgs,
        taxman_code_hash,
        &taxman::InstantiateMsg { config: fee_cfg },
        "dango/taxman",
        "dango/taxman",
    )?;

    // Instantiate the oracle contract.
    let oracle = instantiate(
        &mut msgs,
        oracle_code_hash,
        &oracle::InstantiateMsg {
            guardian_sets: wormhole_guardian_sets,
            price_sources,
        },
        "dango/oracle",
        "dango/oracle",
    )?;

    let vesting = instantiate(
        &mut msgs,
        vesting_code_hash,
        &vesting::InstantiateMsg {
            unlocking_cliff,
            unlocking_period,
        },
        "dango/vesting",
        "dango/vesting",
    )?;

    let contracts = Contracts {
        account_factory,
        bank,
        dex,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
    };

    let permissions = Permissions {
        upload: Permission::Nobody,
        instantiate: Permission::Somebodies(btree_set! { account_factory }),
    };

    let config = Config {
        owner: addresses.get(&owner).cloned().unwrap(),
        bank,
        taxman,
        // Important: DEX cronjob is to be invoked at end of every block.
        cronjobs: btree_map! {
            dex => Duration::ZERO,
            warp => Duration::from_days(1),
        },
        permissions,
        max_orphan_age,
    };

    let app_config = AppConfig {
        addresses: AppAddresses {
            account_factory,
            dex,
            hyperlane: Hyperlane { ism, mailbox, va },
            lending,
            oracle,
            warp,
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

use {
    dango_types::{
        account_factory::{self, AccountType, NewUserSalt, Username},
        amm::{self, FeeRate},
        auth::Key,
        bank,
        config::{AppAddresses, AppConfig},
        ibc_transfer,
        lending::{self, LendingAppConfig, Market},
        oracle::{
            self, GuardianSet, PriceSource, ETH_USD_ID, GUARDIANS_ADDRESSES, GUARDIAN_SETS_INDEX,
            USDC_USD_ID, WBTC_USD_ID,
        },
        taxman, token_factory,
    },
    grug::{
        btree_map, btree_set, Addr, Binary, Coin, Coins, Config, Denom, Duration, GenesisState,
        Hash160, Hash256, HashExt, Inner, JsonSerExt, Message, NonZero, Permission, Permissions,
        StdResult, Udec128, Uint128, GENESIS_SENDER,
    },
    serde::Serialize,
    std::{collections::BTreeMap, error::Error, fs, io, path::Path, str::FromStr},
};

pub type GenesisUsers = BTreeMap<Username, GenesisUser>;

pub type Addresses = BTreeMap<Username, Addr>;

#[grug::derive(Serde)]
pub struct Contracts {
    pub account_factory: Addr,
    pub amm: Addr,
    pub bank: Addr,
    pub ibc_transfer: Addr,
    pub lending: Addr,
    pub oracle: Addr,
    pub taxman: Addr,
    pub token_factory: Addr,
}

#[derive(Clone, Copy)]
pub struct Codes<T> {
    pub account_factory: T,
    pub account_margin: T,
    pub account_safe: T,
    pub account_spot: T,
    pub amm: T,
    pub bank: T,
    pub ibc_transfer: T,
    pub lending: T,
    pub oracle: T,
    pub taxman: T,
    pub token_factory: T,
}

pub struct GenesisUser {
    pub key: Key,
    pub key_hash: Hash160,
    pub balances: Coins,
}

pub fn read_wasm_files(artifacts_dir: &Path) -> io::Result<Codes<Vec<u8>>> {
    let account_factory = fs::read(artifacts_dir.join("dango_account_factory.wasm"))?;
    let account_margin = fs::read(artifacts_dir.join("dango_account_margin.wasm"))?;
    let account_safe = fs::read(artifacts_dir.join("dango_account_safe.wasm"))?;
    let account_spot = fs::read(artifacts_dir.join("dango_account_spot.wasm"))?;
    let amm = fs::read(artifacts_dir.join("dango_amm.wasm"))?;
    let bank = fs::read(artifacts_dir.join("dango_bank.wasm"))?;
    let ibc_transfer = fs::read(artifacts_dir.join("dango_ibc_transfer.wasm"))?;
    let lending = fs::read(artifacts_dir.join("dango_lending.wasm"))?;
    let oracle = fs::read(artifacts_dir.join("dango_oracle.wasm"))?;
    let taxman = fs::read(artifacts_dir.join("dango_taxman.wasm"))?;
    let token_factory = fs::read(artifacts_dir.join("dango_token_factory.wasm"))?;

    Ok(Codes {
        account_factory,
        account_margin,
        account_safe,
        account_spot,
        amm,
        bank,
        ibc_transfer,
        lending,
        oracle,
        taxman,
        token_factory,
    })
}

pub fn build_genesis<T, D>(
    codes: Codes<T>,
    genesis_users: GenesisUsers,
    owner: &Username,
    fee_denom: D,
    fee_rate: Udec128,
    token_creation_fee: Option<Uint128>,
    max_orphan_age: Duration,
) -> anyhow::Result<(GenesisState, Contracts, Addresses)>
where
    T: Into<Binary>,
    D: TryInto<Denom>,
    D::Error: Error + Send + Sync + 'static,
{
    let mut msgs = Vec::new();

    let fee_denom = fee_denom.try_into()?;

    // Upload all the codes and compute code hashes.
    let account_factory_code_hash = upload(&mut msgs, codes.account_factory);
    let account_margin_code_hash = upload(&mut msgs, codes.account_margin);
    let account_safe_code_hash = upload(&mut msgs, codes.account_safe);
    let account_spot_code_hash = upload(&mut msgs, codes.account_spot);
    let amm_code_hash = upload(&mut msgs, codes.amm);
    let bank_code_hash = upload(&mut msgs, codes.bank);
    let ibc_transfer_code_hash = upload(&mut msgs, codes.ibc_transfer);
    let lending_code_hash = upload(&mut msgs, codes.lending);
    let oracle_code_hash = upload(&mut msgs, codes.oracle);
    let taxman_code_hash = upload(&mut msgs, codes.taxman);
    let token_factory_code_hash = upload(&mut msgs, codes.token_factory);

    // Instantiate account factory.
    let keys = genesis_users
        .values()
        .map(|user| (user.key_hash, user.key))
        .collect();
    let users = genesis_users
        .iter()
        .map(|(username, user)| (username.clone(), user.key_hash))
        .collect();
    let account_factory = instantiate(
        &mut msgs,
        account_factory_code_hash,
        &account_factory::InstantiateMsg {
            code_hashes: btree_map! {
                AccountType::Margin => account_margin_code_hash,
                AccountType::Safe   => account_safe_code_hash,
                AccountType::Spot   => account_spot_code_hash,
            },
            keys,
            users,
        },
        "dango/account_factory",
        "dango/account_factory",
    )?;

    // Derive the addresses of the genesis accounts that were just created.
    let addresses = genesis_users
        .iter()
        .map(|(username, user)| {
            let salt = NewUserSalt {
                username,
                key: user.key,
                key_hash: user.key_hash,
            }
            .into_bytes();
            let address = Addr::derive(account_factory, account_spot_code_hash, &salt);
            Ok((username.clone(), address))
        })
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    // Instantiate the IBC transfer contract.
    let ibc_transfer = instantiate(
        &mut msgs,
        ibc_transfer_code_hash,
        &ibc_transfer::InstantiateMsg {},
        "dango/ibc_transfer",
        "dango/ibc_transfer",
    )?;

    // Instantiate the token factory contract.
    let token_creation_fee = token_creation_fee
        .map(|amount| Coin::new(fee_denom.clone(), amount).and_then(NonZero::new))
        .transpose()?;
    let token_factory = instantiate(
        &mut msgs,
        token_factory_code_hash,
        &token_factory::InstantiateMsg {
            config: token_factory::Config { token_creation_fee },
        },
        "dango/token_factory",
        "dango/token_factory",
    )?;

    // Instantiate the AMM contract.
    let amm = instantiate(
        &mut msgs,
        amm_code_hash,
        &amm::InstantiateMsg {
            config: amm::Config {
                protocol_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(10)), // 0.1%
                pool_creation_fee: NonZero::new_unchecked(Coin::new(
                    fee_denom.clone(),
                    10_000_000,
                )?), // 10 USDC
            },
        },
        "dango/amm",
        "dango/amm",
    )?;

    // Instantiate the lending pool contract.
    let lending = instantiate(
        &mut msgs,
        lending_code_hash,
        &lending::InstantiateMsg {
            markets: btree_map! {
                fee_denom.clone() => Market {
                    // TODO
                }
            },
        },
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

    // Create the `namespaces` map needed for instantiating bank.
    // Token factory gets the "factory" namespace.
    // IBC trasfer gets the "ibc" namespace.
    let namespaces = btree_map! {
        amm::NAMESPACE.clone()           => amm,
        ibc_transfer::NAMESPACE.clone()  => ibc_transfer,
        lending::NAMESPACE.clone()       => lending,
        token_factory::NAMESPACE.clone() => token_factory,
    };

    // Instantiate the bank contract.
    let bank = instantiate(
        &mut msgs,
        bank_code_hash,
        &bank::InstantiateMsg {
            balances,
            namespaces,
        },
        "dango/bank",
        "dango/bank",
    )?;

    // Instantiate the taxman contract.
    let taxman = instantiate(
        &mut msgs,
        taxman_code_hash,
        &taxman::InstantiateMsg {
            config: taxman::Config {
                fee_denom,
                fee_rate,
            },
        },
        "dango/taxman",
        "dango/taxman",
    )?;

    // Instantiate the oracle contract.
    let oracle = instantiate(
        &mut msgs,
        oracle_code_hash,
        &oracle::InstantiateMsg {
            guardian_sets: btree_map! {
                GUARDIAN_SETS_INDEX => GuardianSet {
                    addresses: GUARDIANS_ADDRESSES
                        .into_iter()
                        .map(|addr| {
                            let bytes = Binary::from_str(addr)
                                .unwrap()
                                .into_inner()
                                .try_into()
                                .unwrap();
                            Hash160::from_inner(bytes)
                        })
                        .collect(),
                    expiration_time: None,
                },
            },
            price_sources: btree_map! {
                Denom::from_str("usdc").unwrap() => PriceSource::Pyth { id: USDC_USD_ID, precision: 6 },
                Denom::from_str("btc").unwrap()  => PriceSource::Pyth { id: WBTC_USD_ID, precision: 8 },
                Denom::from_str("eth").unwrap()  => PriceSource::Pyth { id: ETH_USD_ID, precision: 18 },
            },
        },
        "dango/oracle",
        "dango/oracle",
    )?;

    let contracts = Contracts {
        account_factory,
        amm,
        bank,
        ibc_transfer,
        lending,
        oracle,
        taxman,
        token_factory,
    };

    let permissions = Permissions {
        upload: Permission::Nobody,
        instantiate: Permission::Somebodies(btree_set! { account_factory }),
    };

    let config = Config {
        owner: addresses.get(owner).cloned().unwrap(),
        bank,
        taxman,
        cronjobs: BTreeMap::new(),
        permissions,
        max_orphan_age,
    };

    let lending_app_config = LendingAppConfig {
        collateral_powers: btree_map! {},
    };
    let app_config = AppConfig {
        addresses: AppAddresses {
            account_factory,
            ibc_transfer,
            lending,
            oracle,
        },
        lending: lending_app_config,
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

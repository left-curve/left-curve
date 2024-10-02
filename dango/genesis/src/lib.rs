use {
    anyhow::anyhow,
    dango_types::{
        account_factory::{self, AccountType, NewUserSalt, Username},
        amm::{self, FeeRate},
        auth::Key,
        bank,
        config::{ACCOUNT_FACTORY_KEY, IBC_TRANSFER_KEY},
        mock_ibc_transfer, taxman, token_factory,
    },
    grug::{
        btree_map, btree_set, Addr, Binary, Coin, Coins, Config, Denom, GenesisState, Hash160,
        Hash256, HashExt, JsonSerExt, Message, NonZero, Part, Permission, Permissions, StdResult,
        Udec128, Uint128, GENESIS_SENDER,
    },
    serde::Serialize,
    std::{collections::BTreeMap, error::Error, str::FromStr},
};

pub type GenesisUsers = BTreeMap<Username, GenesisUser>;

pub type Addresses = BTreeMap<Username, Addr>;

#[grug::derive(Serde)]
pub struct Contracts {
    pub account_factory: Addr,
    pub amm: Addr,
    pub bank: Addr,
    pub fee_recipient: Addr,
    pub ibc_transfer: Addr,
    pub owner: Addr,
    pub taxman: Addr,
    pub token_factory: Addr,
}

#[derive(Clone, Copy)]
pub struct Codes<T> {
    pub account_factory: T,
    pub account_spot: T,
    pub account_safe: T,
    pub amm: T,
    pub bank: T,
    pub ibc_transfer: T,
    pub taxman: T,
    pub token_factory: T,
}

pub struct GenesisUser {
    pub key: Key,
    pub key_hash: Hash160,
    pub balances: Coins,
}

pub fn build_genesis<T, D>(
    codes: Codes<T>,
    genesis_users: GenesisUsers,
    owner: &Username,
    fee_recipient: &Username,
    fee_denom: D,
    fee_rate: Udec128,
    denom_creation_fee: Uint128,
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
    let account_spot_code_hash = upload(&mut msgs, codes.account_spot);
    let account_safe_code_hash = upload(&mut msgs, codes.account_safe);
    let amm_code_hash = upload(&mut msgs, codes.amm);
    let bank_code_hash = upload(&mut msgs, codes.bank);
    let ibc_transfer_code_hash = upload(&mut msgs, codes.ibc_transfer);
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
                AccountType::Spot => account_spot_code_hash,
                AccountType::Safe => account_safe_code_hash,
            },
            keys,
            users,
        },
        "account_factory",
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
            let address = Addr::compute(account_factory, account_spot_code_hash, &salt);
            Ok((username.clone(), address))
        })
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    // Find the owner and fee recipient addresses.
    let owner = addresses
        .get(owner)
        .cloned()
        .ok_or_else(|| anyhow!("can't find address for username `{owner}`"))?;
    let fee_recipient = addresses
        .get(fee_recipient)
        .cloned()
        .ok_or_else(|| anyhow!("can't find address for username `{fee_recipient}`"))?;

    // Instantiate the IBC transfer contract.
    let ibc_transfer = instantiate(
        &mut msgs,
        ibc_transfer_code_hash,
        &mock_ibc_transfer::InstantiateMsg {},
        "ibc_transfer",
    )?;

    // Instantiate the token factory contract.
    let token_factory = instantiate(
        &mut msgs,
        token_factory_code_hash,
        &token_factory::InstantiateMsg {
            denom_creation_fee: Coin::new(fee_denom.clone(), denom_creation_fee)?,
        },
        "token_factory",
    )?;

    // Instantiate the AMM contract.
    let amm = instantiate(
        &mut msgs,
        amm_code_hash,
        &amm::InstantiateMsg {
            config: amm::Config {
                fee_recipient,
                protocol_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(10)), // 0.1%
                pool_creation_fee: NonZero::new_unchecked(Coin::new(
                    fee_denom.clone(),
                    10_000_000,
                )?), // 10 USDC
            },
        },
        "amm",
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
        Part::from_str(amm::NAMESPACE)? => amm,
        Part::from_str(token_factory::NAMESPACE)? => token_factory,
        Part::from_str(mock_ibc_transfer::NAMESPACE)? => ibc_transfer,
    };

    // Instantiate the bank contract.
    let bank = instantiate(
        &mut msgs,
        bank_code_hash,
        &bank::InstantiateMsg {
            balances,
            namespaces,
        },
        "bank",
    )?;

    // Instantiate the taxman contract.
    let taxman = instantiate(
        &mut msgs,
        taxman_code_hash,
        &taxman::InstantiateMsg {
            config: taxman::Config {
                fee_recipient,
                fee_denom,
                fee_rate,
            },
        },
        "taxman",
    )?;

    let contracts = Contracts {
        account_factory,
        amm,
        bank,
        fee_recipient,
        ibc_transfer,
        owner,
        taxman,
        token_factory,
    };

    let permissions = Permissions {
        upload: Permission::Nobody,
        instantiate: Permission::Somebodies(btree_set! { account_factory }),
    };

    let config = Config {
        owner,
        bank,
        taxman,
        cronjobs: BTreeMap::new(),
        permissions,
    };

    let app_configs = btree_map! {
        ACCOUNT_FACTORY_KEY.to_string() => account_factory.to_json_value()?,
        IBC_TRANSFER_KEY.to_string() => ibc_transfer.to_json_value()?,
    };

    let genesis_state = GenesisState {
        config,
        msgs,
        app_configs,
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

fn instantiate<M, S>(
    msgs: &mut Vec<Message>,
    code_hash: Hash256,
    msg: &M,
    salt: S,
) -> anyhow::Result<Addr>
where
    M: Serialize,
    S: Into<Binary>,
{
    let salt = salt.into();
    let address = Addr::compute(GENESIS_SENDER, code_hash, &salt);

    msgs.push(Message::instantiate(
        code_hash,
        msg,
        salt,
        Coins::new(),
        None,
    )?);

    Ok(address)
}
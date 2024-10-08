use {
    crate::{Accounts, TestAccount},
    dango_genesis::{build_genesis, Codes, Contracts, GenesisUser},
    grug::{
        btree_map, BlockInfo, Coins, ContractBuilder, ContractWrapper, Duration, NumberConst,
        TestSuite, Timestamp, Udec128, Uint128, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT,
    },
};

pub fn setup_test() -> anyhow::Result<(TestSuite, Accounts, Codes<ContractWrapper>, Contracts)> {
    let account_factory = ContractBuilder::new(Box::new(dango_account_factory::instantiate))
        .with_execute(Box::new(dango_account_factory::execute))
        .with_query(Box::new(dango_account_factory::query))
        .with_authenticate(Box::new(dango_account_factory::authenticate))
        .build();

    let account_spot = ContractBuilder::new(Box::new(dango_account_spot::instantiate))
        .with_authenticate(Box::new(dango_account_spot::authenticate))
        .with_receive(Box::new(dango_account_spot::receive))
        .with_query(Box::new(dango_account_spot::query))
        .build();

    let account_safe = ContractBuilder::new(Box::new(dango_account_safe::instantiate))
        .with_authenticate(Box::new(dango_account_safe::authenticate))
        .with_receive(Box::new(dango_account_safe::receive))
        .with_execute(Box::new(dango_account_safe::execute))
        .with_query(Box::new(dango_account_safe::query))
        .build();

    let amm = ContractBuilder::new(Box::new(dango_amm::instantiate))
        .with_execute(Box::new(dango_amm::execute))
        .with_query(Box::new(dango_amm::query))
        .build();

    let bank = ContractBuilder::new(Box::new(dango_bank::instantiate))
        .with_execute(Box::new(dango_bank::execute))
        .with_bank_execute(Box::new(dango_bank::bank_execute))
        .with_bank_query(Box::new(dango_bank::bank_query))
        .build();

    let ibc_transfer = ContractBuilder::new(Box::new(dango_ibc_transfer::instantiate))
        .with_execute(Box::new(dango_ibc_transfer::execute))
        .build();

    let taxman = ContractBuilder::new(Box::new(dango_taxman::instantiate))
        .with_execute(Box::new(dango_taxman::execute))
        .with_query(Box::new(dango_taxman::query))
        .with_withhold_fee(Box::new(dango_taxman::withhold_fee))
        .with_finalize_fee(Box::new(dango_taxman::finalize_fee))
        .build();

    let token_factory = ContractBuilder::new(Box::new(dango_token_factory::instantiate))
        .with_execute(Box::new(dango_token_factory::execute))
        .with_query(Box::new(dango_token_factory::query))
        .build();

    let codes = Codes {
        account_factory,
        account_spot,
        account_safe,
        amm,
        bank,
        ibc_transfer,
        taxman,
        token_factory,
    };

    let owner = TestAccount::new_random("owner")?;
    let fee_recipient = TestAccount::new_random("fee_recipient")?;
    let relayer = TestAccount::new_random("relayer")?;

    let (genesis_state, contracts, addresses) = build_genesis(
        codes,
        btree_map! {
            owner.username.clone() => GenesisUser {
                key: owner.key.clone(),
                key_hash: owner.key_hash,
                balances: Coins::one("uusdc", 100_000_000_000)?,
            },
            fee_recipient.username.clone() => GenesisUser {
                key: fee_recipient.key.clone(),
                key_hash: fee_recipient.key_hash,
                balances: Coins::new(),
            },
            relayer.username.clone() => GenesisUser {
                key: relayer.key.clone(),
                key_hash: relayer.key_hash,
                balances: btree_map! {
                    "uusdc" => 100_000_000_000_000,
                    "uatom" => 100_000_000_000_000,
                    "uosmo" => 100_000_000_000_000,
                }
                .try_into()?,
            },
        },
        &owner.username,
        &fee_recipient.username,
        "uusdc",
        Udec128::ZERO,
        Uint128::new(10_000_000),
    )?;

    let suite = TestSuite::new(
        "dev-1".to_string(),
        Duration::from_millis(250),
        1_000_000,
        BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: Timestamp::from_seconds(0),
        },
        genesis_state,
    )?;

    let accounts = Accounts {
        owner: owner.set_address(&addresses),
        fee_recipient: fee_recipient.set_address(&addresses),
        relayer: relayer.set_address(&addresses),
    };

    Ok((suite, accounts, codes, contracts))
}

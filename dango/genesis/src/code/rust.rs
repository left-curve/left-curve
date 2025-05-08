use {
    crate::Codes,
    dango_types::config::Hyperlane,
    grug::{ContractBuilder, ContractWrapper},
};

/// Create genesis contract codes for the Rust VM.
///
/// ## Note
///
/// Contracts in this function are **order-sensitive**.
///
/// To add a new contract to a live network, only _append it to the end_.
/// Do NOT alter the order of existing contracts.
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

    let gateway = ContractBuilder::new(Box::new(dango_gateway::instantiate))
        .with_execute(Box::new(dango_gateway::execute))
        .with_query(Box::new(dango_gateway::query))
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
        .build();

    Codes {
        account_factory,
        account_margin,
        account_multi,
        account_spot,
        bank,
        dex,
        gateway,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
    }
}

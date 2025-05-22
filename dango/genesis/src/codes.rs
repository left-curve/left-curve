use {
    crate::Codes,
    dango_types::config::Hyperlane,
    grug::{Binary, ContractBuilder, ContractWrapper},
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::RustVm,
    grug_vm_wasm::WasmVm,
    std::{fs, path::Path},
};

/// Get the binary codes for Dango smart contracts, for use in building the
/// genesis state.
pub trait GenesisCodes {
    type Code: Clone + Into<Binary>;

    fn genesis_codes() -> Codes<Self::Code>;
}

impl GenesisCodes for RustVm {
    type Code = ContractWrapper;

    fn genesis_codes() -> Codes<ContractWrapper> {
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
            .with_cron_execute(Box::new(dango_gateway::cron_execute))
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

        let bitcoin = ContractBuilder::new(Box::new(dango_bitcoin::instantiate))
            .with_execute(Box::new(dango_bitcoin::execute))
            .with_query(Box::new(dango_bitcoin::query))
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
            bitcoin,
        }
    }
}

impl GenesisCodes for WasmVm {
    type Code = Vec<u8>;

    fn genesis_codes() -> Codes<Vec<u8>> {
        let artifacts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");

        let account_factory = fs::read(artifacts_dir.join("dango_account_factory.wasm")).unwrap();
        let account_margin = fs::read(artifacts_dir.join("dango_account_margin.wasm")).unwrap();
        let account_multi = fs::read(artifacts_dir.join("dango_account_multi.wasm")).unwrap();
        let account_spot = fs::read(artifacts_dir.join("dango_account_spot.wasm")).unwrap();
        let bank = fs::read(artifacts_dir.join("dango_bank.wasm")).unwrap();
        let dex = fs::read(artifacts_dir.join("dango_dex.wasm")).unwrap();
        let gateway = fs::read(artifacts_dir.join("dango_gateway.wasm")).unwrap();
        let ism = fs::read(artifacts_dir.join("hyperlane_ism.wasm")).unwrap();
        let mailbox = fs::read(artifacts_dir.join("hyperlane_mailbox.wasm")).unwrap();
        let va = fs::read(artifacts_dir.join("hyperlane_va.wasm")).unwrap();
        let lending = fs::read(artifacts_dir.join("dango_lending.wasm")).unwrap();
        let oracle = fs::read(artifacts_dir.join("dango_oracle.wasm")).unwrap();
        let taxman = fs::read(artifacts_dir.join("dango_taxman.wasm")).unwrap();
        let vesting = fs::read(artifacts_dir.join("dango_vesting.wasm")).unwrap();
        let warp = fs::read(artifacts_dir.join("hyperlane_warp.wasm")).unwrap();
        let bitcoin = fs::read(artifacts_dir.join("dango_bitcoin.wasm")).unwrap();

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
            bitcoin,
        }
    }
}

impl GenesisCodes for HybridVm {
    type Code = <RustVm as GenesisCodes>::Code;

    fn genesis_codes() -> Codes<Self::Code> {
        RustVm::genesis_codes()
    }
}

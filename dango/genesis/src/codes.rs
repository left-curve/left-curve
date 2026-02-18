use {
    crate::Codes,
    dango_types::config::Hyperlane,
    grug::{Binary, ContractBuilder, ContractWrapper},
    grug_vm_rust::RustVm,
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

        let account_multi = ContractBuilder::new(Box::new(dango_account_multi::instantiate))
            .with_authenticate(Box::new(dango_account_multi::authenticate))
            .with_receive(Box::new(dango_account_multi::receive))
            .with_execute(Box::new(dango_account_multi::execute))
            .with_query(Box::new(dango_account_multi::query))
            .build();

        let account_single = ContractBuilder::new(Box::new(dango_account_single::instantiate))
            .with_authenticate(Box::new(dango_account_single::authenticate))
            .with_receive(Box::new(dango_account_single::receive))
            .with_query(Box::new(dango_account_single::query))
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
            .with_reply(Box::new(dango_dex::reply))
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

        #[cfg(feature = "metrics")]
        {
            dango_dex::metrics::init_metrics();
            dango_oracle::metrics::init_metrics();
            dango_taxman::metrics::init_metrics();
            // TODO: add other contracts that emit metrics
        }

        Codes {
            account_factory,
            account_multi,
            account_single,
            bank,
            dex,
            gateway,
            hyperlane: Hyperlane { ism, mailbox, va },
            oracle,
            taxman,
            vesting,
            warp,
        }
    }
}

// TODO: implement `GenesisCodes` for `WasmVm` and `HybridVm`.
// For now, we don't want to include them here because wasmer v6 has changed to
// the Business Source License. We want to make sure anything in the `dango/`
// directory does NOT have dependency on it.

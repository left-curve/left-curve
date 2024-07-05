use {
    grug_app::Vm,
    grug_types::Binary,
    grug_vm_rust::{ContractBuilder, RustVm},
};

/// Describes a VM that can be used in the [`TestSuite`](crate::TestSuite).
///
/// Other than implementing the `Vm` trait, the VM must come with default bank
/// and account contract implementations for use by the [`TestBuilder`](crate::TestBuilder).
pub trait TestVm: Vm {
    fn default_account_code() -> Binary;

    fn default_bank_code() -> Binary;
}

impl TestVm for RustVm {
    fn default_account_code() -> Binary {
        ContractBuilder::new(Box::new(grug_account::instantiate))
            .with_execute(Box::new(grug_account::execute))
            .with_receive(Box::new(grug_account::receive))
            .with_query(Box::new(grug_account::query))
            .build()
            .into_bytes()
            .into()
    }

    fn default_bank_code() -> Binary {
        ContractBuilder::new(Box::new(grug_bank::instantiate))
            .with_execute(Box::new(grug_bank::execute))
            .with_query(Box::new(grug_bank::query))
            .with_bank_execute(Box::new(grug_bank::bank_execute))
            .with_bank_query(Box::new(grug_bank::bank_query))
            .build()
            .into_bytes()
            .into()
    }
}

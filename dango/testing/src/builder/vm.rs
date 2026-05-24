use {
    grug_app::Vm,
    grug_types::{Binary, Hash256, HashExt},
    grug_vm_hybrid::HybridVm,
    grug_vm_rust::{ContractBuilder, RustVm},
    grug_vm_wasm::WasmVm,
    std::{collections::HashSet, sync::LazyLock},
};

/// Describes a VM that can be used in the [`TestSuite`](super::TestSuite).
///
/// Other than implementing the `Vm` trait, the VM must come with default bank
/// and account contract implementations for use by the [`TestBuilder`](super::TestBuilder).
pub trait TestVm: Vm {
    fn default_account_code() -> Binary;

    fn default_bank_code() -> Binary;

    fn default_taxman_code() -> Binary;
}

impl TestVm for RustVm {
    fn default_account_code() -> Binary {
        ContractBuilder::new(Box::new(grug_mock_account::instantiate))
            .with_execute(Box::new(grug_mock_account::execute))
            .with_receive(Box::new(grug_mock_account::receive))
            .with_query(Box::new(grug_mock_account::query))
            .with_authenticate(Box::new(grug_mock_account::authenticate))
            .build()
            .to_bytes()
            .into()
    }

    fn default_bank_code() -> Binary {
        ContractBuilder::new(Box::new(grug_mock_bank::instantiate))
            .with_execute(Box::new(grug_mock_bank::execute))
            .with_query(Box::new(grug_mock_bank::query))
            .with_bank_execute(Box::new(grug_mock_bank::bank_execute))
            .with_bank_query(Box::new(grug_mock_bank::bank_query))
            .build()
            .to_bytes()
            .into()
    }

    fn default_taxman_code() -> Binary {
        ContractBuilder::new(Box::new(grug_mock_taxman::instantiate))
            .with_query(Box::new(grug_mock_taxman::query))
            .with_withhold_fee(Box::new(grug_mock_taxman::withhold_fee))
            .with_finalize_fee(Box::new(grug_mock_taxman::finalize_fee))
            .build()
            .to_bytes()
            .into()
    }
}

// ---- impl TestVm for WasmVm ----

impl TestVm for WasmVm {
    fn default_account_code() -> Binary {
        let code: &[u8] = include_bytes!("../../testdata/grug_mock_account.wasm");
        code.into()
    }

    fn default_bank_code() -> Binary {
        let code: &[u8] = include_bytes!("../../testdata/grug_mock_bank.wasm");
        code.into()
    }

    fn default_taxman_code() -> Binary {
        let code: &[u8] = include_bytes!("../../testdata/grug_mock_taxman.wasm");
        code.into()
    }
}

// ---- impl TestVm for HybridVm ----

static DEFAULT_ACCOUNT_CODE: LazyLock<Binary> = LazyLock::new(RustVm::default_account_code);
static DEFAULT_BANK_CODE: LazyLock<Binary> = LazyLock::new(RustVm::default_bank_code);
static DEFAULT_TAXMAN_CODE: LazyLock<Binary> = LazyLock::new(RustVm::default_taxman_code);

pub fn new_hybrid_vm_testing<T>(wasm_cache_capacity: usize, code_hashes_for_rust: T) -> HybridVm
where
    T: IntoIterator<Item = Hash256>,
{
    let mut finalize = HashSet::from([
        DEFAULT_ACCOUNT_CODE.hash256(),
        DEFAULT_BANK_CODE.hash256(),
        DEFAULT_TAXMAN_CODE.hash256(),
    ]);

    finalize.extend(code_hashes_for_rust);

    HybridVm::new(wasm_cache_capacity, finalize)
}

impl TestVm for HybridVm {
    fn default_account_code() -> Binary {
        DEFAULT_ACCOUNT_CODE.clone()
    }

    fn default_bank_code() -> Binary {
        DEFAULT_BANK_CODE.clone()
    }

    fn default_taxman_code() -> Binary {
        DEFAULT_TAXMAN_CODE.clone()
    }
}

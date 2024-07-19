use {crate::WasmVm, grug_testing::TestVm, grug_types::Binary};

impl TestVm for WasmVm {
    fn default_account_code() -> Binary {
        let code: &[u8] = include_bytes!("../testdata/grug_account.wasm");
        code.into()
    }

    fn default_bank_code() -> Binary {
        let code: &[u8] = include_bytes!("../testdata/grug_bank.wasm");
        code.into()
    }
}

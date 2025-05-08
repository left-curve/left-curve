use {
    crate::Codes,
    dango_types::config::Hyperlane,
    std::{fs, io, path::Path},
};

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
    let gateway = fs::read(artifacts_dir.join("dango_gateway.wasm"))?;
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
        gateway,
        hyperlane: Hyperlane { ism, mailbox, va },
        lending,
        oracle,
        taxman,
        vesting,
        warp,
    })
}

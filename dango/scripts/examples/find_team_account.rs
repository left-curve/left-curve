use {
    dango_genesis::GenesisCodes,
    dango_types::account_factory,
    grug::{Addr, Hash256, JsonSerExt, Query, QueryWasmSmartRequest, addr},
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_vm_rust::RustVm,
    sha2::{Digest, Sha256},
    std::path::PathBuf,
};

const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

fn main() -> anyhow::Result<()> {
    let db_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("deploy/downloaded-db/mainnet/inter1/data");

    println!("Opening DB at: {}", db_path.display());

    let _codes = RustVm::genesis_codes();

    let db = DiskDb::<SimpleCommitment>::open(&db_path)?;

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
        env!("CARGO_PKG_VERSION"),
    );

    // Compute key_hash: SHA-256 of the lowercased Ethereum address string (including 0x prefix)
    let eth_address = "0xec16b574ddf7c1b1039aad3b7c170408b7aaa4ac";
    let hash_bytes: [u8; 32] = Sha256::digest(eth_address.as_bytes()).into();
    let key_hash = Hash256::from_inner(hash_bytes);

    println!("Ethereum address: {eth_address}");
    println!("Key hash: {key_hash}");

    // Query the account factory for ForgotUsername
    let res = app.do_query_app(
        Query::WasmSmart(QueryWasmSmartRequest {
            contract: ACCOUNT_FACTORY,
            msg: account_factory::QueryMsg::ForgotUsername {
                key_hash,
                start_after: None,
                limit: None,
            }
            .to_json_value()?,
        }),
        None,
        false,
    )?;

    println!("\nForgotUsername result:");
    println!("{}", res.to_json_string_pretty()?);

    Ok(())
}

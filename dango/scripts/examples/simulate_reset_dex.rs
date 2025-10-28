use {
    dango_genesis::GenesisCodes,
    dango_testing::{TestAccount, TestSuite},
    dango_types::{account, account_factory::Username, dex},
    grug::{
        Addr, Coins, Duration, JsonSerExt, QuerierExt, Query, QueryStatusRequest, ResultExt,
        StdResult, addr,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    hex_literal::hex,
    std::{path::PathBuf, str::FromStr},
};

const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

const OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const OWNER_USERNAME: &str = "owner";

/// For demonstration purpose only; do not use this in production.
const OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const FROM_HEIGHT: u64 = 150721; // exclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let _codes = RustVm::genesis_codes();

    let db = MemDb::<SimpleCommitment>::recover(cwd.join(format!("db-{FROM_HEIGHT}.borsh")))?;

    // In this DB snapshot, the chain owner has been changed to another account.
    // Let's change it back to `test1`, so we can use `test1` to sign the reset
    // DEX transaction.
    db.with_state_storage_mut(|storage| {
        grug_app::CONFIG.update(storage, |mut cfg| -> StdResult<_> {
            cfg.owner = OWNER;
            Ok(cfg)
        })
    })?;

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    let status = app
        .do_query_app(Query::Status(QueryStatusRequest {}), FROM_HEIGHT, false)?
        .as_status();

    let mut suite = TestSuite::new_with_app(
        app,
        status.chain_id,
        status.last_finalized_block,
        Duration::from_millis(200),
        u64::MAX,
    );

    let owner_nonce = suite
        .query_wasm_smart(OWNER, account::spot::QuerySeenNoncesRequest {})
        .should_succeed()
        .pop_last()
        .unwrap();

    let mut owner =
        TestAccount::new_from_private_key(Username::from_str(OWNER_USERNAME)?, OWNER_PRIVATE_KEY)
            .set_address(OWNER)
            .set_nonce(owner_nonce + 1);

    // Reset and DEX. Ensure the call succeeds.
    let outcome = suite
        .execute(
            &mut owner,
            DEX,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::Reset {}),
            Coins::new(),
        )
        .should_succeed();
    println!("{}", outcome.events.to_json_string_pretty()?);

    // Ensure the DEX is unpaused.
    suite
        .query_wasm_smart(DEX, dex::QueryPausedRequest {})
        .should_succeed_and_equal(false);

    Ok(())
}

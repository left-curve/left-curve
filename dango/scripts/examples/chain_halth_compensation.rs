use {
    dango_genesis::GenesisCodes,
    dango_types::{UsdPrice, UsdValue, config::AppConfig, perps::UserState},
    grug::{
        Addr, Borsh, Bound, Codec, Dec128_6, Dec128_24, Denom, JsonDeExt, JsonSerExt,
        MultiplyFraction, Number, NumberConst, PrimaryKey, Query, QueryAppConfigRequest, Signed,
        Udec128_6, btree_map,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_vm_rust::RustVm,
    std::{collections::BTreeMap, path::PathBuf, str::FromStr, sync::LazyLock},
};

mod utils {

    use super::*;

    pub fn trim_first_prefix(bytes: &[u8]) -> &[u8] {
        let len = u16::from_be_bytes(bytes[0..2].try_into().unwrap()) as usize;
        &bytes[2 + len..]
    }

    pub fn prices(low: &str, high: &str) -> (UsdPrice, UsdPrice) {
        let min = UsdPrice::new(Dec128_6::from_str(low).unwrap());
        let max = UsdPrice::new(Dec128_6::from_str(high).unwrap());

        assert!(min < max);
        (min, max)
    }

    pub fn denom(denom: &str) -> Denom {
        Denom::from_str(denom).unwrap()
    }

    #[grug::derive(Serde)]
    pub struct ShareCompensation {
        pub shares: Udec128_6,
        pub missing_gain: UsdValue,
    }

    #[macro_export]
    macro_rules! ns_prefix {
        ($ns:literal) => {{
            const NS: &[u8] = $ns;
            const LEN: usize = NS.len() + 2;
            const fn build() -> [u8; LEN] {
                let mut out = [0u8; LEN];
                out[0] = (NS.len() >> 8) as u8;
                out[1] = NS.len() as u8;
                let mut i = 0;
                while i < NS.len() {
                    out[i + 2] = NS[i];
                    i += 1;
                }
                out
            }
            &build()
    }};
    }
}

use utils::*;

const LOWER_BOUND: &[u8] = ns_prefix!(b"us");
const UPPER_BOUND: &[u8] = ns_prefix!(b"ut");

const PRICES: LazyLock<BTreeMap<Denom, (UsdPrice, UsdPrice)>> = LazyLock::new(|| {
    btree_map! {
        denom("perp/btcusd") => prices("70685", "72602"),
        denom("perp/ethusd") => prices("2180", "2242"),
        denom("perp/solusd") => prices("81.66", "83.83"),
        denom("perp/hypeusd") => prices("41.4", "43.74"),
    }
});

const BLOCK_HEIGHT: u64 = 17708991;

const POINTS_TO_SHARE: u128 = 100_000;

// Chain health happens at Mon Apr 13 2026 10:32:21 GMT+0000
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path: PathBuf = home::home_dir()
        .expect("failed to resolve home directory")
        .join(".dango")
        .join("data");
    let db = DiskDb::<SimpleCommitment>::open(path).unwrap();

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
        "pippo",
    );

    let state = app
        .do_query_app(Query::status(), None, false)?
        .into_status();

    assert_eq!(
        state.last_finalized_block.height, BLOCK_HEIGHT,
        "block height mismatch"
    );

    println!(
        "ts: {}",
        state.last_finalized_block.timestamp.into_seconds()
    );

    for (p, (min, max)) in PRICES.iter() {
        println!("{}: {} - {}", p, min, max);
    }

    let cfg: AppConfig = app
        .do_query_app(Query::AppConfig(QueryAppConfigRequest {}), None, false)
        .unwrap()
        .into_app_config()
        .deserialize_json()?;

    let users = app
        .do_query_app(
            Query::wasm_scan(
                cfg.addresses.perps,
                Some(Bound::Inclusive(LOWER_BOUND.to_vec().into())),
                Some(Bound::Exclusive(UPPER_BOUND.to_vec().into())),
                Some(u32::MAX),
            ),
            None,
            false,
        )?
        .into_wasm_scan()
        .into_iter()
        .map(|(k, v)| {
            let addr = Addr::from_slice(trim_first_prefix(&k))?;
            let state: UserState = Borsh::decode(&v)?;

            Ok((addr, state))
        })
        .collect::<anyhow::Result<BTreeMap<Addr, UserState>>>()?;

    let mut total_missing_gain = UsdValue::ZERO;

    let mut missing_gain_per_user = BTreeMap::new();

    for (addr, state) in &users {
        for (pair_id, position) in &state.positions {
            let (min, max) = PRICES[pair_id];

            // Long
            if position.size.is_positive() {
                let delta = max.checked_sub(position.entry_price)?;

                if delta.is_negative() {
                    println!("delta is negative: {}", delta);
                    continue;
                }

                let pnl = delta.checked_mul(position.size)?;

                assert!(pnl.is_positive());

                total_missing_gain.checked_add_assign(pnl)?;

                missing_gain_per_user
                    .entry(addr)
                    .or_insert(UsdValue::ZERO)
                    .checked_add_assign(pnl)?;
            } else {
                let delta = position.entry_price.checked_sub(min)?;

                if delta.is_negative() {
                    println!("delta is negative: {}", delta);
                    continue;
                }

                let pnl = delta.checked_mul(position.size.checked_abs()?)?;

                assert!(pnl.is_positive());

                total_missing_gain.checked_add_assign(pnl)?;

                missing_gain_per_user
                    .entry(addr)
                    .or_insert(UsdValue::ZERO)
                    .checked_add_assign(pnl)?;
            }
        }
    }

    println!("Total missing gain: {}", total_missing_gain);

    let points_to_share = Udec128_6::new(POINTS_TO_SHARE);

    let mut computed_shares = Udec128_6::ZERO;

    let mut shares_per_user = BTreeMap::new();

    for (addr, missing_gain) in missing_gain_per_user {
        let ratio = Dec128_24::checked_from_ratio(
            missing_gain.into_inner().0,
            total_missing_gain.into_inner().0,
        )?
        .checked_into_unsigned()?;

        println!(
            "ratio: {}, missing gain: {} total missing gain: {}",
            ratio, missing_gain, total_missing_gain
        );

        let shares = points_to_share.checked_mul_dec(ratio)?;
        computed_shares.checked_add_assign(shares)?;

        shares_per_user.insert(addr, ShareCompensation {
            shares,
            missing_gain,
        });
    }

    // save to json file
    let json = shares_per_user.to_json_string_pretty()?;
    std::fs::write("compensation.json", json)?;

    Ok(())
}

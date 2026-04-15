use {
    dango_genesis::GenesisCodes,
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        account_factory::{self, UserIndex},
        config::AppConfig,
        perps::UserState,
    },
    grug::{
        Addr, Borsh, Bound, Codec, Dec128_6, Dec128_24, Denom, IsZero, JsonDeExt, JsonSerExt,
        MultiplyFraction, Number, NumberConst, PrimaryKey, Query, QueryAppConfigRequest, Signed,
        Udec128_6, Udec128_24, Uint128, addr, btree_map, btree_set,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_vm_rust::RustVm,
    std::{
        collections::{BTreeMap, BTreeSet},
        path::PathBuf,
        str::FromStr,
        sync::LazyLock,
    },
};

mod utils {

    use dango_types::perps::PairId;

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
    pub struct Compensation {
        pub vault: Udec128_6,
        pub unrealized: Udec128_6,
    }

    #[grug::derive(Serde)]
    pub struct PositionSnapshot {
        pub size: Quantity,
        pub entry_price: UsdPrice,
        pub reference_price: UsdPrice,
        pub unrealized_pnl: UsdValue,
    }

    #[grug::derive(Serde)]
    pub struct UserSnapshot {
        pub shares: Uint128,
        pub positions: BTreeMap<PairId, PositionSnapshot>,
        pub total_unrealized_pnl: UsdValue,
    }

    #[grug::derive(Serde)]
    pub struct UsersStateOutput {
        pub total_unrealized_pnl: UsdValue,
        pub users: BTreeMap<UserIndex, UserSnapshot>,
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

static PRICES: LazyLock<BTreeMap<Denom, (UsdPrice, UsdPrice)>> = LazyLock::new(|| {
    btree_map! {
        denom("perp/btcusd") => prices("70685", "76030"),
        denom("perp/ethusd") => prices("2180", "2415"),
        denom("perp/solusd") => prices("81.66", "87.66"),
        denom("perp/hypeusd") => prices("41.4", "45.2"),
    }
});

static BLACKLISTED_ADDRESSES: LazyLock<BTreeSet<Addr>> = LazyLock::new(|| {
    btree_set! {
        addr!("40e296f81c0d2a2baaf60b3cfcd21f0a742a9a9b"),
        addr!("88342ab46accd424252751f06fa2a5da0a0fa0d9"),
    }
});
const BLOCK_HEIGHT: u64 = 17708991;

const POINTS_UNREALIZED: u128 = 100_000;
const POINTS_VAULT: u128 = 35_000;

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
        "dango-1", // chain-id unused for offline state reads
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

    // ------------------------------------------------------------------
    // Pass 1: build per-user snapshots and accumulate totals
    // ------------------------------------------------------------------

    let mut total_missing_gain = UsdValue::ZERO;
    let mut total_vault_shares = Uint128::ZERO;
    let mut users_state: BTreeMap<UserIndex, UserSnapshot> = BTreeMap::new();

    for (addr, state) in &users {
        if addr == cfg.addresses.perps || BLACKLISTED_ADDRESSES.contains(addr) {
            continue;
        }

        let user_index = app
            .do_query_app(
                Query::wasm_smart(
                    cfg.addresses.account_factory,
                    &account_factory::QueryMsg::Account { address: *addr },
                )?,
                None,
                false,
            )?
            .into_wasm_smart()
            .deserialize_json::<account_factory::Account>()?
            .owner;

        let mut snapshot = UserSnapshot {
            shares: state.vault_shares,
            positions: BTreeMap::new(),
            total_unrealized_pnl: UsdValue::ZERO,
        };

        for (pair_id, position) in &state.positions {
            let (min, max) = PRICES[pair_id];

            let (reference_price, pnl) = if position.size.is_positive() {
                let delta = max.checked_sub(position.entry_price)?;
                if delta.is_negative() {
                    (max, UsdValue::ZERO)
                } else {
                    let pnl = delta.checked_mul(position.size)?;
                    assert!(pnl.is_positive());
                    (max, pnl)
                }
            } else {
                let delta = position.entry_price.checked_sub(min)?;
                if delta.is_negative() {
                    (min, UsdValue::ZERO)
                } else {
                    let pnl = delta.checked_mul(position.size.checked_abs()?)?;
                    assert!(pnl.is_positive());
                    (min, pnl)
                }
            };

            snapshot
                .positions
                .insert(pair_id.clone(), PositionSnapshot {
                    size: position.size,
                    entry_price: position.entry_price,
                    reference_price,
                    unrealized_pnl: pnl,
                });
            snapshot.total_unrealized_pnl.checked_add_assign(pnl)?;
        }

        total_missing_gain.checked_add_assign(snapshot.total_unrealized_pnl)?;
        total_vault_shares.checked_add_assign(state.vault_shares)?;

        if !snapshot.positions.is_empty() || snapshot.shares > Uint128::ZERO {
            users_state.insert(user_index, snapshot);
        }
    }

    println!("Total missing gain: {}", total_missing_gain);
    println!("Total vault shares: {}", total_vault_shares);

    // ------------------------------------------------------------------
    // Pass 2: compute compensation points from the snapshots
    // ------------------------------------------------------------------

    let points_unrealized = Udec128_6::new(POINTS_UNREALIZED);
    let points_vault = Udec128_6::new(POINTS_VAULT);

    let mut computed_points_unrealized = Udec128_6::ZERO;
    let mut computed_points_vault = Udec128_6::ZERO;
    let mut compensation = BTreeMap::new();

    for (&user_index, snapshot) in &users_state {
        if snapshot.total_unrealized_pnl.is_zero() && snapshot.shares.is_zero() {
            continue;
        }

        let ratio_unrealized = Dec128_24::checked_from_ratio(
            snapshot.total_unrealized_pnl.into_inner().0,
            total_missing_gain.into_inner().0,
        )?
        .checked_into_unsigned()?;

        let user_points_unrealized = points_unrealized.checked_mul_dec(ratio_unrealized)?;
        computed_points_unrealized.checked_add_assign(user_points_unrealized)?;

        let ratio_vault = Udec128_24::checked_from_ratio(snapshot.shares, total_vault_shares)?;

        let user_points_vault = points_vault.checked_mul_dec(ratio_vault)?;
        computed_points_vault.checked_add_assign(user_points_vault)?;

        println!(
            "User {}: vault={} unrealized={} ratio_unrealized={} ratio_vault={}",
            user_index, user_points_vault, user_points_unrealized, ratio_unrealized, ratio_vault,
        );

        compensation.insert(user_index, Compensation {
            vault: user_points_vault,
            unrealized: user_points_unrealized,
        });
    }

    println!("Computed points unrealized: {}", computed_points_unrealized);
    println!("Computed points vault: {}", computed_points_vault);

    // ------------------------------------------------------------------
    // Save outputs
    // ------------------------------------------------------------------

    let output_dir = PathBuf::from("compensation_output");
    std::fs::create_dir_all(&output_dir)?;

    std::fs::write(
        output_dir.join("users_state.json"),
        UsersStateOutput {
            total_unrealized_pnl: total_missing_gain,
            users: users_state,
        }
        .to_json_string_pretty()?,
    )?;

    std::fs::write(
        output_dir.join("compensation.json"),
        compensation.to_json_string_pretty()?,
    )?;

    Ok(())
}

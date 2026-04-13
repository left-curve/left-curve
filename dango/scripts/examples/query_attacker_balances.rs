use {
    dango_genesis::GenesisCodes,
    dango_types::{UsdValue, constants::usdc, perps},
    grug::{
        Addr, Dec128_6, Int128, IsZero, JsonDeExt, JsonSerExt, MultiplyRatio, Number, NumberConst,
        Query, QueryWasmSmartRequest, Uint128, addr,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_vm_rust::RustVm,
    std::{collections::BTreeMap, path::PathBuf},
};

const ATTACKER_1: Addr = addr!("023ef9e3e20caca6ef3743cbfba6469d69978999");
const ATTACKER_2: Addr = addr!("0e85f43a9e45a7c8835ded188890b7e57033b78f");
const PERPS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

// Team accounts (user larry, user_index 11)
const TEAM_ACCOUNT_11: Addr = addr!("40e296f81c0d2a2baaf60b3cfcd21f0a742a9a9b");
const TEAM_ACCOUNT_17: Addr = addr!("88342ab46accd424252751f06fa2a5da0a0fa0d9");

// Virtual shares/assets used in vault share pricing (from dango/perps/src/lib.rs)
const VIRTUAL_SHARES: Uint128 = Uint128::new(1_000_000);
const VIRTUAL_ASSETS: UsdValue = UsdValue::new_int(1);

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

    let usdc_denom = usdc::DENOM.clone();

    // -----------------------------------------------------------------------
    // 1. Attacker spot USDC balances
    // -----------------------------------------------------------------------
    println!("\n=== Attacker Spot USDC Balances ===\n");

    let balance1 = app
        .do_query_app(Query::balance(ATTACKER_1, usdc_denom.clone()), None, false)?
        .into_balance();
    println!(
        "Attacker 1 ({ATTACKER_1}): {} ({})",
        balance1.amount,
        format_usdc(balance1.amount.0)
    );

    let balance2 = app
        .do_query_app(Query::balance(ATTACKER_2, usdc_denom.clone()), None, false)?
        .into_balance();
    println!(
        "Attacker 2 ({ATTACKER_2}): {} ({})",
        balance2.amount,
        format_usdc(balance2.amount.0)
    );

    let attacker_total = balance1.amount.0 + balance2.amount.0;
    println!(
        "\nTotal attacker spot USDC: {}",
        format_usdc(attacker_total)
    );

    // -----------------------------------------------------------------------
    // 2. Perps contract spot USDC balance
    // -----------------------------------------------------------------------
    println!("\n=== Perps Contract Spot USDC Balance ===\n");

    let perps_balance = app
        .do_query_app(Query::balance(PERPS, usdc_denom), None, false)?
        .into_balance();
    println!(
        "Perps contract ({PERPS}): {} ({})",
        perps_balance.amount,
        format_usdc(perps_balance.amount.0)
    );

    // -----------------------------------------------------------------------
    // 3. Sum of all non-attacker user margins and pending unlocks
    // -----------------------------------------------------------------------
    println!("\n=== User Liabilities (excluding attackers) ===\n");

    let mut total_margin = UsdValue::ZERO;
    let mut total_unlocks = UsdValue::ZERO;
    let mut margin_user_count: u32 = 0;
    let mut unlock_count: u32 = 0;
    let mut start_after: Option<Addr> = None;

    loop {
        let res = app.do_query_app(
            Query::WasmSmart(QueryWasmSmartRequest {
                contract: PERPS,
                msg: perps::QueryMsg::UserStates {
                    start_after,
                    limit: Some(100),
                }
                .to_json_value()?,
            }),
            None,
            false,
        )?;

        let batch: BTreeMap<Addr, perps::UserState> = res.into_wasm_smart().deserialize_json()?;

        if batch.is_empty() {
            break;
        }

        for (addr, state) in &batch {
            if *addr == ATTACKER_1 || *addr == ATTACKER_2 {
                println!("  [skip] attacker {addr}: margin = {}", state.margin);
                continue;
            }

            if !state.margin.is_zero() {
                total_margin = total_margin.checked_add(state.margin)?;
                margin_user_count += 1;
            }

            for unlock in &state.unlocks {
                total_unlocks = total_unlocks.checked_add(unlock.amount_to_release)?;
                unlock_count += 1;
            }
        }

        // Set cursor for next page
        start_after = batch.keys().last().cloned();

        // If we got fewer than the limit, we're done
        if batch.len() < 100 {
            break;
        }
    }

    println!("Non-attacker users with margin: {margin_user_count}");
    println!("Total margin liability:         {total_margin}");
    println!("Pending unlocks:                {unlock_count}");
    println!("Total unlock liability:         {total_unlocks}");

    let total_liability = total_margin.checked_add(total_unlocks)?;
    println!("\nTotal liability (margin + unlocks): {total_liability}");

    // -----------------------------------------------------------------------
    // 4. Team vault share analysis — simulate immediate withdraw + forfeit
    // -----------------------------------------------------------------------
    println!("\n=== Team Vault Share Analysis ===\n");

    // Query global state for vault_share_supply.
    let global_state: perps::State = app
        .do_query_app(
            Query::WasmSmart(QueryWasmSmartRequest {
                contract: PERPS,
                msg: perps::QueryMsg::State {}.to_json_value()?,
            }),
            None,
            false,
        )?
        .into_wasm_smart()
        .deserialize_json()?;

    println!("Vault share supply: {}", global_state.vault_share_supply);
    println!("Insurance fund:     {}", global_state.insurance_fund);
    println!("Treasury:           {}", global_state.treasury);

    // Query vault UserState (stored at the perps contract address itself).
    let vault_state: Option<perps::UserState> = app
        .do_query_app(
            Query::WasmSmart(QueryWasmSmartRequest {
                contract: PERPS,
                msg: perps::QueryMsg::UserState { user: PERPS }.to_json_value()?,
            }),
            None,
            false,
        )?
        .into_wasm_smart()
        .deserialize_json()?;

    let vault_state = vault_state.expect("vault UserState should exist");
    // Under zero-PnL assumption, vault equity ≈ vault margin.
    let vault_margin = vault_state.margin;
    println!("Vault margin (≈ equity under zero-PnL): {vault_margin}");

    // Query both team accounts.
    let mut team_total_margin = UsdValue::ZERO;
    let mut team_total_shares = Uint128::ZERO;
    let mut team_total_unlocks = UsdValue::ZERO;

    for (label, team_addr) in [
        ("Account 11", TEAM_ACCOUNT_11),
        ("Account 17", TEAM_ACCOUNT_17),
    ] {
        let team_state: Option<perps::UserState> = app
            .do_query_app(
                Query::WasmSmart(QueryWasmSmartRequest {
                    contract: PERPS,
                    msg: perps::QueryMsg::UserState { user: team_addr }.to_json_value()?,
                }),
                None,
                false,
            )?
            .into_wasm_smart()
            .deserialize_json()?;

        if let Some(state) = team_state {
            let unlock_sum = state.unlocks.iter().try_fold(UsdValue::ZERO, |acc, u| {
                acc.checked_add(u.amount_to_release)
            })?;

            println!(
                "  Team {label} ({team_addr}): margin={}, vault_shares={}, unlocks={}",
                state.margin, state.vault_shares, unlock_sum,
            );

            team_total_margin = team_total_margin.checked_add(state.margin)?;
            team_total_shares = team_total_shares.checked_add(state.vault_shares)?;
            team_total_unlocks = team_total_unlocks.checked_add(unlock_sum)?;
        } else {
            println!("  Team {label} ({team_addr}): no perps state");
        }
    }

    // Compute vault share value for team's shares.
    // amount_to_release = effective_equity * shares / effective_supply
    let effective_supply = global_state
        .vault_share_supply
        .checked_add(VIRTUAL_SHARES)?;
    let effective_equity = vault_margin.checked_add(VIRTUAL_ASSETS)?;

    let team_share_value = if team_total_shares.is_non_zero() {
        let raw = effective_equity
            .into_inner()
            .0
            .checked_multiply_ratio_floor(
                Int128::new(i128::try_from(team_total_shares.0)?),
                Int128::new(i128::try_from(effective_supply.0)?),
            )?;
        UsdValue::new(Dec128_6::raw(raw))
    } else {
        UsdValue::ZERO
    };

    println!("\nTeam vault share value:  {team_share_value}");
    println!("Team existing margin:    {team_total_margin}");
    println!("Team pending unlocks:    {team_total_unlocks}");

    let team_forfeit = team_share_value
        .checked_add(team_total_margin)?
        .checked_add(team_total_unlocks)?;
    println!("Total team forfeit:      {team_forfeit}");

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    let adjusted_liability = total_liability.checked_sub(team_forfeit)?;

    println!("\n=== Shortfall Analysis ===\n");
    println!(
        "Perps contract USDC balance:   {}",
        format_usdc(perps_balance.amount.0)
    );
    println!("Total margin liability:        {total_margin}");
    println!("Total unlock liability:        {total_unlocks}");
    println!("Total liability:               {total_liability}");
    println!("  - team forfeit:              {team_forfeit}");
    println!("Adjusted liability:            {adjusted_liability}");
    println!(
        "Attacker funds on-chain:       {}",
        format_usdc(attacker_total)
    );

    Ok(())
}

fn format_usdc(raw: u128) -> String {
    let dollars = raw / 1_000_000;
    let cents = (raw % 1_000_000) / 10_000;
    format!("${dollars}.{cents:02}")
}

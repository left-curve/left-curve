use {
    crate::{
        context::Context,
        entities::perps_fees::PerpsFees,
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    dango_types::{UsdValue, perps::FeeDistributed},
    grug::{
        Addr, BlockAndBlockOutcomeWithHttpDetails, CommitmentStatus, EventName, EventStatus,
        EvtCron, FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus, JsonDeExt,
        NaiveFlatten, Number as _, NumberConst, SearchEvent, Signed, Udec128_6,
    },
};

impl Indexer {
    pub(crate) async fn store_perps_fees(
        perps_addr: &Addr,
        ctx: &grug_app::IndexerContext,
        context: &Context,
    ) -> Result<()> {
        let block_and_block_outcome = ctx
            .get::<BlockAndBlockOutcomeWithHttpDetails>()
            .ok_or(IndexerError::missing_block_or_block_outcome())?;

        let block_height = block_and_block_outcome.block.info.height;
        let created_at = DateTime::<Utc>::from_naive_utc_and_offset(
            block_and_block_outcome
                .block
                .info
                .timestamp
                .to_naive_date_time(),
            Utc,
        );

        let mut acc = FeesAccumulator::default();

        // User-submitted txs: `FeeDistributed` can surface here when an action
        // (e.g. closing a position directly) settles a fee-paying fill inline.
        for tx_outcome in block_and_block_outcome.block_outcome.tx_outcomes.iter() {
            if tx_outcome.result.is_err() {
                continue;
            }

            for event in tx_outcome.events.clone().flat() {
                if event.commitment_status != FlatCommitmentStatus::Committed {
                    continue;
                }

                let FlatEvent::ContractEvent(ref contract_event) = event.event else {
                    continue;
                };

                if contract_event.contract != *perps_addr {
                    continue;
                }

                if contract_event.ty == FeeDistributed::EVENT_NAME {
                    process_fee_distributed(contract_event, &mut acc, block_height);
                }
            }
        }

        // End-of-block cron: the usual path where the perps settlement emits
        // `FeeDistributed` for matched / liquidated fills.
        for outcome in &block_and_block_outcome.block_outcome.cron_outcomes {
            let CommitmentStatus::Committed(EventStatus::Ok(EvtCron {
                guest_event: EventStatus::Ok(ref event),
                ..
            })) = outcome.cron_event
            else {
                continue;
            };

            if event.contract != perps_addr {
                continue;
            }

            for event in event
                .clone()
                .naive_flatten(FlatCommitmentStatus::Committed, FlatEventStatus::Ok)
            {
                let FlatEventInfo {
                    event: FlatEvent::ContractEvent(ref contract_event),
                    commitment_status: FlatCommitmentStatus::Committed,
                    event_status: FlatEventStatus::Ok,
                    ..
                } = event
                else {
                    continue;
                };

                if contract_event.ty == FeeDistributed::EVENT_NAME {
                    process_fee_distributed(contract_event, &mut acc, block_height);
                }
            }
        }

        if acc.count == 0 {
            return Ok(());
        }

        #[cfg(feature = "metrics")]
        {
            metrics::counter!("indexer.clickhouse.perps_fees.rows_inserted.total").increment(1);
            metrics::counter!("indexer.clickhouse.perps_fees.events_processed.total")
                .increment(acc.count as u64);
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            block_height,
            fee_events = acc.count,
            "Inserting perps_fees row"
        );

        let row = PerpsFees {
            block_height,
            created_at,
            protocol_fee: acc.protocol_fee,
            vault_fee: acc.vault_fee,
            referee_rebate: acc.referee_rebate,
            referrer_payout: acc.referrer_payout,
            fee_events_count: acc.count,
        };

        let clickhouse_client = context.clickhouse_client().clone();
        let mut inserter = clickhouse_client
            .inserter::<PerpsFees>("perps_fees")
            .with_max_rows(1);

        inserter.write(&row).await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to write perps_fees row: {row:#?}: {_err}");
        })?;

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for perps_fees: {_err}");
        })?;

        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for perps_fees: {_err}");
        })?;

        Ok(())
    }
}

/// Running totals for the current block. All fields are `Udec128_6` (unsigned)
/// because the contract guarantees non-negativity by construction; the
/// per-event guard below rejects any violation before it reaches here.
#[derive(Default)]
struct FeesAccumulator {
    protocol_fee: Udec128_6,
    vault_fee: Udec128_6,
    referee_rebate: Udec128_6,
    referrer_payout: Udec128_6,
    count: u32,
}

fn process_fee_distributed(
    contract_event: &grug_types::CheckedContractEvent,
    acc: &mut FeesAccumulator,
    block_height: u64,
) {
    let fee: FeeDistributed = match contract_event.data.clone().deserialize_json() {
        Ok(e) => e,
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                block_height,
                err = %_err,
                "Failed to deserialize FeeDistributed event; skipping"
            );
            return;
        },
    };

    let payer = fee.payer_addr;

    let protocol_fee = match to_non_negative(fee.protocol_fee, block_height, payer, "protocol_fee")
    {
        Some(v) => v,
        None => return,
    };
    let vault_fee = match to_non_negative(fee.vault_fee, block_height, payer, "vault_fee") {
        Some(v) => v,
        None => return,
    };

    // `commissions[0]` is the rebate to the payer; `commissions[1..]` go to
    // the referrer chain. The contract emits an empty vec when the payer has
    // no referrer or referrals are globally inactive — both treated as zeros.
    let referee_rebate = match fee.commissions.first().copied() {
        Some(v) => match to_non_negative(v, block_height, payer, "referee_rebate") {
            Some(v) => v,
            None => return,
        },
        None => Udec128_6::ZERO,
    };

    let mut referrer_payout = Udec128_6::ZERO;
    for c in fee.commissions.iter().copied().skip(1) {
        let Some(v) = to_non_negative(c, block_height, payer, "referrer_payout") else {
            return;
        };
        if referrer_payout.checked_add_assign(v).is_err() {
            #[cfg(feature = "tracing")]
            tracing::error!(
                block_height,
                %payer,
                "referrer_payout sum overflowed within a single FeeDistributed; skipping event"
            );
            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.clickhouse.perps_fees.overflow.total").increment(1);
            return;
        }
    }

    if acc.protocol_fee.checked_add_assign(protocol_fee).is_err()
        || acc.vault_fee.checked_add_assign(vault_fee).is_err()
        || acc
            .referee_rebate
            .checked_add_assign(referee_rebate)
            .is_err()
        || acc
            .referrer_payout
            .checked_add_assign(referrer_payout)
            .is_err()
    {
        #[cfg(feature = "tracing")]
        tracing::error!(
            block_height,
            %payer,
            "perps_fees block accumulator overflowed; skipping event"
        );
        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.perps_fees.overflow.total").increment(1);
        return;
    }

    acc.count = acc.count.saturating_add(1);
}

/// Convert a signed `UsdValue` to `Udec128_6`, enforcing the contract's
/// non-negativity invariant. A negative value indicates a contract-side bug
/// (param validation allowed `taker_fee + maker_fee < 0` or a fee-rate outside
/// [0, 1]); we log loudly, bump a metric, and drop the event rather than
/// corrupt the aggregates by wrapping.
#[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
fn to_non_negative(
    v: UsdValue,
    block_height: u64,
    payer: Addr,
    field: &'static str,
) -> Option<Udec128_6> {
    if v.is_negative() {
        #[cfg(feature = "tracing")]
        tracing::error!(
            block_height,
            %payer,
            field,
            value = %v,
            "FeeDistributed invariant violated: field is negative; skipping event"
        );
        #[cfg(feature = "metrics")]
        metrics::counter!(
            "indexer.clickhouse.perps_fees.invariant_violations.total",
            "field" => field,
        )
        .increment(1);
        return None;
    }

    v.into_inner()
        .checked_into_unsigned()
        .inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!(
                block_height,
                %payer,
                field,
                err = %_err,
                "FeeDistributed value failed Dec128_6 -> Udec128_6 conversion; skipping event"
            );
        })
        .ok()
}

//! Perps-exchange event types, extraction from a `BlockOutcome`, and the
//! client-supplied filter, for the `perps_events2` subscription.
//!
//! Every event emitted by the perps contract is captured (filtered only by
//! emitting contract address) — there is no event-name whitelist, so new perps
//! event types appear in the feed automatically. Clients narrow the feed with
//! the `event_types` / `pair_ids` / `users` filter.

use {
    crate::recent_stream::HasHeight,
    async_graphql::SimpleObject,
    dango_primitives::{
        Addr, BlockOutcome, CheckedContractEvent, Denom, FlatCommitmentStatus, FlatEvent, Inner,
        JsonDeExt, SearchEvent,
    },
    std::collections::HashSet,
};

/// A single perps-contract event, as streamed to clients.
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "PerpsEvent2")]
pub struct PerpsEvent {
    /// Per-block ordinal across all perps-contract events in the block.
    pub idx: u32,

    /// Raw on-chain event type string, e.g. `"order_filled"`, `"liquidated"`.
    #[graphql(name = "eventType")]
    pub event_type: String,

    /// The event's subject address (its `user` field), if present. Each perps
    /// event names a single participant — a fill's two sides are separate
    /// events — so this is what the `user` filter matches on. `None` for events
    /// without a `user` field (e.g. fee distribution).
    pub user: Option<String>,

    /// The market the event pertains to (its `pair_id` field), if present.
    #[graphql(name = "pairId")]
    pub pair_id: Option<String>,

    /// The raw event payload.
    pub data: async_graphql::Json<serde_json::Value>,
}

/// All perps-contract events emitted in one block. Doubles as the ring item
/// (holding every event) and the streamed output (holding the filtered subset).
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "PerpsEvent2Batch")]
pub struct PerpsEventBlock {
    #[graphql(name = "blockHeight")]
    pub block_height: u64,

    /// Block timestamp, RFC 3339.
    #[graphql(name = "createdAt")]
    pub created_at: String,

    pub events: Vec<PerpsEvent>,
}

impl HasHeight for PerpsEventBlock {
    fn height(&self) -> u64 {
        self.block_height
    }
}

/// Extract every committed perps-contract event from a block's outcome.
///
/// `BlockOutcome::flat()` folds both tx and cron outcomes into one ordered
/// list, so liquidations/ADL emitted in cron context are captured alongside
/// user-submitted order events.
pub fn extract_perps_event_block(
    block_height: u64,
    created_at: String,
    outcome: BlockOutcome,
    perps_addr: Addr,
) -> PerpsEventBlock {
    let mut events = Vec::new();
    let mut idx = 0u32;

    for flat in outcome.flat() {
        if flat.commitment_status != FlatCommitmentStatus::Committed {
            continue;
        }

        let FlatEvent::ContractEvent(ref contract_event) = flat.event else {
            continue;
        };

        if contract_event.contract != perps_addr {
            continue;
        }

        let (user, pair_id) = extract_user_and_pair(contract_event);

        events.push(PerpsEvent {
            idx,
            event_type: contract_event.ty.clone(),
            user,
            pair_id,
            data: async_graphql::Json(contract_event.data.clone().into_inner()),
        });

        idx += 1;
    }

    PerpsEventBlock {
        block_height,
        created_at,
        events,
    }
}

/// Best-effort extraction of the `user` and `pair_id` fields shared by most
/// perps events. Events lacking either field simply yield `None` there (they
/// can still match an `event_type` filter).
fn extract_user_and_pair(event: &CheckedContractEvent) -> (Option<String>, Option<String>) {
    #[derive(Default, serde::Deserialize)]
    struct UserAndPair {
        #[serde(default)]
        user: Option<Addr>,

        #[serde(default)]
        pair_id: Option<Denom>,
    }

    let parsed: UserAndPair = event.data.clone().deserialize_json().unwrap_or_default();

    (
        parsed.user.map(|a| a.to_string()),
        parsed.pair_id.map(|d| d.to_string()),
    )
}

/// Build the per-block projection closure for a `perps_events2` subscription.
///
/// The three predicates AND together. For each, `None` means "do not filter on
/// this field" (matches everything), while `Some(set)` keeps only events whose
/// field is in the set — so an empty set matches nothing. A block with no
/// matching events projects to `None` (it is suppressed, never emitted as an
/// empty batch).
pub fn make_perps_filter(
    event_types: Option<HashSet<String>>,
    pair_ids: Option<HashSet<String>>,
    users: Option<HashSet<String>>,
) -> impl Fn(&PerpsEventBlock) -> Option<PerpsEventBlock> + Send + 'static {
    move |block: &PerpsEventBlock| {
        let matched = block
            .events
            .iter()
            .filter(|event| {
                if let Some(event_types) = &event_types
                    && !event_types.contains(&event.event_type)
                {
                    return false;
                }

                // An event without a `pair_id` cannot match a pair filter.
                if let Some(pair_ids) = &pair_ids {
                    let Some(pair_id) = &event.pair_id else {
                        return false;
                    };

                    if !pair_ids.contains(pair_id) {
                        return false;
                    }
                }

                // An event without a `user` cannot match a user filter.
                if let Some(users) = &users {
                    let Some(user) = &event.user else {
                        return false;
                    };

                    if !users.contains(user) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect::<Vec<_>>();

        if matched.is_empty() {
            None
        } else {
            Some(PerpsEventBlock {
                block_height: block.block_height,
                created_at: block.created_at.clone(),
                events: matched,
            })
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn event(idx: u32, ty: &str, pair: Option<&str>, user: Option<u8>) -> PerpsEvent {
        PerpsEvent {
            idx,
            event_type: ty.to_string(),
            user: user.map(|i| Addr::mock(i).to_string()),
            pair_id: pair.map(ToString::to_string),
            data: async_graphql::Json(serde_json::json!({})),
        }
    }

    fn block() -> PerpsEventBlock {
        PerpsEventBlock {
            block_height: 7,
            created_at: "2026-06-18T00:00:00Z".to_string(),
            events: vec![
                event(0, "order_filled", Some("perp/btcusd"), Some(1)),
                event(1, "liquidated", Some("perp/ethusd"), Some(3)),
                event(2, "order_persisted", Some("perp/btcusd"), Some(1)),
            ],
        }
    }

    fn types(names: &[&str]) -> Option<HashSet<String>> {
        Some(names.iter().map(ToString::to_string).collect())
    }

    fn pairs(names: &[&str]) -> Option<HashSet<String>> {
        Some(names.iter().map(ToString::to_string).collect())
    }

    fn users(idxs: &[u8]) -> Option<HashSet<String>> {
        Some(idxs.iter().map(|i| Addr::mock(*i).to_string()).collect())
    }

    #[test]
    fn no_filter_matches_whole_block() {
        let f = make_perps_filter(None, None, None);
        let out = f(&block()).unwrap();
        assert_eq!(out.events.len(), 3);
        assert_eq!(out.block_height, 7);
    }

    #[test]
    fn event_type_filter_is_a_set() {
        let f = make_perps_filter(types(&["order_filled", "liquidated"]), None, None);
        let out = f(&block()).unwrap();
        assert_eq!(
            out.events
                .iter()
                .map(|e| e.event_type.as_str())
                .collect::<Vec<_>>(),
            vec!["order_filled", "liquidated"]
        );
    }

    #[test]
    fn pair_filter() {
        let f = make_perps_filter(None, pairs(&["perp/btcusd"]), None);
        let out = f(&block()).unwrap();
        assert_eq!(out.events.iter().map(|e| e.idx).collect::<Vec<_>>(), vec![
            0, 2
        ]);
    }

    #[test]
    fn user_filter_matches_user_field() {
        // mock(1) is the `user` of both the order_filled and order_persisted.
        let f = make_perps_filter(None, None, users(&[1]));
        let out = f(&block()).unwrap();
        assert_eq!(out.events.iter().map(|e| e.idx).collect::<Vec<_>>(), vec![
            0, 2
        ]);

        // mock(2) is not the `user` of any event here.
        let f = make_perps_filter(None, None, users(&[2]));
        assert!(f(&block()).is_none());
    }

    #[test]
    fn combined_filters_and_together() {
        let f = make_perps_filter(
            types(&["order_persisted"]),
            pairs(&["perp/btcusd"]),
            users(&[1]),
        );
        let out = f(&block()).unwrap();
        assert_eq!(out.events.iter().map(|e| e.idx).collect::<Vec<_>>(), vec![
            2
        ]);
    }

    #[test]
    fn no_match_suppresses_block() {
        let f = make_perps_filter(types(&["deleveraged"]), None, None);
        assert!(f(&block()).is_none());
    }

    #[test]
    fn empty_set_matches_nothing() {
        // `Some(empty)` means "filter on this field, but no value qualifies" —
        // distinct from `None` (do not filter). It must suppress the block.
        let f = make_perps_filter(Some(HashSet::new()), None, None);
        assert!(f(&block()).is_none());

        let f = make_perps_filter(None, None, Some(HashSet::new()));
        assert!(f(&block()).is_none());
    }
}

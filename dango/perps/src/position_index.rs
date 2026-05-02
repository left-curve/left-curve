use {
    crate::state::{LONGS, SHORTS},
    dango_order_book::UsdPrice,
    dango_types::perps::{PairId, Position},
    grug::{Addr, StdResult, Storage},
};

/// Describes how the LONGS/SHORTS index should be updated after a fill.
#[derive(Debug)]
pub struct PositionIndexUpdate {
    pub pair_id: PairId,
    pub user: Addr,
    /// Old entry to remove: (entry_price, was_long).
    pub old_entry: Option<(UsdPrice, bool)>,
    /// New entry to insert: (entry_price, is_long).
    pub new_entry: Option<(UsdPrice, bool)>,
}

/// Build a `PositionIndexUpdate` by comparing old and new position state.
/// Returns `None` if the index entry is unchanged.
pub fn compute_position_diff(
    pair_id: &PairId,
    user: Addr,
    old_pos: Option<&Position>,
    new_pos: Option<&Position>,
) -> Option<PositionIndexUpdate> {
    let old_entry = old_pos.map(|p| (p.entry_price, p.size.is_positive()));
    let new_entry = new_pos.map(|p| (p.entry_price, p.size.is_positive()));

    if old_entry == new_entry {
        return None;
    }

    Some(PositionIndexUpdate {
        pair_id: pair_id.clone(),
        user,
        old_entry,
        new_entry,
    })
}

/// Apply position index updates to storage (insert/remove LONGS/SHORTS).
pub fn apply_position_index_updates(
    storage: &mut dyn Storage,
    updates: &[PositionIndexUpdate],
) -> StdResult<()> {
    for update in updates {
        if let Some((price, is_long)) = update.old_entry {
            if is_long {
                LONGS.remove(storage, (update.pair_id.clone(), price, update.user));
            } else {
                SHORTS.remove(storage, (update.pair_id.clone(), price, update.user));
            }
        }

        if let Some((price, is_long)) = update.new_entry {
            if is_long {
                LONGS.insert(storage, (update.pair_id.clone(), price, update.user))?;
            } else {
                SHORTS.insert(storage, (update.pair_id.clone(), price, update.user))?;
            }
        }
    }

    Ok(())
}

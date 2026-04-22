# Purity rule for `dango-perps` internal functions

**TL;DR**: Internal business-logic functions in `dango-perps` that take
caller-persistable state MUST take it via `&` (shared reference) only.
They return every updated copy as owned values in a dedicated
`*Outcome` struct. Leaf helpers called only from within a pure
ancestor's locally-owned buffers MAY keep `&mut` parameters.
`EventBuilder` is the one deliberate exception — it stays `&mut`
everywhere.

## Three function classes

Trader-facing handlers in `dango-perps` are structured in three layers.
Each layer has a fixed signature contract and a fixed role:

| outer                                 | intermediate                           | inner                                     |
| ------------------------------------- | -------------------------------------- | ----------------------------------------- |
| `submit_order`                        | `_submit_order`                        | `compute_submit_order_outcome`            |
| `cancel_one_order`                    | `_cancel_one_order`                    | `compute_cancel_one_order_outcome` (leaf) |
| `cancel_one_order_by_client_order_id` | `_cancel_one_order_by_client_order_id` | `compute_cancel_one_order_outcome` (leaf) |
| `cancel_all_orders`                   | `_cancel_all_orders`                   | `compute_cancel_all_orders_outcome`       |

- **outer**: takes `MutableCtx`, returns `Response`. Directly called by
  the contract's `execute` entry point. Does nothing except create an
  `EventBuilder`, call the intermediate, and wrap the accumulated events
  in a `Response`.

- **intermediate** (`_`-prefixed): takes individual components of
  `MutableCtx` (`&mut dyn Storage`, `QuerierWrapper`, `Timestamp`,
  sender/contract `Addr`s, …) plus `&mut EventBuilder`. Returns
  `Result<()>`; side effects are written to the passed-in `storage` and
  `events`. The caller assembles the `Response`. Called by other outer
  or intermediate functions — in particular, `batch_update_orders`'s
  outer calls several intermediates in a loop, and the grug `Buffer`
  provides atomic rollback if any one returns `Err`.

- **inner**: pure logic. Takes caller-persistable state by shared
  reference (`&State`, `&PairState`, `&UserState`), never mutates the
  caller's inputs, and returns owned updated copies in a dedicated
  `*Outcome` struct. The intermediate then applies the outcome to
  storage. The rest of this document details the invariants that make
  this pattern safe.

## The rule, in detail

> **Public-within-module functions that take caller-persistable state
> MUST take it via `&` (shared reference) only and return every
> updated copy as owned values in a dedicated `*Outcome` struct.**
> Leaf helpers called only from within a pure ancestor's
> locally-owned buffers MAY keep `&mut` parameters.

### "Caller-persistable state" means

- `State` (global protocol state — `STATE`)
- `PairState` (per-pair state — `PAIR_STATES`)
- `UserState` (per-user state — `USER_STATES`)
- `BTreeMap<Addr, UserState>` (a collection of the above, threaded
  through matching as `maker_states`)

All four derive `Clone` cheaply. `UserState` has a small `BTreeMap`
of positions (typically <10) and a small `VecDeque` of unlocks
(typically <5). `PairState` and `State` are a handful of numeric
fields each. The clone cost at function entry is negligible.

### Explicitly NOT "caller-persistable state"

- **`&mut dyn Storage`** — grug handles transaction-level rollback at
  the block boundary, so storage writes that occur before an `Err` are
  discarded automatically. `&mut dyn Storage` stays.
- **`&mut OracleQuerier`** — it's a query cache, not caller state. A
  half-populated cache on `Err` is fine because the cache is dropped
  with the rest of the call frame.
- **`&mut EventBuilder`** — events are used offchain only; no impact on
  onchain logics.

### Pure set (must be `&`-only for caller state)

| File                                     | Function                                                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `src/trade/submit_order.rs`              | `compute_submit_order_outcome`, `match_order`, `store_limit_order`, `store_post_only_limit_order` |
| `src/trade/cancel_order.rs`              | `compute_cancel_all_orders_outcome`                                                               |
| `src/maintain/liquidate.rs`              | `_liquidate`                                                                                      |
| `src/cron/process_conditional_orders.rs` | `process_triggered_order`                                                                         |
| `src/cron/process_unlocks.rs`            | `process_unlock_for_user`                                                                         |
| `src/referral/apply_fee_commissions.rs`  | `apply_fee_commissions`                                                                           |

`refresh_orders` (in `src/vault/refresh.rs`) is not in the pure set: it's a
simple entry point with a single read/mutate path and no post-mutation
failure point, so the bug class doesn't apply and the ceremony of splitting
it into inner/outer wouldn't earn its keep.

### Leaf-helper exception (pragmatic tier)

Leaf helpers called only from within a pure ancestor's locally-owned
buffers MAY keep `&mut` parameters. The ancestor has already cloned
its inputs at entry; any error discards the ancestor's locals along
with the helper's partial writes. So the bug class doesn't apply.

Current leaf exceptions:

- `settle_fill` — only called from `match_order` / `execute_adl`,
  both of which clone their state at entry.
- `settle_pnls` — only called from `compute_submit_order_outcome` /
  `_liquidate`, operating on locals.
- `compute_cancel_one_order_outcome` — only called from
  `compute_cancel_all_orders_outcome` (which owns the local `user_state`) and
  from the intermediate `_cancel_one_order` /
  `_cancel_one_order_by_client_order_id` (top-level one-shots that
  don't compose into a failing ancestor).
- `execute_close_schedule` / `execute_adl` — private helpers of
  `_liquidate`. The ancestor `_liquidate` clones its inputs at entry,
  so any error discards the helpers' partial writes along with the
  rest of `_liquidate`'s locals.
- `vault::_add_liquidity` / `vault::_remove_liquidity` — kept out of
  the previous refactor's revert for the same reason: single-path,
  no post-mutation failure point.

A follow-up "strict tier" would convert these leaves too, eliminating
the exception. That's tracked as future work; the pragmatic tier
ships first so the urgent state-corruption bug class is closed with
the smallest possible diff.

## Approach: clone-at-entry + deferred-delta collectors

### Dense state structs — clone at entry, return owned

```rust
fn compute_submit_order_outcome(
    // ...
    state: &State,
    pair_state: &PairState,
    taker_state: &UserState,
    // ...
) -> anyhow::Result<SubmitOrderOutcome> {
    // Clone at entry — subsequent code mutates locals freely.
    let mut state = state.clone();
    let mut pair_state = pair_state.clone();
    let mut taker_state = taker_state.clone();

    // ... do work, may `return Err(...)` at any point ...

    // On success, the locals move into the outcome struct.
    Ok(SubmitOrderOutcome {
        state,
        pair_state,
        taker_state,
        // ...
    })
}
```

On `Err`, the locals are dropped and the caller sees nothing. On `Ok`,
the caller destructures the outcome and writes it to storage.

### Naturally-accumulator collectors — return explicit deltas

Collections that the pre-refactor code already returned as deferred
tuples (`order_mutations`, `index_updates`, `volumes`, `maker_states`,
`fee_breakdowns`, `pnls`, `fees`) stay as explicit delta fields in the
outcome structs. They're already accumulator-shaped — no need to diff
snapshots or reinvent the accumulation pattern.

This is a hybrid: clone-and-return for dense state, deferred-delta for
collectors. The two styles co-exist in one `*Outcome` struct.

## For new contributors

When adding a new public-within-module function to `dango-perps`, ask:

1. **Does it take any of `State`, `PairState`, `UserState`, or
   `BTreeMap<Addr, UserState>` as input?**
   - If yes: take them as `&`, define a `*Outcome` struct immediately
     above the function, return owned updated copies in the outcome
     on success. Clone at entry into local `mut` bindings so the body
     can mutate freely.
   - If no: regular function signatures are fine.

2. **Can the function fail at _any_ point after the first mutation of
   caller state?**
   - If yes: pure rule applies — no `&mut` on caller state.
   - If no (infallible, or only fails before any mutation): the rule
     doesn't mechanically apply, but following it anyway is still
     recommended for future-proofing.

3. **Is the function called only from within another pure function's
   locally-owned buffers?**
   - If yes: the leaf-helper exception applies; `&mut` is allowed.
   - Document which ancestor(s) own the buffer in the function's
     rustdoc, so a future refactor that adds a new caller can
     re-evaluate whether the exception still holds.

4. **Does the function emit events?**
   - `events: &mut EventBuilder` is the one allowed `&mut` on
     caller-persistable state. Do not put `EventBuilder` in the
     outcome struct. Be aware of the ghost-event limitation above.

# Purity rule for `dango-perps` internal functions

## TL;DR

Internal business-logic functions in `dango-perps` that take
caller-persistable state MUST take it via `&` (shared reference) only.
They return every updated copy as owned values in a dedicated
`*Outcome` struct. Leaf helpers called only from within a pure
ancestor's locally-owned buffers MAY keep `&mut` parameters.
`EventBuilder` is the one deliberate exception — it stays `&mut`
everywhere.

## Why this rule exists

On 2026-04-08 at block 20191499, dango-testnet-1 panicked inside
`match_order` with `attempt to subtract with overflow` at
`maker_state.open_order_count -= 1`. Root cause:

1. `cron::process_triggered_order` called `_submit_order` with
   `&mut taker_state`, `&mut pair_state`, etc.
2. Inside `_submit_order → match_order`, self-trade prevention
   detected a collision with the user's own resting order and
   mutated `taker_state.open_order_count -= 1` / `taker_state.reserved_margin -= …`
   in memory, queuing the order for removal in a `order_mutations` list.
3. `match_order` then bailed out with `"no liquidity at acceptable price!"`
   before the caller ever reached the `apply order_mutations` step.
4. `_submit_order` propagated the error. `process_triggered_order` caught
   it to gracefully cancel the conditional order and then `USER_STATES.save(...)`'d
   the partially-mutated `taker_state` — `open_order_count` one below
   the user's actual on-book order count, and the corresponding reserved
   margin leaked.
5. Over many such incidents, one user drifted to `open_order_count = 0`
   and `reserved_margin = −$177,831` (physically impossible) while still
   holding two resting bids on the book worth `+$34,037.59`. The next
   taker that filled one of those orders underflowed the count.

The bug class is: **a function takes caller-persistable state via
`&mut`, mutates it partway, then returns `Err`. The caller swallows the
error and persists the partially-mutated state.**

A local workaround (snapshotting and restoring caller state around the
call site) is in place at `cron::process_triggered_order` and is
covered by the regression test
`conditional_order_self_trade_failure_preserves_user_state` in
`dango/testing/tests/perps/conditional_orders.rs`. But a workaround at
one call site doesn't prevent the same bug from appearing at the next
call site — any future caller that catches an internal function's
error has to remember the same ritual. The purity rule is a
_structural_ fix: if a function can't take `&mut` on caller state,
the bug class becomes impossible to write.

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
- **`&mut EventBuilder`** — the one deliberate exception. See the
  Events section below.

### Pure set (must be `&`-only for caller state)

| File                                    | Function                                                                           |
| --------------------------------------- | ---------------------------------------------------------------------------------- |
| `src/trade/submit_order.rs`             | `_submit_order`, `match_order`, `store_limit_order`, `store_post_only_limit_order` |
| `src/trade/cancel_order.rs`             | `_cancel_all_orders`                                                               |
| `src/maintain/liquidate.rs`             | `_liquidate`, `execute_close_schedule`, `execute_adl`                              |
| `src/cron.rs`                           | `process_triggered_order`, `process_unlock_for_user`                               |
| `src/vault/refresh.rs`                  | `_refresh_orders` (extracted from the `refresh_orders` entry point)                |
| `src/referral/apply_fee_commissions.rs` | `apply_fee_commissions`                                                            |

### Leaf-helper exception (pragmatic tier)

Leaf helpers called only from within a pure ancestor's locally-owned
buffers MAY keep `&mut` parameters. The ancestor has already cloned
its inputs at entry; any error discards the ancestor's locals along
with the helper's partial writes. So the bug class doesn't apply.

Current leaf exceptions:

- `settle_fill` — only called from `match_order` / `execute_adl`,
  both of which clone their state at entry.
- `settle_pnls` — only called from `_submit_order` / `_liquidate`,
  operating on locals.
- `_cancel_one_order` — only called from `_cancel_all_orders`
  (which owns the local `user_state`) and from `cancel_one_order`
  (a top-level one-shot that doesn't compose into a failing ancestor).
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
fn _submit_order(
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

## Events handling (exception)

`EventBuilder` stays `&mut` everywhere, including at the module
boundary. It is **not** carried in any outcome struct.

### Rationale

- Events are append-only metadata for off-chain consumers (indexers,
  UI notifications), not ledger state. A function that pushes an
  event mid-flight and then returns `Err` leaves a "ghost" event in
  the caller's builder. This is a real but _lower-severity_ bug class
  than state corruption — at worst it emits an event for work that
  didn't commit, which downstream consumers can detect and ignore.
- Including `EventBuilder` in every outcome struct would add a field
  to every outcome, require an `outer.events.extend(inner.events)`
  discipline at every call site, and grow the refactor's surface for
  marginal benefit.
- Today's signatures already thread `events: &mut EventBuilder`
  everywhere; keeping it avoids unrelated churn at every call site.

### Known limitation

If a pure inner function (e.g. `_submit_order`) pushes `OrderFilled` /
`OrderRemoved` events during match, then the outer function errors
later, those events may still be committed to the outer `EventBuilder`
and eventually emitted in the `Response`. This exists today and is
preserved deliberately. Future work on event-level atomicity can pick
it up as a separate effort.

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

# Review of book/-enrichment pass

## Verdict: APPROVE

The drafter's two Pass-1 contradictions are correctly diagnosed and fixed.
The Pass-2 concept lead-ins are tight, book-grounded, and pages that
should have stayed terse (`getCodeHash`, `getNextAccountIndex`,
`getVaultSnapshots`, `cancelConditionalOrder`) were correctly left alone.
TS/Python concept paragraphs are aligned where the protocol concept is
shared. The Rust gap claim is legitimate.

## Must-fix

None. No claim materially misleads users in a way that would survive
contact with the source code or the book.

## Should-fix

- `sdk/docs/pages/python/api/methods/exchange/withdraw_margin.mdx:40` —
  "The contract converts USD to settlement-currency base units at the
  current oracle price." Book/perps/1-margin §3 step 4 says "at the
  **fixed $1 rate** (floor-rounded to base units)." The book's §1
  overview prose loosely says "oracle price" but the §3 algorithm is
  authoritative, and the corrected TS `withdrawMargin.mdx` already uses
  "fixed $1 rate." The Python doc faithfully mirrors a source-code
  comment in `sdk/python/dango/exchange.py:217-218` that has the same
  bug — the fix needs to land in the source comment as well, or the
  Python doc needs to override it. Either is fine; consistency matters
  more than the location.

- `sdk/docs/pages/typescript/actions/perps/vaultRemoveLiquidity.mdx:8`
  and `sdk/docs/pages/python/api/methods/exchange/remove_liquidity.mdx:7`
   — "a cooldown is enforced to prevent LPs from front-running known
  losses." The book (`perps/5-vault` §3) describes the cooldown
  mechanism but does not state the rationale. Plausible reading, but
  this is editorial inference, not book-grounded. Drop the motivation
  clause, keep the mechanism.

- `sdk/docs/pages/typescript/actions/perps/getPerpsOrdersByUser.mdx:7`
   — invariant claim "the sum of `reserved_margin` across these orders
  equals `user_state.reservedMargin`" is plausible from §7 margin /
  §10 order-matching but isn't stated as such in the book. Either
  source-verify against the contract (recommended) or weaken to
  "contributes to" rather than "equals."

## Nits / observations

- `sdk/docs/pages/typescript/actions/account-factory/updateUsername.mdx:6-9`
   — the doc explicitly flags a contradiction with the book ("the
  protocol book describes the original `name` field as immutable… this
  action surfaces a chain-side rename path that is only available
  where the contract permits it"). This is honest and well-scoped;
  worth a footnote in the book's account-factory §3 to remove the
  ambiguity, but outside this pass's scope.

- "volume-tiered fee" wording in `submit_order.mdx`,
  `submitPerpsOrder.mdx`, etc. is correct (book/perps/8-api.md
  lines 841-842 confirm `maker_fee_rates` / `taker_fee_rates` are
  `RateSchedule` with volume tiers) but the inline §-reference in
  these pages points at `2-order-matching`, which says "by role".
  A reader who follows the link won't see the volume-tier piece.
  Optionally cite §8-api alongside.

- TS `getPerpsState.mdx` and the Python `perps_state.mdx` counterpart
  are slightly asymmetric: TS got an enriched lead-in about the
  insurance fund and isolation; Python's `perps_state.mdx` (not opened
  in this review since it's already-aligned per the verification
  report) should be sanity-checked for parallel framing if not done.

## Spot checks performed

1. `sdk/docs/pages/typescript/actions/perps/withdrawMargin.mdx`
   vs `sdk/typescript/dango/src/actions/perps/mutations/withdrawMargin.ts`
   — message body `{ trade: { withdraw: { amount } } }` confirms USD
   value, no attached funds. Doc is accurate.
2. `sdk/docs/pages/typescript/actions/perps/vaultAddLiquidity.mdx`
   vs `sdk/typescript/dango/src/actions/perps/mutations/vaultAddLiquidity.ts`
   — message body `{ vault: { addLiquidity: { amount, … } } }` with
   no funds map confirms debit-from-trading-margin semantics. Doc is
   accurate.
3. `sdk/docs/pages/typescript/actions/perps/depositMargin.mdx`
   vs `sdk/typescript/dango/src/actions/perps/mutations/depositMargin.ts`
   — `funds: { "bridge/usdc": amount }` confirms base units of
   settlement currency are attached. Doc is accurate.
4. `sdk/docs/pages/python/api/methods/exchange/withdraw_margin.mdx`
   vs `sdk/python/dango/exchange.py` lines 210-233 — wire shape and
   "USD" claim match the source. The "current oracle price" assertion
   appears in both source comment and doc; see Should-fix above.
5. `sdk/docs/pages/python/api/methods/exchange/submit_conditional_order.mdx`
   vs `sdk/typescript/dango/src/actions/perps/mutations/submitConditionalOrder.ts`
   and book/perps/8-api.md §6.7 — "Conditional orders are always
   reduce-only with zero reserved margin" is verbatim in the book.
   Doc is accurate.
6. `sdk/docs/pages/typescript/actions/account-factory/registerUser.mdx`
   vs `sdk/typescript/dango/src/actions/account-factory/mutations/registerUser.ts`
   — `sender: addresses.accountFactory`, `credential: null` confirms
   "submitted by the account factory itself" claim. Doc is accurate.
7. `sdk/docs/pages/python/api/methods/exchange/liquidate.mdx` vs
   book/perps/4-liquidation-and-adl.md §3a, §3b, §4, §7 — claims about
   "largest MM contributors", "ADL at bankruptcy price", "zero fees on
   liquidation fills", and "insurance fund (not vault)" all match the
   book.

## Rust gap assessment

Legitimate. Confirmed by inspecting `sdk/rust/src/lib.rs`
(exports `client`, `keystore`, `secret`, `signer`, `subscription`
only) and `sdk/rust/examples/` (two read-only examples:
`subscribe_order_filled.rs`, `trade_history_csv.rs`). The Rust SDK
ships a low-level transport (HTTP + GraphQL via `indexer_graphql_types`)
plus signing/keystore primitives — no perps mutation wrappers
(`deposit_margin`, `submit_limit_order`, `vault_add_liquidity`, etc.)
exist, so there are no protocol-touching Rust pages to enrich. The
existing Rust docs (`pages/rust/api/clients/{HttpClient,WsClient,…}`,
`pages/rust/api/traits/Secret.mdx`) correctly scope to those
primitives.

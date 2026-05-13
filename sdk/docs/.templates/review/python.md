# Python SDK Docs Review

## Summary (counts)

- **Pages reviewed:** 135 `.mdx` files under `sdk/docs/pages/python/`
  - 1 index, 3 getting-started, 7 concepts, 6 classes, 13 exchange methods, 30 info methods, 4 websocket-manager methods, 36 types, 4 functions, 5 errors, 1 migration root, 1 missing-methods, 8 hl-compat/exchange, 16 hl-compat/info
- **Build status:** FAIL — dead links from `migration/hyperliquid.mdx` to non-existent `./hl-compat/exchange/order`
- **DEX-disabled callout violations:** 27+ pages missing the required `:::warning[DEX currently disabled]` callout (every native Exchange method page, the Exchange class page, all 8 HL-compat exchange pages, both project-setup and transactions concepts where DEX writes are demonstrated)
- **Sidebar/page mismatches:** 4 implemented HL-compat Exchange methods have no page (`order`, `bulk_orders`, `cancel`, `bulk_cancel`) and are also missing from the sidebar
- **Template-mostly-compliant pages:** all reference pages follow Action / Type / Client template structure (one-line description → signature → example → params → returns → notes → see also). Concept pages follow the "What this teaches" + "Next" frame.
- **Voice/style:** clean — no AI-tells found ("leverage", "delve", "comprehensive", em-dashes in prose), no banned words ("easy"/"simple"/"just"), no marketing fluff
- **Code blocks:** consistently use ```python; values are realistic; imports always present
- **Conceptual accuracy:** Cloid asymmetry, market_open `px` ignore, set_expires_after no-op, query_order_by_oid user-ignore, 167/10s + 30 subs limits, graphql-transport-ws single-connection model — all present and correctly worded in the migration page and concept pages.

## Coverage check

### Missing pages (must-fix)

The inventory says 12 HL-compat Exchange methods are implemented. Pages exist for 8:

| Method | Source location | Page status |
|--------|-----------------|-------------|
| `order` | `hyperliquid_compatibility/exchange.py:414` | **MISSING** |
| `bulk_orders` | `hyperliquid_compatibility/exchange.py:451` | **MISSING** |
| `cancel` | `hyperliquid_compatibility/exchange.py:522` | **MISSING** |
| `bulk_cancel` | `hyperliquid_compatibility/exchange.py:539` | **MISSING** |
| `cancel_by_cloid` | `:560` | present |
| `bulk_cancel_by_cloid` | `:574` | present |
| `modify_order` | `:600` | present |
| `bulk_modify_orders_new` | `:639` | present |
| `market_open` | `:701` | present |
| `market_close` | `:746` | present |
| `set_referrer` | `:827` | present |
| `set_expires_after` | `:847` | present |

`order` is the most-prominent missing page — the migration root page links to it twice (causing the build failure).

HL-compat Info coverage: all 16 implemented methods have pages.

Native methods: full coverage (13 Exchange + 30 Info + 4 WebsocketManager + 4 module functions + 5 errors).

### Missing pages (should-fix / nits)

The sidebar advertises 36 types but the inventory lists 70+. The gap is acceptable because most type aliases (`Uint64`, `Uint128`, `UsdValue`, `UsdPrice`, etc.) are mentioned inline in `concepts/encoding-and-types.mdx`. Listing every `NewType("X", str)` would add noise. However:

- `Wallet` Protocol — not documented as a Client or Type page. Referenced from `Exchange.mdx`, `concepts/signers.mdx`, and `Secp256k1Wallet.mdx`. Inventory verification TODO #3 calls this out.
- `LiquidityDepthResponse` — referenced in `liquidity_depth.mdx` returns section without a target page (no link, but inconsistent with other "see [`X`]" patterns).
- Event payload TypedDicts (`OrderFilled`, `Liquidated`, `ConditionalOrderPlaced`, etc.) — only `OrderRemoved` is documented. The unions referenced from `PerpsEvent.mdx`'s example `cast(OrderFilled, event["data"])` won't resolve.
- `ChildOrder`, `OrderKind`, `SubmitOrderRequest`, `CancelOrderRequest`, `Message`, `Metadata`, `Credential`, `Key`, `Signature`, `Unlock` — referenced by signatures but no pages.

### Orphans / sidebar issues

- Sidebar references all 8 hl-compat Exchange pages and 16 hl-compat Info pages — every link resolves.
- Every native method, class, type, error, function page is reachable from the sidebar.
- No orphan pages found.

### `missing-methods.mdx` content

The page exists but is a near-empty placeholder. It says "the following methods…raise" twice with no actual list. The inventory lists 42 Exchange + 25 Info methods (with verifiable names like `update_leverage`, `usd_class_transfer`, `spot_user_state`, `funding_history`, etc.). Either:
- Enumerate the 67 method names (recommended, even as a flat list grouped by category, since the migration page already promises "the full list of 42 on Exchange, 25 on Info"), OR
- Cut this page and update the migration page's link in `## Methods that are not implemented` to remove it.

## Findings

### Must-fix

#### 1. DEX-disabled warning callout absent on every applicable page

| File | Issue | Why | Fix |
|------|-------|-----|-----|
| `api/classes/Exchange.mdx` | No `:::warning[DEX currently disabled]` | The Exchange class documents the DEX surface; per STYLE_GUIDE §"Status callouts", required at the top | Add the warning block immediately under the one-line description |
| `api/methods/exchange/submit_order.mdx` | Missing warning | Documents a DEX trading action | Add warning |
| `api/methods/exchange/submit_limit_order.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/submit_market_order.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/submit_conditional_order.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/cancel_order.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/cancel_conditional_order.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/batch_update_orders.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/deposit_margin.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/withdraw_margin.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/add_liquidity.mdx` | Missing warning | DEX (vault is part of DEX) | Add warning |
| `api/methods/exchange/remove_liquidity.mdx` | Missing warning | DEX | Add warning |
| `api/methods/exchange/set_referral.mdx` | Missing warning | DEX referral | Add warning |
| `api/methods/exchange/liquidate.mdx` | Missing warning | DEX liquidation | Add warning |
| `migration/hl-compat/exchange/market_open.mdx` | Missing warning | DEX trading | Add warning |
| `migration/hl-compat/exchange/market_close.mdx` | Missing warning | DEX trading | Add warning |
| `migration/hl-compat/exchange/modify_order.mdx` | Missing warning | DEX (batch cancel+submit) | Add warning |
| `migration/hl-compat/exchange/bulk_modify_orders_new.mdx` | Missing warning | DEX | Add warning |
| `migration/hl-compat/exchange/cancel_by_cloid.mdx` | Missing warning | DEX | Add warning |
| `migration/hl-compat/exchange/bulk_cancel_by_cloid.mdx` | Missing warning | DEX | Add warning |
| `migration/hl-compat/exchange/set_referrer.mdx` | Missing warning | DEX referral | Add warning |
| `migration/hl-compat/exchange/set_expires_after.mdx` | (judgement) | Currently a no-op store; arguably not a DEX call site | Decide whether to include callout — leaning yes since the method's "expected" effect is on DEX trading |
| `getting-started/project-setup.mdx` | Example calls `exchange.submit_limit_order(...)` without callout | Concept-adjacent but demonstrates a disabled call | Add a callout just before "## A first signed transaction" |
| `concepts/transactions.mdx` | Example calls `exchange.submit_limit_order(...)` without callout | Demonstrates DEX submit | Add callout |

Required snippet (per the task spec):

```mdx
:::warning[DEX currently disabled]
The Dango DEX is currently disabled. Calls described on this page will not execute on the live network until the DEX is enabled.
:::
```

#### 2. Build fails: dead link to non-existent page

- **File:** `migration/hyperliquid.mdx`
- **Issue:** Two links target `./hl-compat/exchange/order`; the page does not exist.
  - Line 109: "see the [HL-compat method pages](./hl-compat/exchange/order) under this section."
  - Line 129: "[HL-compat `Exchange.order`](./hl-compat/exchange/order) — start the per-method tour"
- **Why:** Reported by `pnpm --filter @left-curve/docs build` as a dead link, blocking the build.
- **Fix:** Create `migration/hl-compat/exchange/order.mdx` (and the missing `bulk_orders`, `cancel`, `bulk_cancel` pages). Then add the four to the `hlCompatExchange` array in `vocs.config.ts`. After that, the links resolve and the build passes.

#### 3. Four HL-compat Exchange methods have no page

- **Files (to create):**
  - `migration/hl-compat/exchange/order.mdx`
  - `migration/hl-compat/exchange/bulk_orders.mdx`
  - `migration/hl-compat/exchange/cancel.mdx`
  - `migration/hl-compat/exchange/bulk_cancel.mdx`
- **Why:** Inventory explicitly lists 12 implemented Exchange methods (verified against `sdk/python/dango/hyperliquid_compatibility/exchange.py`). The drafter only produced 8. The migration page's worked-example mapping table (`HL-compat method` → `Native equivalent`) references these by name, so readers expect to click through.
- **Fix:** Author each using the Action template. Signatures from source:
  - `order(self, name, is_buy, sz, limit_px, order_type, reduce_only=False, cloid=None, builder=None)`; raises `NotImplementedError` on `builder is not None`; single-order shortcut to `submit_order`.
  - `bulk_orders(self, order_requests, *, builder=None, grouping="na")`; raises on non-`"na"` grouping or non-None builder.
  - `cancel(self, name, oid)`; verifies `name` for parity (it is NOT discarded — the source just doesn't use it for the cancel call).
  - `bulk_cancel(self, cancel_requests)`; each request has `coin` and `oid`.

#### 4. `migration/missing-methods.mdx` is a placeholder with no content

- **File:** `migration/missing-methods.mdx`
- **Issue:** The page is referenced from the migration root page ("See [Missing methods] for the full list (42 on Exchange, 25 on Info).") but contains zero method names. It just says "the following methods … currently raise" with no following list.
- **Why:** Readers click here expecting the enumerated list of every `NotImplementedError` stub.
- **Fix:** Add the 42 Exchange and 25 Info method names grouped by category (the inventory groups them as Margin/leverage, Scheduling, Transfers, Builder fee, Multi-sig, Sub-accounts, Agents, HYPE-specific, Spot deploys, Perp deploys for Exchange; and Spot, Staking, Multi-sig, Permissionless listing, Abstraction, TWAP, Time-series, Funding history, Phase-16 deferred, Cloid lookup for Info). A flat list per class is also acceptable.

### Should-fix

#### 5. Cross-page links to `Wallet` Protocol resolve to a concept page, not a Type page

- **Files:** `getting-started/project-setup.mdx` line 60 links `[Wallet](../concepts/signers)`; `api/classes/SingleSigner.mdx` shows `wallet: Wallet` in the constructor but no page to click through to.
- **Why:** Inventory TODO #3 flags this: "Recommend documenting both" the Protocol and the concrete `Secp256k1Wallet`. The concept page treats `Wallet` as the abstract interface but is not a Reference page.
- **Fix:** Add `api/types/Wallet.mdx` documenting the Protocol (`address`, `key`, `key_hash`, `sign`) and update inline references to link there. Alternatively, document `Wallet` on `Secp256k1Wallet.mdx` as a sibling section.

#### 6. `PerpsEvent.mdx` example casts to `OrderFilled` but no page exists for it

- **File:** `api/types/PerpsEvent.mdx`
- **Issue:** Inline `cast(OrderFilled, event["data"])` in the example, but `OrderFilled.mdx` does not exist. Same problem for `Liquidated`, `OrderPersisted`, `ConditionalOrderPlaced`, etc.
- **Why:** Either link to the per-event TypedDict page (after creating it) or remove the cast example.
- **Fix:** Either create `api/types/OrderFilled.mdx` (and add to sidebar) or rewrite the example to use `dict.get(...)` without casting.

#### 7. Examples reference `exchange._info` and `exchange._chain_id` — name-mangled access

- **File:** `api/methods/info/simulate.mdx`, `api/methods/info/broadcast_tx_sync.mdx`, `concepts/transactions.mdx`
- **Issue:** Examples access `exchange._info` / `exchange._chain_id` directly. These are leading-underscore (convention private) attributes.
- **Why:** STYLE_GUIDE §"Verification responsibility" says examples must be runnable. Using underscore attributes in public examples is fine if documented (and the concept page does document this), but the **method** pages don't disclaim it.
- **Fix:** Either expose public properties (out of docs scope) or add a one-liner note on `simulate.mdx` / `broadcast_tx_sync.mdx`: "the example reaches into private attributes for brevity — see [Concepts: Transactions](../../../concepts/transactions#direct-access-to-the-pipeline) for the full rationale."

#### 8. HL-compat Info constructor not documented

- **File:** No `migration/hl-compat/info.mdx` (or similar) class page.
- **Issue:** HL-compat Exchange and Info classes have substantial behavior (constructor gates: `account_address` required, `vault_address`/`spot_meta` rejected; `base_url=None` defaults to `LOCAL_API_URL`). Currently this lives only in the migration root page's "What you must change explicitly" section.
- **Why:** Consumers need a dedicated Client page to know how to instantiate the HL-compat facade.
- **Fix:** Add `migration/hl-compat/Exchange.mdx` and `migration/hl-compat/Info.mdx` using the Client template. Add to the migration sidebar.

#### 9. `concepts/clients.mdx` links to `./signers` but uses unrelated filename

- **File:** `concepts/clients.mdx` line 80
- **Issue:** The filename is `signers.mdx`; the cross-language consistency review (`.templates/review/cross-language.md` line 13) calls this out as filename divergence — TS and Rust use `signers-and-authentication.mdx`.
- **Why:** Filename style drift. Internally the link resolves (page exists), so this is not a build break, but the inconsistency is real.
- **Fix:** Rename `concepts/signers.mdx` → `concepts/signers-and-authentication.mdx` and update incoming links, OR leave as-is and document the language-specific filename in the sitemap.

#### 10. Trailing/banned cross-language refs missing

- **Files:** All concept pages.
- **Issue:** STYLE_GUIDE permits cross-language refs in Concept pages but the Python concepts don't include any. `concepts/rate-limits.mdx` should arguably mention that all three SDKs share the 167/10s + 30 subs caps.
- **Why:** Style-guide-permitted, not required. Sitemap §"Cross-language linking policy" says cross-language links are **allowed** on Concept pages.
- **Fix:** Optional. Consider adding "See also: [TS rate limits](../../typescript/concepts/rate-limits) and [Rust rate limits](../../rust/concepts/rate-limits)" footers if you adopt the cross-language linking pattern (note: TS and Rust drafters did not, per cross-language review).

### Nits

#### 11. `concepts/clients.mdx` references `Concepts: Clients` self-link

- **File:** `api/classes/Exchange.mdx:155`, `api/classes/Info.mdx:130`, others
- **Issue:** "See also" links to `Concepts: Transactions` etc.; pages have `../../concepts/transactions` paths. Some have `../concepts/clients` and others have `../../concepts/clients`. Both resolve from their respective depth. Verified to load — no fix needed but the relative depths are inconsistent between sibling directories.

#### 12. `concepts/signers.mdx` uses `wallet` in scope before defining it

- **File:** `concepts/signers.mdx` lines 21-25
- **Issue:** Example shows `Secp256k1Wallet.from_mnemonic("test test test ...", Addr("0xaccount"))` and then `SingleSigner(wallet, Addr("0xaccount"))`. The `wallet` variable is not introduced — implicit re-binding from the previous line. Reader can infer but it's a small lapse.
- **Fix:** Bind `wallet = Secp256k1Wallet.from_mnemonic(...)` on its own line, then use it.

#### 13. `concepts/encoding-and-types.mdx` says `TimeInForce.GTC.value == "GTC"` etc. and notes mixed case conventions

- **File:** `concepts/encoding-and-types.mdx` line 51-62
- **Issue:** Section claims `TriggerDirection.ABOVE.value == "above"`. Source confirms `class TriggerDirection(StrEnum): ABOVE = "above"`. Section also mentions `KeyType` uppercase wire values and `CandleInterval` uppercase indexer wire form. All verified accurate.
- **Fix:** none — kept for the record.

#### 14. `Exchange.mdx` `set_referral` parameter description says "Negative values are rejected" — verified against source

- **File:** `api/methods/exchange/set_referral.mdx`
- **Issue:** Matches source guard at `exchange.py:702-704`. Also rejects `bool` and empty string. Page documents both.
- **Fix:** none — kept for the record.

#### 15. `simulate.mdx` example references undefined `exchange` and `messages`

- **File:** `api/methods/info/simulate.mdx:20`
- **Issue:** `unsigned = exchange.signer.build_unsigned_tx(messages, exchange._chain_id)` — `exchange` and `messages` are not declared in the snippet. Reader has to infer.
- **Fix:** Add the missing setup lines for an example that is "runnable in principle" (STYLE_GUIDE §"Examples").

#### 16. `concepts/transactions.mdx` "Direct access to the pipeline" example accesses private attributes

- **File:** `concepts/transactions.mdx:84-87`
- **Issue:** `exchange.signer.build_unsigned_tx(messages, exchange._chain_id)` and `exchange._info.simulate(...)`. The page does explicitly call this out: "The `_info` and `_chain_id` attributes are name-mangled by convention (leading underscore) — accessing them is fine for power users who need this seam." Good — this is the right place to document it.
- **Fix:** none — kept for the record. The reference page should link here as per Finding #7.

#### 17. `concepts/error-handling.mdx` Markdown code fence reads "text" for the hierarchy

- **File:** `concepts/error-handling.mdx:9`
- **Issue:** Uses ```text fenced. Acceptable.
- **Fix:** none.

#### 18. `Exchange.mdx` end-to-end example does NOT pass `account_address` matching the wallet's underlying address

- **File:** `api/classes/Exchange.mdx:121-127`
- **Issue:** `Account.from_key("0x...")` then `account_address=Addr("0x...")` — both are placeholder hex. Readers may not realize Dango account address ≠ EVM-derived address. The migration page and project-setup page both make this clear, but the Exchange class page only notes the decoupling in `Configuration → wallet`.
- **Fix:** Add a one-sentence callout in the end-to-end example: `# NOTE: Dango account address is independent from the wallet's EVM-derived address.`

#### 19. The `Exchange` page lists "Conditional orders (TP/SL)" but the wider concept doesn't appear in concepts/

- **File:** `api/classes/Exchange.mdx:88-93`, no matching concept page
- **Issue:** Reasonable since `submit_conditional_order` is well-documented at the method level. No Concept-page TP/SL primer exists, but the method page covers it.
- **Fix:** none. Kept for the record.

#### 20. Sidebar `Wallet` Protocol absence

- **File:** `vocs.config.ts:425-434`
- **Issue:** The "Classes" sidebar section lists 6 entries: `API`, `Exchange`, `Info`, `WebsocketManager`, `Secp256k1Wallet`, `SingleSigner`. The inventory also notes `Wallet` as a Protocol that should arguably appear.
- **Fix:** Optional — per Finding #5, document the Wallet Protocol either inline or via a new Type page.

## Build status

```
$ pnpm --filter @left-curve/docs build
…
12:31:54 AM [vite] found dead links:
  ./hl-compat/exchange/order in /Volumes/SanDisk Extreme SSD Media/Projects/leftcurve/monorepo/sdk/docs/pages/python/migration/hyperliquid.mdx
  ./hl-compat/exchange/order in /Volumes/SanDisk Extreme SSD Media/Projects/leftcurve/monorepo/sdk/docs/pages/python/migration/hyperliquid.mdx
…
✖ bundles failed to build: [postbuild] deadlinks found.
```

Failure cause: the migration root page references `./hl-compat/exchange/order` twice, and the page does not exist. After creating `order.mdx` (or removing the link), the Python section builds clean.

Two equivalent fixes:
1. Create `migration/hl-compat/exchange/order.mdx` (preferred — this restores parity with the inventory's 12-implemented count and resolves both link locations in one move).
2. Replace `./hl-compat/exchange/order` with `./hl-compat/exchange/market_open` (or any other existing page) — fragile and doesn't fix the underlying coverage gap.

## Items needing user judgment

- **Type-page granularity.** Inventory enumerates 70+ types; sidebar advertises 36. The omitted ones are mostly NewType aliases (`Uint64`, `Uint128`, `UsdValue`, `UsdPrice`, identifier brands) and event-payload TypedDicts (`OrderFilled`, `Liquidated`, ...). The encoding-and-types concept page covers the alias semantics. The event TypedDicts are arguably worth dedicated pages since callers `cast(OrderFilled, event["data"])` based on `eventType` strings — but adding 50+ pages may not be worth the noise. Recommend documenting the event payload TypedDicts as a single combined "Event payloads" reference page rather than 16 individual pages.
- **`set_expires_after` and the DEX warning.** The page documents that the method is a no-op store, not a DEX call. Whether to also warn "DEX disabled" is a judgement call — the underlying intent is DEX-related but the actual code path doesn't execute on chain.
- **`missing-methods.mdx` strategy.** Either fully enumerate or cut entirely. Splitting the difference (a stub that says "see the source") is worse than either alternative.
- **`Wallet` Protocol page.** The cleanest answer is a dedicated Type page, but the protocol has 4 members and is mostly used as a parameter type. A short section on `Secp256k1Wallet.mdx` may suffice — your call.

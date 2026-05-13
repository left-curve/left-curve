# Cross-Language Consistency Review

## Summary (counts)

- **Languages reviewed:** TypeScript, Python, Rust
- **Shared concept pages reviewed per language:** 7 (clients, signers, transactions, subscriptions, encoding-and-types, error-handling, rate-limits) plus index + 3 getting-started pages
- **Total `.mdx` files cross-read:** 33 (11 per language)
- **Major inconsistencies found:** 5
- **Minor inconsistencies / style drift:** 6
- **DEX-disabled callout violations:** 3 (none of the three Transactions pages contain the required `:::warning[DEX currently disabled]` callout, even though all three reference DEX/perps actions)
- **Branding / naming drift items:** 3
- **Linking policy violations:** 0 (no cross-language jumps found; one orphan link in Python clients.mdx is intra-language)
- **Filename divergence:** 1 (Python uses `signers.mdx`; TS and Rust use `signers-and-authentication.mdx`)

## Per-concept findings

### Clients

**Same story?** Mostly yes. All three pages teach: "pick a read client for queries, a writer for transactions." All three open with a mental model section, then enumerate the available client types.

**Findings:**
- TS and Rust both present a binary public/signer split as a primary mental model. Python presents a tri-class model (`Info` / `Exchange` / `WebsocketManager`), which is correct for the Python SDK but the parallel reader has to learn that `Info` ≈ `createPublicClient` ≈ `HttpClient`-for-queries, and `Exchange` ≈ `createSignerClient`. Worth adding one cross-mapping sentence in each "mental model" block (sitemap.md already allows cross-language links on Concept pages).
- Python concept page links to `[Signers & Authentication](./signers)` — but the file is `signers.mdx`. TS and Rust use the filename `signers-and-authentication.mdx`. The actual filename drift is real (not just a link error).
- TS includes a "Tree-shakable style" section; Python includes a "Read-only via `API`" section; Rust shows `QueryClientExt` blanket trait methods. Per-language adaptations are appropriate.

### Signers

**Same story?** Yes — all three teach the same architectural separation between key material and account state.

**Findings:**
- All three use "decouples key from account" framing (with `user_index` / `nonce` discussed) — consistent and accurate.
- All three correctly state that the signer increments the nonce optimistically on every sign call (success or failure).
- TS shows `PrivateKeySigner.fromMnemonic`, Rust shows `Secp256k1::from_mnemonic` with BIP-44 coin type 60, Python shows `Secp256k1Wallet.from_mnemonic` with `coin_type=60` default. Consistent.
- Filename divergence (above): Python is `signers.mdx`, others are `signers-and-authentication.mdx`. The sitemap.md uses "Signers & Authentication" as the canonical section name; Python should rename to match.

### Transactions

**Same story?** Yes for the lifecycle (build messages → simulate → sign → broadcast → poll). Different gas-buffer multipliers across SDKs (intentional per source: TS uses 1.3×, Rust example uses 1.5×, Python adds a fixed `DEFAULT_GAS_OVERHEAD` of 770_000). Each is correct for its SDK but readers crossing languages will notice — worth a one-line callout in each.

**Findings:**
- **CRITICAL: missing DEX-disabled callout.** All three Transactions pages reference DEX/perps actions (TS links to `swapExactAmountIn`; Python uses `submit_limit_order` as the worked example; Rust shows `dex::ExecuteMsg::SubmitOrders`). Per the style guide, every page documenting a DEX action must open with `:::warning[DEX currently disabled]`. None of the three Transactions concept pages include it. Either add the callout, or revise the examples to use non-DEX messages (TS could use `transfer`, Python could use `deposit_margin`, Rust already uses `transfer` in its main worked example and only mentions DEX in a small snippet).
- TS lifecycle says SDK "polls `queryTx` up to 30 times at 500ms intervals" — explicit. Rust says "wrap the loop in a bounded retry helper" and uses `0..40` in code. Python doesn't expose a polling helper here. These are SDK-specific facts; the reader needs to know each. Document the gap explicitly.
- Failure mode taxonomy is identical in spirit: simulation failure / commit failure / never lands. Wording differs but the model is shared.

### Subscriptions

**Same story?** Yes — all three describe one WebSocket per connection, multiplexed subscriptions, server keepalive at 30 s, client `ping` every 15 s, 30-sub cap, callback / stream-item delivery model.

**Findings:**
- TS describes a fixed list of "WS-only" subscriptions and a smaller list of "HTTP-fallback" subscriptions. Python and Rust don't have this fallback — WS-only is the only model. Consider noting in TS that this fallback is TS-specific.
- TS list has a typo: under "Four subscriptions fall back to HTTP polling," five items are listed (`tradesSubscription`, `perpsTradesSubscription`, `allPairStatsSubscription`, `allPerpsPairStatsSubscription`, `queryAppSubscription`). Either the count is wrong or one of the items doesn't belong.
- TS uses inline-callback `next:`/`error:` style on its example. Python uses positional callback + manual sub-id. Rust returns a `Stream`. The three styles are language-idiomatic and appropriate; no action needed.

### Encoding and Types

**Same story?** Yes — all three teach base-units-on-the-wire, plus the snake_case/camelCase boundary (Python is most explicit on this).

**Findings:**
- **Decimal numeric helper mismatch with packages page.** TS encoding-and-types line 37 has:
  ```ts
  import { Decimal, formatUnits, parseUnits } from "@left-curve/sdk"
  ```
  But TS `concepts/packages.mdx` lists `Decimal` as NOT re-exported from `@left-curve/sdk` (line 29: "Examples: `Decimal`, `sha256`, …"). Line 21 of the same encoding-and-types page correctly imports it from `@left-curve/utils`. Fix the import on line 37.
- Python explains the `dango_decimal()` helper and its 6-decimal canonicalization. TS uses `Decimal` directly (no canonicalization helper). Rust has no Decimal — uses raw `u128` / `Uint128`. Each is correct for its SDK; readers should not expect parity here.
- Python encoding-and-types is the most thorough on the case boundary (smart-contract = snake_case, indexer = camelCase). TS only mentions "snake_case on the wire and camelCase in TypeScript" with the SDK converting automatically. Rust does not discuss the snake/camel split at all because grug serde handles it. Each is correct.

### Error handling

**Same story?** Largely yes — every page teaches "what to catch, when to retry, how subscription errors differ."

**Findings:**
- TS hierarchy is `BaseError` → `HttpRequestError` / `TimeoutError` / `UrlRequiredError`. Python is `Error` → `ClientError` / `ServerError` / `GraphQLError` / `TxFailed`. Rust has only `anyhow::Error` (one-shots) and `WsError` (subscriptions). All three are documented accurately for their SDK.
- All three mention "subscription errors don't raise / arrive through a different path" (TS: error callback; Python: `_error` envelope; Rust: `Err(WsError::Subscription)` stream item). Consistent in spirit.
- TS has a candid caveat that `BaseError` is not re-exported today and `instanceof` requires importing from sub-paths. Python and Rust don't have this issue — their error classes are public. Acceptable.
- Rust's `Err(e) if e.to_string().contains("429")` pattern for detecting rate-limit responses is a string match; Python uses the same pattern (`if "429" in str(exc)`); TS uses `withRetry`. Cross-language readers will notice the lack of structured 429 detection; could be acknowledged in each rate-limits page.

### Rate Limits

**Same story?** Yes — identical limits (167 reqs / 10 s, 30 subs / WS connection) explicitly stated in all three. All three explicitly state the SDK does NOT auto-handle.

**Findings:**
- Wording for the HTTP limit varies slightly:
  - TS: "167 requests per 10 seconds per IP"
  - Python: "167 requests / 10 s … from one IP"
  - Rust: "167 requests per 10 seconds, per source IP"
  Same fact, three phrasings — acceptable but could be tightened.
- All three describe the same sharding strategy (one connection per `client` / `Info` / `Session`).
- TS rate-limits page "Next" section links back to Packages. Python and Rust "Next" sections link forward to Subscriptions / Error handling. Inconsistent terminal-navigation; the sitemap places `rate-limits` last, so all three should arguably link forward to the API Reference, or all three should link back. Pick a convention.

## Terminology drift

| Concept | TS | Python | Rust | Recommendation |
|---------|-----|--------|------|----------------|
| Read-only client | "public client" / `createPublicClient` | `Info` | `HttpClient` (queries via trait methods) | Each is SDK-native — keep, but each `clients.mdx` should add a one-sentence cross-language mapping for readers jumping SDKs. |
| Write client | "signer client" / `createSignerClient` | `Exchange` | `HttpClient` + `SingleSigner` (no unified writer) | Same as above. |
| "Transaction" | message list + signing → tx | `Tx` envelope → `BroadcastTxOutcome` | `NonEmpty<Vec<Message>>` → `Tx` | Identical concept; phrasing varies. OK. |
| "Subscription" | "WebSocket subscription" | "subscription frame on a managed connection" | "`graphql-transport-ws` subscription" | All correct; Rust is most precise about the wire protocol. Consider adding the protocol name (`graphql-transport-ws`) to TS and Python for parity. |
| Section name "Signers & Authentication" | `signers-and-authentication` (consistent w/ sitemap) | `signers` (deviates) | `signers-and-authentication` (consistent) | Rename Python file to match. Multiple intra-Python links are already broken: `clients.mdx` → `./signers`, `transactions.mdx` → `./signers`, but the file is named `signers.mdx`, so they happen to resolve. The sitemap canonical name is "Signers & Authentication" — Python is the outlier. |
| Base units terminology | "base units" + `Decimal` for human | "base units" vs `UsdValue` for withdraws (intentional asymmetry) | "base units" only; no `Decimal` | All three say "base units." Asymmetry inside Python is documented. OK. |
| "Connection" (WS) | "WebSocket connection" / "WS" | "WebSocket connection" | "`Session` (owns one socket)" / "connection" | Consistent enough. |

## Factual drift

1. **Mainnet HTTP URL.** The TypeScript `getting-started/project-setup.mdx` (line 18) claims mainnet is `https://api.dango.zone`. Every other doc page in the repo (Python, Rust, and the actual source at `sdk/typescript/dango/src/chains/definitions/mainnet.ts`) uses `https://api-mainnet.dango.zone`. **TS is wrong** — fix the table.

2. **TS `Decimal` re-export.** TS `concepts/encoding-and-types.mdx` (line 37) imports `Decimal` from `@left-curve/sdk`, but TS `concepts/packages.mdx` explicitly says `Decimal` is NOT re-exported from `@left-curve/sdk` and must come from `@left-curve/utils`. Two pages disagree; the packages page matches reality (verified at `sdk/typescript/utils`). **Fix the import on line 37** of encoding-and-types.mdx.

3. **Python first-call uses `MAINNET_API_URL` while TS first-call uses `testnet`.** Acceptable per per-language adaptation (Python's HL roots default to mainnet), but cross-language readers may notice. Worth a one-line acknowledgement in either intro.

4. **Subscription HTTP-fallback count mismatch (TS).** TS subscriptions page claims "Four subscriptions fall back to HTTP polling" but then lists five (`tradesSubscription`, `perpsTradesSubscription`, `allPairStatsSubscription`, `allPerpsPairStatsSubscription`, `queryAppSubscription`). Either the count or one list entry is wrong.

5. **TS first-call gas-buffer mention.** TS transactions page says simulate returns `gasUsed * 1.3` — Rust example uses 1.5×, Python adds a fixed 770_000. Per-SDK correct; readers crossing should not assume parity.

## Branding/naming drift

1. **Capitalisation of "Dango SDK" / "TypeScript SDK".**
   - TS index H1: `# TypeScript SDK` (no "Dango" prefix)
   - Python index H1: `# Dango Python SDK`
   - Rust index H1: `# Dango Rust SDK`
   - Recommendation: TS H1 should be `# Dango TypeScript SDK` for parity, or all three should drop the "Dango" prefix.

2. **Section header capitalisation in "Next" footers.**
   - TS uses "Project Setup" (Title Case).
   - Python uses "Project setup" (Sentence case).
   - Rust uses "Project setup" (Sentence case).
   - Both styles appear consistently within each language, but the divergence shows up when readers jump SDKs. The style guide doesn't pick one — pick one.

3. **"5-minute hello world" vs "five-minute hello-world."** The sitemap uses "5-minute hello world." TS uses "five-minute hello world." Python uses "five minutes." Rust uses "five-minute hello-world." Minor — pick one.

4. **Package distribution names.** TS says SDK ships as `@left-curve/sdk` and 5 siblings; Python says it ships as `dango`; Rust says it ships as `dango-sdk`. All accurate; no drift.

## Linking policy violations

- **No cross-language jumps found** in any of the reviewed Concept pages — they correctly stay within their own language section.
- Python `concepts/clients.mdx` → `./signers`, `concepts/transactions.mdx` → `./signers` resolve only because the Python file is named `signers.mdx` (a sitemap deviation; see filename drift). If the file is renamed to `signers-and-authentication.mdx` for sitemap conformance, both intra-Python concept links break.
- Per style guide §"Linking": "Inside a Reference page: only intra-language links." Spot-checked Reference pages in `sdk/docs/pages/{ts,python,rust}/api/**` for the relevant types — no cross-language links found. Reference policy is upheld.

## Build status

No build attempted by this reviewer (scope: cross-language consistency only). The factual drift items above (mainnet URL, `Decimal` import path, subscription count, Python signers file naming) would not break the Vocs build because Vocs does not type-check MDX code blocks. They will surface as runtime errors if a reader copies the examples verbatim.

## Top recommendations (in priority order)

1. **Add `:::warning[DEX currently disabled]` callout** to all three Transactions concept pages where DEX/perps usage is shown, or remove the DEX examples from the body. This is a style-guide hard requirement.
2. **Fix mainnet URL** in TS `getting-started/project-setup.mdx` line 18 (`https://api.dango.zone` → `https://api-mainnet.dango.zone`).
3. **Fix `Decimal` import** in TS `concepts/encoding-and-types.mdx` line 37 — should come from `@left-curve/utils`, not `@left-curve/sdk`.
4. **Decide on Python signers filename.** Either rename `signers.mdx` → `signers-and-authentication.mdx` (sitemap conformance) and update intra-Python links, or amend the sitemap to allow per-language shorter names.
5. **Reconcile TS subscriptions HTTP-fallback count vs list** in `concepts/subscriptions.mdx`.
6. **Add a one-line cross-mapping** in each `concepts/clients.mdx` ("read-only client" / "writer client") so readers crossing SDKs find the analogous shape quickly.
7. **Pick a "Next" convention** for the terminal concept page (`rate-limits.mdx`) across all three SDKs — currently inconsistent (TS links back; Python and Rust link forward).
8. **Standardize index H1** ("Dango TypeScript SDK" vs "TypeScript SDK") and case across "Next" links.

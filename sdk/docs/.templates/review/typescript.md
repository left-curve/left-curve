# TypeScript SDK Docs Review

## Summary
- Pages reviewed: 168
- Must-fix: 5
- Should-fix: 9
- Nits: 8

## Coverage check

### Inventory items NOT documented
The inventory enumerates many items intentionally not given a dedicated page (per sitemap, only "major" symbols get pages). The following are exported by the SDK but have no dedicated page; all are reasonable to keep undocumented as separate pages, but several are referenced by other pages and should be linked appropriately:

- `createTransport` — referenced in every example; no dedicated page. Inventory's verification TODO calls out that this should be the primary public surface of the transport package. Currently it only surfaces via `Concepts: Rate Limits` (mentions `batch`) and example imports. Consider a Client page or Concept add‑on.
- Chain configs `local`, `devnet`, `testnet`, `mainnet` — surfaced in `getting-started/project-setup.mdx` only.
- Account helpers `computeAddress`, `isValidAddress`, `createAccountSalt`, `createKeyHash`, `createSignBytes`, `toAccount` — referenced from concept/types pages but no dedicated page. Several types pages link to `../concepts/encoding-and-types` as if these helpers were documented there; that file does not actually document them.
- Signers `PrivateKeySigner`, `createSessionSigner` — only `signers-and-authentication.mdx` Concept covers them; no Reference page.
- Action builders `publicActions`, `signerActions` (and per‑domain builders) — only referenced in `createBaseClient.mdx`.
- `Direction`, `OrderType`, `TimeInForceOption` const maps — re‑exported from `@left-curve/sdk` but absent.
- Most `@left-curve/types` types (Inventory lists 178; sidebar lists 42). Sitemap policy is "major types only", so this is acceptable — but the README at `index.mdx` links only `./types/Address` as the types entrypoint.
- `@left-curve/crypto` actions (`sha256`, `keccak256`, `Secp256k1`, `Ed25519`, etc.), `@left-curve/encoding` actions, `@left-curve/utils` actions (`Decimal`, `formatNumber`, `withRetry`, etc.) — no dedicated pages; references are scattered across Concept pages.
- Errors: re-export note in inventory says `BaseError`/`HttpRequestError`/`TimeoutError`/`UrlRequiredError` are NOT re‑exported from `@left-curve/sdk` entry. Docs partly note this but the example imports use `@left-curve/sdk/errors` which is also not exported (see Must-fix).

### Pages without corresponding inventory entry
None — every `.mdx` file under `pages/typescript/` corresponds to a documented action/type/client/error/concept in the inventory.

### Sidebar entries pointing to missing files
None. All `link:` values in `vocs.config.ts` resolve to an existing `.mdx`.

### Files not in sidebar
- `/typescript/index` — intentional (root page).

### Excluded items
None present:
- No `mockValidatorSet` / `mockValidatorSign` page.
- No `INFURA_URLS` page.
- No `webrtrc` page or references.
- No `CurveInvariant` / `CurveInvariants` page (the type is referenced inside `PairParams` only, which is fine).

## Findings

### Must-fix

- **File**: `sdk/docs/pages/typescript/actions/dex/*.mdx` (all 17 pages) and `sdk/docs/pages/typescript/actions/perps/*.mdx` (all 27 pages).
  - **Issue**: None of the DEX or Perps action pages opens with the required `:::warning[DEX currently disabled]` callout. The Style Guide names this as required for every page under `actions/dex/` and `actions/perps/`.
  - **Why it must fix**: Required by `STYLE_GUIDE.md` §"Status callouts"; the explicit task brief flags any missing callout as Must‑fix.
  - **Suggested fix**: Insert the canonical callout immediately under the H1 description on each of the 44 pages:
    ```mdx
    :::warning[DEX currently disabled]
    The Dango DEX is currently disabled. Calls described on this page will not execute on the live network until the DEX is enabled.
    :::
    ```

- **File**: `sdk/docs/pages/typescript/errors/BaseError.mdx` (and reflected in `concepts/packages.mdx`).
  - **Issue**: The example `import { BaseError } from "@left-curve/sdk/errors"` does not resolve. `sdk/typescript/dango/package.json` only exposes `.`, `./actions`, and `./hyperlane` exports; there is no `./errors` subpath. The `BaseError` is also not re-exported from `@left-curve/sdk/index.ts`. The page itself acknowledges the limitation in Notes but still uses the broken import in the Construction example.
  - **Why it must fix**: Factually wrong / runnable‑example contract — a reader copying the import will get a module‑resolution error.
  - **Suggested fix**: Either (a) drop the example import and demonstrate `instanceof Error && err.name === "BaseError"` narrowing (matches Notes section), or (b) propose adding `./errors` to the package exports and re-export from the entry barrel (per Inventory Verification TODOs). Until (b) lands, the page must not show an import that does not resolve. Same for the `packages.mdx` line:
    ```ts
    import { BaseError } from "@left-curve/sdk/errors" // currently NOT exported — see Error Handling
    ```
    The trailing comment says "currently NOT exported" but the line itself is still presented as a valid example.

- **File**: `sdk/docs/pages/typescript/getting-started/project-setup.mdx`.
  - **Issue**: The chain‑id table is factually wrong for two of four chains:
    - `local` is documented as `dev-1`; source `sdk/typescript/dango/src/chains/definitions/local.ts` says `localdango-1`.
    - `mainnet` is documented as URL `https://api.dango.zone`; source says `https://api-mainnet.dango.zone`.
  - **Why it must fix**: Anyone copying these values will hit the wrong endpoint or assert the wrong chain id.
  - **Suggested fix**: Replace the `local` row with `id: localdango-1`, replace the `mainnet` URL with `https://api-mainnet.dango.zone`. (testnet and devnet rows match source.)

- **File**: `sdk/docs/pages/typescript/concepts/clients.mdx`.
  - **Issue**: The end of the file uses `Next:` semantics as a content section heading rather than a footer per the concept template. More importantly, the `signers-and-authentication.mdx` claim that `PrivateKeySigner` "signs the full payload it receives (both signTx and signArbitrary)" is incorrect: `sdk/typescript/dango/src/signers/privateKey.ts:50` extracts `payload.message` from the `ArbitraryDoc` and signs only that. The only behavioral divergence from `createSessionSigner` is the value of the returned `signed:` field (full `payload` vs `message`).
  - **Why it must fix**: Conceptually wrong — readers will reason incorrectly about what the primary signer actually signs.
  - **Suggested fix**: Rewrite the "PrivateKeySigner" paragraph in `concepts/signers-and-authentication.mdx`:
    > Both `signTx` and `signArbitrary` hash `payload.message` (after canonical serialization) and sign the hash. The two signer implementations differ only in the `signed` field they return: `PrivateKeySigner.signArbitrary` returns `signed: payload`, while `createSessionSigner.signArbitrary` returns `signed: message`.
  - Update `types/Signer.mdx` "Built-in implementations" bullet to remove the now‑misleading note on session signer divergence.

- **File**: `sdk/docs/pages/typescript/concepts/rate-limits.mdx`.
  - **Issue**: `createTransport({ batch: true })` is shown — but the actual API is `createTransport(url?, config?)` where `batch` is on the second arg. The page shows `createTransport(undefined, { batch: true })` correctly later, but the type signature in the `Concepts: Rate Limits` section ought to align with the source. Also: `batch` is described as `boolean | JsonRpcBatchOptions` (paraphrased) but the source type is just `batch?: boolean` (with the actual `maxSize: 20, maxWait: 20` hard-coded internally when `batch: true`).
  - **Why it must fix**: API surface mismatch — a reader will try `batch: { maxSize: 50 }` and the type-checker will reject it.
  - **Suggested fix**: Replace the description of `batch` with "Enable HTTP request batching. When `true`, concurrent requests are coalesced with `maxSize: 20` and `maxWait: 20ms`. Currently a boolean toggle; the batch window is not configurable from the public API."

### Should-fix

- **File**: `sdk/docs/pages/typescript/concepts/subscriptions.mdx`.
  - **Issue**: Lists `queryAppSubscription` under the WS+HTTP-fallback section but in the "WS-only" list omits it. Specifically, the WS-only bullets miss `queryAppSubscription`, while it is then listed in the fallback section — correct — but the list is given five bullets ("Four subscriptions") which contradicts itself. Also missing from WS-only: it doesn't mention that `eventsByAddressesSubscription` is WS-only.
  - **Suggested fix**: Re-count the lists: the WS-only set is `blockSubscription`, `accountSubscription`, `candlesSubscription`, `eventsSubscription`, `eventsByAddressesSubscription`, `transferSubscription`, `perpsCandlesSubscription`. The fallback set is `tradesSubscription`, `perpsTradesSubscription`, `allPairStatsSubscription`, `allPerpsPairStatsSubscription`, `queryAppSubscription` (5 items, not "Four").

- **File**: `sdk/docs/pages/typescript/concepts/error-handling.mdx`.
  - **Issue**: Says errors must be imported from "the type package's internal paths" — but the actual paths are `#errors/*` aliases inside the `@left-curve/sdk` (dango) package, not `@left-curve/types`. The phrase "type package" is misleading.
  - **Suggested fix**: Rewrite the sentence to: "import them via the dango package's internal `#errors/*` paths or check by `name`."

- **File**: `sdk/docs/pages/typescript/concepts/rate-limits.mdx`.
  - **Issue**: The "Plan accordingly for production workloads" close is acceptable but the example list of "75 pairs" is contrived (use a realistic number like 100). The mock unsub assignment for `unsubs` is unused.
  - **Suggested fix**: Either consume `unsubs` (return it, or comment "later: unsubs.forEach(u => u())") or drop the `unsubs` binding for a `for…of` over each pair.

- **File**: `sdk/docs/pages/typescript/clients/createPublicClient.mdx`.
  - **Issue**: Method tables omit `queryIndexer`. The inventory groups `queryIndexer` under indexer queries and the sidebar includes it, but the createPublicClient Methods table for Indexer does not list it.
  - **Suggested fix**: Add row for `[queryIndexer](../actions/indexer/queryIndexer)` to the Indexer table.

- **File**: `sdk/docs/pages/typescript/actions/indexer/queryIndexer.mdx`.
  - **Issue**: The Example calls `client.request<>(...)` instead of `client.queryIndexer(...)`. The Notes say "queryIndexer is not on the client surface" but it actually IS on the client (per `indexerActions.ts` and the sidebar entry), so the steered-around example creates confusion.
  - **Suggested fix**: Show `await client.queryIndexer<T>({ document, variables })` (matches the page's own signature), drop the misleading Note.

- **File**: `sdk/docs/pages/typescript/actions/gateway/getWithdrawalFee.mdx` and `transferRemote.mdx`.
  - **Issue**: Required Gateway-namespacing note is present, but the link target `[Concepts: Clients](../../concepts/clients)` lands on a page that mentions gateway namespacing only at the bottom. Reasonable, but the See-also link reads as if the concept page is the primary source.
  - **Suggested fix**: Keep the link; the Gateway page note already states the rule.

- **File**: `sdk/docs/pages/typescript/actions/account-factory/registerAccount.mdx`.
  - **Issue**: Signature shows `txParameters` as the second positional argument with `{ gasLimit?: number }`. The actual source is `txParameters: TxParameters` (an exported type). Documented shape is correct in practice (`TxParameters = { gasLimit?: number }`), but referring to the underlying `TxParameters` type matches the inventory.
  - **Suggested fix**: Annotate as `txParameters?: TxParameters` and link to a future `TxParameters` page (or inline `{ gasLimit?: number }` keeps the doc readable — current form is acceptable).

- **File**: `sdk/docs/pages/typescript/actions/app/queryStatus.mdx`.
  - **Issue**: The returned `BlockInfo.height` is described as `string`. Source `queryStatus.ts:34` returns `response.block.blockHeight.toString()` so the runtime string is correct, but the field name on the wire is `blockHeight` (camelCase from indexer) and `queryStatus.ts` maps it to `height: response.block.blockHeight.toString()`. The doc correctly reports `height` — confirming `BlockInfo`. Good.
  - **Suggested fix**: No change required; flag only because `BlockInfo` page also says `height: string` while inventory marks block heights elsewhere as `number`. Internally inconsistent but accurate to source.

- **File**: `sdk/docs/pages/typescript/actions/app/getBalance.mdx`.
  - **Issue**: Notes warn about `2^53` precision, but Inventory's verification TODO calls out that the function is also inconsistent with other balance‑shaped fields (which use `string`). Doc would benefit from one line that says "all other balance APIs return strings; this one is an outlier."
  - **Suggested fix**: Append to the Notes: "All other balance-shaped APIs (`getBalances`, `getSupply`, etc.) return strings; `getBalance` is an outlier."

### Nits

- **File**: `sdk/docs/pages/typescript/clients/createPublicClient.mdx`.
  - **Issue**: Notes uses "just" ("Method calls are just property reads…"). The Style Guide bans "just".
  - **Suggested fix**: "Method calls are property reads followed by function calls."

- **File**: `sdk/docs/pages/typescript/actions/indexer/queryBlock.mdx`.
  - **Issue**: Uses "just" in See‑also ("just the chain id…"). Same Style Guide rule.
  - **Suggested fix**: "the chain id and latest block header only."

- **File**: `sdk/docs/pages/typescript/types/PerpsEvent.mdx`.
  - **Issue**: H1 description uses an em-dash (`A perps event — fill, liquidation, or deleverage.`). Style Guide says "No em-dashes in body prose"; the question is whether the one-line H1 description counts. Other one-liners in the repo use em-dashes liberally — the rule is enforced softly throughout. If treating consistently: many other Type pages also use em-dashes in body (e.g. concepts/encoding-and-types.mdx). Recommend either documenting the exemption (em-dash allowed in field definition lists and short clauses) or globally replacing with semicolons.

- **File**: `sdk/docs/pages/typescript/concepts/error-handling.mdx`.
  - **Issue**: `Pages` H2 section is not part of the Concept template. It's a useful bonus but renames the See‑also pattern. Consider folding into the existing Next: list or renaming to "See also".

- **File**: `sdk/docs/pages/typescript/actions/account-factory/registerUser.mdx`.
  - **Issue**: Example doesn't show how to construct a `signature: Signature` — the comment says "Build the signature off-band per the chain's registration protocol" but no link to where that's documented. This is honest but a dead end for the reader.

- **File**: `sdk/docs/pages/typescript/actions/app/upgrade.mdx`.
  - **Issue**: Example uses `await upgrade(client, {...})` (free-function form) and Notes explain why. Consistent with `configure` which has the same pattern. Reads well; tab showing both forms (as suggested by the Style Guide) would be a small polish.

- **File**: `sdk/docs/pages/typescript/actions/app/broadcastTxSync.mdx`.
  - **Issue**: Example uses `credential: { /* ... */ }` and `data: { /* metadata */ }` — comments rather than realistic values. Style Guide says examples should be runnable in principle and avoid empty placeholders.
  - **Suggested fix**: Either inline a realistic credential/data, or shorten the example to show only the call shape and link to `signAndBroadcastTx` for the realistic flow.

- **File**: `sdk/docs/pages/typescript/actions/app/queryStatus.mdx`.
  - **Issue**: The See-also points to `queryBlock` and `queryApp`, but `queryStatus` is the chain-id and latest-block accessor, so adding a link to `queryTx` (since both surface tx_response) or to `Concepts: Transactions` would round out navigation.

## Build status

Run `pnpm --filter @left-curve/docs build`:

- **Status**: FAIL.
- **Cause**: Dead link `./hl-compat/exchange/order` in `sdk/docs/pages/python/migration/hyperliquid.mdx`. This is a Python-side failure, not a TypeScript page issue.
- **TypeScript-related build errors**: None. All `.mdx` files under `pages/typescript/` compile; no missing imports, no broken intra-typescript links, no malformed frontmatter.

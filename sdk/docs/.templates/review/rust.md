# Rust SDK Docs Review

## Summary

- **Pages reviewed:** 49 `.mdx` files under `sdk/docs/pages/rust/`
  - 1 root (`index.mdx`)
  - 3 getting-started
  - 7 concepts
  - 5 client pages (`HttpClient`, `WsClient`, `Session`, `SingleSigner`, `Keystore`)
  - 14 `HttpClient` methods
  - 4 `WsClient` methods + 1 `Session` method
  - 12 `SingleSigner` methods
  - 2 `Keystore` methods
  - 2 subscription pages (`SubscriptionStream`, `SubscriptionVariables`)
  - 1 trait (`Secret`), 1 type (`PageInfo`), 1 error (`WsError`)
- **Must-fix:** 7 (broken imports, broken constructor calls, wrong field names in 3 method examples, inaccurate `BroadcastTxOutcome` description, missing DEX-disabled warning on Transactions concept)
- **Should-fix:** 8
- **Nits:** 7
- **Build status (Rust-attributable):** PASS. Build fails overall but only due to Python deadlinks (`./hl-compat/exchange/order`). No Rust-attributable errors. No deadlinks emitted from any `pages/rust/` file.

## Coverage

- Every inventory method has a dedicated page or is consolidated (`paginate_*` macro-generated set, the 6 trait method impls on `HttpClient`, the 11 `SingleSigner` methods incl. trait impls, the 2 `Keystore` statics, both `WsClient`/`Session` `subscribe` impls).
- Every sidebar link in `vocs.config.ts` resolves to a file on disk; spot-checked by enumerating the directory tree and the `rustSidebar()` function.
- Every file under `pages/rust/` is reachable from the sidebar (no orphans).
- `SubscriptionVariables.mdx` is a **single** consolidated page with the required 13-variant table (variant module → query type → description). Correctly NOT broken into 13 pages.
- `QueryClientExt` methods are flagged as `(from grug)` on the `HttpClient` page and a parallel table appears on the Clients concept. Link target is `https://docs.rs/grug`, a placeholder until grug is on docs.rs — acceptable.
- `SubscribeBlock`–`SubscribeQueryStatus` (14 marker structs) and `SubscribeEventByAddresses` correctly identified. The trait is implemented for 13 of the 14 (no `subscribe_event_by_addresses` impl on the marker side; only on the `Variables`) — but the docs say "13 codegen variants" consistently.

## Must-fix

1. **`pages/rust/getting-started/first-call.mdx:10` — broken import.** Example uses `use dango_sdk::{BlockClient, HttpClient};` but `BlockClient` is a `grug` trait and is **not** re-exported by `dango_sdk` (`lib.rs:7` does `pub use {client::*, indexer_graphql_types::*, keystore::*, secret::*, signer::*, subscription::*};` — no grug re-exports). The same paragraph below the example correctly states "`BlockClient` is the grug trait that supplies `query_block`" — so the example contradicts itself. Fix: `use dango_sdk::HttpClient;` plus `use grug::BlockClient;`.

2. **`pages/rust/api/methods/single-signer/with_user_index.mdx:32` — broken constructor call.** Example uses `UserIndex::new(42)`. `UserIndex` is a type alias for `u32` (`dango/types/src/account_factory/username.rs:15`) — there is no `::new` constructor. Use a plain integer: `.with_user_index(42u32)` or `.with_user_index(42)`.

3. **`pages/rust/api/methods/http-client/query_app.mdx:34` — wrong enum variant syntax.** Example calls `client.query_app(Query::Config {}, None)`. `grug::Query::Config(QueryConfigRequest)` is a tuple variant, not a struct variant, so struct-literal syntax does not compile. Fix: `Query::Config(QueryConfigRequest {})` (and import `QueryConfigRequest`). Or pick a different example variant.

4. **`pages/rust/api/methods/http-client/paginate_transactions.mdx:29` — wrong field name.** Example sets `sender: Some(...)` but `transactions::Variables` has `sender_address` (from GraphQL `$senderAddress`). Fix: `sender_address: Some(...)`.

5. **`pages/rust/api/methods/http-client/paginate_events.mdx:29` — non-existent field.** Example sets `event_type: Some("dango_dex::order_filled".into())` but `events::Variables` only has `after`, `before`, `first`, `last`, `sort_by` (see `indexer/graphql-types/src/schemas/queries/events.graphql`). Fix: drop the `event_type` filter (or move the example to use a Variables struct that does have an event-type filter — `subscribe_events::Variables` has a `CheckValue` filter, but that's a subscription, not a paginated query). Easiest: drop the filter and call `events::Variables::default()`.

6. **`pages/rust/api/methods/http-client/broadcast_tx.mdx:40` — inaccurate Returns description.** Says `BroadcastTxOutcome` is "`tx_hash`, success bit, optional error message". The actual struct (`grug/types/src/outcome.rs:285`) is `{ tx_hash: Hash256, check_tx: CheckTxOutcome }`. There is no top-level success bit or error message — the success result is `check_tx.result: Result<…, …>` and the convenience accessor is `into_result() -> Result<BroadcastTxSuccess, BroadcastTxError>`. Update the field list and link to `BroadcastTxOutcome::into_result` (or note it).

7. **`pages/rust/concepts/transactions.mdx` — missing DEX-disabled warning.** The page demonstrates submitting a DEX `Message` (lines 38–48 show `Message::execute(/* contract */, &dex::ExecuteMsg::SubmitOrders { … }, Coins::new())`). Per the style guide (`STYLE_GUIDE.md` §"Status callouts → DEX currently disabled") any page documenting DEX action submission must open with `:::warning[DEX currently disabled]…:::`. The warning is **absent**. Add the warning callout immediately under the H1 description (per task requirement).

## Should-fix

1. **`pages/rust/concepts/encoding-and-types.mdx:66` — overstated derive claim.** Says "All generated types derive `Debug + Clone + PartialEq + Eq`. `Variables` types derive `Default`…". Per `indexer/graphql-types/src/lib.rs`, the codegen macro splits derives: `response_derives = "Debug, Clone, PartialEq, Eq"` (for `ResponseData` and `*Nodes`), but `variables_derives = "Debug, Clone, Default"` — so `Variables` types do **not** derive `PartialEq, Eq`. Rephrase to: "Response types derive `Debug + Clone + PartialEq + Eq`. Variables types derive `Debug + Clone + Default`."

2. **`pages/rust/concepts/signers-and-authentication.mdx:25` — `Mnemonic::new` error conversion.** `Mnemonic::new(…)?` uses `?` against `bip32::Error`, but `bip32::Error` does not implement `std::error::Error` in older versions (the Secret page example correctly uses `.map_err(|_| anyhow::anyhow!("bad mnemonic"))?`). Either drop the snippet, switch to `.map_err`, or document the dependency on `anyhow::Error: From<bip32::Error>`.

3. **`pages/rust/concepts/signers-and-authentication.mdx:73` — example signature returns `_, _`.** `async fn for_key(…) -> Result<SingleSigner<Secp256k1, _, _>> {…}` uses `_` in a non-inferable position (the return type of a free function). Rust will reject this with "the trait bound is not satisfied" / "type annotations needed". Replace with `Result<SingleSigner<Secp256k1, Defined<UserIndex>, Undefined<Nonce>>>` (and import the typestate marker types).

4. **`pages/rust/api/methods/single-signer/with_user_index.mdx:23`, `with_nonce.mdx:23`, `new.mdx:23` — return-type `_` placeholders.** Same issue as above: `fn build() -> Result<SingleSigner<Secp256k1, _, _>>` is not valid Rust outside trait/return-position inference. These compile under `Result<SingleSigner<Secp256k1, _, _>>` only if the placeholders are inferable from a return value — but the value's typestate generics are unrelated to the call site. Fix by spelling out the typestate, e.g. `Result<SingleSigner<Secp256k1, Defined<UserIndex>, Undefined<Nonce>>>`.

5. **`pages/rust/api/methods/single-signer/sign_transaction.mdx:56` — "mutates self.nonce on every success" framing.** The source mutates **even when the inner `Secret::sign_transaction` errors out before that line, since `*self.nonce.inner_mut() += 1` happens at the top of the body, before signing.** Reviewing `signer.rs:271-272`:
   ```rust
   let nonce = self.nonce();
   *self.nonce.inner_mut() += 1;
   ```
   the increment is unconditional. Worth tightening the wording from "on every success" to "on every call (before signing); a `StdError` from JSON encoding or `Secret::sign_transaction` leaves `nonce` already incremented".

6. **`pages/rust/api/methods/keystore/from_file.mdx:7` — return type missing `dango_sdk::` qualifier.** Listed as `-> anyhow::Result<[u8; 32]>`. The actual source uses `anyhow::Result<[u8; 32]>` (correct), but the function lives at `dango_sdk::Keystore::from_file` and the example imports `dango_sdk::Keystore`. The signature block doesn't show `impl Keystore { … }` framing the way the `write_to_file` page does. Wrap with `impl Keystore { pub fn from_file(…) … }` for symmetry with `write_to_file.mdx`.

7. **`pages/rust/concepts/clients.mdx:60` — typo/dead link to grug docs.** Link reads `[grug `QueryClientExt` docs](https://docs.rs/grug)` but the crate is `grug` and may not be published to docs.rs (it is part of this monorepo). Verify before publish; either drop the external link or link to the grug source on GitHub.

8. **`pages/rust/concepts/subscriptions.mdx:132` — "Send but not Sync" claim.** The source type is `Pin<Box<dyn Stream<Item = Result<Response<T>, WsError>> + Send>>` — explicitly `+ Send`, no `Sync` bound. The doc says "subscription streams are `Send` but not `Sync`. Move them into a single task". This is correct technically (Sync is not auto-implemented for trait objects without an explicit bound), but the doc could be more precise: this is **not** unusual or restrictive for `Stream` consumers; `Stream::next` only needs `&mut self`, not shared access. Soften the wording or drop the warning — most users will hit this only if they try to share `&stream` across tasks, which is unusual.

## Nits

1. **`first-call.mdx:21–22` — `block.info.timestamp` is a `grug::Timestamp`.** `println!("timestamp: {}", block.info.timestamp);` will work if `Timestamp` implements `Display`; verify in source.

2. **`signers-and-authentication.mdx:103` — example after `.from_file` returns `[u8; 32]`.** Doc says "wrap with `Eip712::from_bytes(bytes)?`" but the example uses `Secp256k1::from_bytes(bytes)?`. Add a parallel `Eip712` line or clarify in a sentence.

3. **`api/methods/http-client/simulate.mdx:24` — `signer: &SingleSigner<impl dango_sdk::Secret>` uses `impl Trait` in function arg.** That's fine on modern Rust but the typestate generics `I, N` default to `Defined<…>` — explicit. No action needed; flag because TS/Python pages don't use this idiom.

4. **`api/methods/single-signer/sign_transaction.mdx:34` — `Coins::one("bridge/usdc", 100_u128)`.** Numeric literal `100_u128` is fine but the rest of the codebase uses `100_000_000_u128` (8-decimals USDC). Realistic value tip per style guide.

5. **`api/clients/Keystore.mdx:30` — struct field types in code block.** Uses concrete numeric literals (`ByteArray<33>` instead of `ByteArray<SECP256K1_COMPRESSED_PUBKEY_LEN>`). Matches the public source convention since the constants are private. Fine; flagged because the inventory note `Excluded items` lists those constants as private — so the doc is consistent with the inventory's deliberate exposure choice.

6. **`api/subscriptions/SubscriptionStream.mdx:5–9` — `Pin<Box<dyn Stream<…> + Send>>` alias.** Doc correctly omits `Sync` from the bound. Good.

7. **`api/clients/Session.mdx:95` — "The first nine bits of every subscription id…"** Source uses `AtomicU64` with `fetch_add(1, Ordering::Relaxed).to_string()` — IDs are decimal strings of a u64 counter, not bit-fields. Reword: "Subscription ids are decimal strings produced by an `AtomicU64` per session."

## Items needing user judgment

1. **`first-call.mdx` & `concepts/clients.mdx` — `query_balance` from `QueryClientExt`.** The Rust SDK doesn't re-export grug's `QueryClientExt`. The doc tells readers to `use grug::QueryClientExt`. This is correct, but the inventory's Verification TODO #8 asks "decide whether to document them in the `Client` page or link out to grug". The current pages take both approaches (list inline + link to grug). This is the correct call given the goal is a complete reference, but raise with the user whether to:
   - Re-export `QueryClientExt`, `BlockClient`, etc. from `dango_sdk` (clean ergonomics)
   - Keep the current "import from grug" model and update docs to explicitly say so

2. **`concepts/subscriptions.mdx:9` — "graphql-transport-ws".** The page calls this protocol "the `graphql-ws` rewrite that `async-graphql` speaks". Strictly correct but a bit confusing for readers who'll Google "graphql-ws" and land on the older protocol. Consider naming the spec version explicitly (`graphql-transport-ws` / `graphql-ws@5+`) or linking the spec.

3. **`concepts/rate-limits.mdx:36` — string-matching on `429`.** The example uses `e.to_string().contains("429")`. This is brittle: the SDK's `error_for_status` wraps `reqwest::Error` and appends the response body. Whether the literal `"429"` appears in the message depends on `reqwest::Error::Display`. Document the brittleness or recommend `err.downcast_ref::<reqwest::Error>().and_then(|e| e.status())` instead.

4. **DEX warning scope.** The style guide and task spec require the DEX warning on the **Transactions concept page** "and any method/example that demonstrates submitting a DEX `Message`". The only Rust page demonstrating a DEX message is `concepts/transactions.mdx` (snippet lines 38–48). `concepts/rate-limits.mdx` mentions `SubscribeTrades` but for read-only subscription, which is not a DEX action. So a single warning on `transactions.mdx` covers the requirement. Confirm with the user.

## Build status

- `pnpm --filter @left-curve/docs build`: **FAIL** overall.
- **Rust-attributable failures:** **none.**
- Sole failure mode is Python deadlinks: `./hl-compat/exchange/order in sdk/docs/pages/python/migration/hyperliquid.mdx` (×2). No `pages/rust/` files produced deadlinks or other errors. All intra-Rust links resolved.

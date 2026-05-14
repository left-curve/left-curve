# Python SDK Inventory

The Python SDK ships a single distribution package `dango`. Both `__init__.py` files (the root `dango/__init__.py` and `dango/hyperliquid_compatibility/__init__.py`) are empty — there is no curated re-export list. The public surface is "everything not prefixed with `_`" in the submodules below. Drafters should document by submodule path, not by re-export.

## Summary

- Native classes: 6 (`API`, `Exchange`, `Info`, `WebsocketManager`, `Secp256k1Wallet`, `SingleSigner`) + 1 `Wallet` Protocol
- Native methods (across classes, excluding `__init__` and private `_*`): 71
  - `API`: 2, `Exchange`: 13 (+2 properties), `Info`: 30, `WebsocketManager`: 4, `Secp256k1Wallet`: 5 (+5 properties) + 1 instance method, `SingleSigner`: 5 + 1 property-equivalent attribute, `Wallet` (Protocol): 1 method + 3 properties
- Module-level functions: 4 (`paginate_all`, `dango_decimal`, `sign_doc_canonical_json`, `sign_doc_sha256`)
- Types / Dataclasses / TypedDicts: 70+ (see breakdown below)
- Errors: 5 (`Error`, `ClientError`, `ServerError`, `GraphQLError`, `TxFailed`)
- HL-compat surface: 2 classes (`Exchange`, `Info`) / 95 methods (54 on Exchange — 12 implemented, 42 raise `NotImplementedError`; 41 on Info — 16 implemented, 25 raise `NotImplementedError`) + 1 `Cloid` class + 5 wire helper functions + 30+ HL-shaped TypedDicts

## Native surface

### Classes

#### `API`

Sync GraphQL POST client; base class for `Info` and `Exchange`. Source: `sdk/python/dango/api.py`

Constructor:
- `API(base_url: str, *, timeout: float | None = None)`

Methods:
- `query(document: str, variables: dict[str, Any] | None = None) -> dict[str, Any]` — POST a GraphQL document; return the `data` field. Raises `ServerError`/`ClientError`/`GraphQLError` per status class.
- `query_typed(document: str, variables: dict[str, Any] | None = None, *, response_type: type[T]) -> T` — same as `query` but `cast(T, ...)` the result. No runtime validation.

#### `Exchange`

Build, sign, and broadcast Dango perps transactions on behalf of one account. Subclass of `API`. Source: `sdk/python/dango/exchange.py`

Constructor:
- `Exchange(wallet: Wallet | LocalAccount, base_url: str, *, account_address: Addr, user_index: int | None = None, next_nonce: int | None = None, chain_id: str | None = None, timeout: float | None = None, info: Info | None = None, perps_contract: Addr | None = None)`

Class attribute:
- `DEFAULT_GAS_OVERHEAD: Final[int] = GAS_OVERHEAD_SECP256K1` — gas added on top of simulated `gas_used` to cover signature verification.

Properties:
- `address -> Addr` — the Dango account address this Exchange transacts as.
- `signer -> SingleSigner` — the underlying signer; exposed for manual nonce tweaks in tests.

Methods (margin):
- `deposit_margin(amount: int) -> dict[str, Any]` — deposit USDC into the perps margin sub-account; `amount` is base units (Uint128).
- `withdraw_margin(amount: float | str | Decimal) -> dict[str, Any]` — withdraw USDC; `amount` is USD (6-decimal `UsdValue`).

Methods (orders):
- `submit_order(pair_id: PairId, size: float | int | str | Decimal, kind: OrderKind, *, reduce_only: bool = False, tp: ChildOrder | None = None, sl: ChildOrder | None = None) -> dict[str, Any]` — place a single perps order; size is signed (positive = buy, negative = sell).
- `cancel_order(spec: OrderId | ClientOrderIdRef | Literal["all"]) -> dict[str, Any]` — cancel by chain OrderId, `ClientOrderIdRef`, or `"all"` for every open order.
- `batch_update_orders(actions: list[SubmitOrCancelAction]) -> dict[str, Any]` — submit and/or cancel multiple orders atomically.
- `submit_market_order(pair_id, size, *, max_slippage = 0.01, reduce_only = False, tp = None, sl = None) -> dict[str, Any]` — convenience: market order with slippage cap.
- `submit_limit_order(pair_id, size, limit_price, *, time_in_force: TimeInForce = TimeInForce.GTC, client_order_id: int | None = None, reduce_only = False, tp = None, sl = None) -> dict[str, Any]` — convenience: limit order.

Methods (conditional orders / TP/SL):
- `submit_conditional_order(pair_id: PairId, size: float | int | str | Decimal | None, trigger_price, trigger_direction: TriggerDirection, max_slippage) -> dict[str, Any]` — place a TP/SL order; reduce-only by construction. `size=None` closes the entire position at trigger.
- `cancel_conditional_order(spec: CancelConditionalSpec) -> dict[str, Any]` — cancel a conditional order by `ConditionalOrderRef`, `AllForPair`, or `"all"`.

Methods (vault):
- `add_liquidity(amount: float | int | str | Decimal, *, min_shares_to_mint: int | None = None) -> dict[str, Any]` — debit USD margin to mint LP shares.
- `remove_liquidity(shares_to_burn: int) -> dict[str, Any]` — burn LP shares (subject to cooldown).

Methods (referrals / liquidation):
- `set_referral(referrer: int | str) -> dict[str, Any]` — bind signer as referee of `referrer` (user_index or username).
- `liquidate(user: Addr) -> dict[str, Any]` — force-close an underwater user's positions (permissionless).

#### `Info`

Low-level GraphQL query primitives; read-side surface used by every consumer. Subclass of `API`. Source: `sdk/python/dango/info.py`

Constructor:
- `Info(base_url: str, *, skip_ws: bool = False, timeout: float | None = None, perps_contract: Addr | None = None)`

Methods (chain queries):
- `query_status() -> dict[str, Any]` — chain ID and latest block info.
- `query_app(request: dict[str, Any], *, height: int | None = None) -> Any` — generic `queryApp` wrapper; returns the raw kind-keyed envelope.
- `query_app_smart(contract: Addr, msg: dict[str, Any], *, height: int | None = None) -> Any` — convenience for `{wasm_smart: ...}` queries; unwraps the envelope.
- `query_app_multi(queries: list[dict[str, Any]], *, height: int | None = None) -> list[dict[str, Any]]` — atomically run multiple queries at one height (API §1.4).
- `simulate(tx: UnsignedTx) -> dict[str, Any]` — dry-run an UnsignedTx; returns `gas_used`/`gas_limit`/`result`.
- `broadcast_tx_sync(tx: Tx) -> dict[str, Any]` — submit a signed Tx (GraphQL mutation); returns the BroadcastTxOutcome envelope.

Methods (perps contract queries):
- `perps_param() -> Param` — global perps parameters.
- `perps_state() -> State` — global perps runtime state.
- `pair_param(pair_id: PairId) -> PairParam | None` — per-pair parameters; `None` if pair not configured.
- `pair_params(*, start_after: PairId | None = None, limit: int = 30) -> dict[PairId, PairParam]` — enumerate per-pair parameters; paginated.
- `pair_state(pair_id: PairId) -> PairState | None` — per-pair runtime state (OI, funding, current rate).
- `pair_states(*, start_after: PairId | None = None, limit: int = 30) -> dict[PairId, PairState]` — enumerate per-pair states.
- `liquidity_depth(pair_id: PairId, *, bucket_size: str, limit: int | None = None) -> LiquidityDepthResponse` — aggregated bid/ask depth.
- `user_state(user: Addr) -> UserState | None` — margin, positions, vault shares, unlocks.
- `user_state_extended(user: Addr, *, include_equity = True, include_available_margin = True, include_maintenance_margin = True, include_unrealized_pnl = True, include_unrealized_funding = True, include_liquidation_price = False) -> UserStateExtended | None` — `user_state` plus computed equity/margin/PnL fields per the include flags.
- `orders_by_user(user: Addr) -> dict[OrderId, dict[str, Any]]` — all resting limit orders for a user.
- `order(order_id: OrderId) -> dict[str, Any] | None` — single resting order by id.
- `volume(user: Addr, *, since: int | None = None) -> str` — user's cumulative trading volume in USD (6-decimal string).

Methods (indexer queries):
- `perps_candles(pair_id: PairId, interval: CandleInterval, *, later_than: str | None = None, earlier_than: str | None = None, first: int | None = None, after: str | None = None) -> Connection[PerpsCandle]` — OHLCV candles; cursor-paginated.
- `perps_events(*, user_addr=None, event_type=None, pair_id=None, block_height=None, first=None, after=None, sort_by: PerpsEventSortBy = PerpsEventSortBy.BLOCK_HEIGHT_DESC) -> Connection[PerpsEvent]` — indexer event stream; cursor-paginated.
- `perps_pair_stats(pair_id: PairId) -> PerpsPairStats` — 24h price/volume stats for one pair.
- `all_perps_pair_stats() -> list[PerpsPairStats]` — 24h stats for every active pair.
- `perps_events_all(*, user_addr=None, event_type=None, pair_id=None, block_height=None, sort_by=PerpsEventSortBy.BLOCK_HEIGHT_DESC, page_size: int = 100) -> Iterator[PerpsEvent]` — iterate every matching event, walking pages internally.

Methods (subscriptions; return `int` subscription id):
- `subscribe_perps_trades(pair_id: PairId, callback: Callable[[Trade], None]) -> int` — stream perps trade fills for one pair.
- `subscribe_perps_candles(pair_id: PairId, interval: CandleInterval, callback: Callable[[PerpsCandle], None]) -> int` — stream OHLCV candles.
- `subscribe_query_app(request: dict[str, Any], callback: Callable[[dict[str, Any]], None], *, block_interval: int = 10) -> int` — re-run a queryApp every N blocks.
- `subscribe_user_events(user: Addr, callback: Callable[[PerpsEvent], None], *, event_types: list[str] | None = None) -> int` — stream events for one user (optionally type-filtered).
- `subscribe_block(callback: Callable[[Block], None]) -> int` — stream every newly-finalized block.
- `unsubscribe(subscription_id: int) -> bool` — drop a subscription and tell the server to stop streaming.
- `disconnect_websocket() -> None` — close the WebSocket connection and clean up the manager thread.

#### `WebsocketManager`

Thread-based `graphql-transport-ws` subscription manager. Subclass of `threading.Thread` (`daemon=True`). Source: `sdk/python/dango/websocket_manager.py`

Note: in practice this is constructed lazily by `Info` on first `subscribe_*` call and not by users directly — but it is not `_`-prefixed, so it is part of the public surface.

Constructor:
- `WebsocketManager(base_url: str)`

Methods:
- `run() -> None` — `threading.Thread` entry point; blocks in `run_forever()` until `stop()`.
- `stop() -> None` — signal shutdown and close the WebSocket.
- `subscribe(document: str, variables: dict[str, Any], callback: Callable[[dict[str, Any]], None]) -> int` — register a subscription; returns an int id.
- `unsubscribe(subscription_id: int) -> bool` — drop a subscription; returns `False` if id was unknown.

#### `Secp256k1Wallet`

32-byte secp256k1 secret plus the Dango account address it controls. Concrete implementation of the `Wallet` Protocol. Source: `sdk/python/dango/utils/signing.py`

Constructor:
- `Secp256k1Wallet(secret: bytes, address: Addr)` — secret must be 32 bytes in `[1, n-1]`.

Class methods (factories):
- `random(cls, address: Addr) -> Secp256k1Wallet` — generate a new wallet from CSPRNG.
- `from_bytes(cls, secret: bytes, address: Addr) -> Secp256k1Wallet` — wrap a raw 32-byte secret.
- `from_mnemonic(cls, mnemonic: str, address: Addr, *, coin_type: int = 60) -> Secp256k1Wallet` — BIP-39 mnemonic + BIP-44 path `m/44'/{coin_type}'/0'/0/0`.
- `from_eth_account(cls, account: LocalAccount, address: Addr) -> Secp256k1Wallet` — re-use a LocalAccount's secret as a Dango secp256k1 key (key_tag=1, NOT EIP-712).

Properties:
- `address -> Addr` — the Dango account address supplied at construction.
- `secret_bytes -> bytes` — raw 32-byte secret. Sensitive.
- `public_key_compressed -> bytes` — 33-byte compressed pubkey.
- `key -> Key` — wire-shape `{"secp256k1": "<base64 of 33-byte compressed pubkey>"}`.
- `key_hash -> Hash256` — `SHA-256(compressed_pubkey)` as uppercase hex.

Methods:
- `sign(sign_doc: SignDoc) -> Signature` — secp256k1 signature over `SHA-256(canonical_json(sign_doc))`; returns 64-byte r||s in the `{"secp256k1": "<base64>"}` envelope.

#### `SingleSigner`

Stateful per-account signer; tracks `user_index` and `next_nonce`, produces signed `Tx` envelopes. Source: `sdk/python/dango/utils/signing.py`

Constructor:
- `SingleSigner(wallet: Wallet, address: Addr, *, user_index: int | None = None, next_nonce: int | None = None)`

Attributes (mutable):
- `wallet: Wallet`, `address: Addr`, `user_index: int | None`, `next_nonce: int | None`

Class method:
- `auto_resolve(cls, wallet: Wallet, address: Addr, info: _QueryClient) -> SingleSigner` — construct and populate `user_index`/`next_nonce` by querying the chain.

Methods:
- `query_user_index(info: _QueryClient) -> int` — look up the address's user_index via the account-factory contract.
- `query_next_nonce(info: _QueryClient) -> int` — compute `next_nonce` from the seen-nonces sliding window.
- `build_unsigned_tx(messages: list[Message], chain_id: str) -> UnsignedTx` — wrap messages + Metadata into an UnsignedTx for `Info.simulate()`.
- `sign_tx(messages: list[Message], chain_id: str, gas_limit: int) -> Tx` — sign and return a Tx; optimistically increments `next_nonce` on success or failure.

Note: `_QueryClient` is a private structural `Protocol` (leading underscore); its single method `query_app_smart(...)` is satisfied by `Info`.

#### `Wallet` (Protocol)

`@runtime_checkable` Protocol describing the abstract signing identity. Source: `sdk/python/dango/utils/signing.py`

Members:
- `address: Addr` (property)
- `key: Key` (property)
- `key_hash: Hash256` (property)
- `sign(sign_doc: SignDoc) -> Signature` (method)

Implementations in tree: `Secp256k1Wallet`. The docstring documents that future Passkey/Session wallets will also satisfy this.

### Module-level functions

- `paginate_all(fetch_page: Callable[[str | None, int], Connection[T]], *, page_size: int = 100) -> Iterator[T]` — yield every node across all forward-paginated pages. | source: `sdk/python/dango/info.py`
- `dango_decimal(x: float | int | str | Decimal, max_places: int = 6) -> str` — return canonical fixed-decimal string form of `x`; raises if precision is lost. The canonical wire encoding for all `UsdValue`/`UsdPrice`/`Dimensionless`/`Quantity` fields. | source: `sdk/python/dango/utils/types.py`
- `sign_doc_canonical_json(sign_doc: SignDoc) -> bytes` — encode a SignDoc as canonical JSON (recursive `sort_keys`, no whitespace, drops `None` from `data`). | source: `sdk/python/dango/utils/signing.py`
- `sign_doc_sha256(sign_doc: SignDoc) -> bytes` — SHA-256 digest of canonical JSON; the 32-byte payload that gets signed. | source: `sdk/python/dango/utils/signing.py`

### Types / Dataclasses / TypedDicts

All types live under `sdk/python/dango/utils/types.py` unless noted. Listed by category.

Identifier `NewType` aliases (all wrap `str`/`int`):
- `Addr`, `Hash256`, `Binary`, `PairId`, `OrderId`, `ConditionalOrderId`, `FillId`, `ClientOrderId`, `UserIndex`, `Nonce`, `Dimensionless`, `Quantity`, `UsdValue`, `UsdPrice`, `FundingPerUnit`, `FundingRate`, `Days`, `Timestamp`, `Duration`, `Uint64`, `Uint128`, `Referrer`, `Referee`, `FeeShareRatio`, `CommissionRate` — 25 aliases.

Enums (all `StrEnum`):
- `TimeInForce` — `GTC` / `IOC` / `POST`.
- `TriggerDirection` — `ABOVE` / `BELOW`.
- `CandleInterval` — `ONE_SECOND` ... `ONE_WEEK`. Indexer GraphQL enum form.
- `ReasonForOrderRemoval` — `FILLED` / `CANCELED` / `POSITION_CLOSED` / `SELF_TRADE_PREVENTION` / `LIQUIDATED` / `DELEVERAGED` / `SLIPPAGE_EXCEEDED` / `PRICE_BAND_VIOLATION` / `SLIPPAGE_CAP_TIGHTENED`.
- `KeyType` — `SECP256R1` / `SECP256K1` / `ETHEREUM`. Uppercase wire form (indexer GraphQL).
- `AccountStatus` — `INACTIVE` / `ACTIVE` / `FROZEN`.
- `PerpsEventSortBy` — `BLOCK_HEIGHT_ASC` / `BLOCK_HEIGHT_DESC`.

Auth/credentials TypedDicts:
- `Key` (union of `{secp256r1}` / `{secp256k1}` / `{ethereum}` variants)
- `ClientData`, `PasskeySignature`, `Eip712Signature`
- `Signature` (union of `{passkey}` / `{secp256k1}` / `{eip712}` variants)
- `StandardCredential`, `SessionInfo`, `SessionCredential`
- `Credential` (union of `{standard}` / `{session}` variants)
- `Metadata` (per-tx metadata)

Tx primitives:
- `Message` (= `dict[str, object]`, type alias not class)
- `SignDoc`, `UnsignedTx`, `Tx` (TypedDicts)

Order primitives — wire-shape TypedDicts:
- `MarketKind`, `LimitKind`
- `OrderKind` (= `MarketKind | LimitKind`)
- `ChildOrder`
- `SubmitOrderRequest`
- `CancelOrderRequest` (union: `{one}` / `{one_by_client_order_id}` / `Literal["all"]`)
- `SubmitOrCancelOrderRequest` (union: `{submit}` / `{cancel}`)
- `CancelConditionalOrderRequest` (union: `{one: {pair_id, trigger_direction}}` / `{all_for_pair: {pair_id}}` / `Literal["all"]`)
- `ConditionalOrder`

Order primitives — user-facing dataclasses (frozen):
- `ClientOrderIdRef(value: int)` — disambiguates from `OrderId` at runtime.
- `SubmitAction(pair_id, size, kind, reduce_only=False, tp=None, sl=None)` — user-facing form of `SubmitOrderRequest`.
- `CancelAction(spec: OrderId | ClientOrderIdRef | Literal["all"])` — user-facing form of `CancelOrderRequest`.
- `SubmitOrCancelAction` (= `SubmitAction | CancelAction`)
- `ConditionalOrderRef(pair_id, trigger_direction)`
- `AllForPair(pair_id)`
- `CancelConditionalSpec` (= `ConditionalOrderRef | AllForPair | Literal["all"]`)

Position / user state TypedDicts:
- `Position`, `PositionExtended`, `Unlock`, `UserState`, `UserStateExtended`

Pair / market data TypedDicts:
- `RateSchedule`, `ReferrerSettings`, `RefereeStats`
- `Param` (global perps parameters)
- `State` (global perps runtime state)
- `PairParam`, `PairState`
- `LiquidityDepth`, `LiquidityDepthResponse`
- `QueryOrderResponse`, `QueryOrdersByUserResponseItem`
- `UserReferralData`

Event TypedDicts (the on-chain perps event union):
- `Deposited`, `Withdrew`, `LiquidityAdded`, `LiquidityUnlocking`, `LiquidityReleased`
- `OrderFilled`, `OrderPersisted`, `OrderRemoved`
- `ConditionalOrderPlaced`, `ConditionalOrderTriggered`, `ConditionalOrderRemoved`
- `Liquidated`, `Deleveraged`, `BadDebtCovered`, `FeeDistributed`, `ReferralSet`

Indexer wire-shape TypedDicts (camelCase keys preserved):
- `PerpsCandle`, `PerpsEvent`, `PerpsPairStats`, `Trade`
- `BlockTransaction`, `BlockEvent`, `Block`

Pagination dataclasses (frozen, snake_case attributes):
- `PageInfo(has_previous_page, has_next_page, start_cursor, end_cursor)`
- `Connection[T](nodes, page_info)` — generic via PEP 695 syntax.

### Errors

All inherit from `Error`. Source: `sdk/python/dango/utils/error.py`

- `Error` — base class for all SDK-raised exceptions. | source: `sdk/python/dango/utils/error.py`
- `ClientError` — raised on a 4xx HTTP response from the GraphQL endpoint. | source: `sdk/python/dango/utils/error.py`
- `ServerError` — raised on 5xx, network failures (DNS, connection refused, timeout, SSL), or non-JSON bodies. | source: `sdk/python/dango/utils/error.py`
- `GraphQLError` — raised when a GraphQL response carries a non-empty `errors` array, or is missing both `data` and `errors`. | source: `sdk/python/dango/utils/error.py`
- `TxFailed` — raised when `broadcastTxSync` returns an `err` result. | source: `sdk/python/dango/utils/error.py`

### Constants

Re-importable from `dango.utils.constants` (not on the empty `dango.__init__`):

- URLs: `MAINNET_API_URL`, `TESTNET_API_URL`, `LOCAL_API_URL`
- Chain IDs: `CHAIN_ID_MAINNET` (`"dango-1"`), `CHAIN_ID_TESTNET` (`"dango-testnet-1"`)
- Contract addresses: `ACCOUNT_FACTORY_CONTRACT`, `ORACLE_CONTRACT`, `PERPS_CONTRACT_MAINNET`, `PERPS_CONTRACT_TESTNET`
- Tx primitives: `SETTLEMENT_DENOM` (`"bridge/usdc"`), `SETTLEMENT_DECIMALS` (`6`), `GAS_OVERHEAD_SECP256K1` (`770_000`)

## Hyperliquid compatibility surface (`dango.hyperliquid_compatibility`)

The HL-compat package mirrors upstream `hyperliquid` so traders can swap `from hyperliquid.X import Y` → `from dango.hyperliquid_compatibility.X import Y` with no other source changes. Both `hyperliquid_compatibility/__init__.py` and the top-level `dango/__init__.py` are empty — consumers import from submodules directly.

Submodules:
- `dango.hyperliquid_compatibility.exchange` — write-side facade.
- `dango.hyperliquid_compatibility.info` — read-side facade + subscription dispatcher.
- `dango.hyperliquid_compatibility.types` — HL-shaped TypedDicts, `Cloid`, wire helpers.
- `dango.hyperliquid_compatibility.constants` — re-export of Dango URL constants under the HL path.

### Classes

#### `Exchange` (HL-compat)

HL-shaped facade over native `dango.exchange.Exchange`. Source: `sdk/python/dango/hyperliquid_compatibility/exchange.py`

Constructor mirrors HL's signature byte-for-byte, with two strict gates:
- `Exchange(wallet, base_url=None, meta=None, vault_address=None, account_address=None, spot_meta=None, perp_dexs=None, timeout=None, perps_contract=None)`
- Raises `NotImplementedError` if `vault_address is not None` or `spot_meta is not None`.
- Raises `ValueError` if `account_address is None` (HL allowed defaulting to wallet address; Dango requires explicit decoupled account address).
- `base_url=None` defaults to `LOCAL_API_URL`, NOT HL's mainnet URL.

Class attribute:
- `DEFAULT_SLIPPAGE: Final[float] = 0.05` — matches HL.

Instance attributes:
- `base_url`, `wallet`, `account_address`, `vault_address` (always `None`), `expires_after`, `info` (HL-shaped Info), `_native` (native Exchange).

Implemented methods (12):
- `order(name, is_buy, sz, limit_px, order_type: OrderType, reduce_only=False, cloid=None, builder=None) -> dict` — single HL-style order → native `submit_order`. Raises `NotImplementedError` if `builder is not None`.
- `bulk_orders(order_requests: Iterable[OrderRequest], *, builder=None, grouping: Grouping = "na") -> dict` — batched HL-style orders → native `batch_update_orders` (or `submit_order` if only one). Raises `NotImplementedError` on `builder is not None` or non-`"na"` grouping.
- `cancel(name: str, oid: int) -> dict` — cancel by chain oid; verifies `name` for parity.
- `bulk_cancel(cancel_requests: Iterable[dict[str, Any]]) -> dict` — batched cancels by oid.
- `cancel_by_cloid(name: str, cloid: Cloid) -> dict` — cancel by cloid; hashes 16-byte HL cloid to Uint64.
- `bulk_cancel_by_cloid(cancel_requests: Iterable[dict[str, Any]]) -> dict` — batched cancels by cloid.
- `modify_order(oid: int | Cloid, name, is_buy, sz, limit_px, order_type, reduce_only=False, cloid=None) -> dict` — atomic cancel+replace via batch.
- `bulk_modify_orders_new(modify_requests: Iterable[dict[str, Any]]) -> dict` — batched cancel+submit pairs.
- `market_open(name, is_buy, sz, *, px=None, slippage=DEFAULT_SLIPPAGE, cloid=None, builder=None) -> dict` — market order with slippage cap. `px` is accepted-and-ignored. Raises `NotImplementedError` on `cloid is not None` or `builder is not None`.
- `market_close(coin, *, sz=None, px=None, slippage=DEFAULT_SLIPPAGE, cloid=None, builder=None) -> dict` — reduce-only close in `coin`; reads position via `info.user_state` to determine direction.
- `set_referrer(code: str) -> dict` — bind signer as referee; forwards str to native `set_referral`.
- `set_expires_after(expires_after: int | None) -> None` — stores the HL `expiresAfter` ms hint. NOTE: currently a no-op with state storage; not threaded through to native sign path (Phase 17 known gap).

Methods that raise `NotImplementedError` (42 stubs):
- Margin/leverage: `update_leverage`, `update_isolated_margin`
- Scheduling: `schedule_cancel`
- Transfers: `usd_class_transfer`, `send_asset`, `vault_usd_transfer`, `sub_account_transfer`, `sub_account_spot_transfer`, `usd_transfer`, `spot_transfer`, `withdraw_from_bridge`
- Builder fee: `approve_builder_fee`
- Multi-sig: `convert_to_multi_sig_user`, `multi_sig`
- Sub-accounts: `create_sub_account`
- Agents/abstraction: `approve_agent`, `agent_enable_dex_abstraction`, `agent_set_abstraction`, `user_dex_abstraction`, `user_set_abstraction`
- HYPE-specific: `token_delegate`, `use_big_blocks`, `c_signer_unjail_self`, `c_signer_jail_self`, `c_validator_register`, `c_validator_change_profile`, `c_validator_unregister`, `noop`, `gossip_priority_bid`
- Spot deploys (10 methods): `spot_deploy_register_token`, `spot_deploy_user_genesis`, `spot_deploy_enable_freeze_privilege`, `spot_deploy_freeze_user`, `spot_deploy_revoke_freeze_privilege`, `spot_deploy_enable_quote_token`, `spot_deploy_token_action_inner`, `spot_deploy_genesis`, `spot_deploy_register_spot`, `spot_deploy_register_hyperliquidity`, `spot_deploy_set_deployer_trading_fee_share`
- Perp deploys: `perp_deploy_register_asset`, `perp_deploy_set_oracle`

#### `Info` (HL-compat)

HL-shaped facade over native `dango.info.Info`. Source: `sdk/python/dango/hyperliquid_compatibility/info.py`

Constructor:
- `Info(base_url=None, skip_ws=False, meta: Meta | None = None, perp_dexs=None, timeout=None, perps_contract=None)`
- `base_url=None` defaults to `LOCAL_API_URL`.
- Builds a coin↔pair_id resolver from `meta` (offline) or from a live `pair_params()` fetch (online).

Instance attributes:
- `coin_to_asset: dict[str, int]`, `name_to_coin: dict[str, str]`, `asset_to_sz_decimals: dict[int, int]`, `coin_to_pair: dict[str, PairId]`
- `_native` (native Info), `_perp_dexs`

Resolver methods:
- `name_to_pair(name: str) -> PairId` — coin name → Dango pair_id.
- `name_to_asset(name: str) -> int` — coin name → integer asset index.

Implemented read methods (13):
- `user_state(address: str, dex: str = "") -> dict` — HL `clearinghouseState`; reshapes `UserStateExtended`. `dex` accepted-and-ignored.
- `open_orders(address: str, dex: str = "") -> list[dict]` — flattens Dango's `orders_by_user` map to an HL list.
- `all_mids(dex: str = "") -> dict[str, str]` — coin → mid price.
- `meta(dex: str = "") -> Meta` — HL perp universe metadata.
- `meta_and_asset_ctxs() -> list` — bundles `meta` + per-asset `PerpAssetCtx`.
- `l2_snapshot(name: str) -> L2BookData` — bid/ask depth at finest bucket.
- `candles_snapshot(name: str, interval: str, start: int, end: int) -> list[dict]` — OHLCV in a time window. `interval` is HL form (`"1m"`/`"5m"`/...).
- `user_fills(address: str) -> list[Fill]` — flat list of trade fills.
- `user_fills_by_time(addr: str, start: int, end: int | None = None) -> list[Fill]` — fills in a time range.
- `query_order_by_oid(user: str, oid: int | str) -> dict` — `user` is accepted-and-ignored; Dango orders are keyed by oid alone.
- `historical_orders(user: str) -> list[dict]` — zips persisted + removed events to reconstruct order lifecycle.

Implemented subscription methods (3):
- `subscribe(subscription: Subscription, callback: Callable[[Any], None]) -> int` — dispatches by `subscription["type"]` to one of 10 private `_subscribe_*` handlers. Raises `NotImplementedError` for `"userFundings"`, `"webData2"`, `"userNonFundingLedgerUpdates"`. Raises `ValueError` on unknown type.
- `unsubscribe(subscription: Subscription, subscription_id: int) -> bool` — forwards to native.
- `disconnect_websocket() -> None` — closes the underlying WebSocket.

Private subscription dispatch helpers (one per HL channel): `_subscribe_trades`, `_subscribe_candle`, `_subscribe_user_events`, `_subscribe_user_fills`, `_subscribe_order_updates`, `_subscribe_l2_book`, `_subscribe_bbo`, `_subscribe_all_mids` (itself raises `NotImplementedError`), `_subscribe_active_asset_ctx`, `_subscribe_active_asset_data`. L2/bbo/activeAsset* poll via native `subscribe_query_app(block_interval=1)`.

Methods that raise `NotImplementedError` (25 stubs):
- Spot: `spot_user_state`, `spot_meta`, `spot_meta_and_asset_ctxs`, `query_spot_deploy_auction_status`
- Staking: `user_staking_summary`, `user_staking_delegations`, `user_staking_rewards`, `delegator_history`
- Multi-sig: `query_user_to_multi_sig_signers`
- Permissionless listing: `query_perp_deploy_auction_status`
- Abstraction: `query_user_dex_abstraction_state`, `query_user_abstraction_state`
- TWAP: `user_twap_slice_fills`
- Time-series: `portfolio`, `user_role`, `user_rate_limit`, `extra_agents`
- Funding history: `funding_history`, `user_funding_history` (Dango folds funding into realized PnL)
- Phase-16 deferred: `user_non_funding_ledger_updates`, `query_referral_state`, `query_sub_accounts`, `frontend_open_orders`, `user_fees`, `user_vault_equities`
- Cloid lookup: `query_order_by_cloid` (Dango doesn't store cloid → oid mapping at rest)

### Module-level functions (HL-compat)

- `dango_decimal_to_hl_str(x: str) -> str` — strip trailing zeros from a Dango canonical decimal string for HL wire shape. `"1.230000"` → `"1.23"`, `"0.000000"` → `"0"`, `"-1.500000"` → `"-1.5"`. | source: `sdk/python/dango/hyperliquid_compatibility/types.py` (also re-exported from `exchange.py` via `__all__`).
- `hl_resting_entry(oid: int) -> HlRestingEntry` — `{"resting": {"oid": oid}}` per-order status. | source: `types.py`
- `hl_filled_entry(*, total_sz: str, avg_px: str, oid: int) -> HlFilledEntry` — `{"filled": {...}}` per-order status. | source: `types.py`
- `hl_error_entry(message: str) -> HlErrorEntry` — `{"error": message}` per-order status. | source: `types.py`
- `hl_status_envelope(*, response_type: str, statuses=None, error=None) -> dict[str, Any]` — wrap a Dango outcome into HL's `{status, response}` envelope. | source: `types.py`

### Types (HL-compat)

All in `sdk/python/dango/hyperliquid_compatibility/types.py` (camelCase preserved per HL wire shape).

The `Cloid` class:
- `Cloid(raw_cloid: str)` — validates `"0x"`-prefixed 32-hex-char string.
- `from_int(cloid: int) -> Cloid`, `from_str(cloid: str) -> Cloid` (staticmethods)
- `to_raw() -> str`, `to_uint64() -> int` (SHA-256 hash to Uint64, lossy 128→64 bit collapse)
- `__str__`, `__repr__`

HL-shaped TypedDicts (organized by upstream HL module of origin):

Domain (`hyperliquid/utils/types.py`):
- `AssetInfo`, `Meta`
- `SpotAssetInfo`, `SpotTokenInfo`, `SpotMeta`, `SpotAssetCtx`
- `Side` (= `Literal["A", "B"]`), `SpotMetaAndAssetCtxs` (= `tuple[SpotMeta, list[SpotAssetCtx]]`)

Subscriptions (one TypedDict per HL channel; all carry `type: Literal[...]`):
- `AllMidsSubscription`, `BboSubscription`, `L2BookSubscription`, `TradesSubscription`, `UserEventsSubscription`, `UserFillsSubscription`, `CandleSubscription`, `OrderUpdatesSubscription`, `UserFundingsSubscription`, `UserNonFundingLedgerUpdatesSubscription`, `WebData2Subscription`, `ActiveAssetCtxSubscription`, `ActiveAssetDataSubscription`
- `Subscription` (union of all above)

WebSocket data + envelope TypedDicts:
- `AllMidsData`, `AllMidsMsg`
- `L2Level`, `L2BookData`, `L2BookMsg`
- `BboData`, `BboMsg`
- `PongMsg`
- `Trade`, `TradesMsg`
- `CrossLeverage`, `IsolatedLeverage`, `Leverage` (union)
- `PerpAssetCtx`, `ActiveAssetCtx`, `ActiveSpotAssetCtx`, `ActiveAssetCtxMsg`, `ActiveSpotAssetCtxMsg`
- `ActiveAssetData`, `ActiveAssetDataMsg`
- `Fill`
- `UserEventsData`, `UserEventsMsg`
- `UserFillsData`, `UserFillsMsg`
- `OtherWsMsg` (catch-all for `candle`/`orderUpdates`/`userFundings`/`userNonFundingLedgerUpdates`/`webData2`)
- `WsMsg` (union)

Builder/abstraction (mostly unused — included for type-import parity):
- `BuilderInfo`
- `Abstraction` (= `Literal["unifiedAccount", "portfolioMargin", "disabled"]`)
- `AgentAbstraction` (= `Literal["u", "p", "i"]`)
- `PerpDexSchemaInput`

Order types (`hyperliquid/utils/signing.py`):
- `Tif` (= `Literal["Alo", "Ioc", "Gtc"]`), `Tpsl` (= `Literal["tp", "sl"]`)
- `LimitOrderType`, `TriggerOrderType`, `TriggerOrderTypeWire`
- `OrderType`, `OrderTypeWire` (`total=False`; carry one of `limit`/`trigger`)
- `OrderRequest` (snake_case Python-API form), `OrderWire` (single-letter wire form)
- `OidOrCloid` (= `int | Cloid`)
- `ModifyRequest`, `ModifyWire`
- `CancelRequest`, `CancelByCloidRequest`
- `PriorityGrouping`, `Grouping` (= `Literal["na", "normalTpsl", "positionTpsl"] | PriorityGrouping`)
- `Order` (HL python-side struct), `ScheduleCancelAction`

Status entry type aliases (used by `hl_*_entry` factories):
- `HlRestingEntry`, `HlFilledEntry`, `HlErrorEntry`, `HlStatusEntry` (union)

### Constants (HL-compat)

`dango.hyperliquid_compatibility.constants` re-exports:
- `MAINNET_API_URL`, `TESTNET_API_URL`, `LOCAL_API_URL` (Dango URLs, byte-compatible attribute names with upstream HL `hyperliquid.utils.constants`).

### Methods that diverge from HL upstream

These are the substantive deviations a migrator needs to know:
- `Exchange.__init__`: requires `account_address` (HL would default to wallet's EVM address). Rejects `vault_address` and `spot_meta` non-None values. Defaults `base_url` to `LOCAL_API_URL`, not HL mainnet.
- `Exchange.market_open` / `Exchange.market_close`: `px` argument is silently ignored — Dango computes its own slippage band against mark price. `cloid` raises `NotImplementedError` because native `submit_market_order` has no `client_order_id` parameter.
- `Exchange.set_expires_after`: stores the value but does not currently thread it into signed metadata (known gap).
- `Exchange.order` / `Exchange.market_open` / `Exchange.market_close`: `builder` parameter raises `NotImplementedError` — Dango has no builder-fee marketplace.
- `Exchange.bulk_orders`: `grouping != "na"` (i.e. `normalTpsl` / `positionTpsl`) raises `NotImplementedError`. HL TP/SL order grouping requires reshape; not yet implemented.
- `_hl_order_type_to_dango_kind`: HL `{"trigger": ...}` order type raises `NotImplementedError` — use native `Exchange.submit_conditional_order` instead.
- `Cloid.to_uint64`: HL cloid is 16 bytes (128 bits); Dango ClientOrderId is Uint64 (64 bits). Cloid is hashed lossily; **the cloid returned in responses is NOT the cloid you sent** — callers needing round-trip identity must maintain their own mapping.
- `Info.__init__`: `perp_dexs` is accepted and recorded but unused (Dango has no permissionless DEX listing).
- `Info.candles_snapshot`: HL accepts more intervals than Dango supports. Unsupported intervals raise `ValueError`. Supported: `1m`, `5m`, `15m`, `1h`, `4h`, `1d`, `1w`.
- `Info.user_state`: `assetPositions` entries report `marginUsed="0"` and `returnOnEquity="0"` — Dango doesn't track per-position margin/RoE (cross-only). `leverage.value=1` always; `maxLeverage=1` always. `cumFunding` series are always `"0"` (funding is folded into realized PnL on each fill, not tracked as a series).
- `Info.meta`: `szDecimals` is hardcoded to `SETTLEMENT_DECIMALS = 6` for every asset (Dango uses a uniform decimals; HL is per-asset).
- `Info.meta_and_asset_ctxs`: `PerpAssetCtx.markPx`, `oraclePx`, `midPx` all report the same value (the indexer's `currentPrice`). HL would distinguish.
- `Info.l2_snapshot`: HL doesn't take a `bucket_size`; we pick the smallest from `pair_param.bucket_sizes`. The L2 levels carry `n=1` always (Dango doesn't expose per-bucket order count). `time=0` always (no server timestamp on depth queries).
- `Info.user_fills` / `user_fills_by_time`: Dango emits both maker and taker rows per fill; the wrapper dedupes by `fill_id` and prefers the taker side.
- `Info.query_order_by_oid`: `user` argument is accepted-and-ignored (Dango orders are keyed by oid alone).
- `Info.subscribe(type="allMids")`: not yet implemented; raises `NotImplementedError`.
- `Info.subscribe(type="activeAssetCtx")` and `activeAssetData`: `markPx`/`oraclePx` report `"0"` until parallel `pair_stats` polling is wired in.
- Several methods are explicitly marked as Phase-16-deferred (need event-shape reshape or NAV computation), not "no Dango analog": `user_non_funding_ledger_updates`, `query_referral_state`, `query_sub_accounts`, `frontend_open_orders`, `user_fees`, `user_vault_equities`.

### Methods that wrap native equivalents

Direct mappings the migration documentation should highlight:

| HL-compat | Native | Notes |
|-----------|--------|-------|
| `Exchange.order` (single-order branch of `bulk_orders`) | `Exchange.submit_order` | HL `is_buy + sz` → Dango signed `size`; `OrderType.limit.tif` ("Gtc"/"Ioc"/"Alo") → `TimeInForce` ("GTC"/"IOC"/"POST") |
| `Exchange.bulk_orders` (multi-order) | `Exchange.batch_update_orders` | Wraps each request as a `SubmitAction` |
| `Exchange.cancel` | `Exchange.cancel_order(OrderId(str(oid)))` | |
| `Exchange.bulk_cancel` | `Exchange.batch_update_orders` of `CancelAction(OrderId(...))` | |
| `Exchange.cancel_by_cloid` | `Exchange.cancel_order(ClientOrderIdRef(cloid.to_uint64()))` | Lossy hash |
| `Exchange.bulk_cancel_by_cloid` | `Exchange.batch_update_orders` of `CancelAction(ClientOrderIdRef(...))` | |
| `Exchange.modify_order` / `bulk_modify_orders_new` | `Exchange.batch_update_orders` of paired Cancel+Submit | No first-class modify on chain |
| `Exchange.market_open` | `Exchange.submit_market_order` | |
| `Exchange.market_close` | `Exchange.submit_market_order(reduce_only=True)` | Reads position via `Info.user_state` first |
| `Exchange.set_referrer` | `Exchange.set_referral` | Forwards str (username) |
| `Info.user_state` | `Info.user_state_extended` + reshape | |
| `Info.open_orders` | `Info.orders_by_user` + reshape | |
| `Info.all_mids` | `Info.all_perps_pair_stats` | Uses `currentPrice` as mid |
| `Info.meta` | `Info.pair_params` | Synthesizes `szDecimals=6` |
| `Info.meta_and_asset_ctxs` | `Info.pair_params` + `Info.pair_states` + `Info.all_perps_pair_stats` | |
| `Info.l2_snapshot` | `Info.pair_param` + `Info.liquidity_depth` | Picks smallest bucket size |
| `Info.candles_snapshot` | `Info.perps_candles` | HL interval string → `CandleInterval` enum |
| `Info.user_fills` / `_by_time` | `Info.perps_events_all(event_type="order_filled")` + dedupe | |
| `Info.query_order_by_oid` | `Info.order` | `user` argument unused |
| `Info.historical_orders` | `Info.perps_events_all(event_type="order_persisted")` + `..."order_removed"` | Joins by `order_id` |
| `Info.subscribe(type="trades")` | `Info.subscribe_perps_trades` | Drops maker side; dedupes by `fill_id` |
| `Info.subscribe(type="candle")` | `Info.subscribe_perps_candles` | |
| `Info.subscribe(type="userEvents" / "userFills")` | `Info.subscribe_user_events(event_types=["order_filled"])` | Different envelope per branch |
| `Info.subscribe(type="orderUpdates")` | `Info.subscribe_user_events(event_types=["order_persisted", "order_removed"])` | |
| `Info.subscribe(type="l2Book" / "bbo" / "activeAssetCtx" / "activeAssetData")` | `Info.subscribe_query_app(block_interval=1)` | Polling-backed subscriptions |
| `Info.unsubscribe` | `Info.unsubscribe` | Ignores the `subscription` arg |
| `Info.disconnect_websocket` | `Info.disconnect_websocket` | |

## Excluded items (do not document)

- Anything in `dango._graphql/` — `_`-prefixed package containing vendored `.graphql` documents and an empty `__init__.py`. Not a public surface.
- Private structural protocols: `_QueryClient` (in `signing.py`) — leading `_`.
- Private wire-shape TypedDicts whose names start with `_`: `_KeySecp256r1`, `_KeySecp256k1`, `_KeyEthereum`, `_SignaturePasskey`, `_SignatureSecp256k1`, `_SignatureEip712`, `_CredentialStandard`, `_CredentialSession`, `_MarketPayload`, `_LimitPayload`, `_CancelOne`, `_CancelOneByClientOrderId`, `_SubmitOrCancelSubmit`, `_SubmitOrCancelCancel`, `_CancelConditionalOnePayload`, `_CancelConditionalOne`, `_CancelConditionalAllForPairPayload`, `_CancelConditionalAllForPair`. These are leading-underscore building blocks of the documented unions (`Key`, `Signature`, `Credential`, `OrderKind`, `CancelOrderRequest`, `SubmitOrCancelOrderRequest`, `CancelConditionalOrderRequest`).
- Module-private helpers: `_unwrap_node`, `_make_page_info`, `_make_connection`, `_format_graphql_errors`, `_make_ws_url`, `_parse_id`, and all HL-compat `_hl_*` / `_reshape_*` / `_to_hl_str` / `_pair_id_to_coin` / `_coin_to_pair_id` / `_ms_to_iso_str` / `_isotime_to_ms` / `_timestamp_ns_to_ms` / `_dedupe_fills` / `_extract_error_message` / `_signed_size` / `_native_outcome_to_*_envelope` / `_build_submit_action` / `_hl_tif_to_dango` / `_hl_interval_to_dango` / `_hl_order_type_to_dango_kind` helpers.
- Private members on `SingleSigner`: `_require_user_index`, `_require_next_nonce`.
- Module-internal `_QUERIES`, `_MUTATIONS`, `_SUBSCRIPTIONS`, `_QUERY_*`, `_MUTATION_*`, `_SUB_*` constants — loaded once at import time.
- The empty top-level `dango/__init__.py` and `dango/hyperliquid_compatibility/__init__.py` — no curated re-exports.
- The `examples/` directory under `sdk/python/` — illustrative, not part of the public SDK surface.

## Verification TODOs (drafters must confirm before writing)

1. **`__init__.py` plan**: both root and HL-compat `__init__.py` are currently empty (zero lines). Decide whether the docs should advertise importing from submodule paths (e.g. `from dango.exchange import Exchange`) or whether a re-export layer is planned before the docs ship. If the latter, the inventory's "import path" notes will need updating.
2. **`WebsocketManager` public status**: it has no underscore prefix and is constructed by `Info` lazily, but most callers will never touch it. Confirm whether it deserves a top-level Client page or only a passing mention in the Subscriptions concept page.
3. **`Wallet` Protocol vs `Secp256k1Wallet` concrete class**: should the Clients section document the Protocol (extensibility) or the concrete class (only currently shipping implementation)? Recommend documenting both.
4. **`SingleSigner` public status**: exposed via `Exchange.signer` property. Docs should clarify whether direct construction is supported or whether it's strictly an internal helper exposed for test hooks.
5. **Cloid asymmetry**: every HL-compat order/cancel page that mentions `cloid` MUST surface the SHA-256 lossy hash warning. Drafters should treat this as a load-bearing migration note, not a footnote.
6. **`Exchange.set_expires_after` no-op gap**: known Phase-17 gap (stored but not threaded). Doc page must call this out — silent acceptance is the actual behavior.
7. **`Info.subscribe(type="allMids")` not implemented**: the dispatcher raises despite being listed in `Subscription` union. Confirm whether to omit it from docs entirely or document it as a known gap.
8. **`Info.subscribe(type="activeAssetCtx"/"activeAssetData")`**: `markPx`/`oraclePx` zero-value gap. Documentation should clearly mark these fields as currently unpopulated.
9. **HL-compat method count vs HL upstream**: 42 stubs on Exchange + 25 on Info. Confirm with HL SDK 2024 release that the stub list is up to date — HL ships new methods periodically.
10. **`dango.utils.constants` exports**: not re-exported from `dango.__init__`. Decide whether the docs should reference `from dango.utils.constants import ...` (current reality) or wait for a re-export layer.
11. **`dango_decimal_to_hl_str` is re-exported from `dango.hyperliquid_compatibility.exchange.__all__`** but its definition lives in `types.py`. Drafters should choose one canonical import path.
12. **`Trade.sz` upstream type mismatch**: the HL upstream `Trade` TypedDict annotates `sz: int` but the wire shape is a decimal string. The `_subscribe_trades` wrapper emits the string anyway (cast through `int` for the type checker). Document the upstream typo so traders don't try `int(...)` casts.

"""Low-level GraphQL query primitives; subclass of API used by every read path."""

from __future__ import annotations

from collections.abc import Callable, Iterator
from importlib.resources import files
from typing import Any, Final, cast

from dango.api import API
from dango.sse_manager import SseManager
from dango.utils.constants import PERPS_CONTRACT_MAINNET
from dango.utils.types import (
    Addr,
    Block,
    CandleInterval,
    Connection,
    LiquidityDepthResponse,
    OrderId,
    PageInfo,
    PairId,
    PairParam,
    PairState,
    Param,
    PerpsCandle,
    PerpsEvent,
    PerpsEvent2Batch,
    PerpsEventSortBy,
    PerpsPairStats,
    State,
    Trade,
    Tx,
    UnsignedTx,
    UserState,
    UserStateExtended,
)
from dango.websocket_manager import WebsocketManager

# Load the vendored .graphql documents at import time. importlib.resources is
# the standard way to read package data files; it works under wheels, zip
# imports, and editable installs alike, where naive `open(__file__/..)` would
# break. Reading once here also avoids repeating the disk I/O on every query.
_QUERIES = files("dango._graphql.queries")
_MUTATIONS = files("dango._graphql.mutations")

_QUERY_STATUS: Final[str] = _QUERIES.joinpath("queryStatus.graphql").read_text(encoding="utf-8")
_QUERY_APP: Final[str] = _QUERIES.joinpath("queryApp.graphql").read_text(encoding="utf-8")
_QUERY_SIMULATE: Final[str] = _QUERIES.joinpath("simulate.graphql").read_text(encoding="utf-8")
_MUTATION_BROADCAST_TX_SYNC: Final[str] = _MUTATIONS.joinpath("broadcastTxSync.graphql").read_text(
    encoding="utf-8"
)

# Indexer-side documents (Phase 8). These run against the indexer DB rather
# than the chain's `query_app` endpoint, but the transport is the same
# /graphql POST so they sit alongside the chain queries above.
_QUERY_PERPS_CANDLES: Final[str] = _QUERIES.joinpath("perpsCandles.graphql").read_text(
    encoding="utf-8"
)
_QUERY_PERPS_EVENTS: Final[str] = _QUERIES.joinpath("perpsEvents.graphql").read_text(
    encoding="utf-8"
)
_QUERY_PERPS_PAIR_STATS: Final[str] = _QUERIES.joinpath("perpsPairStats.graphql").read_text(
    encoding="utf-8"
)
_QUERY_ALL_PERPS_PAIR_STATS: Final[str] = _QUERIES.joinpath("allPerpsPairStats.graphql").read_text(
    encoding="utf-8"
)

# Subscription documents (Phase 10). These run over the graphql-transport-ws
# protocol via `WebsocketManager`, not over HTTP. Loaded once at import time
# for the same reason as the queries above — avoid the repeated disk read
# every time `subscribe_*` is called.
_SUBSCRIPTIONS = files("dango._graphql.subscriptions")
_SUB_PERPS_CANDLES: Final[str] = _SUBSCRIPTIONS.joinpath("perpsCandles.graphql").read_text(
    encoding="utf-8"
)
_SUB_BLOCK: Final[str] = _SUBSCRIPTIONS.joinpath("block.graphql").read_text(encoding="utf-8")
_SUB_EVENTS: Final[str] = _SUBSCRIPTIONS.joinpath("events.graphql").read_text(encoding="utf-8")
_SUB_QUERY_APP: Final[str] = _SUBSCRIPTIONS.joinpath("queryApp.graphql").read_text(encoding="utf-8")
# `perpsEvents2` is served over REST/SSE — see `Info.subscribe_perps_events2`,
# which uses `SseManager` instead of a graphql-transport-ws document.

# perpsTrades is supported by the chain — `Subscription.perpsTrades` is
# defined in indexer/graphql-types/src/schemas/schema.graphql, and
# the API doc §8.2 documents the contract — but the Python SDK does not
# yet vendor a subscription document for it. We hand-write the document
# here so the Python SDK can ship perps trade streams. Once
# `indexer/graphql-types/src/schemas/subscriptions/perpsTrades.graphql`
# is also vendored as a symlink, this constant should be replaced with a
# symlink read alongside the others above.
_SUB_PERPS_TRADES: Final[str] = """
subscription SubscribePerpsTrades($pairId: String!) {
  perpsTrades(pairId: $pairId) {
    orderId
    pairId
    user
    fillPrice
    fillSize
    closingSize
    openingSize
    realizedPnl
    fee
    createdAt
    blockHeight
    tradeIdx
    fillId
    isMaker
  }
}
""".strip()


def _unwrap_node[T](payload: dict[str, Any], field: str, _typ: type[T]) -> Any:
    """Pull `payload.data.<field>` from a graphql-transport-ws `next` message."""

    # WebsocketManager wraps server-side errors as `{"_error": payload}`;
    # forward such envelopes verbatim so the user's callback can detect
    # them via `if "_error" in event:` and surface the failure. Any other
    # shape is a normal `next` payload, where the GraphQL response sits
    # under `payload.data.<root_field>` (e.g. `data.perpsCandles`).
    #
    # `_typ` is unused at runtime but documents the intended unwrap target
    # at each call site and leaves room for future structural validation
    # (e.g. via pydantic) without reshaping the call signature.
    if "_error" in payload:
        return payload

    # A `next` message can also carry GraphQL errors in-band as
    # `{"data": null, "errors": [...]}` — e.g. an unknown field on a server
    # that predates the operation, or a resolver failure. Surface those
    # through the same `{"_error": ...}` convention so callbacks have a single
    # error path and never receive a bare `None` they have to guard against.
    errors = payload.get("errors")
    if errors:
        return {"_error": errors}

    data = payload.get("data") or {}

    return data.get(field)


def _make_page_info(d: dict[str, Any]) -> PageInfo:
    """Convert a wire `pageInfo` dict (camelCase) to the snake_case `PageInfo` dataclass."""

    # `PageInfo` is one of the only places we cross the indexer-wire/Python
    # convention boundary (see the section comment in `dango/utils/types.py`).
    # Everything else (PerpsCandle, PerpsEvent, ...) keeps the camelCase wire
    # shape verbatim, so this helper is the single rename point.
    return PageInfo(
        has_previous_page=d["hasPreviousPage"],
        has_next_page=d["hasNextPage"],
        start_cursor=d.get("startCursor"),
        end_cursor=d.get("endCursor"),
    )


def _make_connection(d: dict[str, Any], node_key: str = "nodes") -> Connection[Any]:
    """Wrap a wire `Connection`-shaped dict in a typed `Connection[Any]` dataclass."""

    # `node_key` is parameterized only because GraphQL technically permits a
    # connection's nodes field to be aliased; in practice every document we
    # vendor uses the literal `nodes`. We don't narrow the type parameter
    # here (returning `Connection[Any]`) — callers `cast()` to the precise
    # `Connection[PerpsCandle]` / `Connection[PerpsEvent]` to avoid
    # duplicating the helper for each node type.
    return Connection(
        nodes=d[node_key],
        page_info=_make_page_info(d["pageInfo"]),
    )


def paginate_all[T](
    fetch_page: Callable[[str | None, int], Connection[T]],
    *,
    page_size: int = 100,
) -> Iterator[T]:
    """Yield every node across all forward-paginated pages."""

    # Forward-only pagination: walk via after-cursor + first-count. Mirrors
    # `sdk/rust/src/client.rs::paginate_all` but returns a generator rather
    # than a `Vec` so memory stays bounded over very long event histories.
    # Backward pagination (last/before) is intentionally out of scope for v1
    # — callers can fall back to the underlying `perps_events` query if they
    # need a tail-anchored walk.
    after: str | None = None

    while True:
        page = fetch_page(after, page_size)

        yield from page.nodes

        # Stop when the chain says no more, or when (defensively) the cursor
        # is missing — both indicate the page is the last. The end-cursor
        # check guards against a non-conforming server that returns
        # `has_next_page=true` but no cursor: rather than spin on the same
        # page forever, we treat it as terminal.
        if not page.page_info.has_next_page or page.page_info.end_cursor is None:
            break

        after = page.page_info.end_cursor


class Info(API):
    """Read primitives over GraphQL: status, queryApp variants, simulate, broadcast."""

    def __init__(
        self,
        base_url: str,
        *,
        skip_ws: bool = False,
        timeout: float | None = None,
        perps_contract: Addr | None = None,
    ) -> None:
        super().__init__(base_url, timeout=timeout)

        # `skip_ws` is a Phase 9 placeholder: the WebSocket subscription layer
        # will read it to bypass the ws/ subscription pipeline (e.g. in tests
        # or when the indexer's GraphQL-over-WS endpoint is unavailable). It
        # has no effect in Phase 6 — stored here so the Phase 6 constructor
        # signature is forward-compatible with future phases.
        self.skip_ws: bool = skip_ws

        # The perps contract address differs between mainnet and testnet
        # (see constants.py). Default to mainnet so the common case is
        # zero-config; pass `PERPS_CONTRACT_TESTNET` (or any other deployment
        # address) explicitly when running against a non-mainnet chain. Wrap
        # in `Addr(...)` so the stored field is the typed alias regardless of
        # whether the caller passed a typed `Addr` or a plain `str` constant.
        self.perps_contract: Addr = Addr(perps_contract or PERPS_CONTRACT_MAINNET)

        # WebsocketManager is created on first subscribe via the `_ws`
        # property — see its docstring for the rationale. Storing `None`
        # here keeps the construction cost off the hot path for callers
        # who only do read queries or who pass `skip_ws=True`.
        self._ws_manager: WebsocketManager | None = None

        # SseManager is the REST/SSE transport behind `subscribe_perps_events2`,
        # created lazily via the `_sse` property (same rationale as `_ws`).
        self._sse_manager: SseManager | None = None

    def query_status(self) -> dict[str, Any]:
        """Chain ID and latest block info."""

        data = self.query(_QUERY_STATUS)

        return cast("dict[str, Any]", data["queryStatus"])

    def query_app(
        self,
        request: dict[str, Any],
        *,
        height: int | None = None,
    ) -> Any:
        """Generic `queryApp` wrapper. Returns the raw kind-keyed envelope."""

        # The chain returns `{<kind>: <data>}` for every request variant —
        # e.g. `{"wasm_smart": <contract response>}`, `{"multi": [...]}`,
        # `{"config": ...}`. We deliberately return the wrapped shape here
        # so kind-specific callers can pick their own typed return shape;
        # the convenience methods `query_app_smart` and `query_app_multi`
        # do the unwrap.
        data = self.query(
            _QUERY_APP,
            variables={"request": request, "height": height},
        )

        return data["queryApp"]

    def query_app_smart(
        self,
        contract: Addr,
        msg: dict[str, Any],
        *,
        height: int | None = None,
    ) -> Any:
        """Convenience for `{wasm_smart: {contract, msg}}` queries; unwraps the envelope."""

        # `query_app` returns the kind-keyed wrapper `{"wasm_smart": <inner>}`;
        # unwrap so callers see the contract's own response shape directly.
        # `query_app_multi` does the same with its `["multi"]` unwrap.

        return self.query_app(
            {"wasm_smart": {"contract": contract, "msg": msg}},
            height=height,
        )["wasm_smart"]

    def query_app_multi(
        self,
        queries: list[dict[str, Any]],
        *,
        height: int | None = None,
    ) -> list[dict[str, Any]]:
        """Atomically run multiple queries at one block height (API §1.4)."""

        # Each result is wrapped as `{"Ok": <value>}` or `{"Err": "<msg>"}`.
        # We deliberately return the raw wrappers instead of auto-unwrapping
        # so callers can decide per-element how to handle partial failures —
        # by design, one query in the batch may fail without aborting the
        # whole batch, and an auto-unwrap that raised on the first Err would
        # collapse that signal.
        result = self.query_app({"multi": queries}, height=height)

        return cast("list[dict[str, Any]]", result["multi"])

    def simulate(self, tx: UnsignedTx) -> dict[str, Any]:
        """Dry-run an UnsignedTx; returns gas_used, gas_limit, and result."""

        data = self.query(_QUERY_SIMULATE, variables={"tx": tx})

        return cast("dict[str, Any]", data["simulate"])

    def broadcast_tx_sync(self, tx: Tx) -> dict[str, Any]:
        """Submit a signed Tx; returns the BroadcastTxOutcome envelope."""

        # broadcastTxSync is a GraphQL mutation, not a query. We still send
        # it through `self.query()` because that helper is HTTP-level: it
        # POSTs `{query, variables}` to /graphql and the GraphQL server
        # routes by the document's operation keyword. The query/mutation
        # distinction lives inside the document string, not in the
        # transport.
        data = self.query(_MUTATION_BROADCAST_TX_SYNC, variables={"tx": tx})

        return cast("dict[str, Any]", data["broadcastTxSync"])

    # --- Perps queries -------------------------------------------------------
    #
    # Each method below is a typed wrapper around `query_app_smart` against
    # the perps smart contract. The wire format is determined by the Rust
    # `QueryMsg` enum (see `dango/types/src/perps.rs`); its serde external
    # tagging produces snake_case keys like `{"pair_param": {...}}`. We pass
    # those msg dicts through verbatim and rely on `cast` to narrow the
    # resulting JSON to the typed shapes defined in `dango.utils.types`.
    # Optional fields (e.g. `start_after`, `limit`) are forwarded as-is —
    # `None` becomes JSON `null`, which the Rust side accepts via
    # `Option<T>`.

    def perps_param(self) -> Param:
        """Global perps parameters: fee schedules, OI cap, funding period, etc."""

        return cast("Param", self.query_app_smart(self.perps_contract, {"param": {}}))

    def perps_state(self) -> State:
        """Global perps state: last funding time, vault share supply, treasury, insurance fund."""

        return cast("State", self.query_app_smart(self.perps_contract, {"state": {}}))

    def pair_param(self, pair_id: PairId) -> PairParam | None:
        """Per-pair parameters; `None` if the pair is not configured."""

        response = self.query_app_smart(
            self.perps_contract,
            {"pair_param": {"pair_id": pair_id}},
        )

        return cast("PairParam | None", response)

    def pair_params(
        self,
        *,
        start_after: PairId | None = None,
        limit: int = 30,
    ) -> dict[PairId, PairParam]:
        """Enumerate per-pair parameters; paginated via (start_after, limit)."""

        return cast(
            "dict[PairId, PairParam]",
            self.query_app_smart(
                self.perps_contract,
                {"pair_params": {"start_after": start_after, "limit": limit}},
            ),
        )

    def pair_state(self, pair_id: PairId) -> PairState | None:
        """Per-pair runtime state: open interest, funding accumulator, current rate."""

        response = self.query_app_smart(
            self.perps_contract,
            {"pair_state": {"pair_id": pair_id}},
        )

        return cast("PairState | None", response)

    def pair_states(
        self,
        *,
        start_after: PairId | None = None,
        limit: int = 30,
    ) -> dict[PairId, PairState]:
        """Enumerate per-pair states; paginated via (start_after, limit)."""

        return cast(
            "dict[PairId, PairState]",
            self.query_app_smart(
                self.perps_contract,
                {"pair_states": {"start_after": start_after, "limit": limit}},
            ),
        )

    def liquidity_depth(
        self,
        pair_id: PairId,
        *,
        bucket_size: str,
        limit: int | None = None,
    ) -> LiquidityDepthResponse:
        """Aggregated bid/ask depth at a price-bucket granularity."""

        # `bucket_size` is the wire form of `UsdPrice` — a 6-decimal fixed-
        # point string, e.g. "10.000000". The contract requires the value to
        # match one of the entries in this pair's `pair_param.bucket_sizes`;
        # the SDK does not validate that constraint client-side because the
        # rejection cost is just a single failed query. Callers that want to
        # avoid the round-trip should fetch `pair_param(pair_id).bucket_sizes`
        # first and pick a value from that list.
        return cast(
            "LiquidityDepthResponse",
            self.query_app_smart(
                self.perps_contract,
                {
                    "liquidity_depth": {
                        "pair_id": pair_id,
                        "bucket_size": bucket_size,
                        "limit": limit,
                    },
                },
            ),
        )

    def user_state(self, user: Addr) -> UserState | None:
        """A user's deposited margin, positions, vault shares, and pending unlocks."""

        response = self.query_app_smart(
            self.perps_contract,
            {"user_state": {"user": user}},
        )

        return cast("UserState | None", response)

    def user_state_extended(
        self,
        user: Addr,
        *,
        include_equity: bool = True,
        include_available_margin: bool = True,
        include_maintenance_margin: bool = True,
        include_unrealized_pnl: bool = True,
        include_unrealized_funding: bool = True,
        include_liquidation_price: bool = False,
    ) -> UserStateExtended | None:
        """User state plus computed equity / margin / PnL fields per the include_* knobs."""

        # The Rust `QueryMsg::UserStateExtended` variant has a 7th boolean,
        # `include_all`, that overrides every per-flag knob when true. We
        # deliberately omit it from the Python signature — there shouldn't
        # be two ways to ask for the same thing, and the per-flag knobs are
        # the more granular API. Serde's `#[serde(default)]` on the Rust side
        # makes the field default to `false` when absent, so omitting it from
        # the request is correct (and we explicitly do NOT send
        # `include_all: false` so this design choice is visible on the wire).
        return cast(
            "UserStateExtended | None",
            self.query_app_smart(
                self.perps_contract,
                {
                    "user_state_extended": {
                        "user": user,
                        "include_equity": include_equity,
                        "include_available_margin": include_available_margin,
                        "include_maintenance_margin": include_maintenance_margin,
                        "include_unrealized_pnl": include_unrealized_pnl,
                        "include_unrealized_funding": include_unrealized_funding,
                        "include_liquidation_price": include_liquidation_price,
                    },
                },
            ),
        )

    def orders_by_user(self, user: Addr) -> dict[OrderId, dict[str, Any]]:
        """All resting limit orders for a user, keyed by OrderId."""

        # The Rust response is a `BTreeMap<OrderId, QueryOrdersByUserResponseItem>`
        # which serde encodes as a JSON object (string-keyed). The value type
        # is left as `dict[str, Any]` rather than a TypedDict because the
        # roadmap signature uses an opaque dict here; callers that want
        # typed access can re-cast to `QueryOrdersByUserResponseItem`.
        return cast(
            "dict[OrderId, dict[str, Any]]",
            self.query_app_smart(self.perps_contract, {"orders_by_user": {"user": user}}),
        )

    def order(self, order_id: OrderId) -> dict[str, Any] | None:
        """A single resting limit order by ID; `None` if the order does not exist."""

        # As with `orders_by_user`, we return an opaque dict per the roadmap
        # signature. The on-the-wire shape is `QueryOrderResponse` from
        # `dango.utils.types`; cast there if you need typed access.
        response = self.query_app_smart(
            self.perps_contract,
            {"order": {"order_id": order_id}},
        )

        return cast("dict[str, Any] | None", response)

    def volume(self, user: Addr, *, since: int | None = None) -> str:
        """User's cumulative trading volume in USD; `since` is a ns-timestamp filter."""

        # The contract returns a `UsdValue`, which is a 6-decimal fixed-point
        # string (e.g. `"1250000.000000"`); we surface that string directly
        # without parsing into Decimal to keep this method allocation-free
        # and to leave numeric handling to the caller's preferred library.
        # `since=None` means lifetime volume; otherwise interpret `since` as
        # a nanosecond timestamp lower bound (matches Rust `Timestamp` wire
        # format, which the contract receives as a stringified integer but
        # accepts as a JSON number too).
        return cast(
            "str",
            self.query_app_smart(
                self.perps_contract,
                {"volume": {"user": user, "since": since}},
            ),
        )

    # --- Indexer queries -----------------------------------------------------
    #
    # These methods query the indexer GraphQL API rather than the chain's
    # `query_app` endpoint. Wire keys are camelCase (per the .graphql
    # documents) — see the convention-boundary comment in
    # `dango/utils/types.py`. The TypedDicts returned here keep camelCase
    # attribute names; only `Connection`/`PageInfo` cross over to snake_case.

    def perps_candles(
        self,
        pair_id: PairId,
        interval: CandleInterval,
        *,
        later_than: str | None = None,
        earlier_than: str | None = None,
        first: int | None = None,
        after: str | None = None,
    ) -> Connection[PerpsCandle]:
        """OHLCV candles for one pair at one interval; cursor-paginated."""

        # `interval.value` rather than `interval` because the GraphQL enum
        # variable is typed `CandleInterval!` and json-encodes as the bare
        # uppercase name (e.g. `"ONE_MINUTE"`). Passing the StrEnum object
        # directly would serialize as the enum's repr, not the wire form.
        data = self.query(
            _QUERY_PERPS_CANDLES,
            variables={
                "pairId": pair_id,
                "interval": interval.value,
                "laterThan": later_than,
                "earlierThan": earlier_than,
                "first": first,
                "after": after,
            },
        )

        return cast("Connection[PerpsCandle]", _make_connection(data["perpsCandles"]))

    def perps_events(
        self,
        *,
        user_addr: Addr | None = None,
        event_type: str | None = None,
        pair_id: PairId | None = None,
        block_height: int | None = None,
        first: int | None = None,
        after: str | None = None,
        sort_by: PerpsEventSortBy = PerpsEventSortBy.BLOCK_HEIGHT_DESC,
    ) -> Connection[PerpsEvent]:
        """Indexer events stream with filter + sort knobs; cursor-paginated."""

        # The default sort matches the indexer's own default ordering
        # (`BLOCK_HEIGHT_DESC`), so the most recent events come first when
        # the caller doesn't override. `event_type` is left as a free-form
        # string here because the indexer schema does not constrain it to an
        # enum on the GraphQL side.
        data = self.query(
            _QUERY_PERPS_EVENTS,
            variables={
                "userAddr": user_addr,
                "eventType": event_type,
                "pairId": pair_id,
                "blockHeight": block_height,
                "first": first,
                "after": after,
                "sortBy": sort_by.value,
            },
        )

        return cast("Connection[PerpsEvent]", _make_connection(data["perpsEvents"]))

    def perps_pair_stats(self, pair_id: PairId) -> PerpsPairStats:
        """24h price/volume stats for one pair."""

        # The vendored `perpsPairStats.graphql` document declares its
        # variable as `$pair_id` (snake_case) — this is an anomaly versus
        # sibling indexer queries that use `$pairId`. We send `pair_id`
        # verbatim to match the document; if the document is regenerated to
        # camelCase upstream, this kwarg key needs to flip too.
        data = self.query(_QUERY_PERPS_PAIR_STATS, variables={"pair_id": pair_id})

        return cast("PerpsPairStats", data["perpsPairStats"])

    def all_perps_pair_stats(self) -> list[PerpsPairStats]:
        """24h stats for every active pair."""

        data = self.query(_QUERY_ALL_PERPS_PAIR_STATS)

        return cast("list[PerpsPairStats]", data["allPerpsPairStats"])

    def perps_events_all(
        self,
        *,
        user_addr: Addr | None = None,
        event_type: str | None = None,
        pair_id: PairId | None = None,
        block_height: int | None = None,
        sort_by: PerpsEventSortBy = PerpsEventSortBy.BLOCK_HEIGHT_DESC,
        page_size: int = 100,
    ) -> Iterator[PerpsEvent]:
        """Iterate every perps event matching the filter, walking pages internally."""

        # Thin wrapper over `paginate_all`: rebinds the filter kwargs at
        # each fetch so they're constant across pages, and lets the cursor
        # walker provide `(after, first)` per call. Returns the generator
        # eagerly so callers can `for event in info.perps_events_all(...)`
        # without an extra `()` step.
        return paginate_all(
            lambda after, first: self.perps_events(
                user_addr=user_addr,
                event_type=event_type,
                pair_id=pair_id,
                block_height=block_height,
                first=first,
                after=after,
                sort_by=sort_by,
            ),
            page_size=page_size,
        )

    # --- Subscriptions -------------------------------------------------------
    #
    # Real-time streams over graphql-transport-ws. Each `subscribe_*` method
    # registers a callback with the underlying `WebsocketManager`, returns an
    # int subscription id, and unwraps the GraphQL `next` payload to the
    # natural shape (one `Trade` per fill, one `Block` per block, etc.) so
    # callers don't have to reach into `payload["data"][...]` themselves.
    # Server-side errors flow through the same callback as `{"_error": ...}`
    # — see `_unwrap_node` for the convention.

    @property
    def _ws(self) -> WebsocketManager:
        """Lazily create and start the WebsocketManager on first subscription."""

        # Creating the connection only on first subscribe means callers who
        # only do read queries (or who pass `skip_ws=True`) never pay the
        # cost of an idle WebSocket — the indexer doesn't need to be reached
        # at all unless a subscription is actually opened.
        if self._ws_manager is None:
            if self.skip_ws:
                raise RuntimeError(
                    "WebSocket disabled (skip_ws=True); "
                    "construct Info with skip_ws=False to enable subscriptions",
                )

            self._ws_manager = WebsocketManager(self.base_url)
            self._ws_manager.start()

        return self._ws_manager

    @property
    def _sse(self) -> SseManager:
        """Lazily create the SseManager on first REST/SSE subscription."""

        # Same lazy rationale as `_ws`: callers who only run read queries (or
        # pass `skip_ws=True`) never pay for an idle streaming transport. The
        # SSE feed is HTTP, but `skip_ws` gates all subscriptions uniformly.
        if self._sse_manager is None:
            if self.skip_ws:
                raise RuntimeError(
                    "subscriptions disabled (skip_ws=True); "
                    "construct Info with skip_ws=False to enable subscriptions",
                )

            self._sse_manager = SseManager(self.base_url)

        return self._sse_manager

    def subscribe_perps_trades(
        self,
        pair_id: PairId,
        callback: Callable[[Trade], None],
    ) -> int:
        """Stream real-time perps trade fills for one pair."""

        # Per API doc §8.2 the server replays cached recent trades on
        # connect, then streams new fills as they happen. Each `next`
        # message carries one trade; we unwrap to the inner Trade so the
        # callback signature matches what the user expects.
        return self._ws.subscribe(
            _SUB_PERPS_TRADES,
            {"pairId": pair_id},
            lambda payload: callback(_unwrap_node(payload, "perpsTrades", Trade)),
        )

    def subscribe_perps_candles(
        self,
        pair_id: PairId,
        interval: CandleInterval,
        callback: Callable[[PerpsCandle], None],
    ) -> int:
        """Stream OHLCV candles for one pair at one interval."""

        # `interval.value` rather than `interval` because the GraphQL
        # variable is typed `CandleInterval!` and is sent as the bare
        # uppercase enum name (e.g. `"ONE_MINUTE"`) — same convention as
        # `perps_candles` above.
        #
        # The `perpsCandles` subscription field is GraphQL-typed as a list
        # (the server may emit multiple candles in one push when catching
        # up after a reconnect), but the callback contract is one candle
        # at a time — matching `subscribe_perps_trades`. We iterate here
        # so callers don't have to.
        def fan_out(payload: dict[str, Any]) -> None:
            node = _unwrap_node(payload, "perpsCandles", PerpsCandle)

            if isinstance(node, list):
                for candle in node:
                    callback(cast("PerpsCandle", candle))
            else:
                # Error envelope (`{"_error": ...}`) or unexpected shape
                # — forward as-is so the caller's error handling fires.
                callback(cast("PerpsCandle", node))

        return self._ws.subscribe(
            _SUB_PERPS_CANDLES,
            {"pairId": pair_id, "interval": interval.value, "laterThan": None},
            fan_out,
        )

    def subscribe_query_app(
        self,
        request: dict[str, Any],
        callback: Callable[[dict[str, Any]], None],
        *,
        block_interval: int = 10,
    ) -> int:
        """Re-run a queryApp request every N blocks; callback gets {response, blockHeight}."""

        # `block_interval` is in *blocks*, not seconds. The default of 10
        # corresponds to roughly every 10 seconds at Dango's ~1s block
        # time; callers driving a UI may want a smaller value, while
        # background pollers can use a larger one.
        #
        # The chain wraps the response in the same kind-keyed envelope
        # used by the HTTP path (`{"response": {"wasm_smart": <inner>},
        # ...}` / `{"response": {"multi": [...]}, ...}`). Mirror the
        # `query_app_smart` / `query_app_multi` HTTP-side unwrap so
        # callbacks see the contract response directly under
        # `payload["response"]`. Single-key requests are auto-unwrapped;
        # the rare multi-key request passes through untouched.
        request_kind = next(iter(request)) if len(request) == 1 else None

        def unwrap(payload: dict[str, Any]) -> None:
            unwrapped = _unwrap_node(payload, "queryApp", dict)

            if (
                request_kind is not None
                and isinstance(unwrapped, dict)
                and "_error" not in unwrapped
            ):
                response = unwrapped.get("response")
                if isinstance(response, dict) and request_kind in response:
                    unwrapped = {**unwrapped, "response": response[request_kind]}

            callback(unwrapped)

        return self._ws.subscribe(
            _SUB_QUERY_APP,
            {"request": request, "blockInterval": block_interval},
            unwrap,
        )

    def subscribe_user_events(
        self,
        user: Addr,
        callback: Callable[[PerpsEvent], None],
        *,
        event_types: list[str] | None = None,
    ) -> int:
        """Stream events for one user, optionally filtered by event_type list."""

        # The `events` subscription accepts a `[Filter!]` array. Per the
        # indexer (`indexer/httpd/src/graphql/subscription/event.rs:138-150`),
        # entries within one filter are AND-combined and the array of
        # filters is OR-combined. So:
        #   * always pin `data.user` to the given address (one check
        #     entry inside the filter's `data` list);
        #   * if `event_types` is provided, emit one filter entry per
        #     type, each carrying the same user check, so the result is
        #     `(type=A AND user=X) OR (type=B AND user=X)` — the union
        #     of types intersected with the user.
        user_data = {"path": ["user"], "checkMode": "EQUAL", "value": [user]}

        if event_types:
            filters: list[dict[str, Any]] = [{"type": t, "data": [user_data]} for t in event_types]
        else:
            filters = [{"data": [user_data]}]

        return self._ws.subscribe(
            _SUB_EVENTS,
            {"sinceBlockHeight": None, "filter": filters},
            lambda payload: callback(_unwrap_node(payload, "events", PerpsEvent)),
        )

    def subscribe_block(self, callback: Callable[[Block], None]) -> int:
        """Stream every newly-finalized block."""

        # No variables — the `block` subscription has no filter knobs,
        # the server pushes every block as it commits. `Block` payloads
        # include the full transaction list and flattened event list,
        # see `subscriptions/block.graphql`.
        return self._ws.subscribe(
            _SUB_BLOCK,
            {},
            lambda payload: callback(_unwrap_node(payload, "block", Block)),
        )

    def subscribe_perps_events2(
        self,
        callback: Callable[[PerpsEvent2Batch], None],
        *,
        since_block_height: int | None = None,
        event_types: list[str] | None = None,
        pair_ids: list[str] | None = None,
        users: list[str] | None = None,
        order_ids: list[str] | None = None,
        client_order_ids: list[str] | None = None,
    ) -> int:
        """Stream every perps-contract event, grouped per block, over REST/SSE.

        The richer superset of `subscribe_perps_trades` (which streams fills
        only): order lifecycle, fills, liquidations, and deleveraging. With no
        filter the full firehose is streamed. Each Server-Sent event is one
        block's batch, so the callback receives a `PerpsEvent2Batch` (block
        height/timestamp + the matching events) — not one event at a time. A
        resync (HTTP 409), a subscription-limit hit (HTTP 429), or a connection
        error is delivered as `{"_error": ...}`, the same envelope the
        WebSocket subscriptions use.
        """

        # Each filter defaults to None (not filtered on that field) and is sent
        # as a comma-separated query parameter; an absent (or empty) parameter
        # matches everything. The five filters AND together — see API doc §8.3.
        # A `client_order_id` is unique only per sender, so combine
        # `client_order_ids` with `users` to single out one trader's order.
        params: dict[str, str] = {}
        if since_block_height is not None:
            params["since"] = str(since_block_height)
        for key, values in (
            ("eventTypes", event_types),
            ("pairIds", pair_ids),
            ("users", users),
            ("orderIds", order_ids),
            ("clientOrderIds", client_order_ids),
        ):
            if values is not None:
                params[key] = ",".join(values)

        # The SSE payload is already the bare batch (no GraphQL `data` wrapper),
        # so the callback is passed straight through. The cast bridges the
        # batch-typed public callback to the manager's generic dict callback —
        # which also carries the `{"_error": ...}` envelope, exactly as the WS
        # path does.
        return self._sse.subscribe(
            "perps/events/stream",
            params,
            cast("Callable[[dict[str, Any]], None]", callback),
        )

    def unsubscribe(self, subscription_id: int) -> bool:
        """Drop a subscription locally and tell the server to stop streaming.

        The WebSocket and SSE transports keep independent id spaces; this tries
        the WS manager first, then the SSE manager, so a single id resolves to
        whichever transport owns it.
        """

        # Returning False (rather than raising) when no manager exists
        # mirrors `WebsocketManager.unsubscribe`'s "id-not-found" return
        # — callers can blanket-call this on shutdown without first
        # checking whether they ever opened a subscription.
        if self._ws_manager is not None and self._ws_manager.unsubscribe(subscription_id):
            return True

        if self._sse_manager is not None:
            return self._sse_manager.unsubscribe(subscription_id)

        return False

    def disconnect_websocket(self) -> None:
        """Close the streaming connections and clean up the manager threads."""

        # 5s grace period is generous for each manager to wind down after
        # `stop()`. The worker threads are daemons, so even if `join` returns
        # without a thread actually finishing (e.g. socket stuck), Python won't
        # hang on interpreter exit waiting for it. Clearing the references lets
        # a future `_ws` / `_sse` access spin up a fresh manager — useful if
        # the user reconnects after an explicit disconnect.
        if self._ws_manager is not None:
            self._ws_manager.stop()
            self._ws_manager.join(timeout=5.0)
            self._ws_manager = None

        if self._sse_manager is not None:
            self._sse_manager.stop()
            self._sse_manager.join(timeout=5.0)
            self._sse_manager = None

"""Low-level GraphQL query primitives; subclass of API used by every read path."""

from __future__ import annotations

from importlib.resources import files
from typing import Any, Final, cast

from dango.api import API
from dango.utils.constants import PERPS_CONTRACT_MAINNET
from dango.utils.types import (
    Addr,
    LiquidityDepthResponse,
    OrderId,
    PairId,
    PairParam,
    PairState,
    Param,
    State,
    Tx,
    UnsignedTx,
    UserState,
    UserStateExtended,
)

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
        """Generic `queryApp` wrapper. Return type varies by request shape."""
        # The response shape depends entirely on the request kind: `wasm_smart`
        # returns the contract's JSON response (any shape), `multi` returns
        # `{multi: [...]}`, `config` returns the chain config, etc. Returning
        # `Any` rather than `dict` lets typed callers (Phase 7's domain
        # methods) declare their own precise return types without first
        # casting away an inaccurate `dict[str, Any]`.
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
        """Convenience for `{wasm_smart: {contract, msg}}` queries."""
        return self.query_app(
            {"wasm_smart": {"contract": contract, "msg": msg}},
            height=height,
        )

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

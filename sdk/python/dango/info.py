"""Low-level GraphQL query primitives; subclass of API used by every read path."""

from __future__ import annotations

from importlib.resources import files
from typing import Any, Final, cast

from dango.api import API
from dango.utils.types import Addr, Tx, UnsignedTx

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
    ) -> None:
        super().__init__(base_url, timeout=timeout)
        # `skip_ws` is a Phase 9 placeholder: the WebSocket subscription layer
        # will read it to bypass the ws/ subscription pipeline (e.g. in tests
        # or when the indexer's GraphQL-over-WS endpoint is unavailable). It
        # has no effect in Phase 6 — stored here so the Phase 6 constructor
        # signature is forward-compatible with future phases.
        self.skip_ws: bool = skip_ws

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

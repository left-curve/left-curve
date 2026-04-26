"""Tests for dango.info.Info — GraphQL query primitives."""

from __future__ import annotations

import json
from typing import Any, cast

from pytest_httpserver import HTTPServer
from werkzeug.wrappers import Request, Response

from dango.api import API
from dango.info import Info
from dango.utils.signing import _QueryClient
from dango.utils.types import Addr, Metadata, Nonce, Tx, UnsignedTx, UserIndex

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")
_CONTRACT_ADDRESS = Addr("0x000000000000000000000000000000000000cafe")


def _info(httpserver: HTTPServer) -> Info:
    return Info(httpserver.url_for("/").rstrip("/"))


def _capture_request(
    httpserver: HTTPServer,
    response: dict[str, Any],
) -> list[dict[str, Any]]:
    """Stub /graphql to return `response` and capture each inbound JSON body."""
    # Returning the captured-bodies list lets the caller call the SDK method
    # under test and then assert on `captured[0]` to verify the on-the-wire
    # query/variables shape.
    captured: list[dict[str, Any]] = []

    def handler(request: Request) -> Response:
        captured.append(cast("dict[str, Any]", request.get_json()))
        return Response(json.dumps(response), mimetype="application/json")

    httpserver.expect_request("/graphql", method="POST").respond_with_handler(handler)
    return captured


def _demo_unsigned_tx() -> UnsignedTx:
    return UnsignedTx(
        sender=_DEMO_ADDRESS,
        msgs=[],
        data=Metadata(
            user_index=UserIndex(1),
            chain_id="dango-1",
            nonce=Nonce(0),
            expiry=None,
        ),
    )


def _demo_tx() -> Tx:
    return Tx(
        sender=_DEMO_ADDRESS,
        gas_limit=100_000,
        msgs=[],
        data=Metadata(
            user_index=UserIndex(1),
            chain_id="dango-1",
            nonce=Nonce(0),
            expiry=None,
        ),
        credential=cast(
            Any,
            {"standard": {"key_hash": "0" * 64, "signature": {"secp256k1": "AAAA"}}},
        ),
    )


class TestConstruction:
    def test_inherits_from_api(self) -> None:
        """Info is an API subclass; HTTP error mapping is inherited."""
        info = Info("http://example.com")
        assert isinstance(info, API)

    def test_skip_ws_default_false(self) -> None:
        """Phase 9 placeholder defaults to off."""
        info = Info("http://example.com")
        assert info.skip_ws is False

    def test_skip_ws_can_be_set(self) -> None:
        """skip_ws is wired through the constructor."""
        info = Info("http://example.com", skip_ws=True)
        assert info.skip_ws is True

    def test_strips_trailing_slash(self) -> None:
        """Inherited from API — Info applies the same base_url normalization."""
        info = Info("http://example.com/")
        assert info.base_url == "http://example.com"


class TestQueryStatus:
    def test_returns_chain_id_and_block(self, httpserver: HTTPServer) -> None:
        """Posts QueryStatus and unwraps to {chainId, block}."""
        block = {"blockHeight": 42, "timestamp": "2025-01-01T00:00:00Z", "hash": "0xabc"}
        _capture_request(
            httpserver,
            {"data": {"queryStatus": {"chainId": "dango-1", "block": block}}},
        )
        result = _info(httpserver).query_status()
        assert result == {"chainId": "dango-1", "block": block}

    def test_posts_query_status_document(self, httpserver: HTTPServer) -> None:
        """The wire body has the QueryStatus GraphQL document."""
        captured = _capture_request(
            httpserver,
            {"data": {"queryStatus": {"chainId": "dango-1", "block": {}}}},
        )
        _info(httpserver).query_status()
        assert "QueryStatus" in captured[0]["query"]
        assert "queryStatus" in captured[0]["query"]
        # query_status has no variables; API.query() sends {} for that case.
        assert captured[0]["variables"] == {}


class TestQueryApp:
    def test_passes_request_and_height(self, httpserver: HTTPServer) -> None:
        """request and height are forwarded as GraphQL variables."""
        captured = _capture_request(
            httpserver,
            {"data": {"queryApp": {"some": "value"}}},
        )
        _info(httpserver).query_app({"config": {}}, height=100)
        assert captured[0]["variables"] == {"request": {"config": {}}, "height": 100}

    def test_returns_raw_query_app_value(self, httpserver: HTTPServer) -> None:
        """The unwrapped `queryApp` field is returned untouched."""
        # Use a non-dict value to prove we don't wrap or coerce the response.
        _capture_request(httpserver, {"data": {"queryApp": [1, 2, 3]}})
        result = _info(httpserver).query_app({"some": "request"})
        assert result == [1, 2, 3]

    def test_default_height_is_null(self, httpserver: HTTPServer) -> None:
        """Omitting height sends `null` so the server uses latest block."""
        captured = _capture_request(httpserver, {"data": {"queryApp": {}}})
        _info(httpserver).query_app({"config": {}})
        assert captured[0]["variables"] == {"request": {"config": {}}, "height": None}


class TestQueryAppSmart:
    def test_wraps_in_wasm_smart(self, httpserver: HTTPServer) -> None:
        """The request becomes {wasm_smart: {contract, msg}}."""
        captured = _capture_request(
            httpserver,
            {"data": {"queryApp": {"result": "ok"}}},
        )
        _info(httpserver).query_app_smart(_CONTRACT_ADDRESS, {"foo": "bar"})
        assert captured[0]["variables"]["request"] == {
            "wasm_smart": {"contract": _CONTRACT_ADDRESS, "msg": {"foo": "bar"}}
        }

    def test_satisfies_query_client_protocol(self) -> None:
        """Phase 5's _QueryClient Protocol is satisfied structurally."""
        # _QueryClient is `runtime_checkable`-able (Protocol with one method)
        # in spirit, but in Phase 5 it's not annotated as such. We instead
        # cast through the Protocol to confirm static (mypy) compatibility,
        # and rely on a duck-typed `hasattr` check for runtime.
        info = Info("http://example.com")
        client: _QueryClient = info  # static type-check via assignment
        assert hasattr(client, "query_app_smart")
        assert callable(client.query_app_smart)


class TestQueryAppMulti:
    def test_wraps_in_multi(self, httpserver: HTTPServer) -> None:
        """The request becomes {multi: [...]}."""
        queries: list[dict[str, Any]] = [{"config": {}}, {"info": {}}]
        captured = _capture_request(
            httpserver,
            {"data": {"queryApp": {"multi": [{"Ok": 1}, {"Ok": 2}]}}},
        )
        _info(httpserver).query_app_multi(queries)
        assert captured[0]["variables"]["request"] == {"multi": queries}

    def test_returns_raw_ok_err_wrappers(self, httpserver: HTTPServer) -> None:
        """Heterogeneous results are returned with their Ok/Err wrappers intact."""
        # The list is heterogeneous on purpose — one Ok and one Err — to prove
        # that Info does not auto-unwrap or short-circuit on the first Err.
        wrappers = [{"Ok": {"value": 1}}, {"Err": "boom"}]
        _capture_request(
            httpserver,
            {"data": {"queryApp": {"multi": wrappers}}},
        )
        result = _info(httpserver).query_app_multi([{"q1": {}}, {"q2": {}}])
        assert result == wrappers


class TestSimulate:
    def test_passes_tx_variable(self, httpserver: HTTPServer) -> None:
        """The UnsignedTx is forwarded as the `tx` GraphQL variable."""
        tx = _demo_unsigned_tx()
        captured = _capture_request(
            httpserver,
            {"data": {"simulate": {"gas_used": 1, "gas_limit": 2, "result": {}}}},
        )
        _info(httpserver).simulate(tx)
        assert captured[0]["variables"] == {"tx": tx}

    def test_returns_simulate_envelope(self, httpserver: HTTPServer) -> None:
        """Returns {gas_used, gas_limit, result}."""
        envelope = {"gas_used": 12_345, "gas_limit": 100_000, "result": {"Ok": []}}
        _capture_request(httpserver, {"data": {"simulate": envelope}})
        result = _info(httpserver).simulate(_demo_unsigned_tx())
        assert result == envelope


class TestBroadcastTxSync:
    def test_posts_mutation_document(self, httpserver: HTTPServer) -> None:
        """The wire body has the BroadcastTxSync GraphQL mutation document."""
        tx = _demo_tx()
        captured = _capture_request(
            httpserver,
            {"data": {"broadcastTxSync": {"check_tx": {}, "deliver_tx": {}}}},
        )
        _info(httpserver).broadcast_tx_sync(tx)
        assert "mutation" in captured[0]["query"]
        assert "BroadcastTxSync" in captured[0]["query"]
        assert captured[0]["variables"] == {"tx": tx}

    def test_returns_broadcast_outcome(self, httpserver: HTTPServer) -> None:
        """The unwrapped `broadcastTxSync` envelope is returned."""
        outcome = {"check_tx": {"code": 0}, "deliver_tx": {"code": 0, "events": []}}
        _capture_request(httpserver, {"data": {"broadcastTxSync": outcome}})
        result = _info(httpserver).broadcast_tx_sync(_demo_tx())
        assert result == outcome

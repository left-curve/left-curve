"""Tests for dango.info.Info — indexer GraphQL queries (Phase 8)."""

from __future__ import annotations

import dataclasses
import json
from typing import Any, cast

import pytest
from pytest_httpserver import HTTPServer
from werkzeug.wrappers import Request, Response

from dango.info import Info, paginate_all
from dango.utils.types import (
    Addr,
    CandleInterval,
    Connection,
    PageInfo,
    PairId,
    PerpsCandle,
    PerpsEvent,
    PerpsEventSortBy,
)

# Re-used demo addresses / pair IDs. The bytes are arbitrary — `..beef`
# tail makes assertions readable in failure output.
_DEMO_USER = Addr("0x000000000000000000000000000000000000beef")
_DEMO_PAIR = PairId("perp/btcusd")


def _info(httpserver: HTTPServer) -> Info:
    """Build an Info bound to the local httpserver."""
    return Info(httpserver.url_for("/").rstrip("/"))


def _capture_request(
    httpserver: HTTPServer,
    response: dict[str, Any],
) -> list[dict[str, Any]]:
    """Stub /graphql to return `response` and capture each inbound JSON body.

    Mirrors the helper used by the Phase 6/7 tests so assertions stay
    consistent across phases. Returning the captured-bodies list lets the
    caller assert on the on-the-wire query / variables shape after invoking
    the SDK method under test.
    """
    captured: list[dict[str, Any]] = []

    def handler(request: Request) -> Response:
        captured.append(cast("dict[str, Any]", request.get_json()))
        return Response(json.dumps(response), mimetype="application/json")

    httpserver.expect_request("/graphql", method="POST").respond_with_handler(handler)
    return captured


def _empty_page_info() -> dict[str, Any]:
    """Build a wire `pageInfo` dict for tests that don't care about pagination."""
    # Returns the minimal fully-populated pageInfo: no next or previous page,
    # both cursors null. Tests that exercise pagination override individual
    # fields by spreading this dict.
    return {
        "hasPreviousPage": False,
        "hasNextPage": False,
        "startCursor": None,
        "endCursor": None,
    }


# --- Type sanity -------------------------------------------------------------


class TestTypes:
    def test_page_info_is_frozen(self) -> None:
        """`PageInfo` is a frozen dataclass; assignment after construction raises."""
        # Frozen-ness matters because `paginate_all` and downstream consumers
        # treat the cursor state as immutable; an accidental mutation could
        # send the wrong cursor on the next page request.
        info = PageInfo(
            has_previous_page=False,
            has_next_page=True,
            start_cursor=None,
            end_cursor="abc",
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            info.has_next_page = False  # type: ignore[misc]

    def test_connection_is_generic(self) -> None:
        """`Connection[T]` parameterizes by node type and exposes nodes/page_info."""
        # Pin the public shape of the dataclass: a connection is always
        # `(nodes: list[T], page_info: PageInfo)`. The `Connection[int]`
        # subscript here is purely a static-typing exercise — it's preserved
        # by `__class_getitem__` and verified at runtime via attribute
        # presence.
        conn: Connection[int] = Connection(
            nodes=[1, 2, 3],
            page_info=PageInfo(
                has_previous_page=False,
                has_next_page=False,
                start_cursor=None,
                end_cursor=None,
            ),
        )
        assert conn.nodes == [1, 2, 3]
        assert conn.page_info.has_next_page is False


# --- paginate_all helper -----------------------------------------------------


class TestPaginateAll:
    def test_yields_single_page_then_stops(self) -> None:
        """A single page with `has_next_page=False` is exhausted after one fetch."""
        # We use a small fake `fetch_page` that records its inputs so we can
        # assert (a) the helper actually invoked the callable, and (b) only
        # invoked it once.
        calls: list[tuple[str | None, int]] = []

        def fetch(after: str | None, first: int) -> Connection[int]:
            calls.append((after, first))
            return Connection(
                nodes=[1, 2, 3],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=False,
                    start_cursor=None,
                    end_cursor=None,
                ),
            )

        assert list(paginate_all(fetch, page_size=10)) == [1, 2, 3]
        assert calls == [(None, 10)]

    def test_walks_multiple_pages(self) -> None:
        """`has_next_page=True` causes another fetch with the last `end_cursor`."""
        pages = [
            Connection[int](
                nodes=[1, 2],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=True,
                    start_cursor=None,
                    end_cursor="cursor-a",
                ),
            ),
            Connection[int](
                nodes=[3, 4],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=True,
                    start_cursor=None,
                    end_cursor="cursor-b",
                ),
            ),
            Connection[int](
                nodes=[5],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=False,
                    start_cursor=None,
                    end_cursor=None,
                ),
            ),
        ]
        calls: list[tuple[str | None, int]] = []

        def fetch(after: str | None, first: int) -> Connection[int]:
            calls.append((after, first))
            return pages.pop(0)

        # The helper should hand back the *concatenation* of all pages, in
        # order, and the `after` cursor on each subsequent call should match
        # the previous page's `end_cursor`.
        assert list(paginate_all(fetch, page_size=2)) == [1, 2, 3, 4, 5]
        assert calls == [(None, 2), ("cursor-a", 2), ("cursor-b", 2)]

    def test_stops_on_has_next_page_false(self) -> None:
        """Stopping is driven by `has_next_page`, not by an empty `nodes` list."""
        # The first (and only) page yields a non-empty list but signals
        # `has_next_page=False`. The helper must NOT make a second call —
        # otherwise we'd loop forever on indexers that keep returning the
        # same final page.
        calls: list[tuple[str | None, int]] = []

        def fetch(after: str | None, first: int) -> Connection[int]:
            calls.append((after, first))
            return Connection(
                nodes=[7, 8, 9],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=False,
                    start_cursor=None,
                    end_cursor="some-cursor",
                ),
            )

        assert list(paginate_all(fetch)) == [7, 8, 9]
        assert len(calls) == 1

    def test_stops_when_end_cursor_is_none(self) -> None:
        """Defensive: a `has_next_page=True` with no cursor is treated as terminal."""
        # A non-conforming server that says "more pages exist" but provides
        # no cursor would otherwise spin on the same page forever; the
        # helper guards against that by stopping when end_cursor is null.
        calls: list[tuple[str | None, int]] = []

        def fetch(after: str | None, first: int) -> Connection[int]:
            calls.append((after, first))
            return Connection(
                nodes=[1],
                page_info=PageInfo(
                    has_previous_page=False,
                    has_next_page=True,
                    start_cursor=None,
                    end_cursor=None,
                ),
            )

        assert list(paginate_all(fetch)) == [1]
        assert len(calls) == 1


# --- perps_candles -----------------------------------------------------------


class TestPerpsCandles:
    def test_passes_camel_case_variables(self, httpserver: HTTPServer) -> None:
        """Each kwarg lands on the wire as the corresponding camelCase variable."""
        # The .graphql document declares variables `$pairId`, `$interval`,
        # `$earlierThan`, `$laterThan`, `$first`, `$after` — all camelCase.
        # We assert each one explicitly so a typo in the SDK's variables
        # dict would surface as a precise per-key failure.
        captured = _capture_request(
            httpserver,
            {"data": {"perpsCandles": {"nodes": [], "pageInfo": _empty_page_info()}}},
        )
        _info(httpserver).perps_candles(
            _DEMO_PAIR,
            CandleInterval.ONE_MINUTE,
            later_than="2025-01-01T00:00:00Z",
            earlier_than="2025-01-02T00:00:00Z",
            first=50,
            after="cursor-x",
        )
        variables = captured[0]["variables"]
        assert variables["pairId"] == _DEMO_PAIR
        assert variables["interval"] == "ONE_MINUTE"
        assert variables["laterThan"] == "2025-01-01T00:00:00Z"
        assert variables["earlierThan"] == "2025-01-02T00:00:00Z"
        assert variables["first"] == 50
        assert variables["after"] == "cursor-x"

    def test_unwraps_to_connection(self, httpserver: HTTPServer) -> None:
        """The response is unwrapped to a `Connection[PerpsCandle]` with snake_case PageInfo."""
        node: PerpsCandle = {
            "pairId": _DEMO_PAIR,
            "interval": "ONE_MINUTE",
            "minBlockHeight": 100,
            "maxBlockHeight": 110,
            "open": "65000.000000",
            "high": "66000.000000",
            "low": "64500.000000",
            "close": "65500.000000",
            "volume": "12.500000",
            "volumeUsd": "812500.000000",
            "timeStart": "2025-01-01T00:00:00Z",
            "timeStartUnix": 1735689600,
            "timeEnd": "2025-01-01T00:01:00Z",
            "timeEndUnix": 1735689660,
        }
        page_info = {
            "hasPreviousPage": False,
            "hasNextPage": True,
            "startCursor": "start-cur",
            "endCursor": "end-cur",
        }
        _capture_request(
            httpserver,
            {"data": {"perpsCandles": {"nodes": [node], "pageInfo": page_info}}},
        )
        result = _info(httpserver).perps_candles(_DEMO_PAIR, CandleInterval.ONE_MINUTE)
        # The returned `Connection` carries the candle node verbatim (camelCase
        # keys preserved) and a `PageInfo` translated to snake_case.
        assert result.nodes == [node]
        assert result.page_info == PageInfo(
            has_previous_page=False,
            has_next_page=True,
            start_cursor="start-cur",
            end_cursor="end-cur",
        )

    def test_interval_serializes_as_string_value(self, httpserver: HTTPServer) -> None:
        """`CandleInterval` passes its `.value` (e.g. "ONE_HOUR") on the wire."""
        # The GraphQL enum variable is typed `CandleInterval!`, expecting the
        # bare uppercase name. Without `.value` we'd serialize a Python repr
        # like `<CandleInterval.ONE_HOUR: 'ONE_HOUR'>`, which the indexer
        # would reject.
        captured = _capture_request(
            httpserver,
            {"data": {"perpsCandles": {"nodes": [], "pageInfo": _empty_page_info()}}},
        )
        _info(httpserver).perps_candles(_DEMO_PAIR, CandleInterval.ONE_HOUR)
        assert captured[0]["variables"]["interval"] == "ONE_HOUR"


# --- perps_events ------------------------------------------------------------


class TestPerpsEvents:
    def test_passes_filter_and_sort_variables(self, httpserver: HTTPServer) -> None:
        """All filter kwargs land on the wire as camelCase variables."""
        captured = _capture_request(
            httpserver,
            {"data": {"perpsEvents": {"nodes": [], "pageInfo": _empty_page_info()}}},
        )
        _info(httpserver).perps_events(
            user_addr=_DEMO_USER,
            event_type="OrderFilled",
            pair_id=_DEMO_PAIR,
            block_height=42,
            first=20,
            after="some-cursor",
            sort_by=PerpsEventSortBy.BLOCK_HEIGHT_ASC,
        )
        variables = captured[0]["variables"]
        # Wire keys are camelCase — see the `.graphql` variable declarations.
        assert variables["userAddr"] == _DEMO_USER
        assert variables["eventType"] == "OrderFilled"
        assert variables["pairId"] == _DEMO_PAIR
        assert variables["blockHeight"] == 42
        assert variables["first"] == 20
        assert variables["after"] == "some-cursor"
        assert variables["sortBy"] == "BLOCK_HEIGHT_ASC"

    def test_sort_by_default_is_block_height_desc(self, httpserver: HTTPServer) -> None:
        """Default `sort_by` mirrors the indexer default (`BLOCK_HEIGHT_DESC`)."""
        # Without an explicit kwarg, callers expect newest events first;
        # this pins that contract so a future refactor that flips the
        # default doesn't silently change semantics.
        captured = _capture_request(
            httpserver,
            {"data": {"perpsEvents": {"nodes": [], "pageInfo": _empty_page_info()}}},
        )
        _info(httpserver).perps_events()
        assert captured[0]["variables"]["sortBy"] == "BLOCK_HEIGHT_DESC"

    def test_unwraps_to_connection(self, httpserver: HTTPServer) -> None:
        """The response is unwrapped to a `Connection[PerpsEvent]`."""
        node: PerpsEvent = {
            "idx": 7,
            "blockHeight": 100,
            "txHash": "0xabc",
            "eventType": "OrderFilled",
            "userAddr": _DEMO_USER,
            "pairId": _DEMO_PAIR,
            "data": {"order_id": "42", "fill_price": "65000.000000"},
            "createdAt": "2025-01-01T00:00:00Z",
        }
        _capture_request(
            httpserver,
            {
                "data": {
                    "perpsEvents": {"nodes": [node], "pageInfo": _empty_page_info()},
                },
            },
        )
        result = _info(httpserver).perps_events()
        assert result.nodes == [node]
        # `data` is opaque — keys come straight off the wire (snake_case
        # for inner contract events, even though the outer event keys are
        # camelCase). Asserting on a snake_case nested key proves the
        # SDK does not auto-rename the payload.
        assert result.nodes[0]["data"]["order_id"] == "42"


# --- perps_pair_stats --------------------------------------------------------


class TestPerpsPairStats:
    def test_uses_snake_case_pair_id_variable(self, httpserver: HTTPServer) -> None:
        """The wire variable for this query is `pair_id` (snake_case), an anomaly."""
        # The vendored `perpsPairStats.graphql` declares its variable as
        # `$pair_id`, unlike sibling queries that use `$pairId`. We pin
        # that anomaly here so a regen of the document upstream that flips
        # the name will trip a test instead of silently breaking the wire.
        captured = _capture_request(
            httpserver,
            {
                "data": {
                    "perpsPairStats": {
                        "pairId": _DEMO_PAIR,
                        "currentPrice": "65000.000000",
                        "price24HAgo": "64000.000000",
                        "volume24H": "1000000.000000",
                        "priceChange24H": "0.015625",
                    },
                },
            },
        )
        _info(httpserver).perps_pair_stats(_DEMO_PAIR)
        assert captured[0]["variables"] == {"pair_id": _DEMO_PAIR}

    def test_returns_typed_dict(self, httpserver: HTTPServer) -> None:
        """The response is returned verbatim as a `PerpsPairStats` TypedDict."""
        payload = {
            "pairId": _DEMO_PAIR,
            "currentPrice": "65000.000000",
            "price24HAgo": "64000.000000",
            "volume24H": "1000000.000000",
            "priceChange24H": "0.015625",
        }
        _capture_request(httpserver, {"data": {"perpsPairStats": payload}})
        result = _info(httpserver).perps_pair_stats(_DEMO_PAIR)
        # camelCase keys are preserved (this is the indexer convention
        # boundary in action — no auto-renaming).
        assert result == payload


# --- all_perps_pair_stats ----------------------------------------------------


class TestAllPerpsPairStats:
    def test_returns_list(self, httpserver: HTTPServer) -> None:
        """`all_perps_pair_stats()` returns a flat list of PerpsPairStats dicts."""
        payload = [
            {
                "pairId": _DEMO_PAIR,
                "currentPrice": "65000.000000",
                "price24HAgo": "64000.000000",
                "volume24H": "1000000.000000",
                "priceChange24H": "0.015625",
            },
            {
                "pairId": "perp/ethusd",
                "currentPrice": None,
                "price24HAgo": None,
                "volume24H": "0.000000",
                "priceChange24H": None,
            },
        ]
        captured = _capture_request(httpserver, {"data": {"allPerpsPairStats": payload}})
        result = _info(httpserver).all_perps_pair_stats()
        # No variables: the document takes none. We still assert the empty
        # variables payload because the API helper sends `{}` (not `null`).
        assert captured[0]["variables"] == {}
        assert result == payload


# --- perps_events_all (paginating wrapper) -----------------------------------


class TestPerpsEventsAll:
    def test_walks_all_pages(self, httpserver: HTTPServer) -> None:
        """`perps_events_all` chains paginated `perps_events` calls into one iterator."""
        # We expect exactly two fetches: page 1 returns `hasNextPage=true` and
        # an `endCursor`; page 2 returns `hasNextPage=false`. The helper
        # should walk from page 1 → page 2 and stop, yielding all four
        # events in order.
        page1_node: PerpsEvent = {
            "idx": 1,
            "blockHeight": 10,
            "txHash": "0x01",
            "eventType": "Deposited",
            "userAddr": _DEMO_USER,
            "pairId": _DEMO_PAIR,
            "data": {},
            "createdAt": "2025-01-01T00:00:00Z",
        }
        page1_node2: PerpsEvent = {**page1_node, "idx": 2, "txHash": "0x02"}
        page2_node: PerpsEvent = {**page1_node, "idx": 3, "txHash": "0x03"}
        page2_node2: PerpsEvent = {**page1_node, "idx": 4, "txHash": "0x04"}

        captured: list[dict[str, Any]] = []
        responses = iter(
            [
                {
                    "data": {
                        "perpsEvents": {
                            "nodes": [page1_node, page1_node2],
                            "pageInfo": {
                                "hasPreviousPage": False,
                                "hasNextPage": True,
                                "startCursor": None,
                                "endCursor": "cursor-page-1",
                            },
                        },
                    },
                },
                {
                    "data": {
                        "perpsEvents": {
                            "nodes": [page2_node, page2_node2],
                            "pageInfo": {
                                "hasPreviousPage": False,
                                "hasNextPage": False,
                                "startCursor": None,
                                "endCursor": None,
                            },
                        },
                    },
                },
            ],
        )

        def handler(request: Request) -> Response:
            captured.append(cast("dict[str, Any]", request.get_json()))
            return Response(json.dumps(next(responses)), mimetype="application/json")

        httpserver.expect_request("/graphql", method="POST").respond_with_handler(handler)

        events = list(_info(httpserver).perps_events_all(page_size=2))

        # All four events surface in the order the indexer returned them
        # (across both pages).
        assert [e["idx"] for e in events] == [1, 2, 3, 4]
        # The first request has no `after` cursor; the second uses the first
        # page's `endCursor`. Both requests forward `first=2` (the page_size
        # we passed in).
        assert captured[0]["variables"]["after"] is None
        assert captured[0]["variables"]["first"] == 2
        assert captured[1]["variables"]["after"] == "cursor-page-1"
        assert captured[1]["variables"]["first"] == 2

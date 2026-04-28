"""Tests for dango.hyperliquid_compatibility.info."""

from __future__ import annotations

from typing import Any, cast
from unittest.mock import patch

import pytest

from dango.hyperliquid_compatibility.info import (
    Info,
    _coin_to_pair_id,
    _dedupe_fills,
    _hl_interval_to_dango,
    _isotime_to_ms,
    _ms_to_iso_str,
    _pair_id_to_coin,
    _reshape_candle_to_hl,
    _reshape_fill_to_hl,
    _reshape_l2_to_hl,
    _reshape_order_to_hl,
    _reshape_pair_state_to_perp_ctx,
    _reshape_user_state_to_hl,
    _timestamp_ns_to_ms,
)
from dango.utils.types import Addr, CandleInterval, PairId


# A new lightweight fake — separate from the FakeInfo in `_helpers.py`
# because that one is shaped for Exchange's needs (simulate /
# broadcast_tx_sync), whereas the HL-compat Info wrapper requires the
# perps-side query primitives (`pair_params`, `pair_states`,
# `liquidity_depth`, `user_state_extended`, `orders_by_user`, `order`,
# `all_perps_pair_stats`, `perps_events_all`, `subscribe_*`).
class _FakeNativeInfo:
    """Fake of `dango.info.Info` returning canned native-shaped responses."""

    def __init__(self) -> None:
        self.disconnected = False
        self.unsubscribed: list[int] = []
        # Each subscription helper appends `(method_name, args, callback)`
        # so tests can both pin the dispatch and trigger the callback to
        # verify the reshape.
        self.subscriptions: list[tuple[str, tuple[Any, ...], Any]] = []
        self._next_id = 0
        # Default canned data; tests overwrite as needed.
        self.pair_params_data: dict[str, dict[str, Any]] = {
            "perp/btcusd": {"bucket_sizes": ["0.10000", "1.00000"]},
            "perp/ethusd": {"bucket_sizes": ["0.01000", "0.10000"]},
        }
        self.pair_states_data: dict[str, dict[str, Any]] = {
            "perp/btcusd": {
                "long_oi": "5.000000",
                "short_oi": "5.000000",
                "funding_per_unit": "0.000100",
                "funding_rate": "0.000100",
            },
            "perp/ethusd": {
                "long_oi": "100.000000",
                "short_oi": "100.000000",
                "funding_per_unit": "0.000050",
                "funding_rate": "0.000050",
            },
        }
        self.all_pair_stats_data: list[dict[str, Any]] = [
            {
                "pairId": "perp/btcusd",
                "currentPrice": "60000.000000",
                "price24HAgo": "59000.000000",
                "volume24H": "1000000.000000",
                "priceChange24H": "1000.000000",
            },
            {
                "pairId": "perp/ethusd",
                "currentPrice": "3000.000000",
                "price24HAgo": "2900.000000",
                "volume24H": "500000.000000",
                "priceChange24H": "100.000000",
            },
        ]
        self.user_state_data: dict[str, Any] | None = None
        self.orders_by_user_data: dict[str, dict[str, Any]] = {}
        self.order_data: dict[str, dict[str, Any]] = {}
        self.liquidity_depth_data: dict[str, Any] = {}
        self.perps_events_data: list[dict[str, Any]] = []
        # `perps_contract` is read by the polling subscriptions; provide
        # a stable value so the request shape is deterministic.
        self.perps_contract = Addr("0xabc")
        self.skip_ws = False

    # --- read primitives -----------------------------------------------

    def pair_params(self) -> dict[str, dict[str, Any]]:
        return self.pair_params_data

    def pair_param(self, pair_id: str) -> dict[str, Any] | None:
        return self.pair_params_data.get(pair_id)

    def pair_states(self) -> dict[str, dict[str, Any]]:
        return self.pair_states_data

    def pair_state(self, pair_id: str) -> dict[str, Any] | None:
        return self.pair_states_data.get(pair_id)

    def all_perps_pair_stats(self) -> list[dict[str, Any]]:
        return self.all_pair_stats_data

    def liquidity_depth(
        self, pair_id: str, *, bucket_size: str, limit: int | None = None
    ) -> dict[str, Any]:
        # Echo args for assertion.
        self.last_liquidity_depth_call = (pair_id, bucket_size, limit)
        return self.liquidity_depth_data

    def user_state_extended(self, user: str, **_: Any) -> dict[str, Any] | None:
        return self.user_state_data

    def orders_by_user(self, user: str) -> dict[str, dict[str, Any]]:
        return self.orders_by_user_data

    def order(self, order_id: str) -> dict[str, Any] | None:
        return self.order_data.get(order_id)

    def perps_events_all(self, **kwargs: Any) -> list[dict[str, Any]]:
        # Filter by the `event_type` kwarg if given, else return all.
        event_type = kwargs.get("event_type")
        if event_type is None:
            return list(self.perps_events_data)
        return [ev for ev in self.perps_events_data if ev.get("eventType") == event_type]

    def perps_candles(self, pair_id: str, interval: Any, **kwargs: Any) -> Any:
        from dango.utils.types import Connection, PageInfo

        # Echo args via `last_perps_candles_call` so tests can pin them.
        self.last_perps_candles_call = (pair_id, interval, kwargs)
        return Connection(
            nodes=getattr(self, "perps_candles_nodes", []),
            page_info=PageInfo(
                has_previous_page=False,
                has_next_page=False,
                start_cursor=None,
                end_cursor=None,
            ),
        )

    # --- subscription primitives --------------------------------------

    def subscribe_perps_trades(self, pair_id: str, callback: Any) -> int:
        self._next_id += 1
        self.subscriptions.append(("perps_trades", (pair_id,), callback))
        return self._next_id

    def subscribe_perps_candles(self, pair_id: str, interval: Any, callback: Any) -> int:
        self._next_id += 1
        self.subscriptions.append(("perps_candles", (pair_id, interval), callback))
        return self._next_id

    def subscribe_user_events(
        self, user: str, callback: Any, *, event_types: list[str] | None = None
    ) -> int:
        self._next_id += 1
        self.subscriptions.append(("user_events", (user, event_types), callback))
        return self._next_id

    def subscribe_query_app(self, request: Any, callback: Any, *, block_interval: int = 10) -> int:
        self._next_id += 1
        self.subscriptions.append(("query_app", (request, block_interval), callback))
        return self._next_id

    def unsubscribe(self, subscription_id: int) -> bool:
        self.unsubscribed.append(subscription_id)
        return True

    def disconnect_websocket(self) -> None:
        self.disconnected = True


def _make_info(
    fake: _FakeNativeInfo | None = None,
    **kwargs: Any,
) -> tuple[Info, _FakeNativeInfo]:
    """Build an HL-compat Info with a fake native Info wired in."""

    fake = fake or _FakeNativeInfo()
    # Patch the lazily-imported `Info` class inside the module under
    # test so the constructor wires up the fake instead of a real
    # native Info. This avoids ever touching the network in tests.
    import dango.info

    with patch.object(dango.info, "Info", return_value=fake):
        info = Info(base_url="http://test", **kwargs)
    return info, fake


# --- Pair-name helpers ------------------------------------------------------


class TestPairNameHelpers:
    def test_pair_id_to_coin_strips_prefix_and_suffix(self) -> None:
        """`_pair_id_to_coin` returns uppercase coin name."""

        assert _pair_id_to_coin("perp/btcusd") == "BTC"
        assert _pair_id_to_coin("perp/ethusd") == "ETH"

    def test_pair_id_to_coin_handles_missing_prefix(self) -> None:
        """Missing perp/ prefix is no-op'd; suffix still stripped."""

        assert _pair_id_to_coin("btcusd") == "BTC"

    def test_coin_to_pair_id_roundtrip(self) -> None:
        """`_coin_to_pair_id` produces the inverse of `_pair_id_to_coin`."""

        assert _coin_to_pair_id("BTC") == "perp/btcusd"
        assert _pair_id_to_coin(_coin_to_pair_id("ETH")) == "ETH"


# --- Interval mapping -------------------------------------------------------


class TestIntervalMapping:
    def test_supported_intervals_resolve(self) -> None:
        """Every Dango-backed HL interval string maps to the right enum."""

        cases = {
            "1m": CandleInterval.ONE_MINUTE,
            "5m": CandleInterval.FIVE_MINUTES,
            "15m": CandleInterval.FIFTEEN_MINUTES,
            "1h": CandleInterval.ONE_HOUR,
            "4h": CandleInterval.FOUR_HOURS,
            "1d": CandleInterval.ONE_DAY,
            "1w": CandleInterval.ONE_WEEK,
        }
        for hl_value, dango_value in cases.items():
            assert _hl_interval_to_dango(hl_value) == dango_value

    def test_unsupported_interval_raises(self) -> None:
        """HL intervals Dango doesn't support raise ValueError."""

        with pytest.raises(ValueError, match="unsupported candle interval '3m'"):
            _hl_interval_to_dango("3m")

    def test_unsupported_interval_message_lists_supported(self) -> None:
        """The error message should help the caller pick a valid interval."""

        with pytest.raises(ValueError, match="1m"):
            _hl_interval_to_dango("30m")


# --- Timestamp helpers ------------------------------------------------------


class TestTimestampHelpers:
    def test_timestamp_ns_to_ms_normal(self) -> None:
        """A ns timestamp string converts cleanly to ms int."""

        assert _timestamp_ns_to_ms("1700000000000000000") == 1_700_000_000_000

    def test_timestamp_ns_to_ms_none(self) -> None:
        """None input → 0."""

        assert _timestamp_ns_to_ms(None) == 0

    def test_timestamp_ns_to_ms_invalid(self) -> None:
        """Garbage strings degrade to 0 rather than raising."""

        assert _timestamp_ns_to_ms("not-a-number") == 0

    def test_isotime_to_ms_normal(self) -> None:
        """ISO-8601 with Z suffix parses to ms."""

        assert _isotime_to_ms("2024-01-01T00:00:00Z") == 1_704_067_200_000

    def test_isotime_to_ms_offset(self) -> None:
        """ISO-8601 with +00:00 offset parses to ms."""

        assert _isotime_to_ms("2024-01-01T00:00:00+00:00") == 1_704_067_200_000

    def test_isotime_to_ms_none(self) -> None:
        """None input → 0."""

        assert _isotime_to_ms(None) == 0

    def test_ms_to_iso_str(self) -> None:
        """ms int → ISO 8601 UTC string for indexer DateTime shape."""

        # 1700000000123 ms = 2023-11-14T22:13:20.123Z (UTC). Round-trips
        # to ms precision; the trailing `.123Z` is what the indexer
        # parser wants.
        assert _ms_to_iso_str(1_700_000_000_123) == "2023-11-14T22:13:20.123Z"


# --- Reshape helpers --------------------------------------------------------


class TestReshapeUserState:
    def test_none_user_state_returns_zero_envelope(self) -> None:
        """No user state on chain → HL zero-margin envelope."""

        result = _reshape_user_state_to_hl(None, {})
        assert result["assetPositions"] == []
        assert result["withdrawable"] == "0"
        assert result["marginSummary"] == result["crossMarginSummary"]
        assert result["marginSummary"]["accountValue"] == "0"

    def test_populated_user_state_shape(self) -> None:
        """A populated state surfaces assetPositions and margin summaries."""

        state: Any = {
            "margin": "1000.000000",
            "vault_shares": "0",
            "unlocks": [],
            "reserved_margin": "100.000000",
            "open_order_count": 0,
            "equity": "1100.000000",
            "available_margin": "900.000000",
            "maintenance_margin": "50.000000",
            "positions": {
                PairId("perp/btcusd"): {
                    "size": "0.500000",
                    "entry_price": "60000.000000",
                    "entry_funding_per_unit": "0.000000",
                    "conditional_order_above": None,
                    "conditional_order_below": None,
                    "unrealized_pnl": "100.000000",
                    "unrealized_funding": "0.000000",
                    "liquidation_price": "55000.000000",
                }
            },
        }
        result = _reshape_user_state_to_hl(state, {})
        assert len(result["assetPositions"]) == 1
        position = result["assetPositions"][0]
        assert position["type"] == "oneWay"
        assert position["position"]["coin"] == "BTC"
        assert position["position"]["szi"] == "0.5"
        assert position["position"]["entryPx"] == "60000"
        assert position["position"]["unrealizedPnl"] == "100"
        assert position["position"]["liquidationPx"] == "55000"
        assert position["position"]["leverage"] == {"type": "cross", "value": 1}
        # Position value: 0.5 * 60000 = 30000.
        assert position["position"]["positionValue"] == "30000"
        assert result["withdrawable"] == "900"
        assert result["marginSummary"]["accountValue"] == "1100"
        assert result["marginSummary"]["totalMarginUsed"] == "100"

    def test_short_position_negative_size(self) -> None:
        """A short position keeps szi negative on the wire."""

        state: Any = {
            "margin": "1000.000000",
            "vault_shares": "0",
            "unlocks": [],
            "reserved_margin": "0",
            "open_order_count": 0,
            "equity": "1000.000000",
            "available_margin": "1000.000000",
            "maintenance_margin": "0",
            "positions": {
                PairId("perp/ethusd"): {
                    "size": "-2.000000",
                    "entry_price": "3000.000000",
                    "entry_funding_per_unit": "0.000000",
                    "conditional_order_above": None,
                    "conditional_order_below": None,
                    "unrealized_pnl": None,
                    "unrealized_funding": None,
                    "liquidation_price": None,
                }
            },
        }
        result = _reshape_user_state_to_hl(state, {})
        position = result["assetPositions"][0]
        assert position["position"]["szi"] == "-2"
        assert position["position"]["liquidationPx"] is None


class TestReshapeOrder:
    def test_buy_order_shape(self) -> None:
        """A positive-size order becomes side=B, sz=|size|."""

        order: Any = {
            "pair_id": "perp/btcusd",
            "size": "0.500000",
            "limit_price": "59000.000000",
            "reduce_only": False,
            "reserved_margin": "100.000000",
            "created_at": "1700000000000000000",
        }
        result = _reshape_order_to_hl(order, "42")
        assert result["coin"] == "BTC"
        assert result["side"] == "B"
        assert result["limitPx"] == "59000"
        assert result["sz"] == "0.5"
        assert result["oid"] == "42"
        assert result["timestamp"] == 1_700_000_000_000
        assert result["origSz"] == "0.5"

    def test_sell_order_shape(self) -> None:
        """Negative-size order becomes side=A, sz=|size|."""

        order: Any = {
            "pair_id": "perp/ethusd",
            "size": "-1.250000",
            "limit_price": "3100.000000",
            "reduce_only": False,
            "reserved_margin": "0.000000",
            "created_at": "1700000000000000000",
        }
        result = _reshape_order_to_hl(order, "100")
        assert result["coin"] == "ETH"
        assert result["side"] == "A"
        assert result["sz"] == "1.25"


class TestReshapeFill:
    def test_buy_fill_with_open(self) -> None:
        """An opening long fill maps to side=B, dir='Open Long'."""

        event: Any = {
            "idx": 1,
            "blockHeight": 100,
            "txHash": "0xdeadbeef",
            "eventType": "order_filled",
            "userAddr": "0xuser",
            "pairId": "perp/btcusd",
            "data": {
                "order_id": "1",
                "pair_id": "perp/btcusd",
                "user": "0xuser",
                "fill_price": "60000.000000",
                "fill_size": "0.500000",
                "closing_size": "0.000000",
                "opening_size": "0.500000",
                "realized_pnl": "0.000000",
                "fee": "1.000000",
                "client_order_id": None,
                "fill_id": "fill-1",
                "is_maker": False,
            },
            "createdAt": "2024-01-01T00:00:00Z",
        }
        fill = _reshape_fill_to_hl(event)
        assert fill["coin"] == "BTC"
        assert fill["side"] == "B"
        assert fill["dir"] == "Open Long"
        assert fill["px"] == "60000"
        assert fill["sz"] == "0.5"
        assert fill["closedPnl"] == "0"
        assert fill["fee"] == "1"
        assert fill["hash"] == "0xdeadbeef"
        # `oid` on the Fill TypedDict is annotated `int`, but Dango
        # OrderIds are strings — cast to Any in the test to pin the
        # actual on-the-wire value rather than the annotation.
        assert cast("Any", fill)["oid"] == "1"
        assert fill["feeToken"] == "USDC"
        assert fill["crossed"] is True

    def test_sell_fill_close_long(self) -> None:
        """A closing fill on a long position maps to dir='Close Long'."""

        event: Any = {
            "idx": 2,
            "blockHeight": 200,
            "txHash": "0xabc",
            "eventType": "order_filled",
            "userAddr": "0xuser",
            "pairId": "perp/btcusd",
            "data": {
                "order_id": "5",
                "pair_id": "perp/btcusd",
                "user": "0xuser",
                "fill_price": "61000.000000",
                "fill_size": "-0.500000",
                "closing_size": "0.500000",
                "opening_size": "0.000000",
                "realized_pnl": "500.000000",
                "fee": "1.000000",
                "client_order_id": None,
                "fill_id": "fill-2",
                "is_maker": False,
            },
            "createdAt": "2024-01-02T00:00:00Z",
        }
        fill = _reshape_fill_to_hl(event)
        assert fill["side"] == "A"
        assert fill["dir"] == "Close Long"
        assert fill["closedPnl"] == "500"


class TestDedupeFills:
    def test_keeps_taker_drops_maker(self) -> None:
        """Paired maker+taker rows collapse to a single taker fill."""

        events: list[Any] = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "0.500000",
                    "fill_id": "F1",
                    "is_maker": True,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
            {
                "idx": 2,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "2",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F1",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ]
        fills = _dedupe_fills(events)
        assert len(fills) == 1
        # Taker fee was 1.0; maker would have been 0.5.
        assert fills[0]["fee"] == "1"

    def test_keeps_unique_fill_ids(self) -> None:
        """Distinct fill_ids each survive."""

        events: list[Any] = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F1",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
            {
                "idx": 2,
                "blockHeight": 101,
                "txHash": "0xb",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F2",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:01Z",
            },
        ]
        fills = _dedupe_fills(events)
        assert len(fills) == 2


class TestReshapeL2:
    def test_levels_are_sorted_correctly(self) -> None:
        """Bids descend, asks ascend by numeric price."""

        depth = {
            "bids": {
                "59000.000000": {"size": "1.000000", "notional": "59000.000000"},
                "59500.000000": {"size": "2.000000", "notional": "119000.000000"},
            },
            "asks": {
                "60500.000000": {"size": "0.500000", "notional": "30250.000000"},
                "60000.000000": {"size": "1.000000", "notional": "60000.000000"},
            },
        }
        result = _reshape_l2_to_hl(depth, "BTC", time_ms=12345)
        assert result["coin"] == "BTC"
        assert result["time"] == 12345
        bids, asks = result["levels"]
        # Best bid first → highest price first.
        assert bids[0]["px"] == "59500"
        assert bids[1]["px"] == "59000"
        assert asks[0]["px"] == "60000"
        assert asks[1]["px"] == "60500"
        # Default `n=1` for every level (Dango doesn't expose count).
        assert all(level["n"] == 1 for level in bids + asks)

    def test_empty_sides(self) -> None:
        """Missing bids/asks default to empty lists."""

        result = _reshape_l2_to_hl({}, "BTC", time_ms=0)
        assert result["levels"] == ([], [])


class TestReshapeCandle:
    def test_full_shape(self) -> None:
        """All HL single-letter keys are present and converted."""

        candle: Any = {
            "pairId": "perp/btcusd",
            "interval": "ONE_MINUTE",
            "minBlockHeight": 100,
            "maxBlockHeight": 200,
            "open": "60000.000000",
            "high": "60500.000000",
            "low": "59500.000000",
            "close": "60100.000000",
            "volume": "10.000000",
            "volumeUsd": "601000.000000",
            "timeStart": "2024-01-01T00:00:00Z",
            "timeStartUnix": 1_704_067_200_000,
            "timeEnd": "2024-01-01T00:01:00Z",
            "timeEndUnix": 1_704_067_260_000,
        }
        result = _reshape_candle_to_hl(candle)
        assert result["t"] == 1_704_067_200_000
        assert result["T"] == 1_704_067_260_000
        assert result["s"] == "BTC"
        assert result["i"] == "ONE_MINUTE"
        assert result["o"] == "60000"
        assert result["c"] == "60100"
        assert result["h"] == "60500"
        assert result["l"] == "59500"
        assert result["v"] == "10"
        assert result["n"] == 0


class TestReshapePerpCtx:
    def test_with_state_and_stats(self) -> None:
        """A populated ctx pulls funding/OI from state, prices from stats."""

        pair_state: Any = {
            "long_oi": "5.000000",
            "short_oi": "5.000000",
            "funding_per_unit": "0.000100",
            "funding_rate": "0.000200",
        }
        pair_stats: Any = {
            "pairId": "perp/btcusd",
            "currentPrice": "60000.000000",
            "price24HAgo": "59000.000000",
            "volume24H": "1000000.000000",
            "priceChange24H": "1000.000000",
        }
        ctx = _reshape_pair_state_to_perp_ctx("perp/btcusd", pair_state, pair_stats)
        assert ctx["funding"] == "0.0002"
        assert ctx["openInterest"] == "5"
        assert ctx["markPx"] == "60000"
        assert ctx["oraclePx"] == "60000"
        assert ctx["midPx"] == "60000"
        assert ctx["dayNtlVlm"] == "1000000"
        assert ctx["prevDayPx"] == "59000"
        # day base volume = volume24H / currentPrice.
        # 1000000 / 60000 = 16.6667 (rounded to 6 decimals).
        assert ctx["dayBaseVlm"].startswith("16.")

    def test_no_stats(self) -> None:
        """When pair_stats is missing, prices default to 0 / midPx None."""

        pair_state: Any = {
            "long_oi": "5.000000",
            "short_oi": "5.000000",
            "funding_per_unit": "0.000100",
            "funding_rate": "0.000200",
        }
        ctx = _reshape_pair_state_to_perp_ctx("perp/btcusd", pair_state, None)
        assert ctx["funding"] == "0.0002"
        assert ctx["midPx"] is None
        assert ctx["markPx"] == "0"


# --- Info construction & resolver ------------------------------------------


class TestInfoConstruction:
    def test_default_resolver_from_pair_params(self) -> None:
        """The resolver is populated from native `pair_params` at construction."""

        info, fake = _make_info()
        # Two pairs in the canned data → two coins in the resolver.
        assert "BTC" in info.coin_to_asset
        assert "ETH" in info.coin_to_asset
        assert info.coin_to_pair["BTC"] == "perp/btcusd"
        assert info.coin_to_pair["ETH"] == "perp/ethusd"
        # `name_to_coin` is identity for Dango (no vault aliases).
        assert info.name_to_coin == {"BTC": "BTC", "ETH": "ETH"}
        # All assets share the same szDecimals (Dango is uniform 6).
        assert all(d == 6 for d in info.asset_to_sz_decimals.values())
        # Asset indexes are dense and 0-based.
        assert sorted(info.coin_to_asset.values()) == [0, 1]

    def test_resolver_from_meta(self) -> None:
        """Passing `meta` to the constructor bypasses the live fetch."""

        meta: Any = {
            "universe": [
                {"name": "ETH", "szDecimals": 4},
                {"name": "BTC", "szDecimals": 5},
            ]
        }
        info, _fake = _make_info(meta=meta)
        # Order from `meta` is preserved, not alphabetical.
        assert info.coin_to_asset == {"ETH": 0, "BTC": 1}
        # szDecimals is taken from meta verbatim.
        assert info.asset_to_sz_decimals == {0: 4, 1: 5}

    def test_name_to_pair_unknown_raises(self) -> None:
        """Asking for a pair that doesn't exist raises KeyError."""

        info, _fake = _make_info()
        with pytest.raises(KeyError):
            info.name_to_pair("UNKNOWN")

    def test_name_to_asset_lookup(self) -> None:
        """`name_to_asset` returns the integer asset index."""

        info, _fake = _make_info()
        assert info.name_to_asset("BTC") == info.coin_to_asset["BTC"]


# --- Implemented read methods ----------------------------------------------


class TestUserState:
    def test_passes_address_to_native(self) -> None:
        """`user_state` calls native `user_state_extended` with an Addr."""

        info, fake = _make_info()
        fake.user_state_data = None  # No state on chain.
        result = info.user_state("0xuser")
        assert result["assetPositions"] == []
        assert result["withdrawable"] == "0"

    def test_populated_state(self) -> None:
        """A populated user state surfaces through the reshape."""

        info, fake = _make_info()
        fake.user_state_data = {
            "margin": "1000.000000",
            "vault_shares": "0",
            "unlocks": [],
            "reserved_margin": "0.000000",
            "open_order_count": 0,
            "equity": "1100.000000",
            "available_margin": "1000.000000",
            "maintenance_margin": "0.000000",
            "positions": {},
        }
        result = info.user_state("0xuser")
        assert result["marginSummary"]["accountValue"] == "1100"


class TestOpenOrders:
    def test_flattens_dict_to_list(self) -> None:
        """`open_orders` returns a list, one entry per resting order."""

        info, fake = _make_info()
        fake.orders_by_user_data = {
            "11": {
                "pair_id": "perp/btcusd",
                "size": "0.500000",
                "limit_price": "59000.000000",
                "reduce_only": False,
                "reserved_margin": "100.000000",
                "created_at": "1700000000000000000",
            },
            "12": {
                "pair_id": "perp/ethusd",
                "size": "-1.000000",
                "limit_price": "3100.000000",
                "reduce_only": False,
                "reserved_margin": "0.000000",
                "created_at": "1700000000000000000",
            },
        }
        result = info.open_orders("0xuser")
        assert len(result) == 2
        oids = {row["oid"] for row in result}
        assert oids == {"11", "12"}


class TestAllMids:
    def test_returns_coin_to_mid_map(self) -> None:
        """`all_mids` returns coin → currentPrice as HL strings."""

        info, _fake = _make_info()
        result = info.all_mids()
        assert result == {"BTC": "60000", "ETH": "3000"}

    def test_handles_null_current_price(self) -> None:
        """Null currentPrice degrades to '0' rather than crashing."""

        info, fake = _make_info()
        fake.all_pair_stats_data = [
            {
                "pairId": "perp/btcusd",
                "currentPrice": None,
                "price24HAgo": None,
                "volume24H": "0",
                "priceChange24H": None,
            }
        ]
        result = info.all_mids()
        assert result == {"BTC": "0"}


class TestMeta:
    def test_universe_is_synthesized(self) -> None:
        """`meta` synthesizes universe from pair_params with uniform szDecimals."""

        info, _fake = _make_info()
        result = info.meta()
        names = {asset["name"] for asset in result["universe"]}
        assert names == {"BTC", "ETH"}
        assert all(asset["szDecimals"] == 6 for asset in result["universe"])


class TestMetaAndAssetCtxs:
    def test_returns_meta_and_ctxs(self) -> None:
        """`meta_and_asset_ctxs` returns the [meta, ctxs] pair."""

        info, _fake = _make_info()
        result = info.meta_and_asset_ctxs()
        assert isinstance(result, list)
        assert len(result) == 2
        meta, ctxs = result
        assert {asset["name"] for asset in meta["universe"]} == {"BTC", "ETH"}
        assert len(ctxs) == 2
        # Find BTC ctx
        btc_ctx = next(
            c for c, p in zip(ctxs, meta["universe"], strict=False) if p["name"] == "BTC"
        )
        assert btc_ctx["markPx"] == "60000"


class TestL2Snapshot:
    def test_uses_smallest_bucket_size(self) -> None:
        """`l2_snapshot` picks the smallest bucket_size from pair_param."""

        info, fake = _make_info()
        fake.liquidity_depth_data = {
            "bids": {"59000.000000": {"size": "1.0", "notional": "59000.0"}},
            "asks": {"60000.000000": {"size": "1.0", "notional": "60000.0"}},
        }
        info.l2_snapshot("BTC")
        # The smallest is "0.10000".
        assert fake.last_liquidity_depth_call == ("perp/btcusd", "0.10000", None)

    def test_reshape(self) -> None:
        """`l2_snapshot` returns HL L2 shape."""

        info, fake = _make_info()
        fake.liquidity_depth_data = {
            "bids": {"59000.000000": {"size": "1.000000", "notional": "59000.0"}},
            "asks": {"60000.000000": {"size": "1.000000", "notional": "60000.0"}},
        }
        result = info.l2_snapshot("BTC")
        assert result["coin"] == "BTC"
        assert result["levels"][0][0]["px"] == "59000"
        assert result["levels"][1][0]["px"] == "60000"

    def test_missing_bucket_sizes_raises(self) -> None:
        """If pair_param has no bucket_sizes, l2_snapshot raises."""

        info, fake = _make_info()
        fake.pair_params_data["perp/btcusd"] = {"bucket_sizes": []}
        with pytest.raises(RuntimeError, match="bucket_sizes"):
            info.l2_snapshot("BTC")


class TestCandlesSnapshot:
    def test_passes_interval_and_time_bounds(self) -> None:
        """`candles_snapshot` translates HL interval and ms times to Dango."""

        info, fake = _make_info()
        fake.perps_candles_nodes = []  # type: ignore[attr-defined]
        info.candles_snapshot("BTC", "1m", 1_700_000_000_000, 1_700_000_060_000)
        pair_id, interval, kwargs = fake.last_perps_candles_call
        assert pair_id == "perp/btcusd"
        assert interval == CandleInterval.ONE_MINUTE
        # Indexer's `laterThan` / `earlierThan` are GraphQL DateTime
        # scalars; we forward the ms inputs as ISO 8601 UTC strings.
        assert kwargs["later_than"] == "2023-11-14T22:13:20.000Z"
        assert kwargs["earlier_than"] == "2023-11-14T22:14:20.000Z"

    def test_unsupported_interval_raises(self) -> None:
        """An HL interval Dango doesn't have raises ValueError."""

        info, _fake = _make_info()
        with pytest.raises(ValueError, match="unsupported"):
            info.candles_snapshot("BTC", "30m", 0, 1)

    def test_reshape(self) -> None:
        """`candles_snapshot` returns reshaped HL candles."""

        info, fake = _make_info()
        fake.perps_candles_nodes = [  # type: ignore[attr-defined]
            {
                "pairId": "perp/btcusd",
                "interval": "ONE_MINUTE",
                "minBlockHeight": 100,
                "maxBlockHeight": 200,
                "open": "60000.000000",
                "high": "60500.000000",
                "low": "59500.000000",
                "close": "60100.000000",
                "volume": "10.000000",
                "volumeUsd": "601000.000000",
                "timeStart": "2024-01-01T00:00:00Z",
                "timeStartUnix": 1_704_067_200_000,
                "timeEnd": "2024-01-01T00:01:00Z",
                "timeEndUnix": 1_704_067_260_000,
            }
        ]
        result = info.candles_snapshot("BTC", "1m", 0, 1)
        assert len(result) == 1
        assert result[0]["s"] == "BTC"


class TestUserFills:
    def test_dedupes_by_fill_id(self) -> None:
        """Maker+taker rows with same fill_id collapse to one taker fill."""

        info, fake = _make_info()
        fake.perps_events_data = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "0.500000",
                    "fill_id": "F1",
                    "is_maker": True,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
            {
                "idx": 2,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "2",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F1",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ]
        fills = info.user_fills("0xuser")
        assert len(fills) == 1
        assert fills[0]["fee"] == "1"


class TestUserFillsByTime:
    def test_filters_by_time(self) -> None:
        """`user_fills_by_time` drops events outside [start, end]."""

        # The native `perps_events_all` default-sorts BLOCK_HEIGHT_DESC, so
        # newer events come first; the early-break in `user_fills_by_time`
        # depends on this ordering. Fixture mirrors the real wire shape:
        # MID (newer) first, then EARLY (older).
        info, fake = _make_info()
        fake.perps_events_data = [
            {
                "idx": 2,
                "blockHeight": 200,
                "txHash": "0xb",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "2",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "MID",
                    "is_maker": False,
                },
                "createdAt": "2024-01-02T00:00:00Z",  # 1704153600000 ms
            },
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "0.500000",
                    "closing_size": "0.000000",
                    "opening_size": "0.500000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "EARLY",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",  # 1704067200000 ms
            },
        ]
        # Window contains only MID. The early-break should also short-
        # circuit before EARLY is considered (it's older than `start`).
        fills = info.user_fills_by_time("0xuser", start=1_704_153_600_000, end=1_704_240_000_000)
        assert len(fills) == 1


class TestQueryOrderByOid:
    def test_known_order(self) -> None:
        """A known order returns `{status: 'order', order: ...}`."""

        info, fake = _make_info()
        fake.order_data["1"] = {
            "pair_id": "perp/btcusd",
            "size": "0.500000",
            "limit_price": "59000.000000",
            "reduce_only": False,
            "reserved_margin": "100.000000",
            "created_at": "1700000000000000000",
        }
        result = info.query_order_by_oid("0xuser", 1)
        assert result["status"] == "order"
        assert result["order"]["coin"] == "BTC"
        assert result["order"]["oid"] == "1"

    def test_unknown_order(self) -> None:
        """An unknown oid returns `{status: 'unknownOid'}`."""

        info, _fake = _make_info()
        result = info.query_order_by_oid("0xuser", 999)
        assert result == {"status": "unknownOid"}


class TestHistoricalOrders:
    def test_combines_persisted_and_removed(self) -> None:
        """`historical_orders` zips persisted+removed events into rows."""

        info, fake = _make_info()
        fake.perps_events_data = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_persisted",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "user": "0xuser",
                    "limit_price": "59000.000000",
                    "size": "0.500000",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
            {
                "idx": 2,
                "blockHeight": 200,
                "txHash": "0xb",
                "eventType": "order_removed",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "user": "0xuser",
                    "reason": "filled",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-02T00:00:00Z",
            },
        ]
        rows = info.historical_orders("0xuser")
        assert len(rows) == 1
        assert rows[0]["status"] == "filled"
        assert rows[0]["order"]["coin"] == "BTC"
        assert rows[0]["statusTimestamp"] > rows[0]["order"]["timestamp"]

    def test_persisted_without_removed_is_open(self) -> None:
        """A persisted-only order is reported as `status='open'`."""

        # Coverage for the branch where an order is still resting in the
        # book at query time — no removed event has fired yet.
        info, fake = _make_info()
        fake.perps_events_data = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_persisted",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "7",
                    "pair_id": "perp/btcusd",
                    "user": "0xuser",
                    "limit_price": "59000.000000",
                    "size": "0.500000",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
        ]
        rows = info.historical_orders("0xuser")
        assert len(rows) == 1
        assert rows[0]["status"] == "open"
        assert rows[0]["order"]["oid"] == "7"

    def test_removed_with_canceled_reason(self) -> None:
        """An order_removed event with reason='canceled' surfaces status='canceled'."""

        info, fake = _make_info()
        fake.perps_events_data = [
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_persisted",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "9",
                    "pair_id": "perp/btcusd",
                    "user": "0xuser",
                    "limit_price": "59000.000000",
                    "size": "0.500000",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            },
            {
                "idx": 2,
                "blockHeight": 200,
                "txHash": "0xb",
                "eventType": "order_removed",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "9",
                    "pair_id": "perp/btcusd",
                    "user": "0xuser",
                    "reason": "canceled",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-02T00:00:00Z",
            },
        ]
        rows = info.historical_orders("0xuser")
        assert len(rows) == 1
        assert rows[0]["status"] == "canceled"


# --- NotImplementedError stubs ---------------------------------------------


class TestNotImplementedStubs:
    def _info(self) -> Info:
        info, _fake = _make_info()
        return info

    @pytest.mark.parametrize(
        ("method_name", "args"),
        [
            ("spot_user_state", ("addr",)),
            ("spot_meta", ()),
            ("spot_meta_and_asset_ctxs", ()),
            ("query_spot_deploy_auction_status", ("user",)),
            ("user_staking_summary", ("addr",)),
            ("user_staking_delegations", ("addr",)),
            ("user_staking_rewards", ("addr",)),
            ("delegator_history", ("user",)),
            ("query_user_to_multi_sig_signers", ("user",)),
            ("query_perp_deploy_auction_status", ()),
            ("query_user_dex_abstraction_state", ("user",)),
            ("query_user_abstraction_state", ("user",)),
            ("user_twap_slice_fills", ("user",)),
            ("portfolio", ("user",)),
            ("user_role", ("user",)),
            ("user_rate_limit", ("user",)),
            ("extra_agents", ("user",)),
            ("funding_history", ("BTC", 0)),
            ("user_funding_history", ("user", 0)),
            ("user_non_funding_ledger_updates", ("user", 0)),
            ("query_referral_state", ("user",)),
            ("query_sub_accounts", ("user",)),
            ("frontend_open_orders", ("user",)),
            ("user_fees", ("user",)),
            ("query_order_by_cloid", ("user", "0x" + "a" * 32)),
            ("user_vault_equities", ("user",)),
        ],
    )
    def test_stubs_raise_not_implemented(self, method_name: str, args: tuple[Any, ...]) -> None:
        """Every documented stub raises NotImplementedError."""

        info = self._info()
        with pytest.raises(NotImplementedError):
            getattr(info, method_name)(*args)


# --- Subscriptions --------------------------------------------------------


class TestSubscribeTrades:
    def test_dispatches_to_native_perps_trades(self) -> None:
        """A `trades` subscription routes to `subscribe_perps_trades`."""

        info, fake = _make_info()
        info.subscribe({"type": "trades", "coin": "BTC"}, lambda _: None)
        method, args, _cb = fake.subscriptions[-1]
        assert method == "perps_trades"
        assert args == ("perp/btcusd",)

    def test_dedupes_maker_drops_to_taker(self) -> None:
        """Maker trade is dropped; taker is reshaped and forwarded."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "trades", "coin": "BTC"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        # Send maker first — should NOT propagate.
        callback(
            {
                "orderId": "1",
                "fillPrice": "60000.000000",
                "fillSize": "1.000000",
                "fillId": "F1",
                "isMaker": True,
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        # Send taker — should propagate.
        callback(
            {
                "orderId": "2",
                "fillPrice": "60000.000000",
                "fillSize": "1.000000",
                "fillId": "F1",
                "isMaker": False,
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        # Send another taker with the same fillId — already seen, drop.
        callback(
            {
                "orderId": "3",
                "fillPrice": "60000.000000",
                "fillSize": "1.000000",
                "fillId": "F1",
                "isMaker": False,
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        assert len(received) == 1
        assert received[0]["coin"] == "BTC"
        assert received[0]["px"] == "60000"
        # `sz` regression guard: HL annotates `Trade.sz: int`, but the
        # wire form is a decimal string. Casting via `int(abs(0.5))`
        # would silently emit `sz=0`; pin the string output here.
        assert received[0]["sz"] == "1"
        # `hash` is HL's tx hash, which the indexer's perps-trade stream
        # does NOT expose. We emit empty rather than substitute orderId.
        assert received[0]["hash"] == ""

    def test_fractional_size_does_not_truncate(self) -> None:
        """A 0.5 BTC fill emits sz='0.5', not sz=0 (int-truncation regression guard)."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "trades", "coin": "BTC"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "orderId": "10",
                "fillPrice": "60000.000000",
                "fillSize": "0.500000",
                "fillId": "F2",
                "isMaker": False,
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        assert len(received) == 1
        assert received[0]["sz"] == "0.5"


class TestSubscribeCandle:
    def test_dispatches_to_native(self) -> None:
        """A `candle` subscription routes to `subscribe_perps_candles`."""

        info, fake = _make_info()
        info.subscribe(
            {"type": "candle", "coin": "BTC", "interval": "1m"},
            lambda _: None,
        )
        method, args, _cb = fake.subscriptions[-1]
        assert method == "perps_candles"
        assert args == ("perp/btcusd", CandleInterval.ONE_MINUTE)

    def test_callback_reshapes_candle(self) -> None:
        """Candle events flow through `_reshape_candle_to_hl`."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe(
            {"type": "candle", "coin": "BTC", "interval": "1m"},
            received.append,
        )
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "pairId": "perp/btcusd",
                "interval": "ONE_MINUTE",
                "minBlockHeight": 100,
                "maxBlockHeight": 200,
                "open": "60000.000000",
                "high": "60500.000000",
                "low": "59500.000000",
                "close": "60100.000000",
                "volume": "10.000000",
                "volumeUsd": "601000.000000",
                "timeStart": "2024-01-01T00:00:00Z",
                "timeStartUnix": 1_704_067_200_000,
                "timeEnd": "2024-01-01T00:01:00Z",
                "timeEndUnix": 1_704_067_260_000,
            }
        )
        assert received[0]["s"] == "BTC"


class TestSubscribeUserEvents:
    def test_wraps_into_user_envelope(self) -> None:
        """`userEvents` subscription wraps the fill in the HL envelope."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "userEvents", "user": "0xuser"}, received.append)
        method, args, callback = fake.subscriptions[-1]
        assert method == "user_events"
        # Subscription requested only `order_filled`.
        assert args[1] == ["order_filled"]
        callback(
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "1.000000",
                    "closing_size": "0.000000",
                    "opening_size": "1.000000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F1",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        assert received[0]["channel"] == "user"
        assert "fills" in received[0]["data"]
        assert len(received[0]["data"]["fills"]) == 1


class TestSubscribeUserFills:
    def test_uses_userfills_envelope(self) -> None:
        """`userFills` subscription emits the userFills envelope."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "userFills", "user": "0xuser"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_filled",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "1",
                    "pair_id": "perp/btcusd",
                    "fill_price": "60000.000000",
                    "fill_size": "1.000000",
                    "closing_size": "0.000000",
                    "opening_size": "1.000000",
                    "realized_pnl": "0.000000",
                    "fee": "1.000000",
                    "fill_id": "F1",
                    "is_maker": False,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        assert received[0]["channel"] == "userFills"
        assert received[0]["data"]["isSnapshot"] is False
        assert received[0]["data"]["user"] == "0xuser"


class TestSubscribeOrderUpdates:
    def test_routes_persisted_and_removed(self) -> None:
        """`orderUpdates` registers for both persisted and removed events."""

        info, fake = _make_info()
        info.subscribe({"type": "orderUpdates", "user": "0xuser"}, lambda _: None)
        _method, args, _cb = fake.subscriptions[-1]
        assert sorted(args[1]) == sorted(["order_persisted", "order_removed"])

    def test_persisted_status_is_open(self) -> None:
        """An `order_persisted` event surfaces as status='open'."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "orderUpdates", "user": "0xuser"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "idx": 1,
                "blockHeight": 100,
                "txHash": "0xa",
                "eventType": "order_persisted",
                "userAddr": "0xuser",
                "pairId": "perp/btcusd",
                "data": {
                    "order_id": "10",
                    "pair_id": "perp/btcusd",
                    "limit_price": "59000.000000",
                    "size": "0.500000",
                    "client_order_id": None,
                },
                "createdAt": "2024-01-01T00:00:00Z",
            }
        )
        assert received[0][0]["status"] == "open"
        assert received[0][0]["order"]["coin"] == "BTC"


class TestSubscribeL2Book:
    def test_dispatches_to_query_app_polling(self) -> None:
        """`l2Book` subscription routes to `subscribe_query_app`."""

        info, fake = _make_info()
        info.subscribe({"type": "l2Book", "coin": "BTC"}, lambda _: None)
        method, args, _cb = fake.subscriptions[-1]
        assert method == "query_app"
        # The polled request is `liquidity_depth` against the perps contract.
        request, block_interval = args
        assert block_interval == 1
        assert "liquidity_depth" in request["wasm_smart"]["msg"]

    def test_callback_reshapes_depth(self) -> None:
        """Polling responses are reshaped to HL L2 book."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "l2Book", "coin": "BTC"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "blockHeight": 7,
                "response": {
                    "bids": {"59000.000000": {"size": "1.000000", "notional": "59000.0"}},
                    "asks": {"60000.000000": {"size": "1.000000", "notional": "60000.0"}},
                },
            }
        )
        assert received[0]["coin"] == "BTC"
        assert received[0]["levels"][0][0]["px"] == "59000"
        assert received[0]["time"] == 7000


class TestSubscribeBbo:
    def test_uses_limit_one(self) -> None:
        """`bbo` polls liquidity_depth with limit=1."""

        info, fake = _make_info()
        info.subscribe({"type": "bbo", "coin": "BTC"}, lambda _: None)
        _method, args, _cb = fake.subscriptions[-1]
        request, _interval = args
        assert request["wasm_smart"]["msg"]["liquidity_depth"]["limit"] == 1

    def test_emits_top_levels(self) -> None:
        """The callback gets `{coin, time, bbo: (best_bid, best_ask)}`."""

        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe({"type": "bbo", "coin": "BTC"}, received.append)
        _method, _args, callback = fake.subscriptions[-1]
        callback(
            {
                "blockHeight": 5,
                "response": {
                    "bids": {"59000.000000": {"size": "1.000000", "notional": "59000.0"}},
                    "asks": {"60000.000000": {"size": "1.000000", "notional": "60000.0"}},
                },
            }
        )
        assert received[0]["coin"] == "BTC"
        bid, ask = received[0]["bbo"]
        assert bid["px"] == "59000"
        assert ask["px"] == "60000"


class TestSubscribeAllMids:
    def test_raises_not_implemented(self) -> None:
        """allMids subscription is currently not implemented."""

        info, _fake = _make_info()
        with pytest.raises(NotImplementedError):
            info.subscribe({"type": "allMids"}, lambda _: None)


class TestSubscribeActiveAssetCtx:
    def test_polls_pair_state(self) -> None:
        """`activeAssetCtx` polls `pair_state` per block."""

        info, fake = _make_info()
        info.subscribe({"type": "activeAssetCtx", "coin": "BTC"}, lambda _: None)
        _method, args, _cb = fake.subscriptions[-1]
        request, _interval = args
        assert "pair_state" in request["wasm_smart"]["msg"]


class TestSubscribeActiveAssetData:
    def test_uses_multi_request(self) -> None:
        """`activeAssetData` bundles two queries via `multi`."""

        info, fake = _make_info()
        info.subscribe(
            {"type": "activeAssetData", "user": "0xuser", "coin": "BTC"},
            lambda _: None,
        )
        _method, args, _cb = fake.subscriptions[-1]
        request, _interval = args
        assert "multi" in request

    def test_markpx_zero_until_pair_stats_polled(self) -> None:
        """`markPx` is `"0"`, NOT `pair_state.funding_per_unit` (price ≠ funding rate)."""

        # Regression guard for a bug where `markPx` was sourced from
        # `funding_per_unit` (a per-unit funding accrual ~0.0001), which
        # would mislead any HL client displaying it as a price ~60_000.
        info, fake = _make_info()
        received: list[Any] = []
        info.subscribe(
            {"type": "activeAssetData", "user": "0xuser", "coin": "BTC"},
            received.append,
        )
        _method, _args, callback = fake.subscriptions[-1]
        # `subscribe_query_app` auto-unwraps the kind-keyed envelope, so
        # the callback sees `payload["response"]` as the multi list
        # directly (NOT `{"multi": [...]}`).
        callback(
            {
                "response": [
                    {"Ok": {"available_margin": "1000.000000"}},
                    {"Ok": {"funding_per_unit": "0.000100"}},
                ],
                "blockHeight": 1,
            }
        )
        assert len(received) == 1
        assert received[0]["ctx"]["markPx"] == "0"


class TestSubscribeNotImplemented:
    @pytest.mark.parametrize(
        "subscription",
        [
            {"type": "userFundings", "user": "0xuser"},
            {"type": "webData2", "user": "0xuser"},
            {"type": "userNonFundingLedgerUpdates", "user": "0xuser"},
        ],
    )
    def test_raises_not_implemented(self, subscription: Any) -> None:
        """Subscriptions Dango can't serve raise NotImplementedError."""

        info, _fake = _make_info()
        with pytest.raises(NotImplementedError):
            info.subscribe(subscription, lambda _: None)


class TestUnsubscribe:
    def test_forwards_id_to_native(self) -> None:
        """`unsubscribe` forwards subscription_id to the native unsubscribe."""

        info, fake = _make_info()
        result = info.unsubscribe(
            {"type": "trades", "coin": "BTC"},
            42,
        )
        assert result is True
        assert fake.unsubscribed == [42]


class TestDisconnectWebsocket:
    def test_calls_native(self) -> None:
        """`disconnect_websocket` forwards to the native method."""

        info, fake = _make_info()
        info.disconnect_websocket()
        assert fake.disconnected is True

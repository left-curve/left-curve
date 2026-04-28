"""Tests for dango.exchange.Exchange — order submission, cancellation, batches."""

from __future__ import annotations

from typing import cast

import pytest

from dango.utils.constants import PERPS_CONTRACT_MAINNET
from dango.utils.types import (
    Addr,
    CancelAction,
    ChildOrder,
    ClientOrderIdRef,
    Dimensionless,
    OrderId,
    OrderKind,
    PairId,
    Quantity,
    SubmitAction,
    TimeInForce,
    UsdPrice,
)
from tests._helpers import (
    FakeInfo,
)
from tests._helpers import (
    exchange as _exchange,
)
from tests._helpers import (
    last_inner_msg as _last_inner_msg,
)

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")
_DEMO_PAIR = PairId("perp/btcusd")


def _market_kind(slippage: str = "0.010000") -> OrderKind:
    """Build a `MarketKind` TypedDict with the canonical 6-dp slippage form."""

    return cast("OrderKind", {"market": {"max_slippage": cast("Dimensionless", slippage)}})


class TestSubmitOrder:
    def test_market_buy_wire_shape(self) -> None:
        """submit_order(+1.5, market) emits a positive size and snake_case kind."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_order(_DEMO_PAIR, 1.5, _market_kind())
        # The full inner-msg shape is locked in here. Any unintended
        # change to wire keys (e.g. accidentally re-camelCasing
        # `pair_id` or `reduce_only`) trips this exact-match.
        assert _last_inner_msg(info) == {
            "trade": {
                "submit_order": {
                    "pair_id": _DEMO_PAIR,
                    "size": "1.500000",
                    "kind": {"market": {"max_slippage": "0.010000"}},
                    "reduce_only": False,
                    "tp": None,
                    "sl": None,
                },
            },
        }

    def test_sell_size_is_negative(self) -> None:
        """A negative size encodes a sell; the wire string keeps the leading minus."""

        # Sign convention is the user-facing API contract; this pins
        # it. A regression where size is .abs()'d server-side would
        # silently flip every sell to a buy.
        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_order(_DEMO_PAIR, -2, _market_kind())
        assert _last_inner_msg(info)["trade"]["submit_order"]["size"] == "-2.000000"

    def test_reduce_only_propagates(self) -> None:
        """reduce_only=True flows into the wire dict unchanged."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_order(_DEMO_PAIR, 1, _market_kind(), reduce_only=True)
        assert _last_inner_msg(info)["trade"]["submit_order"]["reduce_only"] is True

    def test_tp_sl_propagate(self) -> None:
        """tp/sl ChildOrders flow through verbatim (no key-rename or coercion)."""

        # ChildOrder is already a wire-shape TypedDict with the exact
        # keys the contract expects, so we hand it to the wire dict
        # by reference. Test pins that we don't accidentally re-shape
        # it (e.g. by str()'ing the trigger price or stripping `size`).
        info = FakeInfo()
        ex = _exchange(info)
        tp: ChildOrder = {
            "trigger_price": cast("UsdPrice", "31000.000000"),
            "max_slippage": cast("Dimensionless", "0.020000"),
            "size": cast("Quantity", "1.000000"),
        }
        sl: ChildOrder = {
            "trigger_price": cast("UsdPrice", "29000.000000"),
            "max_slippage": cast("Dimensionless", "0.020000"),
            "size": None,
        }
        ex.submit_order(_DEMO_PAIR, 1, _market_kind(), tp=tp, sl=sl)
        inner = _last_inner_msg(info)["trade"]["submit_order"]
        assert inner["tp"] == tp
        assert inner["sl"] == sl

    def test_size_zero_is_rejected(self) -> None:
        """Zero size is rejected client-side (positive=buy, negative=sell)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="non-zero"):
            ex.submit_order(_DEMO_PAIR, 0, _market_kind())
        with pytest.raises(ValueError, match="non-zero"):
            ex.submit_order(_DEMO_PAIR, "0", _market_kind())

    def test_nan_size_is_rejected(self) -> None:
        """NaN/Inf sizes raise ValueError (not silently coerced to '0' or 'inf')."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError):
            ex.submit_order(_DEMO_PAIR, float("nan"), _market_kind())
        with pytest.raises(ValueError):
            ex.submit_order(_DEMO_PAIR, float("inf"), _market_kind())

    def test_wraps_in_perps_contract_execute(self) -> None:
        """The execute message targets the perps contract and carries empty funds."""

        # Orders never carry a funds map — margin is consumed from
        # the caller's existing sub-account balance. The contract is
        # the constructor-resolved perps address (production default).
        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_order(_DEMO_PAIR, 1, _market_kind())
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}


class TestCancelOrder:
    def test_cancel_all_is_bare_string(self) -> None:
        """cancel_order('all') produces a bare 'all' (NOT {'all': null})."""

        # Externally-tagged unit variants serialize as the bare snake_case
        # name, not as a single-key dict. This is the most common shape
        # confusion when porting across SDKs; pin it.
        info = FakeInfo()
        ex = _exchange(info)
        ex.cancel_order("all")
        assert _last_inner_msg(info) == {"trade": {"cancel_order": "all"}}

    def test_cancel_by_order_id_emits_one(self) -> None:
        """cancel_order(OrderId('42')) produces {'one': '42'}."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.cancel_order(OrderId("42"))
        assert _last_inner_msg(info) == {"trade": {"cancel_order": {"one": "42"}}}

    def test_cancel_by_client_order_id_ref(self) -> None:
        """ClientOrderIdRef(value=7) becomes {'one_by_client_order_id': '7'}."""

        # The Uint64 wire type is a base-10 integer string; the
        # dataclass holds an int for ergonomics, so we stringify
        # at the boundary.
        info = FakeInfo()
        ex = _exchange(info)
        ex.cancel_order(ClientOrderIdRef(value=7))
        assert _last_inner_msg(info) == {
            "trade": {"cancel_order": {"one_by_client_order_id": "7"}},
        }

    def test_all_check_precedes_str_fallthrough(self) -> None:
        """cancel_order('all') is NOT mistaken for an OrderId of value 'all'."""

        # Regression guard: `OrderId` is a NewType over str, so the
        # naive `isinstance(spec, str)` branch first would route
        # `"all"` through the `{"one": "all"}` path. The implementation
        # tests the literal string before falling through.
        info = FakeInfo()
        ex = _exchange(info)
        ex.cancel_order("all")
        inner = _last_inner_msg(info)["trade"]["cancel_order"]
        assert inner == "all"
        assert inner != {"one": "all"}


class TestBatchUpdateOrders:
    def test_empty_list_is_rejected(self) -> None:
        """An empty actions list is rejected client-side (chain requires len >= 1)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="at least one"):
            ex.batch_update_orders([])

    def test_mixed_submit_and_cancel_shape(self) -> None:
        """A mixed batch produces an ordered list of {'submit':...}/{'cancel':...} dicts."""

        # Order preservation matters because the contract executes
        # actions in array order. A mix of submit + cancel-by-id +
        # cancel-all exercises every wire-shape branch in one assert.
        info = FakeInfo()
        ex = _exchange(info)
        ex.batch_update_orders(
            [
                SubmitAction(
                    pair_id=_DEMO_PAIR,
                    size=1.0,
                    kind=_market_kind(),
                ),
                CancelAction(spec=OrderId("42")),
                CancelAction(spec=ClientOrderIdRef(value=99)),
                CancelAction(spec="all"),
            ]
        )
        inner = _last_inner_msg(info)["trade"]["batch_update_orders"]
        assert inner == [
            {
                "submit": {
                    "pair_id": _DEMO_PAIR,
                    "size": "1.000000",
                    "kind": {"market": {"max_slippage": "0.010000"}},
                    "reduce_only": False,
                    "tp": None,
                    "sl": None,
                },
            },
            {"cancel": {"one": "42"}},
            {"cancel": {"one_by_client_order_id": "99"}},
            {"cancel": "all"},
        ]

    def test_submit_action_zero_size_rejected(self) -> None:
        """Zero size inside a SubmitAction is rejected at build time."""

        # Same guard as `submit_order` — the helper is shared, so the
        # batch path inherits it. This test pins the inheritance: a
        # future refactor that bypasses `_build_submit_order_wire`
        # would break it.
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="non-zero"):
            ex.batch_update_orders(
                [
                    SubmitAction(pair_id=_DEMO_PAIR, size=0, kind=_market_kind()),
                ]
            )


class TestSubmitMarketOrder:
    def test_default_slippage_is_one_percent(self) -> None:
        """submit_market_order defaults max_slippage to 0.01 (1%) on the wire."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_market_order(_DEMO_PAIR, 1.0)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind == {"market": {"max_slippage": "0.010000"}}

    def test_custom_slippage_propagates(self) -> None:
        """Custom max_slippage flows through dango_decimal formatting."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_market_order(_DEMO_PAIR, 1.0, max_slippage=0.05)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind == {"market": {"max_slippage": "0.050000"}}

    def test_kwargs_propagate_to_submit_order(self) -> None:
        """reduce_only/tp/sl flow from convenience helper to submit_order intact."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_market_order(_DEMO_PAIR, 1.0, reduce_only=True)
        assert _last_inner_msg(info)["trade"]["submit_order"]["reduce_only"] is True


class TestSubmitLimitOrder:
    def test_default_gtc_post_only_ioc_serialization(self) -> None:
        """TimeInForce serializes as the upper-case enum value, not the Python name."""

        # The TIF wire form is `GTC`/`IOC`/`POST` per the
        # `#[serde(rename = "GTC")]` etc. attributes on the Rust
        # source. We store `TimeInForce.value` so downstream
        # `json.dumps` outputs the bare string, not `"TimeInForce.GTC"`.
        info = FakeInfo()
        ex = _exchange(info)
        for tif in (TimeInForce.GTC, TimeInForce.IOC, TimeInForce.POST):
            ex.submit_limit_order(_DEMO_PAIR, 1.0, 30_000.0, time_in_force=tif)
            kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
            assert kind["limit"]["time_in_force"] == tif.value
            # Belt-and-braces: assert it's actually a plain str, not
            # a StrEnum member that just happens to compare equal.
            # `type(...)` is exact; `is str` is too strict because
            # Python may interpolate.
            assert type(kind["limit"]["time_in_force"]) is str

    def test_default_tif_is_gtc(self) -> None:
        """Limit orders default to GTC when no time_in_force is supplied."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_limit_order(_DEMO_PAIR, 1.0, 30_000.0)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind["limit"]["time_in_force"] == "GTC"

    def test_limit_price_is_six_decimal_string(self) -> None:
        """Limit price is encoded as a 6-decimal `UsdPrice` string."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_limit_order(_DEMO_PAIR, 1.0, 30_000.5)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind["limit"]["limit_price"] == "30000.500000"

    def test_client_order_id_stringified(self) -> None:
        """A non-None client_order_id is sent as a base-10 decimal string."""

        # Uint64 wire form requires a string. The convenience helper
        # accepts an int for ergonomics and stringifies internally.
        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_limit_order(_DEMO_PAIR, 1.0, 30_000.0, client_order_id=42)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind["limit"]["client_order_id"] == "42"

    def test_client_order_id_none_stays_none(self) -> None:
        """An omitted client_order_id is null on the wire (not the empty string)."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.submit_limit_order(_DEMO_PAIR, 1.0, 30_000.0)
        kind = _last_inner_msg(info)["trade"]["submit_order"]["kind"]
        assert kind["limit"]["client_order_id"] is None

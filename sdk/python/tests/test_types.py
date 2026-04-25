"""Tests for dango.utils.types."""

from decimal import Decimal
from typing import cast

import pytest

from dango.utils.types import (
    CancelOrderRequest,
    ClientOrderId,
    Dimensionless,
    PairId,
    Quantity,
    SubmitOrderRequest,
    TimeInForce,
    UsdPrice,
    dango_decimal,
)


class TestDangoDecimal:
    def test_int_zero(self) -> None:
        assert dango_decimal(0) == "0.000000"

    def test_int_positive(self) -> None:
        assert dango_decimal(42) == "42.000000"

    def test_int_negative(self) -> None:
        assert dango_decimal(-7) == "-7.000000"

    def test_float_zero_decimals(self) -> None:
        assert dango_decimal(1.23) == "1.230000"

    def test_float_integer_value(self) -> None:
        assert dango_decimal(100.0) == "100.000000"

    def test_str_at_max_precision(self) -> None:
        assert dango_decimal("1.234567") == "1.234567"

    def test_str_few_decimals(self) -> None:
        assert dango_decimal("3.14") == "3.140000"

    def test_str_too_many_places_raises(self) -> None:
        with pytest.raises(ValueError, match="more than 6 decimal places"):
            dango_decimal("1.2345678")

    def test_decimal_smallest(self) -> None:
        assert dango_decimal(Decimal("0.000001")) == "0.000001"

    def test_decimal_at_limit(self) -> None:
        assert dango_decimal(Decimal("123.456789")) == "123.456789"

    def test_decimal_too_many_places_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal(Decimal("0.0000001"))

    def test_negative(self) -> None:
        assert dango_decimal(-50000.5) == "-50000.500000"

    def test_negative_string(self) -> None:
        assert dango_decimal("-1.230000") == "-1.230000"

    def test_float_imprecision_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal(0.1 + 0.2)

    def test_nan_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal(float("nan"))

    def test_inf_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal(float("inf"))

    def test_neg_inf_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal(float("-inf"))

    def test_invalid_str_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal("not a number")

    def test_empty_str_raises(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal("")

    def test_invalid_type_list_raises(self) -> None:
        with pytest.raises(TypeError):
            dango_decimal([])  # type: ignore[arg-type]

    def test_invalid_type_none_raises(self) -> None:
        with pytest.raises(TypeError):
            dango_decimal(None)  # type: ignore[arg-type]

    def test_invalid_type_bool_raises(self) -> None:
        with pytest.raises(TypeError):
            dango_decimal(True)

    def test_max_places_override(self) -> None:
        assert dango_decimal("1.23456789", max_places=8) == "1.23456789"

    def test_max_places_override_zero(self) -> None:
        assert dango_decimal(7, max_places=0) == "7"

    def test_max_places_override_still_rejects(self) -> None:
        with pytest.raises(ValueError):
            dango_decimal("1.234", max_places=2)


class TestTypeShapes:
    def test_submit_order_request_market_constructs(self) -> None:
        req: SubmitOrderRequest = {
            "pair_id": PairId("perp/btcusd"),
            "size": Quantity("0.100000"),
            "kind": {"Market": {"max_slippage": Dimensionless("0.050000")}},
            "reduce_only": False,
            "tp": None,
            "sl": None,
        }
        assert req["pair_id"] == "perp/btcusd"
        assert req["size"] == "0.100000"

    def test_submit_order_request_limit_constructs(self) -> None:
        req: SubmitOrderRequest = {
            "pair_id": PairId("perp/btcusd"),
            "size": Quantity("0.100000"),
            "kind": {"Limit": {"limit_price": UsdPrice("50000.000000")}},
            "reduce_only": False,
            "tp": None,
            "sl": None,
        }
        assert req["kind"] == {"Limit": {"limit_price": "50000.000000"}}

    def test_submit_order_request_limit_with_optionals(self) -> None:
        req: SubmitOrderRequest = {
            "pair_id": PairId("perp/btcusd"),
            "size": Quantity("-0.500000"),
            "kind": {
                "Limit": {
                    "limit_price": UsdPrice("50000.000000"),
                    "time_in_force": TimeInForce.IOC,
                    "client_order_id": ClientOrderId("42"),
                }
            },
            "reduce_only": True,
            "tp": None,
            "sl": None,
        }
        assert req["reduce_only"] is True

    def test_cancel_all_is_string(self) -> None:
        cancel: CancelOrderRequest = "All"
        assert cancel == "All"

    def test_cancel_one_is_dict(self) -> None:
        cancel = cast(CancelOrderRequest, {"One": "123"})
        assert cancel == {"One": "123"}

    def test_cancel_one_by_client_order_id(self) -> None:
        cancel = cast(CancelOrderRequest, {"OneByClientOrderId": "42"})
        assert cancel == {"OneByClientOrderId": "42"}

"""Tests for dango.hyperliquid_compatibility.exchange."""

from __future__ import annotations

from typing import Any, cast
from unittest.mock import patch

import pytest

from dango.hyperliquid_compatibility.exchange import (
    Exchange as HlExchange,
)
from dango.hyperliquid_compatibility.exchange import (
    _build_submit_action,
    _extract_error_message,
    _hl_order_type_to_dango_kind,
    _hl_tif_to_dango,
    _native_outcome_to_cancel_envelope,
    _native_outcome_to_resting_envelope,
    _signed_size,
)
from dango.hyperliquid_compatibility.types import Cloid, OrderType
from dango.utils.types import (
    CancelAction,
    ClientOrderIdRef,
    OrderId,
    PairId,
    SubmitAction,
    TimeInForce,
)

# Shared by every test that constructs an Exchange. The wallet doesn't sign
# anything in these tests because the native Exchange is replaced wholesale
# with a fake (see `_FakeNativeExchange` below); the value only needs to be
# the right type-shape.
_DEMO_ADDRESS: str = "0x000000000000000000000000000000000000beef"
_DEMO_PAIR_BTC = PairId("perp/btcusd")
_DEMO_PAIR_ETH = PairId("perp/ethusd")


# A minimal fake of the native Dango Exchange that captures every call.
# The HL-compat Exchange constructs a native `dango.exchange.Exchange`
# internally, so we patch that constructor at module-level (see
# `_make_exchange` below) and inject this fake instead. Type-checked code
# would normally reject this (the field is annotated as the concrete
# native class), but since we're at the test boundary we use the same
# `# type: ignore` escape hatch the rest of the SDK uses.
class _FakeNativeExchange:
    """Captures every native call so tests can pin the wire-shape arguments."""

    def __init__(self) -> None:
        # Each call appends `(method_name, args, kwargs)` so tests can both
        # pin the dispatch and the wire shape.
        self.calls: list[tuple[str, tuple[Any, ...], dict[str, Any]]] = []
        # Per-call return value override. Defaults to a clean success
        # outcome; tests that exercise the error path overwrite this.
        self.return_value: dict[str, Any] = {
            "code": 0,
            "hash": "TXHASH",
            "gas_used": 230_000,
            "events": [],
        }

    def _record(self, name: str, *args: Any, **kwargs: Any) -> dict[str, Any]:
        self.calls.append((name, args, kwargs))
        return self.return_value

    def submit_order(self, *args: Any, **kwargs: Any) -> dict[str, Any]:
        return self._record("submit_order", *args, **kwargs)

    def cancel_order(self, *args: Any, **kwargs: Any) -> dict[str, Any]:
        return self._record("cancel_order", *args, **kwargs)

    def batch_update_orders(self, *args: Any, **kwargs: Any) -> dict[str, Any]:
        return self._record("batch_update_orders", *args, **kwargs)

    def submit_market_order(self, *args: Any, **kwargs: Any) -> dict[str, Any]:
        return self._record("submit_market_order", *args, **kwargs)

    def set_referral(self, *args: Any, **kwargs: Any) -> dict[str, Any]:
        return self._record("set_referral", *args, **kwargs)


# A minimal fake of the HL Info wrapper. The Exchange uses the embedded
# Info for two things: `name_to_pair` (resolve coin → pair_id) and
# `user_state` (used by `market_close` to read positions). We provide
# both with predictable defaults; tests that need a different shape
# overwrite `user_state_data`.
class _FakeHlInfo:
    """Stand-in for the embedded HL Info; covers `name_to_pair` and `user_state`."""

    def __init__(self) -> None:
        self._coin_to_pair: dict[str, PairId] = {
            "BTC": _DEMO_PAIR_BTC,
            "ETH": _DEMO_PAIR_ETH,
        }
        # Default: empty positions list — overwritten by market_close tests.
        self.user_state_data: dict[str, Any] = {"assetPositions": []}

    def name_to_pair(self, name: str) -> PairId:
        return self._coin_to_pair[name]

    def user_state(self, address: str, dex: str = "") -> dict[str, Any]:
        # Echo `address` back via attribute so tests can pin it if they want.
        self.last_user_state_addr = address
        return self.user_state_data


def _make_exchange(
    *,
    native_return: dict[str, Any] | None = None,
    user_state: dict[str, Any] | None = None,
    base_url: str = "http://test",
    account_address: str = _DEMO_ADDRESS,
) -> tuple[HlExchange, _FakeNativeExchange, _FakeHlInfo]:
    """Build an HL-compat Exchange with a fake native Exchange and Info wired in."""

    # Patch both the native Exchange constructor and the HL Info
    # constructor at the point where the wrapper imports them. The
    # wrapper does `from dango.exchange import Exchange as NativeExchange`
    # at module load time, but it also re-imports inside `__init__` for
    # the HL Info; both must be patched simultaneously.
    fake_native = _FakeNativeExchange()
    if native_return is not None:
        fake_native.return_value = native_return
    fake_info = _FakeHlInfo()
    if user_state is not None:
        fake_info.user_state_data = user_state

    import dango.hyperliquid_compatibility.exchange as exchange_module
    import dango.hyperliquid_compatibility.info as info_module

    with (
        patch.object(exchange_module, "NativeExchange", return_value=fake_native),
        patch.object(info_module, "Info", return_value=fake_info),
    ):
        # The wallet here is unused by the fake native Exchange. We pass
        # an opaque object of the right rough shape; eth_account isn't
        # imported because the native Exchange would normally accept it
        # but our fake bypasses signing entirely.
        ex = HlExchange(
            wallet=object(),  # type: ignore[arg-type]
            base_url=base_url,
            account_address=account_address,
        )
    return ex, fake_native, fake_info


# --- Internal helpers -------------------------------------------------------


class TestSignedSize:
    def test_buy_keeps_positive(self) -> None:
        """is_buy=True passes the positive size through unchanged."""

        assert _signed_size(True, 1.5) == 1.5

    def test_sell_negates(self) -> None:
        """is_buy=False negates the size to encode sell direction."""

        assert _signed_size(False, 2.0) == -2.0

    def test_zero_passthrough(self) -> None:
        """Zero remains zero regardless of side flag."""

        # Native Exchange will reject a zero size; we don't second-guess
        # here. The translation alone preserves zero.
        assert _signed_size(True, 0.0) == 0.0
        assert _signed_size(False, 0.0) == -0.0


class TestHlTifToDango:
    def test_gtc(self) -> None:
        """HL 'Gtc' maps to TimeInForce.GTC."""

        assert _hl_tif_to_dango("Gtc") == TimeInForce.GTC

    def test_ioc(self) -> None:
        """HL 'Ioc' maps to TimeInForce.IOC."""

        assert _hl_tif_to_dango("Ioc") == TimeInForce.IOC

    def test_alo_maps_to_post(self) -> None:
        """HL 'Alo' (post-only) maps to Dango POST."""

        # This is the critical asymmetry: HL's TIF spelling is
        # case-sensitive ("Alo" not "alo" or "ALO"), and it maps to
        # Dango's POST variant rather than its own TIF name.
        assert _hl_tif_to_dango("Alo") == TimeInForce.POST

    def test_unknown_raises(self) -> None:
        """Unknown TIF value raises ValueError listing supported options."""

        with pytest.raises(ValueError, match="unsupported HL time_in_force"):
            _hl_tif_to_dango("FOK")


class TestHlOrderTypeToDangoKind:
    def test_limit_gtc_no_cloid(self) -> None:
        """A limit/Gtc order without cloid produces a clean kind dict."""

        order_type: OrderType = {"limit": {"tif": "Gtc"}}
        kind = _hl_order_type_to_dango_kind(order_type, 60_000.0, None)
        # `limit_price` is the chain's `UsdPrice` — a string-encoded
        # decimal. Passing the float verbatim trips a deserialization
        # error on the chain (`expected string-encoded decimal`).
        assert kind == {
            "limit": {
                "limit_price": "60000.000000",
                "time_in_force": "GTC",
                "client_order_id": None,
            },
        }

    def test_limit_with_cloid_hashes_to_uint64_string(self) -> None:
        """A cloid is hashed to its Uint64 form and stringified."""

        # `Cloid("0x..01").to_uint64()` is the deterministic SHA-256
        # prefix value (test_hl_types.py pins this golden number).
        cloid = Cloid("0x00000000000000000000000000000001")
        order_type: OrderType = {"limit": {"tif": "Ioc"}}
        kind = _hl_order_type_to_dango_kind(order_type, 50_000.0, cloid)
        assert kind == {
            "limit": {
                "limit_price": "50000.000000",
                "time_in_force": "IOC",
                "client_order_id": str(cloid.to_uint64()),
            },
        }

    def test_alo_routes_to_post_tif(self) -> None:
        """An Alo HL order produces a Dango POST limit kind."""

        kind = _hl_order_type_to_dango_kind({"limit": {"tif": "Alo"}}, 1.0, None)
        # `OrderKind` is `MarketKind | LimitKind`; we know the limit
        # branch was taken so cast for the dict access.
        kind_dict = cast("dict[str, Any]", kind)
        assert kind_dict["limit"]["time_in_force"] == "POST"

    def test_trigger_branch_raises(self) -> None:
        """Trigger orders are deferred and raise a clear NotImplementedError."""

        # The trigger branch translation is non-trivial — see the spec
        # comment in `_hl_order_type_to_dango_kind`. The error message
        # must surface the alternative path so callers can recover.
        order_type: OrderType = {
            "trigger": {"triggerPx": 50_000.0, "isMarket": True, "tpsl": "tp"},
        }
        with pytest.raises(NotImplementedError, match="HL trigger orders"):
            _hl_order_type_to_dango_kind(order_type, 0.0, None)

    def test_empty_order_type_raises(self) -> None:
        """An order_type missing both 'limit' and 'trigger' raises ValueError."""

        with pytest.raises(ValueError, match="must contain 'limit' or 'trigger'"):
            _hl_order_type_to_dango_kind({}, 1.0, None)


class TestBuildSubmitAction:
    def test_buy_action_signs_size_positive(self) -> None:
        """A buy action keeps size positive and forwards reduce_only."""

        # Cast through `OrderKind` because the literal dict shape is
        # structurally a `LimitKind` but we need the union name for
        # the function signature.
        from dango.utils.types import OrderKind as _OrderKind

        kind = cast(
            "_OrderKind",
            {"limit": {"limit_price": 60_000.0, "time_in_force": "GTC"}},
        )
        action = _build_submit_action(
            _DEMO_PAIR_BTC,
            is_buy=True,
            sz=1.5,
            order_kind=kind,
            reduce_only=False,
        )
        assert isinstance(action, SubmitAction)
        assert action.size == 1.5
        assert action.reduce_only is False

    def test_sell_action_signs_size_negative(self) -> None:
        """A sell action negates the size."""

        from dango.utils.types import OrderKind as _OrderKind

        kind = cast(
            "_OrderKind",
            {"limit": {"limit_price": 3_000.0, "time_in_force": "GTC"}},
        )
        action = _build_submit_action(
            _DEMO_PAIR_ETH,
            is_buy=False,
            sz=2.0,
            order_kind=kind,
            reduce_only=True,
        )
        assert action.size == -2.0
        assert action.reduce_only is True


class TestExtractErrorMessage:
    def test_clean_outcome_returns_none(self) -> None:
        """A success outcome ({code: 0}) yields no error."""

        assert _extract_error_message({"code": 0, "hash": "x", "events": []}) is None

    def test_top_level_error_string(self) -> None:
        """A top-level 'error' string surfaces verbatim."""

        outcome = {"error": "insufficient margin"}
        assert _extract_error_message(outcome) == "insufficient margin"

    def test_top_level_err_string(self) -> None:
        """A top-level 'err' string surfaces verbatim."""

        outcome = {"err": "bad nonce"}
        assert _extract_error_message(outcome) == "bad nonce"

    def test_check_tx_error_string(self) -> None:
        """A check_tx.error string surfaces verbatim."""

        outcome = {"check_tx": {"error": "bad signature"}}
        assert _extract_error_message(outcome) == "bad signature"

    def test_check_tx_nonzero_code(self) -> None:
        """A non-zero check_tx.code without a string error renders a default message."""

        outcome = {"check_tx": {"code": 17}}
        assert _extract_error_message(outcome) == "check_tx failed with code 17"

    def test_top_level_nonzero_code(self) -> None:
        """A non-zero top-level code surfaces as 'tx failed'."""

        outcome = {"code": 1, "hash": "x"}
        assert _extract_error_message(outcome) == "tx failed with code 1"

    def test_result_err_string(self) -> None:
        """A `result.err` string (lowercase) surfaces verbatim."""

        outcome = {"code": 0, "result": {"err": "panic in handler"}}
        assert _extract_error_message(outcome) == "panic in handler"

    def test_result_Err_string(self) -> None:
        """A `result.Err` string (capitalized) is also recognized."""

        # The Rust serde external tag for `Result<_, _>` produces `Err`
        # (uppercase). We accept both forms because we've seen both in
        # different parts of the stack.
        outcome = {"code": 0, "result": {"Err": "rejected"}}
        assert _extract_error_message(outcome) == "rejected"


class TestNativeOutcomeToRestingEnvelope:
    def test_success_emits_resting_per_order(self) -> None:
        """A clean outcome produces one resting entry per submitted order."""

        env = _native_outcome_to_resting_envelope(
            {"code": 0, "hash": "x", "events": []},
            response_type="order",
            expected_count=2,
        )
        assert env == {
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [
                        {"resting": {"oid": 0}},
                        {"resting": {"oid": 0}},
                    ],
                },
            },
        }

    def test_error_short_circuits(self) -> None:
        """An error outcome produces an err envelope, ignoring expected_count."""

        env = _native_outcome_to_resting_envelope(
            {"code": 1, "hash": "x"},
            response_type="order",
            expected_count=5,
        )
        assert env == {"status": "err", "response": "tx failed with code 1"}

    def test_zero_expected_count(self) -> None:
        """Zero expected orders yields an empty statuses list, not None."""

        env = _native_outcome_to_resting_envelope(
            {"code": 0},
            response_type="order",
            expected_count=0,
        )
        assert env["response"]["data"]["statuses"] == []


class TestNativeOutcomeToCancelEnvelope:
    def test_success_emits_one_status_per_cancel(self) -> None:
        """A clean outcome produces one {status: success} per cancel request."""

        env = _native_outcome_to_cancel_envelope(
            {"code": 0, "hash": "x"},
            response_type="cancel",
            expected_count=3,
        )
        assert env["status"] == "ok"
        assert env["response"]["type"] == "cancel"
        assert env["response"]["data"]["statuses"] == [
            {"status": "success"},
            {"status": "success"},
            {"status": "success"},
        ]

    def test_error_emits_err_envelope(self) -> None:
        """A failed cancel surfaces the error string."""

        env = _native_outcome_to_cancel_envelope(
            {"error": "no such order"},
            response_type="cancel",
            expected_count=1,
        )
        assert env == {"status": "err", "response": "no such order"}

    def test_response_type_propagates(self) -> None:
        """`response_type='cancelByCloid'` distinguishes by-cloid from by-oid envelopes."""

        # HL traders dispatch on `result["response"]["type"]`; the cancel
        # helper must accept the caller's chosen string verbatim so cloid
        # cancels and oid cancels surface the right action type.
        env = _native_outcome_to_cancel_envelope(
            {"code": 0, "hash": "x"},
            response_type="cancelByCloid",
            expected_count=1,
        )
        assert env["response"]["type"] == "cancelByCloid"


# --- Constructor ------------------------------------------------------------


class TestExchangeConstructor:
    def test_basic_construction(self) -> None:
        """A clean construction stores wallet/base_url/account_address."""

        ex, _native, _info = _make_exchange()
        assert ex.account_address == _DEMO_ADDRESS
        assert ex.base_url == "http://test"
        assert ex.expires_after is None
        # The wrapper's `info` is the wired fake — pin the type by
        # checking a method that's specific to the HL interface.
        assert ex.info.name_to_pair("BTC") == _DEMO_PAIR_BTC

    def test_vault_address_raises(self) -> None:
        """Passing a non-None vault_address raises NotImplementedError."""

        # HL's vault routing has no analog in Dango (vault liquidity is
        # debited from margin, not signed under a vault address).
        # Silently ignoring would route trades to the wrong "logical
        # account" in HL traders' minds.
        with pytest.raises(NotImplementedError, match="vault_address"):
            HlExchange(
                wallet=object(),  # type: ignore[arg-type]
                base_url="http://test",
                account_address=_DEMO_ADDRESS,
                vault_address="0xvault",
            )

    def test_spot_meta_raises(self) -> None:
        """Passing a non-None spot_meta raises NotImplementedError."""

        with pytest.raises(NotImplementedError, match="spot_meta"):
            HlExchange(
                wallet=object(),  # type: ignore[arg-type]
                base_url="http://test",
                account_address=_DEMO_ADDRESS,
                spot_meta={"universe": [], "tokens": []},
            )

    def test_missing_account_address_raises(self) -> None:
        """account_address is required (no silent default)."""

        with pytest.raises(ValueError, match="account_address is required"):
            HlExchange(
                wallet=object(),  # type: ignore[arg-type]
                base_url="http://test",
            )

    def test_default_slippage_attribute_present(self) -> None:
        """DEFAULT_SLIPPAGE class attribute matches HL's value."""

        # Pin both the value and the location (class attribute) so
        # callers that read `Exchange.DEFAULT_SLIPPAGE` find what they
        # expect from HL.
        assert HlExchange.DEFAULT_SLIPPAGE == 0.05


# --- Order placement --------------------------------------------------------


class TestOrder:
    def test_single_order_routes_to_submit_order(self) -> None:
        """A single-order call uses native submit_order, not batch_update_orders."""

        ex, native, _info = _make_exchange()
        ex.order(
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_000.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        # Exactly one call to the single-order path.
        assert len(native.calls) == 1
        method, args, kwargs = native.calls[0]
        assert method == "submit_order"
        # Positional args: (pair_id, signed_size, kind)
        assert args[0] == _DEMO_PAIR_BTC
        assert args[1] == 0.5  # buy = positive
        assert args[2] == {
            "limit": {
                "limit_price": "60000.000000",
                "time_in_force": "GTC",
                "client_order_id": None,
            },
        }
        assert kwargs == {"reduce_only": False}

    def test_sell_order_signs_size_negative(self) -> None:
        """is_buy=False produces a negative size on the wire."""

        ex, native, _info = _make_exchange()
        ex.order(
            "ETH",
            is_buy=False,
            sz=2.0,
            limit_px=3_000.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        _method, args, _kwargs = native.calls[0]
        assert args[1] == -2.0

    def test_returns_hl_status_envelope(self) -> None:
        """The response is wrapped in HL's status envelope."""

        ex, _native, _info = _make_exchange()
        result = ex.order(
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_000.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        # Pin the canonical resting-envelope shape — one entry per
        # submitted order, with oid=0 (chain-assigned oid is unknown
        # without parsing events).
        assert result == {
            "status": "ok",
            "response": {
                "type": "order",
                "data": {"statuses": [{"resting": {"oid": 0}}]},
            },
        }

    def test_error_outcome_returns_err_envelope(self) -> None:
        """A non-zero broadcast code propagates as an err envelope."""

        ex, _native, _info = _make_exchange(
            native_return={"code": 1, "hash": "x", "error": "rejected"},
        )
        result = ex.order(
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_000.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        assert result == {"status": "err", "response": "rejected"}

    def test_unknown_coin_raises_keyerror(self) -> None:
        """An unknown coin trips the resolver early, before any native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(KeyError):
            ex.order(
                "DOGE",
                is_buy=True,
                sz=1.0,
                limit_px=1.0,
                order_type={"limit": {"tif": "Gtc"}},
            )
        # No native call should have been issued — the resolver must
        # short-circuit before submission.
        assert native.calls == []

    def test_builder_raises(self) -> None:
        """A non-None builder raises NotImplementedError without a native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="builder fee"):
            ex.order(
                "BTC",
                is_buy=True,
                sz=0.5,
                limit_px=60_000.0,
                order_type={"limit": {"tif": "Gtc"}},
                builder={"b": "0xbuilder", "f": 10},
            )
        assert native.calls == []

    def test_cloid_hashes_through(self) -> None:
        """A cloid arrives as the Uint64-stringified form in the order kind."""

        ex, native, _info = _make_exchange()
        cloid = Cloid("0x00000000000000000000000000000001")
        ex.order(
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_000.0,
            order_type={"limit": {"tif": "Gtc"}},
            cloid=cloid,
        )
        _method, args, _kwargs = native.calls[0]
        assert args[2]["limit"]["client_order_id"] == str(cloid.to_uint64())

    def test_reduce_only_flag_propagates(self) -> None:
        """reduce_only=True flows into the native submit_order call."""

        ex, native, _info = _make_exchange()
        ex.order(
            "BTC",
            is_buy=False,
            sz=1.0,
            limit_px=60_000.0,
            order_type={"limit": {"tif": "Gtc"}},
            reduce_only=True,
        )
        _method, _args, kwargs = native.calls[0]
        assert kwargs == {"reduce_only": True}


class TestBulkOrders:
    def test_two_orders_route_to_batch_update(self) -> None:
        """Multiple orders use the batch path."""

        ex, native, _info = _make_exchange()
        ex.bulk_orders(
            [
                {
                    "coin": "BTC",
                    "is_buy": True,
                    "sz": 0.5,
                    "limit_px": 60_000.0,
                    "order_type": {"limit": {"tif": "Gtc"}},
                    "reduce_only": False,
                },
                {
                    "coin": "ETH",
                    "is_buy": False,
                    "sz": 2.0,
                    "limit_px": 3_000.0,
                    "order_type": {"limit": {"tif": "Ioc"}},
                    "reduce_only": False,
                },
            ],
        )
        assert len(native.calls) == 1
        method, args, _kwargs = native.calls[0]
        assert method == "batch_update_orders"
        actions = args[0]
        assert len(actions) == 2
        assert isinstance(actions[0], SubmitAction)
        assert actions[0].pair_id == _DEMO_PAIR_BTC
        assert actions[0].size == 0.5
        assert isinstance(actions[1], SubmitAction)
        assert actions[1].pair_id == _DEMO_PAIR_ETH
        assert actions[1].size == -2.0

    def test_two_orders_returns_two_resting_entries(self) -> None:
        """Bulk envelope has one resting entry per submitted order."""

        ex, _native, _info = _make_exchange()
        result = ex.bulk_orders(
            [
                {
                    "coin": "BTC",
                    "is_buy": True,
                    "sz": 0.5,
                    "limit_px": 60_000.0,
                    "order_type": {"limit": {"tif": "Gtc"}},
                    "reduce_only": False,
                },
                {
                    "coin": "ETH",
                    "is_buy": True,
                    "sz": 1.0,
                    "limit_px": 3_000.0,
                    "order_type": {"limit": {"tif": "Gtc"}},
                    "reduce_only": False,
                },
            ],
        )
        assert result["response"]["data"]["statuses"] == [
            {"resting": {"oid": 0}},
            {"resting": {"oid": 0}},
        ]

    def test_grouping_other_than_na_raises(self) -> None:
        """Non-default grouping raises NotImplementedError."""

        # `normalTpsl` and `positionTpsl` are HL's TP/SL attachment
        # semantics — non-trivial to translate into native
        # `submit_order(tp=..., sl=...)`. Defer rather than half-translate.
        ex, native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="grouping"):
            ex.bulk_orders(
                [
                    {
                        "coin": "BTC",
                        "is_buy": True,
                        "sz": 0.5,
                        "limit_px": 60_000.0,
                        "order_type": {"limit": {"tif": "Gtc"}},
                        "reduce_only": False,
                    }
                ],
                grouping="normalTpsl",
            )
        assert native.calls == []

    def test_builder_raises(self) -> None:
        """A non-None builder argument raises before any native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="builder fee"):
            ex.bulk_orders(
                [
                    {
                        "coin": "BTC",
                        "is_buy": True,
                        "sz": 0.5,
                        "limit_px": 60_000.0,
                        "order_type": {"limit": {"tif": "Gtc"}},
                        "reduce_only": False,
                    },
                ],
                builder={"b": "0xbuilder", "f": 10},
            )
        assert native.calls == []


# --- Cancel -----------------------------------------------------------------


class TestCancel:
    def test_cancel_routes_to_cancel_order(self) -> None:
        """cancel(name, oid) calls native cancel_order(OrderId(str(oid)))."""

        ex, native, _info = _make_exchange()
        ex.cancel("BTC", 42)
        assert len(native.calls) == 1
        method, args, _kwargs = native.calls[0]
        assert method == "cancel_order"
        # OrderId is a NewType("OrderId", str), so the runtime form is plain str.
        assert args[0] == OrderId("42")

    def test_cancel_returns_cancel_envelope(self) -> None:
        """cancel() returns a HL-shaped cancel envelope with one success entry."""

        ex, _native, _info = _make_exchange()
        result = ex.cancel("BTC", 42)
        assert result == {
            "status": "ok",
            "response": {
                "type": "cancel",
                "data": {"statuses": [{"status": "success"}]},
            },
        }

    def test_cancel_unknown_coin_raises(self) -> None:
        """An unknown coin fails the resolver early without a native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(KeyError):
            ex.cancel("DOGE", 42)
        assert native.calls == []

    def test_cancel_error_envelope(self) -> None:
        """A failed broadcast surfaces the error message."""

        ex, _native, _info = _make_exchange(
            native_return={"code": 0, "result": {"err": "no such order"}},
        )
        result = ex.cancel("BTC", 42)
        assert result == {"status": "err", "response": "no such order"}


class TestBulkCancel:
    def test_routes_to_batch_with_order_id_specs(self) -> None:
        """bulk_cancel produces one CancelAction per request."""

        ex, native, _info = _make_exchange()
        ex.bulk_cancel(
            [
                {"coin": "BTC", "oid": 42},
                {"coin": "ETH", "oid": 99},
            ],
        )
        assert len(native.calls) == 1
        method, args, _kwargs = native.calls[0]
        assert method == "batch_update_orders"
        actions = args[0]
        assert len(actions) == 2
        assert all(isinstance(a, CancelAction) for a in actions)
        assert actions[0].spec == OrderId("42")
        assert actions[1].spec == OrderId("99")

    def test_returns_one_status_per_cancel(self) -> None:
        """Bulk cancel envelope has one success status per request."""

        ex, _native, _info = _make_exchange()
        result = ex.bulk_cancel(
            [
                {"coin": "BTC", "oid": 1},
                {"coin": "BTC", "oid": 2},
                {"coin": "BTC", "oid": 3},
            ],
        )
        assert result["response"]["data"]["statuses"] == [
            {"status": "success"},
            {"status": "success"},
            {"status": "success"},
        ]

    def test_empty_list_raises(self) -> None:
        """An empty cancel list raises ValueError before any native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(ValueError, match="at least one"):
            ex.bulk_cancel([])
        assert native.calls == []


class TestCancelByCloid:
    def test_routes_with_uint64_hashed_cloid(self) -> None:
        """cancel_by_cloid hashes the 16-byte cloid and forwards as ClientOrderIdRef."""

        ex, native, _info = _make_exchange()
        cloid = Cloid("0x00000000000000000000000000000001")
        ex.cancel_by_cloid("BTC", cloid)
        method, args, _kwargs = native.calls[0]
        assert method == "cancel_order"
        spec = args[0]
        assert isinstance(spec, ClientOrderIdRef)
        assert spec.value == cloid.to_uint64()


class TestBulkCancelByCloid:
    def test_routes_with_per_request_cloid_hashing(self) -> None:
        """Each cloid is hashed independently."""

        ex, native, _info = _make_exchange()
        cloid_a = Cloid("0x00000000000000000000000000000001")
        cloid_b = Cloid("0xdeadbeefdeadbeefdeadbeefdeadbeef")
        ex.bulk_cancel_by_cloid(
            [
                {"coin": "BTC", "cloid": cloid_a},
                {"coin": "ETH", "cloid": cloid_b},
            ],
        )
        method, args, _kwargs = native.calls[0]
        assert method == "batch_update_orders"
        actions = args[0]
        assert isinstance(actions[0].spec, ClientOrderIdRef)
        assert actions[0].spec.value == cloid_a.to_uint64()
        assert isinstance(actions[1].spec, ClientOrderIdRef)
        assert actions[1].spec.value == cloid_b.to_uint64()

    def test_non_cloid_value_raises(self) -> None:
        """A request with a non-Cloid 'cloid' field raises TypeError."""

        # We deliberately catch the misuse client-side rather than letting
        # `to_uint64()` fail on a string with a more obscure error.
        ex, native, _info = _make_exchange()
        with pytest.raises(TypeError, match="must carry a Cloid"):
            ex.bulk_cancel_by_cloid([{"coin": "BTC", "cloid": "0xabc"}])
        assert native.calls == []

    def test_empty_list_raises(self) -> None:
        """An empty cancel-by-cloid list raises ValueError."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(ValueError, match="at least one"):
            ex.bulk_cancel_by_cloid([])


# --- Modify -----------------------------------------------------------------


class TestModifyOrder:
    def test_emulated_as_cancel_plus_submit(self) -> None:
        """modify_order produces one cancel + one submit in batch_update_orders."""

        ex, native, _info = _make_exchange()
        ex.modify_order(
            42,
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_500.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        method, args, _kwargs = native.calls[0]
        assert method == "batch_update_orders"
        actions = args[0]
        assert len(actions) == 2
        # First action is the cancel of the original oid, second is the
        # new submit. The order matters: cancel first so the chain
        # frees the slot before re-submission.
        assert isinstance(actions[0], CancelAction)
        assert actions[0].spec == OrderId("42")
        assert isinstance(actions[1], SubmitAction)
        assert actions[1].pair_id == _DEMO_PAIR_BTC
        assert actions[1].size == 0.5

    def test_oid_as_cloid_routes_to_client_order_id_ref(self) -> None:
        """An oid of type Cloid is translated to ClientOrderIdRef on the cancel side."""

        ex, native, _info = _make_exchange()
        cloid = Cloid("0x00000000000000000000000000000001")
        ex.modify_order(
            cloid,
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_500.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        method, args, _kwargs = native.calls[0]
        assert method == "batch_update_orders"
        actions = args[0]
        assert isinstance(actions[0], CancelAction)
        assert isinstance(actions[0].spec, ClientOrderIdRef)
        assert actions[0].spec.value == cloid.to_uint64()

    def test_returns_one_resting_per_modify(self) -> None:
        """Response carries one resting entry per modify request, not per action."""

        ex, _native, _info = _make_exchange()
        result = ex.modify_order(
            42,
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_500.0,
            order_type={"limit": {"tif": "Gtc"}},
        )
        # Each modify becomes 2 actions (cancel + submit) but we report
        # one HL status per modify request, not per action.
        assert len(result["response"]["data"]["statuses"]) == 1

    def test_new_cloid_independent_from_oid(self) -> None:
        """A new cloid on the resubmitted order is independent of the original oid."""

        ex, native, _info = _make_exchange()
        new_cloid = Cloid("0xdeadbeefdeadbeefdeadbeefdeadbeef")
        ex.modify_order(
            42,  # cancel by chain oid
            "BTC",
            is_buy=True,
            sz=0.5,
            limit_px=60_500.0,
            order_type={"limit": {"tif": "Gtc"}},
            cloid=new_cloid,  # new cloid for the resubmitted order
        )
        method, args, _kwargs = native.calls[0]
        actions = args[0]
        # Cancel uses the chain oid (int 42 → "42"), not the new cloid.
        assert isinstance(actions[0], CancelAction)
        assert actions[0].spec == OrderId("42")
        # Submit carries the new cloid as Uint64.
        assert isinstance(actions[1], SubmitAction)
        # Cast through `dict` because `OrderKind` is a TypedDict union
        # (MarketKind | LimitKind) and we know the limit branch is taken.
        kind_dict = cast("dict[str, Any]", actions[1].kind)
        assert kind_dict["limit"]["client_order_id"] == str(new_cloid.to_uint64())


class TestBulkModifyOrdersNew:
    def test_batches_all_pairs_into_one_call(self) -> None:
        """Multiple modifies produce a single batch with 2N actions."""

        ex, native, _info = _make_exchange()
        ex.bulk_modify_orders_new(
            [
                {
                    "oid": 42,
                    "order": {
                        "coin": "BTC",
                        "is_buy": True,
                        "sz": 0.5,
                        "limit_px": 60_500.0,
                        "order_type": {"limit": {"tif": "Gtc"}},
                        "reduce_only": False,
                    },
                },
                {
                    "oid": 99,
                    "order": {
                        "coin": "ETH",
                        "is_buy": False,
                        "sz": 1.0,
                        "limit_px": 3_100.0,
                        "order_type": {"limit": {"tif": "Gtc"}},
                        "reduce_only": False,
                    },
                },
            ],
        )
        # One native call carrying 4 actions: cancel A, submit A,
        # cancel B, submit B.
        assert len(native.calls) == 1
        actions = native.calls[0][1][0]
        assert len(actions) == 4

    def test_empty_list_raises(self) -> None:
        """An empty modify list raises ValueError."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(ValueError, match="at least one"):
            ex.bulk_modify_orders_new([])


# --- Market open / close ----------------------------------------------------


class TestMarketOpen:
    def test_routes_to_submit_market_order(self) -> None:
        """market_open uses the native submit_market_order with signed size."""

        ex, native, _info = _make_exchange()
        ex.market_open("BTC", is_buy=True, sz=0.5)
        assert len(native.calls) == 1
        method, args, kwargs = native.calls[0]
        assert method == "submit_market_order"
        assert args[0] == _DEMO_PAIR_BTC
        assert args[1] == 0.5
        # Default slippage matches HL's class attribute.
        assert kwargs == {"max_slippage": 0.05}

    def test_sell_signs_size_negative(self) -> None:
        """A sell market_open negates the size."""

        ex, native, _info = _make_exchange()
        ex.market_open("BTC", is_buy=False, sz=0.5)
        assert native.calls[0][1][1] == -0.5

    def test_custom_slippage_propagates(self) -> None:
        """A caller-supplied slippage flows into max_slippage kwarg."""

        ex, native, _info = _make_exchange()
        ex.market_open("BTC", is_buy=True, sz=0.5, slippage=0.02)
        _method, _args, kwargs = native.calls[0]
        assert kwargs == {"max_slippage": 0.02}

    def test_px_arg_is_ignored(self) -> None:
        """HL's px hint is ignored; Dango computes its own band."""

        # Pin that no leak of `px` makes it into the native call —
        # otherwise we'd silently double-compute slippage.
        ex, native, _info = _make_exchange()
        ex.market_open("BTC", is_buy=True, sz=0.5, px=999_999.0)
        _method, args, kwargs = native.calls[0]
        # No `px` kwarg should be present, and the size should still
        # be the bare HL-style sz.
        assert "px" not in kwargs
        assert args[1] == 0.5

    def test_builder_raises(self) -> None:
        """A non-None builder argument raises before native call."""

        ex, native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="builder fee"):
            ex.market_open(
                "BTC",
                is_buy=True,
                sz=0.5,
                builder={"b": "0xbuilder", "f": 10},
            )
        assert native.calls == []

    def test_cloid_raises(self) -> None:
        """A non-None cloid on a market order raises NotImplementedError."""

        ex, native, _info = _make_exchange()
        cloid = Cloid("0x00000000000000000000000000000001")
        with pytest.raises(NotImplementedError, match="cloid on market orders"):
            ex.market_open("BTC", is_buy=True, sz=0.5, cloid=cloid)
        assert native.calls == []


class TestMarketClose:
    def test_close_long_signs_size_negative(self) -> None:
        """Closing a long position emits a negative (sell) market order."""

        ex, native, _info = _make_exchange(
            user_state={
                "assetPositions": [
                    {
                        "type": "oneWay",
                        "position": {"coin": "BTC", "szi": "0.5"},
                    },
                ],
            },
        )
        ex.market_close("BTC")
        assert len(native.calls) == 1
        method, args, kwargs = native.calls[0]
        assert method == "submit_market_order"
        assert args[0] == _DEMO_PAIR_BTC
        # szi is +0.5, so close direction is sell, signed size = -0.5.
        assert args[1] == -0.5
        # market_close always sets reduce_only=True so the close can't
        # accidentally open a position in the opposite direction.
        assert kwargs == {"max_slippage": 0.05, "reduce_only": True}

    def test_close_short_signs_size_positive(self) -> None:
        """Closing a short position emits a positive (buy) market order."""

        ex, native, _info = _make_exchange(
            user_state={
                "assetPositions": [
                    {
                        "type": "oneWay",
                        "position": {"coin": "BTC", "szi": "-2.0"},
                    },
                ],
            },
        )
        ex.market_close("BTC")
        _method, args, _kwargs = native.calls[0]
        assert args[1] == 2.0  # close-of-short = buy

    def test_user_supplied_sz_partial_close(self) -> None:
        """A caller-supplied sz overrides the position's full size."""

        ex, native, _info = _make_exchange(
            user_state={
                "assetPositions": [
                    {
                        "type": "oneWay",
                        "position": {"coin": "BTC", "szi": "1.0"},
                    },
                ],
            },
        )
        ex.market_close("BTC", sz=0.3)
        _method, args, _kwargs = native.calls[0]
        # Closing a long with 0.3 means selling 0.3.
        assert args[1] == -0.3

    def test_no_position_returns_err(self) -> None:
        """Closing a coin with no open position returns an err envelope."""

        ex, native, _info = _make_exchange(
            user_state={"assetPositions": []},
        )
        result = ex.market_close("BTC")
        assert result == {
            "status": "err",
            "response": "no open position in 'BTC' to close",
        }
        # No native call should have been issued.
        assert native.calls == []

    def test_zero_size_position_returns_err(self) -> None:
        """A position with zero size returns an err envelope."""

        ex, native, _info = _make_exchange(
            user_state={
                "assetPositions": [
                    {
                        "type": "oneWay",
                        "position": {"coin": "BTC", "szi": "0"},
                    },
                ],
            },
        )
        result = ex.market_close("BTC")
        assert result["status"] == "err"
        assert "zero size" in str(result["response"])
        assert native.calls == []

    def test_builder_raises(self) -> None:
        """A non-None builder raises before native call."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="builder fee"):
            ex.market_close("BTC", builder={"b": "0xbuilder", "f": 10})

    def test_cloid_raises(self) -> None:
        """A non-None cloid raises NotImplementedError."""

        ex, _native, _info = _make_exchange()
        cloid = Cloid("0x00000000000000000000000000000001")
        with pytest.raises(NotImplementedError, match="cloid on market orders"):
            ex.market_close("BTC", cloid=cloid)


# --- Referrer ---------------------------------------------------------------


class TestSetReferrer:
    def test_routes_to_set_referral(self) -> None:
        """set_referrer forwards the code (string) to native set_referral."""

        ex, native, _info = _make_exchange()
        ex.set_referrer("alice")
        method, args, _kwargs = native.calls[0]
        assert method == "set_referral"
        assert args[0] == "alice"

    def test_returns_set_referrer_envelope(self) -> None:
        """A clean response carries response.type=setReferrer with empty statuses."""

        ex, _native, _info = _make_exchange()
        result = ex.set_referrer("alice")
        assert result == {
            "status": "ok",
            "response": {
                "type": "setReferrer",
                "data": {"statuses": []},
            },
        }

    def test_error_returns_err_envelope(self) -> None:
        """A failed set_referral surfaces as err envelope."""

        ex, _native, _info = _make_exchange(
            native_return={"code": 0, "result": {"err": "unknown referrer"}},
        )
        result = ex.set_referrer("nobody")
        assert result == {"status": "err", "response": "unknown referrer"}


# --- Expires after ----------------------------------------------------------


class TestSetExpiresAfter:
    def test_records_value(self) -> None:
        """set_expires_after stores the value on self.expires_after."""

        ex, _native, _info = _make_exchange()
        ex.set_expires_after(1_700_000_000_000)
        assert ex.expires_after == 1_700_000_000_000

    def test_set_to_none_clears(self) -> None:
        """Passing None clears any prior value."""

        ex, _native, _info = _make_exchange()
        ex.set_expires_after(1)
        ex.set_expires_after(None)
        assert ex.expires_after is None

    def test_does_not_invoke_native(self) -> None:
        """set_expires_after is a no-op-with-state-storage; no native call."""

        ex, native, _info = _make_exchange()
        ex.set_expires_after(1_700_000_000_000)
        assert native.calls == []


# --- Hard gaps and Phase-17-deferred stubs ----------------------------------


class TestNotImplementedStubs:
    """Pin that every documented gap surfaces a NotImplementedError."""

    def test_update_leverage(self) -> None:
        """update_leverage raises with a clear cross-margin reason."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="cross-margin only"):
            ex.update_leverage(10, "BTC")

    def test_update_isolated_margin(self) -> None:
        """update_isolated_margin raises with isolated-margin gap reason."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="isolated margin"):
            ex.update_isolated_margin(100.0, "BTC")

    def test_schedule_cancel(self) -> None:
        """schedule_cancel raises with a no-scheduled-cancellation reason."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="scheduled cancellation"):
            ex.schedule_cancel(1_700_000_000_000)

    def test_usd_class_transfer(self) -> None:
        """usd_class_transfer raises (perps-only)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="perps-only"):
            ex.usd_class_transfer(100.0, to_perp=True)

    def test_send_asset(self) -> None:
        """send_asset raises (perps-only)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="perps-only"):
            ex.send_asset("0xrecipient", "", "spot", "USDC", 100.0)

    def test_vault_usd_transfer(self) -> None:
        """vault_usd_transfer raises with the add/remove_liquidity hint."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="add_liquidity"):
            ex.vault_usd_transfer("0xvault", is_deposit=True, usd=100)

    def test_sub_account_transfer(self) -> None:
        """sub_account_transfer raises (Phase 17 deferred)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="deferred"):
            ex.sub_account_transfer("0xsub", is_deposit=True, usd=100)

    def test_approve_builder_fee(self) -> None:
        """approve_builder_fee raises (no builder fee marketplace)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="builder fee"):
            ex.approve_builder_fee("0xbuilder", "0.01")

    def test_create_sub_account(self) -> None:
        """create_sub_account raises (Phase 17 deferred)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="deferred"):
            ex.create_sub_account("subname")

    def test_usd_transfer(self) -> None:
        """usd_transfer raises (Phase 17 deferred)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="deferred"):
            ex.usd_transfer(100.0, "0xrecipient")

    def test_withdraw_from_bridge(self) -> None:
        """withdraw_from_bridge raises (Phase 17 deferred — needs Hyperlane)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="deferred"):
            ex.withdraw_from_bridge(100.0, "0xdest")

    def test_spot_transfer(self) -> None:
        """spot_transfer raises (Dango is perps-only)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="perps-only"):
            ex.spot_transfer(100.0, "0xdest", "USDC")

    def test_approve_agent(self) -> None:
        """approve_agent raises (Phase 17 deferred — session credentials)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="deferred"):
            ex.approve_agent("name")

    def test_convert_to_multi_sig_user(self) -> None:
        """convert_to_multi_sig_user raises (multi-sig not exposed)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="multi-sig"):
            ex.convert_to_multi_sig_user(["0xa", "0xb"], 2)

    def test_multi_sig(self) -> None:
        """multi_sig raises (multi-sig not exposed)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="multi-sig"):
            ex.multi_sig("0xuser", {}, [], 1)

    def test_agent_methods_raise(self) -> None:
        """All agent_* methods raise NotImplementedError."""

        # Iterate so future additions to the agent-method group are
        # auto-caught — if a new method is added without the stub, this
        # test will fail with AttributeError.
        ex, _native, _info = _make_exchange()
        cases: list[tuple[str, tuple[Any, ...]]] = [
            ("agent_enable_dex_abstraction", ()),
            ("agent_set_abstraction", ("u",)),
            ("user_dex_abstraction", ("0xuser", True)),
            ("user_set_abstraction", ("0xuser", "unifiedAccount")),
        ]
        for method, args in cases:
            with pytest.raises(NotImplementedError):
                getattr(ex, method)(*args)

    def test_token_delegate(self) -> None:
        """token_delegate raises (no Dango analog)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="no Dango analog"):
            ex.token_delegate("0xvalidator", 100, is_undelegate=False)

    def test_use_big_blocks(self) -> None:
        """use_big_blocks raises (no Dango analog)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="no Dango analog"):
            ex.use_big_blocks(True)

    def test_c_signer_methods(self) -> None:
        """c_signer_unjail_self / c_signer_jail_self raise."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError):
            ex.c_signer_unjail_self()
        with pytest.raises(NotImplementedError):
            ex.c_signer_jail_self()

    def test_c_validator_methods(self) -> None:
        """All c_validator_* methods raise."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError):
            ex.c_validator_register("ip", "n", "d", False, 0, "0xs", True, 0)
        with pytest.raises(NotImplementedError):
            ex.c_validator_change_profile(None, None, None, True, None, None, None)
        with pytest.raises(NotImplementedError):
            ex.c_validator_unregister()

    def test_noop(self) -> None:
        """noop raises (no Dango analog)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="no Dango analog"):
            ex.noop(1)

    def test_gossip_priority_bid(self) -> None:
        """gossip_priority_bid raises (no Dango analog)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="no Dango analog"):
            ex.gossip_priority_bid(1, "ip", 100)

    def test_spot_deploy_methods(self) -> None:
        """All spot_deploy_* methods raise (Dango is perps-only)."""

        ex, _native, _info = _make_exchange()
        # Each entry is `(method_name, positional_args)`. The annotation
        # is required because mypy can't unify the heterogeneous tuples
        # into a single inferred type for `args`.
        cases: list[tuple[str, tuple[Any, ...]]] = [
            ("spot_deploy_register_token", ("t", 6, 6, 100, "FullName")),
            ("spot_deploy_user_genesis", (1, [], [])),
            ("spot_deploy_enable_freeze_privilege", (1,)),
            ("spot_deploy_freeze_user", (1, "0xu", True)),
            ("spot_deploy_revoke_freeze_privilege", (1,)),
            ("spot_deploy_enable_quote_token", (1,)),
            ("spot_deploy_token_action_inner", ("variant", 1)),
            ("spot_deploy_genesis", (1, "1000", False)),
            ("spot_deploy_register_spot", (1, 2)),
            ("spot_deploy_register_hyperliquidity", (1, 1.0, 1.0, 10, None)),
            ("spot_deploy_set_deployer_trading_fee_share", (1, "0.5")),
        ]
        for method, args in cases:
            with pytest.raises(NotImplementedError, match="perps-only"):
                getattr(ex, method)(*args)

    def test_perp_deploy_methods(self) -> None:
        """perp_deploy_* methods raise (no permissionless deploys)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="permissionless perp deploys"):
            ex.perp_deploy_register_asset("dex", None, "C", 6, "1.0", 1, False, None)
        with pytest.raises(NotImplementedError, match="permissionless perp deploys"):
            ex.perp_deploy_set_oracle("dex", {}, [], {})

    def test_sub_account_spot_transfer(self) -> None:
        """sub_account_spot_transfer raises (perps-only)."""

        ex, _native, _info = _make_exchange()
        with pytest.raises(NotImplementedError, match="perps-only"):
            ex.sub_account_spot_transfer("0xs", True, "USDC", 100.0)

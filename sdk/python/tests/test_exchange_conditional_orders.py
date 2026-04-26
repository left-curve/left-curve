"""Tests for dango.exchange.Exchange — conditional order (TP/SL) submission and cancel."""

from __future__ import annotations

from typing import Any, cast

import pytest

from dango.exchange import Exchange
from dango.utils.constants import PERPS_CONTRACT_MAINNET
from dango.utils.signing import Secp256k1Wallet
from dango.utils.types import (
    Addr,
    AllForPair,
    ConditionalOrderRef,
    PairId,
    TriggerDirection,
)

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")
_DEMO_PAIR = PairId("perp/btcusd")


def _wallet() -> Secp256k1Wallet:
    # Match the order tests: a fixed secret keeps any signature output
    # deterministic, even though these tests only check pre-credential
    # message shapes.
    return Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)


# `_FakeInfo` is duplicated verbatim from `test_exchange_orders.py` (which
# in turn copied from `test_exchange.py`). This is now the THIRD copy —
# extracting to `tests/conftest.py` is now warranted but is intentionally
# out of scope for Phase 13 to keep the diff focused. Hoist on the next
# test file that needs it.
class _FakeInfo:
    """Captures simulate/broadcast calls and returns canned responses."""

    def __init__(self) -> None:
        self.simulated: list[dict[str, Any]] = []
        self.broadcasted: list[dict[str, Any]] = []
        self.queried_status_count: int = 0

    def query_status(self) -> dict[str, Any]:
        self.queried_status_count += 1
        return {
            "chainId": "dango-mock-1",
            "block": {"blockHeight": 1, "timestamp": "x", "hash": "y"},
        }

    def query_app_smart(
        self,
        contract: Addr,
        msg: dict[str, Any],
        **_: Any,
    ) -> Any:
        if "account" in msg:
            return {"index": 0, "owner": 42}
        if "seen_nonces" in msg:
            return [3, 4, 5]
        raise AssertionError(f"unexpected query_app_smart: {msg}")

    def simulate(self, tx: dict[str, Any]) -> dict[str, Any]:
        self.simulated.append(tx)
        return {"gas_used": 230_000, "gas_limit": None, "result": {"ok": []}}

    def broadcast_tx_sync(self, tx: dict[str, Any]) -> dict[str, Any]:
        self.broadcasted.append(tx)
        return {"code": 0, "hash": "TXHASH", "gas_used": 230_000, "events": []}


def _exchange(info: _FakeInfo, **kwargs: Any) -> Exchange:
    """Construct an Exchange wired to a mock Info (no real network calls)."""
    return Exchange(
        _wallet(),
        "http://localhost:8080",
        account_address=_DEMO_ADDRESS,
        info=info,  # type: ignore[arg-type]
        **kwargs,
    )


def _last_inner_msg(info: _FakeInfo) -> dict[str, Any]:
    """Pull the inner `{"trade": {...}}` payload out of the most-recent broadcast."""
    sent = info.broadcasted[-1]
    return cast("dict[str, Any]", sent["msgs"][0]["execute"]["msg"])


class TestSubmitConditionalOrder:
    def test_full_wire_shape(self) -> None:
        """submit_conditional_order pins the entire inner submit_conditional_order dict."""
        # End-to-end shape check: any rename of the 5 wire keys
        # (`pair_id`, `size`, `trigger_price`, `trigger_direction`,
        # `max_slippage`) trips this exact-match.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -1.5,
            50_000.0,
            TriggerDirection.ABOVE,
            0.01,
        )
        assert _last_inner_msg(info) == {
            "trade": {
                "submit_conditional_order": {
                    "pair_id": _DEMO_PAIR,
                    "size": "-1.500000",
                    "trigger_price": "50000.000000",
                    "trigger_direction": "above",
                    "max_slippage": "0.010000",
                },
            },
        }

    def test_size_none_emits_json_null(self) -> None:
        """size=None is emitted as Python None (JSON null) — meaning 'close all'."""
        # `None` is wire-distinct from `0`: contract reads `None` as
        # "close entire position at trigger". Any path that maps None
        # to an empty string or the literal "0.000000" would silently
        # downgrade the close-all semantics.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            None,
            50_000.0,
            TriggerDirection.ABOVE,
            0.01,
        )
        inner = _last_inner_msg(info)["trade"]["submit_conditional_order"]
        assert inner["size"] is None

    def test_negative_size_preserves_minus(self) -> None:
        """Negative size flows through with the leading minus intact."""
        # The user is responsible for the sign (per Rust comment), so
        # the SDK must NOT .abs() — that would silently flip a
        # close-long into a buy.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -2,
            50_000.0,
            TriggerDirection.BELOW,
            0.01,
        )
        inner = _last_inner_msg(info)["trade"]["submit_conditional_order"]
        assert inner["size"] == "-2.000000"

    def test_size_zero_is_rejected(self) -> None:
        """Zero size is rejected client-side (positive=close-short, negative=close-long)."""
        info = _FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="non-zero or None"):
            ex.submit_conditional_order(
                _DEMO_PAIR,
                0,
                50_000.0,
                TriggerDirection.ABOVE,
                0.01,
            )
        with pytest.raises(ValueError, match="non-zero or None"):
            ex.submit_conditional_order(
                _DEMO_PAIR,
                "0",
                50_000.0,
                TriggerDirection.ABOVE,
                0.01,
            )

    def test_trigger_direction_above_is_plain_str(self) -> None:
        """TriggerDirection.ABOVE serializes to bare 'above' — same regression guard as TIF."""
        # If we stored the StrEnum member instead of `.value`,
        # `json.dumps` would either pass (StrEnum) or fail
        # (downstream non-Python consumer); pin the unwrapped form.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -1,
            50_000.0,
            TriggerDirection.ABOVE,
            0.01,
        )
        inner = _last_inner_msg(info)["trade"]["submit_conditional_order"]
        assert inner["trigger_direction"] == "above"
        assert type(inner["trigger_direction"]) is str

    def test_trigger_direction_below_serializes(self) -> None:
        """TriggerDirection.BELOW serializes to bare 'below'."""
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -1,
            50_000.0,
            TriggerDirection.BELOW,
            0.01,
        )
        inner = _last_inner_msg(info)["trade"]["submit_conditional_order"]
        assert inner["trigger_direction"] == "below"
        assert type(inner["trigger_direction"]) is str

    def test_wraps_in_perps_contract_execute(self) -> None:
        """The execute message targets the perps contract and carries empty funds."""
        # Conditional orders never carry funds — same as regular
        # orders, margin is consumed from the existing sub-account.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -1,
            50_000.0,
            TriggerDirection.ABOVE,
            0.01,
        )
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}

    def test_pipeline_advances_nonce(self) -> None:
        """submit_conditional_order goes through the simulate/sign/broadcast pipeline."""
        # One end-to-end pipeline check is enough — `_send_action` is
        # shared across all the order methods and already covered
        # extensively in `test_exchange_orders.py`. Pin that
        # `submit_conditional_order` actually delegates rather than
        # bypassing the pipeline (e.g. via a stale cached path).
        info = _FakeInfo()
        ex = _exchange(info)
        starting_nonce = ex.signer.next_nonce
        ex.submit_conditional_order(
            _DEMO_PAIR,
            -1,
            50_000.0,
            TriggerDirection.ABOVE,
            0.01,
        )
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        # `(starting or 0) + 1` mirrors `test_exchange.py`: signer's
        # next_nonce is typed `int | None` for pre-resolution states,
        # but in this constructor path it's always populated.
        assert ex.signer.next_nonce == (starting_nonce or 0) + 1


class TestCancelConditionalOrder:
    def test_cancel_all_is_bare_string(self) -> None:
        """cancel_conditional_order('all') produces a bare 'all' (NOT {'all': null})."""
        # Same externally-tagged-unit-variant rule as the order-side
        # 'all'; pin it here to catch a wire-shape regression that
        # would otherwise only surface server-side.
        info = _FakeInfo()
        ex = _exchange(info)
        ex.cancel_conditional_order("all")
        assert _last_inner_msg(info) == {"trade": {"cancel_conditional_order": "all"}}

    def test_cancel_one_emits_struct_variant(self) -> None:
        """ConditionalOrderRef → {'one': {'pair_id': ..., 'trigger_direction': ...}}."""
        info = _FakeInfo()
        ex = _exchange(info)
        ex.cancel_conditional_order(
            ConditionalOrderRef(_DEMO_PAIR, TriggerDirection.ABOVE),
        )
        assert _last_inner_msg(info) == {
            "trade": {
                "cancel_conditional_order": {
                    "one": {
                        "pair_id": _DEMO_PAIR,
                        "trigger_direction": "above",
                    },
                },
            },
        }

    def test_cancel_one_below(self) -> None:
        """ConditionalOrderRef with BELOW serializes the trigger_direction unchanged."""
        info = _FakeInfo()
        ex = _exchange(info)
        ex.cancel_conditional_order(
            ConditionalOrderRef(_DEMO_PAIR, TriggerDirection.BELOW),
        )
        inner = _last_inner_msg(info)["trade"]["cancel_conditional_order"]
        assert inner == {"one": {"pair_id": _DEMO_PAIR, "trigger_direction": "below"}}

    def test_cancel_all_for_pair(self) -> None:
        """AllForPair → {'all_for_pair': {'pair_id': ...}}."""
        info = _FakeInfo()
        ex = _exchange(info)
        ex.cancel_conditional_order(AllForPair(_DEMO_PAIR))
        assert _last_inner_msg(info) == {
            "trade": {
                "cancel_conditional_order": {"all_for_pair": {"pair_id": _DEMO_PAIR}},
            },
        }

    def test_wraps_in_perps_contract_execute(self) -> None:
        """The cancel execute message targets the perps contract with empty funds."""
        info = _FakeInfo()
        ex = _exchange(info)
        ex.cancel_conditional_order(AllForPair(_DEMO_PAIR))
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}

    def test_pipeline_advances_nonce(self) -> None:
        """cancel_conditional_order goes through the simulate/sign/broadcast pipeline."""
        info = _FakeInfo()
        ex = _exchange(info)
        starting_nonce = ex.signer.next_nonce
        ex.cancel_conditional_order("all")
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        assert ex.signer.next_nonce == (starting_nonce or 0) + 1

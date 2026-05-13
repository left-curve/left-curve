"""Tests for dango.hyperliquid_compatibility.types."""

from __future__ import annotations

import hashlib

import pytest

from dango.hyperliquid_compatibility.types import (
    SIDES,
    # Per-entry factories.
    Cloid,
    Fill,
    HlErrorEntry,
    HlFilledEntry,
    HlRestingEntry,
    HlStatusEntry,
    L2BookSubscription,
    # Subscription / message exports for the import smoke-test.
    L2Level,
    LimitOrderType,
    Meta,
    OrderRequest,
    OrderType,
    OrderWire,
    Subscription,
    TradesSubscription,
    TriggerOrderType,
    UserEventsSubscription,
    UserFillsSubscription,
    WsMsg,
    dango_decimal_to_hl_str,
    hl_error_entry,
    hl_filled_entry,
    hl_resting_entry,
    hl_status_envelope,
)


class TestCloid:
    def test_from_str_round_trip(self) -> None:
        """A 0x-prefixed 16-byte hex string survives Cloid round-trip."""

        raw = "0x123456789abcdef0123456789abcdef0"
        cloid = Cloid.from_str(raw)
        assert cloid.to_raw() == raw

    def test_from_int_round_trip(self) -> None:
        """from_int produces a 0x-prefixed 32-hex-char raw cloid."""

        cloid = Cloid.from_int(1)
        assert cloid.to_raw() == "0x00000000000000000000000000000001"

    def test_from_int_largest_16_byte(self) -> None:
        """from_int handles the maximum 128-bit value."""

        max_128 = (1 << 128) - 1
        cloid = Cloid.from_int(max_128)
        assert cloid.to_raw() == "0x" + "f" * 32

    def test_str_and_repr(self) -> None:
        """str() and repr() both return the raw cloid."""

        raw = "0xdeadbeefdeadbeefdeadbeefdeadbeef"
        cloid = Cloid(raw)
        assert str(cloid) == raw
        assert repr(cloid) == raw

    def test_validation_rejects_missing_0x_prefix(self) -> None:
        """Cloid without 0x prefix is rejected."""

        with pytest.raises(TypeError, match="not a hex string"):
            Cloid("123456789abcdef0123456789abcdef0")

    def test_validation_rejects_short_input(self) -> None:
        """Cloid with fewer than 32 hex chars is rejected."""

        with pytest.raises(TypeError, match="not 16 bytes"):
            Cloid("0xdeadbeef")

    def test_validation_rejects_long_input(self) -> None:
        """Cloid with more than 32 hex chars is rejected."""

        with pytest.raises(TypeError, match="not 16 bytes"):
            Cloid("0x" + "a" * 33)

    def test_to_uint64_golden_value(self) -> None:
        """Hand-computed golden value: SHA-256 of 0x...01 -> u64."""

        raw = "0x00000000000000000000000000000001"
        cloid = Cloid(raw)
        # Hand-derived: sha256(b"0x00000000000000000000000000000001"),
        # first 8 bytes big-endian. Captured here so a refactor that
        # silently changes the hash policy fails this test.
        expected = 13713725250624334454
        assert cloid.to_uint64() == expected

    def test_to_uint64_matches_sha256_spec(self) -> None:
        """to_uint64 follows the documented SHA-256/big-endian recipe."""

        raw = "0xdeadbeefdeadbeefdeadbeefdeadbeef"
        cloid = Cloid(raw)
        expected = int.from_bytes(hashlib.sha256(raw.encode("ascii")).digest()[:8], byteorder="big")
        assert cloid.to_uint64() == expected

    def test_to_uint64_is_deterministic(self) -> None:
        """The same cloid always produces the same uint64 across calls."""

        cloid = Cloid("0x123456789abcdef0123456789abcdef0")
        first = cloid.to_uint64()
        second = cloid.to_uint64()
        third = Cloid.from_str("0x123456789abcdef0123456789abcdef0").to_uint64()
        assert first == second == third

    def test_to_uint64_different_cloids_differ(self) -> None:
        """Different cloids produce different uint64s (collision sanity)."""

        a = Cloid("0x00000000000000000000000000000001").to_uint64()
        b = Cloid("0x00000000000000000000000000000002").to_uint64()
        c = Cloid("0xdeadbeefdeadbeefdeadbeefdeadbeef").to_uint64()
        assert a != b
        assert b != c
        assert a != c

    def test_to_uint64_fits_in_uint64(self) -> None:
        """Result is always within the unsigned 64-bit range."""

        for raw in (
            "0x00000000000000000000000000000001",
            "0xffffffffffffffffffffffffffffffff",
            "0x123456789abcdef0123456789abcdef0",
        ):
            value = Cloid(raw).to_uint64()
            assert 0 <= value < (1 << 64)

    def test_to_uint64_normalizes_case(self) -> None:
        """Uppercase and lowercase hex produce the same uint64."""

        # The hash policy lowercases the hex, so callers that pass
        # uppercase still hit the documented mapping.
        lower = Cloid("0xdeadbeefdeadbeefdeadbeefdeadbeef")
        upper = Cloid("0xDEADBEEFDEADBEEFDEADBEEFDEADBEEF")
        assert lower.to_uint64() == upper.to_uint64()


class TestDangoDecimalToHlStr:
    def test_strips_trailing_zeros(self) -> None:
        """Dango's 6-decimal canonical form trims to HL's compact form."""

        assert dango_decimal_to_hl_str("1.230000") == "1.23"

    def test_drops_decimal_point_when_integer(self) -> None:
        """A fully-zero fraction collapses to a bare integer."""

        assert dango_decimal_to_hl_str("1.000000") == "1"

    def test_zero(self) -> None:
        """0.000000 collapses to '0'."""

        assert dango_decimal_to_hl_str("0.000000") == "0"

    def test_negative(self) -> None:
        """Negative numbers keep their sign and trim trailing zeros."""

        assert dango_decimal_to_hl_str("-1.500000") == "-1.5"

    def test_short_decimal(self) -> None:
        """Inputs already in compact form survive untouched."""

        assert dango_decimal_to_hl_str("1.0") == "1"

    def test_no_decimal_point(self) -> None:
        """Plain integers pass through."""

        assert dango_decimal_to_hl_str("5") == "5"

    def test_multiple_digit_integer_no_scientific_notation(self) -> None:
        """Integers >= 10 must NOT serialise as 1E+1 / 1E+2 / etc."""

        # `Decimal("10").normalize()` is `Decimal("1E+1")`; the helper
        # must format with `:f` so the wire string stays "10".
        assert dango_decimal_to_hl_str("10") == "10"
        assert dango_decimal_to_hl_str("100") == "100"
        assert dango_decimal_to_hl_str("1000.000000") == "1000"

    def test_small_decimal(self) -> None:
        """Sub-unit values keep all significant digits."""

        assert dango_decimal_to_hl_str("0.000001") == "0.000001"

    def test_large_with_trailing_zeros(self) -> None:
        """Stress: large value with mixed trailing zeros."""

        assert dango_decimal_to_hl_str("1234567.890000") == "1234567.89"

    def test_negative_zero_fraction(self) -> None:
        """A negative number with all-zero fraction loses the fraction."""

        assert dango_decimal_to_hl_str("-100.000000") == "-100"

    def test_signed_zero_collapses_to_unsigned(self) -> None:
        """`-0.000000` collapses to `"0"` — no `-0` survives the conversion."""

        # `Decimal.normalize()` preserves the sign on zero, so without a
        # zero short-circuit `-0.000000` would round-trip to `"-0"`,
        # which would be a wire-shape regression that downstream
        # consumers (HL APIs and any JSON-comparing test harness)
        # might or might not tolerate.
        assert dango_decimal_to_hl_str("-0.000000") == "0"
        assert dango_decimal_to_hl_str("-0") == "0"
        assert dango_decimal_to_hl_str("-0.0") == "0"

    def test_preserves_high_precision(self) -> None:
        """Inputs that don't have trailing zeros are preserved as-is."""

        assert dango_decimal_to_hl_str("3.141592") == "3.141592"


class TestHlEntryFactories:
    def test_resting_entry_shape(self) -> None:
        """hl_resting_entry produces the canonical {'resting': {'oid': int}}."""

        entry: HlRestingEntry = hl_resting_entry(1234)
        assert entry == {"resting": {"oid": 1234}}

    def test_filled_entry_shape(self) -> None:
        """hl_filled_entry produces the canonical filled status dict."""

        entry: HlFilledEntry = hl_filled_entry(total_sz="0.5", avg_px="50000", oid=42)
        assert entry == {"filled": {"totalSz": "0.5", "avgPx": "50000", "oid": 42}}

    def test_error_entry_shape(self) -> None:
        """hl_error_entry wraps a string into the per-order error shape."""

        entry: HlErrorEntry = hl_error_entry("insufficient margin")
        assert entry == {"error": "insufficient margin"}


class TestHlStatusEnvelope:
    def test_ok_with_empty_statuses(self) -> None:
        """Default ok envelope has an empty statuses list."""

        env = hl_status_envelope(response_type="order")
        assert env == {"status": "ok", "response": {"type": "order", "data": {"statuses": []}}}

    def test_ok_with_mixed_entries(self) -> None:
        """Mixed resting/filled/error entries pass through verbatim."""

        # Explicit annotation: the union of three dict shapes doesn't
        # auto-coalesce in inference (mypy widens to `list[object]`).
        statuses: list[HlStatusEntry] = [
            hl_resting_entry(1),
            hl_filled_entry(total_sz="2.5", avg_px="100", oid=2),
            hl_error_entry("rejected"),
        ]
        env = hl_status_envelope(response_type="order", statuses=statuses)
        assert env == {
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [
                        {"resting": {"oid": 1}},
                        {"filled": {"totalSz": "2.5", "avgPx": "100", "oid": 2}},
                        {"error": "rejected"},
                    ]
                },
            },
        }

    def test_err_envelope(self) -> None:
        """err envelope drops the data wrapper and inlines the message."""

        env = hl_status_envelope(response_type="order", error="bad nonce")
        assert env == {"status": "err", "response": "bad nonce"}

    def test_err_takes_precedence_over_statuses(self) -> None:
        """If both error and statuses are passed, error wins (HL convention)."""

        env = hl_status_envelope(
            response_type="order",
            statuses=[hl_resting_entry(1)],
            error="boom",
        )
        assert env == {"status": "err", "response": "boom"}

    def test_response_type_is_preserved(self) -> None:
        """Caller-supplied response_type passes through unchanged."""

        for kind in ("order", "cancel", "modify", "batchModify"):
            env = hl_status_envelope(response_type=kind)
            assert env["response"]["type"] == kind


class TestExports:
    """Smoke-test that every advertised TypedDict / type alias actually exists."""

    def test_sides_constant(self) -> None:
        """SIDES is exactly ['A', 'B'] in HL convention order."""

        assert SIDES == ["A", "B"]

    def test_typed_dicts_are_importable(self) -> None:
        """Imported TypedDicts have their expected names (smoke test)."""

        # No isinstance checks — TypedDicts erase at runtime — but the
        # symbols exist and carry their declared __name__ where possible.
        assert Meta.__name__ == "Meta"
        assert L2Level.__name__ == "L2Level"
        assert LimitOrderType.__name__ == "LimitOrderType"
        assert TriggerOrderType.__name__ == "TriggerOrderType"
        assert OrderType.__name__ == "OrderType"
        assert OrderWire.__name__ == "OrderWire"
        assert OrderRequest.__name__ == "OrderRequest"
        assert L2BookSubscription.__name__ == "L2BookSubscription"
        assert TradesSubscription.__name__ == "TradesSubscription"
        assert UserEventsSubscription.__name__ == "UserEventsSubscription"
        assert UserFillsSubscription.__name__ == "UserFillsSubscription"
        assert Fill.__name__ == "Fill"

    def test_union_aliases_exist(self) -> None:
        """Subscription and WsMsg are Union aliases; they're truthy at import."""

        # Python's UnionType objects are truthy; this checks the alias
        # was defined and isn't `None` or an accidental empty union.
        assert Subscription is not None
        assert WsMsg is not None

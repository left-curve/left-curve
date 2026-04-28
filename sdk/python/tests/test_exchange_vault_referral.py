"""Tests for dango.exchange.Exchange — vault, referrals, and liquidation methods (Phase 14)."""

from __future__ import annotations

import pytest

from dango.utils.constants import ACCOUNT_FACTORY_CONTRACT, PERPS_CONTRACT_MAINNET
from dango.utils.types import Addr
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
# Distinct address used as the *target* of liquidation. Must differ from
# `_DEMO_ADDRESS` so the test pins that the wire field carries the user
# argument, not the signer's own account.
_TARGET_ADDRESS = Addr("0x00000000000000000000000000000000feedbeef")


class TestAddLiquidity:
    def test_full_wire_shape_with_min_shares(self) -> None:
        """add_liquidity(amount, min_shares_to_mint=N) pins the full inner dict."""

        # End-to-end shape check: any rename of `amount` /
        # `min_shares_to_mint` or any wire-form regression (e.g.
        # accidentally stringifying amount as base units instead of
        # 6-decimal USD) trips this exact-match.
        info = FakeInfo()
        ex = _exchange(info)
        ex.add_liquidity(1000, min_shares_to_mint=500_000)
        assert _last_inner_msg(info) == {
            "vault": {
                "add_liquidity": {
                    "amount": "1000.000000",
                    "min_shares_to_mint": "500000",
                },
            },
        }

    def test_min_shares_none_is_json_null(self) -> None:
        """min_shares_to_mint=None is emitted as Python None (JSON null)."""

        # The contract's `Option<Uint128>` accepts `null` as "no
        # slippage protection". Any path that maps None to "0", the
        # empty string, or omits the key entirely would silently
        # change the on-chain semantics — so pin the full inner shape
        # with `min_shares_to_mint: None`, plus an explicit `is None`
        # check (defends against a future regression that converts
        # None to a string-typed sentinel).
        info = FakeInfo()
        ex = _exchange(info)
        ex.add_liquidity(1000)
        assert _last_inner_msg(info) == {
            "vault": {"add_liquidity": {"amount": "1000.000000", "min_shares_to_mint": None}},
        }
        inner = _last_inner_msg(info)["vault"]["add_liquidity"]
        assert inner["min_shares_to_mint"] is None

    def test_amount_uses_six_decimal_string(self) -> None:
        """Float amounts go through `dango_decimal` and produce 6-decimal USD strings."""

        # UsdValue is a 6-decimal fixed-point string; the same
        # encoding used by `withdraw_margin`. Pin a non-integer
        # float to confirm we're going through `dango_decimal`
        # (not e.g. `str(amount)` which would produce "12.5").
        info = FakeInfo()
        ex = _exchange(info)
        ex.add_liquidity(12.5)
        inner = _last_inner_msg(info)["vault"]["add_liquidity"]
        assert inner["amount"] == "12.500000"

    def test_rejects_zero_amount(self) -> None:
        """add_liquidity requires a strictly positive amount."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="positive"):
            ex.add_liquidity(0)

    def test_rejects_negative_amount(self) -> None:
        """A negative amount is rejected client-side (chain would also reject)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="positive"):
            ex.add_liquidity(-1)

    def test_rejects_bool_amount(self) -> None:
        """Bool amount is rejected via dango_decimal's bool guard."""

        # `dango_decimal` already rejects bool (see types.py). This
        # test pins that the rejection survives the validation chain
        # without being silently coerced into "1.000000".
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError):
            ex.add_liquidity(True)

    def test_rejects_non_finite_amount(self) -> None:
        """NaN/Inf amounts are rejected (delegated to dango_decimal)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError):
            ex.add_liquidity(float("nan"))
        with pytest.raises(ValueError):
            ex.add_liquidity(float("inf"))

    def test_rejects_negative_min_shares(self) -> None:
        """min_shares_to_mint must be non-negative when provided."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="non-negative"):
            ex.add_liquidity(1000, min_shares_to_mint=-1)

    def test_rejects_bool_min_shares(self) -> None:
        """min_shares_to_mint=True is rejected (bool is technically int)."""

        # Without the bool guard, `True` would coerce to "1" and
        # silently bypass the negative check. Mirrors `deposit_margin`'s
        # bool defense.
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int"):
            ex.add_liquidity(1000, min_shares_to_mint=True)

    def test_zero_min_shares_is_allowed(self) -> None:
        """min_shares_to_mint=0 is allowed (= no slippage protection)."""

        # 0 is the wasteful-but-legal way to disable the guard.
        # Passing None is more idiomatic but 0 must NOT be rejected
        # by the SDK — the chain accepts it.
        info = FakeInfo()
        ex = _exchange(info)
        ex.add_liquidity(1000, min_shares_to_mint=0)
        inner = _last_inner_msg(info)["vault"]["add_liquidity"]
        assert inner["min_shares_to_mint"] == "0"

    def test_wraps_in_perps_contract_execute_with_no_funds(self) -> None:
        """The execute message targets the perps contract and carries empty funds."""

        # Vault liquidity is debited from the user's existing margin —
        # NOT attached as `funds` on the execute. Pin this so a
        # regression that accidentally re-routed it through the deposit
        # funds path would fail loudly.
        info = FakeInfo()
        ex = _exchange(info)
        ex.add_liquidity(1000)
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}


class TestRemoveLiquidity:
    def test_full_wire_shape(self) -> None:
        """remove_liquidity(N) emits the int as a base-10 Uint128 string."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.remove_liquidity(1_000_000)
        assert _last_inner_msg(info) == {
            "vault": {"remove_liquidity": {"shares_to_burn": "1000000"}},
        }

    def test_rejects_float(self) -> None:
        """remove_liquidity requires an int (Uint128); floats are ambiguous."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int"):
            ex.remove_liquidity(1.5)  # type: ignore[arg-type]

    def test_rejects_bool(self) -> None:
        """remove_liquidity rejects bool (subclass-of-int hazard)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int"):
            ex.remove_liquidity(True)

    def test_rejects_zero(self) -> None:
        """Zero shares-to-burn is rejected client-side."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="positive"):
            ex.remove_liquidity(0)

    def test_rejects_negative(self) -> None:
        """A negative share count is rejected client-side."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="positive"):
            ex.remove_liquidity(-1)

    def test_wraps_in_perps_contract_execute_with_no_funds(self) -> None:
        """remove_liquidity targets the perps contract with empty funds."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.remove_liquidity(1_000)
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}


class TestSetReferralByIndex:
    def test_int_referrer_emits_full_wire_shape(self) -> None:
        """set_referral(int) emits {referrer: <int>, referee: <signer.user_index>}."""

        # Both `referrer` and `referee` are u32 on the wire (JSON
        # number, NOT string). Referee is auto-filled from the signer's
        # index — the FakeInfo returns 42 from the account-factory
        # `account` lookup, so that's the value we expect here.
        info = FakeInfo()
        ex = _exchange(info)
        ex.set_referral(7)
        assert _last_inner_msg(info) == {
            "referral": {
                "set_referral": {
                    "referrer": 7,
                    "referee": 42,
                },
            },
        }

    def test_int_referrer_does_not_query_account_factory(self) -> None:
        """An int referrer skips the username lookup entirely."""

        # No round-trip to the account factory: the int is the index
        # already. Inspect the recorded smart_queries — only the
        # constructor's `account` and `seen_nonces` lookups should be
        # present, NOT the `user` variant.
        info = FakeInfo()
        ex = _exchange(info)
        # Reset the queries list AFTER the constructor's auto-resolution
        # so we only count what `set_referral` does.
        info.smart_queries.clear()
        ex.set_referral(7)
        # `set_referral` itself only sends a tx (simulate + broadcast);
        # it should not have made any new queries.
        assert info.smart_queries == []

    def test_rejects_negative_int_referrer(self) -> None:
        """A negative int referrer is rejected (UserIndex is unsigned)."""

        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="non-negative"):
            ex.set_referral(-1)

    def test_rejects_bool_referrer(self) -> None:
        """A bool referrer is rejected (subclass-of-int hazard)."""

        # Without the bool guard, `True` would silently route through
        # the int branch as 1 — a different user. Mirrors the bool
        # defense pattern in the rest of the SDK.
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="bool"):
            ex.set_referral(True)

    def test_rejects_non_int_non_str_referrer(self) -> None:
        """Defensive TypeError for callers that bypass the static `int | str` type."""

        # The static union prevents legitimate callers from passing
        # e.g. a float, but the runtime raise is a friendly fallback
        # for callers that ignore type checking (JSON-fed ingest,
        # untyped REPL use).
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int or str"):
            ex.set_referral(1.5)  # type: ignore[arg-type]


class TestSetReferralByUsername:
    def test_username_resolves_via_account_factory(self) -> None:
        """A str referrer triggers a `{user: {name: ...}}` query and uses the resolved index."""

        # FakeInfo returns `{"index": 7, ...}` for any `user.name`
        # query. Test pins both the query site (call to the account-
        # factory contract with the right msg shape) and the wire
        # output (resolved index 7 ends up in the `referrer` field).
        info = FakeInfo()
        ex = _exchange(info)
        info.smart_queries.clear()
        ex.set_referral("alice")
        # Exactly one new query, against the account-factory contract,
        # with the externally-tagged `User(Name(_))` variant.
        assert len(info.smart_queries) == 1
        contract, msg = info.smart_queries[0]
        assert contract == ACCOUNT_FACTORY_CONTRACT
        assert msg == {"user": {"name": "alice"}}
        # The resolved index from the FakeInfo response (7) is what
        # ends up in the wire `referrer` field. Referee remains the
        # signer's auto-resolved 42.
        assert _last_inner_msg(info) == {
            "referral": {
                "set_referral": {
                    "referrer": 7,
                    "referee": 42,
                },
            },
        }

    def test_rejects_empty_username(self) -> None:
        """An empty string referrer is rejected without round-tripping to the chain."""

        # No reason to ship an empty username — the chain would reject
        # it anyway, and short-circuiting saves a query. The error is a
        # ValueError to match the rest of the value-domain checks.
        info = FakeInfo()
        ex = _exchange(info)
        info.smart_queries.clear()
        with pytest.raises(ValueError, match="non-empty"):
            ex.set_referral("")
        # Belt-and-braces: confirm we did NOT actually query the chain.
        assert info.smart_queries == []


class TestSetReferralWrapper:
    def test_wraps_in_perps_contract_execute_with_no_funds(self) -> None:
        """set_referral targets the perps contract with empty funds."""

        # The referral message routes through perps `ExecuteMsg::Referral`,
        # so the contract is the perps contract, NOT the account factory
        # (which is only queried for username resolution).
        info = FakeInfo()
        ex = _exchange(info)
        ex.set_referral(7)
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}


class TestLiquidate:
    def test_full_wire_shape(self) -> None:
        """liquidate(addr) emits {maintain: {liquidate: {user: <addr>}}}."""

        # The `user` field is the typed Addr verbatim — no transform.
        # The contract handler is permissionless (see
        # `dango/perps/src/maintain/liquidate.rs`); it's the chain's
        # job to decide whether the target is actually liquidatable.
        info = FakeInfo()
        ex = _exchange(info)
        ex.liquidate(_TARGET_ADDRESS)
        assert _last_inner_msg(info) == {
            "maintain": {"liquidate": {"user": _TARGET_ADDRESS}},
        }

    def test_target_address_is_distinct_from_signer(self) -> None:
        """Pin that the wire field carries the user argument, not the signer's own address."""

        # Regression guard: a refactor that accidentally substituted
        # `self.address` for the `user` parameter would silently make
        # every liquidate self-liquidate. The fixed _TARGET_ADDRESS
        # differs from _DEMO_ADDRESS, so this assertion catches the
        # confusion.
        info = FakeInfo()
        ex = _exchange(info)
        ex.liquidate(_TARGET_ADDRESS)
        inner = _last_inner_msg(info)["maintain"]["liquidate"]
        assert inner["user"] == _TARGET_ADDRESS
        assert inner["user"] != _DEMO_ADDRESS

    def test_wraps_in_perps_contract_execute_with_no_funds(self) -> None:
        """liquidate targets the perps contract with empty funds."""

        info = FakeInfo()
        ex = _exchange(info)
        ex.liquidate(_TARGET_ADDRESS)
        execute = info.broadcasted[-1]["msgs"][0]["execute"]
        assert execute["contract"] == PERPS_CONTRACT_MAINNET
        assert execute["funds"] == {}


class TestPipeline:
    def test_add_liquidity_advances_nonce(self) -> None:
        """add_liquidity goes through the simulate/sign/broadcast pipeline."""

        # `_send_action` is shared across every Exchange method and
        # already covered extensively in test_exchange.py. Pin that
        # each of the four new methods actually delegates to it
        # (rather than bypassing the pipeline via a stale path).
        info = FakeInfo()
        ex = _exchange(info)
        starting = ex.signer.next_nonce
        ex.add_liquidity(1000)
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        assert ex.signer.next_nonce == (starting or 0) + 1

    def test_remove_liquidity_advances_nonce(self) -> None:
        """remove_liquidity goes through the simulate/sign/broadcast pipeline."""

        info = FakeInfo()
        ex = _exchange(info)
        starting = ex.signer.next_nonce
        ex.remove_liquidity(1_000)
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        assert ex.signer.next_nonce == (starting or 0) + 1

    def test_set_referral_advances_nonce(self) -> None:
        """set_referral goes through the simulate/sign/broadcast pipeline."""

        info = FakeInfo()
        ex = _exchange(info)
        starting = ex.signer.next_nonce
        ex.set_referral(7)
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        assert ex.signer.next_nonce == (starting or 0) + 1

    def test_liquidate_advances_nonce(self) -> None:
        """liquidate goes through the simulate/sign/broadcast pipeline."""

        info = FakeInfo()
        ex = _exchange(info)
        starting = ex.signer.next_nonce
        ex.liquidate(_TARGET_ADDRESS)
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1
        assert ex.signer.next_nonce == (starting or 0) + 1

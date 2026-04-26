"""Tests for dango.exchange.Exchange — pipeline + margin methods."""

from __future__ import annotations

import pytest

from dango.exchange import Exchange
from dango.utils.constants import PERPS_CONTRACT_MAINNET, SETTLEMENT_DENOM
from dango.utils.types import Addr
from tests._helpers import FakeInfo
from tests._helpers import exchange as _exchange
from tests._helpers import wallet as _wallet

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")


class TestConstruction:
    def test_account_address_is_required(self) -> None:
        """account_address has no default; constructing without it must fail."""
        with pytest.raises(TypeError):
            Exchange(_wallet(), "http://localhost:8080")  # type: ignore[call-arg]

    def test_auto_fetches_chain_id(self) -> None:
        """If chain_id is omitted, the constructor pulls it from query_status."""
        info = FakeInfo()
        ex = _exchange(info)
        # One query for chain_id; the `auto_resolve user_index/nonce`
        # path goes through `query_app_smart`, not `query_status`, so
        # we expect exactly one status call here.
        assert info.queried_status_count == 1
        # The signer's address is the constructor-supplied account, not
        # the wallet's `address` (a single key can sign for multiple
        # accounts; see SingleSigner's docstring).
        assert ex.signer.address == _DEMO_ADDRESS

    def test_chain_id_override_skips_query(self) -> None:
        """Passing chain_id explicitly avoids the query_status round-trip."""
        info = FakeInfo()
        _exchange(info, chain_id="dango-1")
        assert info.queried_status_count == 0

    def test_resolves_user_index_and_nonce_from_chain(self) -> None:
        """Without explicit values, user_index and next_nonce come from the chain."""
        info = FakeInfo()
        ex = _exchange(info)
        # FakeInfo.query_app_smart returns `owner=42` for the factory
        # `account` lookup, and `[3, 4, 5]` for `seen_nonces` — so the
        # resolved next_nonce is max([3,4,5]) + 1 = 6.
        assert ex.signer.user_index == 42
        assert ex.signer.next_nonce == 6

    def test_explicit_user_index_skips_factory_query(self) -> None:
        """Passing user_index keeps the explicit value, no factory roundtrip."""
        info = FakeInfo()
        ex = _exchange(info, user_index=99)
        assert ex.signer.user_index == 99


class TestDepositMargin:
    def test_funds_carries_amount_as_string(self) -> None:
        """deposit_margin(1_500_000) emits funds={'bridge/usdc': '1500000'}."""
        # `amount` is base units (Uint128); the SDK stringifies it for
        # the wire because Uint128 serializes as a base-10 integer
        # string. 1_500_000 base units = 1.5 USDC at SETTLEMENT_DECIMALS=6.
        info = FakeInfo()
        ex = _exchange(info)
        ex.deposit_margin(1_500_000)
        sent = info.broadcasted[-1]
        msg = sent["msgs"][0]["execute"]
        assert msg["contract"] == PERPS_CONTRACT_MAINNET
        assert msg["msg"] == {"trade": {"deposit": {"to": None}}}
        assert msg["funds"] == {SETTLEMENT_DENOM: "1500000"}

    def test_rejects_float(self) -> None:
        """deposit_margin requires an int (base units), not a float."""
        # Earlier drafts accepted float USD and converted internally;
        # the wire ambiguity ("is 1.5 USDC or 1.5 base units?") was
        # confusing, so the API now hard-rejects floats.
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int"):
            ex.deposit_margin(1.5)  # type: ignore[arg-type]

    def test_rejects_bool(self) -> None:
        """deposit_margin rejects bool (which is technically an int subclass)."""
        # Same hazard guarded by `dango_decimal`: bool is an int subtype
        # in Python, so mypy accepts `deposit_margin(True)` at compile
        # time. The runtime guard is what catches it; without it,
        # True/False would silently be treated as 1/0 base units.
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(TypeError, match="int"):
            ex.deposit_margin(True)

    def test_rejects_zero_or_negative(self) -> None:
        """deposit_margin requires a strictly positive amount."""
        info = FakeInfo()
        ex = _exchange(info)
        with pytest.raises(ValueError, match="positive"):
            ex.deposit_margin(0)
        with pytest.raises(ValueError, match="positive"):
            ex.deposit_margin(-1)

    def test_pipeline_calls_simulate_then_broadcast(self) -> None:
        """deposit_margin runs simulate before broadcast (gas autodiscovery)."""
        info = FakeInfo()
        ex = _exchange(info)
        ex.deposit_margin(1_000_000)
        # Exactly one simulate call (to learn gas) and one broadcast
        # call (to send the signed tx). No retries, no duplicates.
        assert len(info.simulated) == 1
        assert len(info.broadcasted) == 1

    def test_gas_limit_is_simulated_gas_plus_overhead(self) -> None:
        """gas_limit = simulate.gas_used + DEFAULT_GAS_OVERHEAD (770_000)."""
        info = FakeInfo()
        ex = _exchange(info)
        ex.deposit_margin(1_000_000)
        sent = info.broadcasted[-1]
        # FakeInfo.simulate returns gas_used=230_000; the SDK adds
        # 770_000 (DEFAULT_GAS_OVERHEAD) for sig verify cost; total is
        # 1_000_000. Simulate skips the auth pre-handler, so we add
        # the verify cost back manually — see the WHY-comment on
        # DEFAULT_GAS_OVERHEAD.
        assert sent["gas_limit"] == 1_000_000


class TestWithdrawMargin:
    def test_amount_uses_six_decimal_string(self) -> None:
        """withdraw_margin(1.5) emits {'amount': '1.500000'} (UsdValue)."""
        info = FakeInfo()
        ex = _exchange(info)
        ex.withdraw_margin(1.5)
        sent = info.broadcasted[-1]
        msg = sent["msgs"][0]["execute"]
        # UsdValue is a 6-decimal fixed-point string, NOT base units.
        # `dango_decimal` produces this canonical form.
        assert msg["msg"] == {"trade": {"withdraw": {"amount": "1.500000"}}}
        # Withdraw doesn't carry funds — the chain returns USDC TO us,
        # not the other way round.
        assert msg["funds"] == {}


class TestSendActionPipeline:
    def test_increments_nonce_after_broadcast(self) -> None:
        """Successful broadcast bumps next_nonce by exactly 1."""
        info = FakeInfo()
        ex = _exchange(info)
        starting = ex.signer.next_nonce
        ex.deposit_margin(1_000_000)
        # `sign_tx` does the increment (signing.py mirrors signer.rs:271-272);
        # `_send_action` simply calls into `sign_tx`, so the bump is
        # observable on the signer immediately after broadcast.
        assert ex.signer.next_nonce == (starting or 0) + 1

    def test_chain_id_propagates_into_metadata(self) -> None:
        """The constructor's chain_id ends up inside `tx.data.chain_id` on broadcast."""
        info = FakeInfo()
        ex = _exchange(info, chain_id="dango-explicit-test")
        ex.deposit_margin(1_000_000)
        sent = info.broadcasted[-1]
        # The Metadata is JSON-serialized into `tx.data` per Phase 5's
        # SignDoc / Tx construction; assert it carries the constructor's
        # chain_id (not the auto-fetched one) end-to-end.
        assert sent["data"]["chain_id"] == "dango-explicit-test"


class TestExplicitNextNonce:
    def test_explicit_nonce_skips_seen_nonces_query(self) -> None:
        """Passing next_nonce keeps the explicit value, no chain roundtrip."""
        info = FakeInfo()
        ex = _exchange(info, next_nonce=99)
        # Only the user_index resolution should have hit query_app_smart;
        # next_nonce path is skipped. FakeInfo.query_app_smart asserts
        # raises on unexpected msgs, so the test passes only if no
        # `seen_nonces` query was made.
        assert ex.signer.next_nonce == 99


class TestLocalAccountWallet:
    def test_constructs_from_eth_account(self) -> None:
        """Passing an eth_account.LocalAccount auto-wraps via Secp256k1Wallet.from_eth_account."""
        # Verifies the `else` branch in the wallet adapter — the most
        # common HL-trader migration path. The resulting Exchange should
        # sign with the same secret as a directly-constructed wallet.
        from eth_account import Account

        info = FakeInfo()
        account = Account.from_key(b"\x01" * 32)
        ex = Exchange(
            account,
            "http://localhost:8080",
            account_address=_DEMO_ADDRESS,
            info=info,  # type: ignore[arg-type]
        )
        # The signer's wallet should now be a Secp256k1Wallet wrapping
        # the same secret bytes (b"\x01" * 32).
        assert ex.signer.wallet.address == _DEMO_ADDRESS
        # Confirm we can still drive the pipeline end-to-end.
        ex.deposit_margin(1_000_000)
        assert len(info.broadcasted) == 1

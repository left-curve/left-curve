"""Tests for dango.utils.signing.SingleSigner — state, queries, build/sign Tx."""

from __future__ import annotations

import base64
from typing import Any, cast

import pytest

from dango.utils.constants import ACCOUNT_FACTORY_CONTRACT
from dango.utils.signing import Secp256k1Wallet, SingleSigner
from dango.utils.types import Addr, Metadata, Tx, UnsignedTx

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")
_OTHER_ADDRESS = Addr("0x000000000000000000000000000000000000feed")


def _wallet() -> Secp256k1Wallet:
    # Fixed secret keeps signature outputs deterministic across tests so we
    # can compare credential shapes without recomputing them by hand.
    return Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)


# `_MockInfo` is a structural Info stand-in: it satisfies the `_QueryClient`
# Protocol that SingleSigner uses, without requiring Phase 6's real Info.
# Responses are keyed by (contract_addr, top-level msg variant) since Dango's
# query enums are externally-tagged (e.g. `{"account": {...}}`,
# `{"seen_nonces": {}}`), so the variant name is the first key in the dict.
class _MockInfo:
    """Records query_app_smart calls and returns canned responses."""

    def __init__(self, responses: dict[tuple[Addr, str], Any]) -> None:
        self.responses = responses
        self.calls: list[tuple[Addr, dict[str, Any]]] = []

    def query_app_smart(
        self,
        contract: Addr,
        msg: dict[str, Any],
        *,
        height: int | None = None,
    ) -> Any:
        self.calls.append((contract, msg))
        # Match by (contract, first key in msg) since msgs are externally-tagged enums.
        key = (contract, next(iter(msg)))
        return self.responses[key]


def _factory_user_response(owner: int, *, index: int = 0) -> dict[str, Any]:
    # The contract returns the full User struct; we only consume `owner`,
    # but the fixture mirrors the wire shape so a response-shape regression
    # in SingleSigner is detectable via these tests.
    return {"index": index, "owner": owner}


def _info_for(address: Addr, *, user_index: int, seen_nonces: list[int]) -> _MockInfo:
    return _MockInfo(
        {
            (Addr(ACCOUNT_FACTORY_CONTRACT), "account"): _factory_user_response(user_index),
            (address, "seen_nonces"): seen_nonces,
        }
    )


class TestConstruction:
    def test_explicit_user_index_and_nonce(self) -> None:
        """Construct with explicit values stores them as-is."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=42, next_nonce=7)
        assert signer.user_index == 42
        assert signer.next_nonce == 7

    def test_unresolved_state_defaults_to_none(self) -> None:
        """Without explicit values, both fields are None."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        assert signer.user_index is None
        assert signer.next_nonce is None

    def test_address_property(self) -> None:
        """Address is taken from the constructor arg, not from the wallet."""
        # Use an address that differs from the wallet's _DEMO_ADDRESS to
        # confirm we don't silently pull from `wallet.address`. A single key
        # can sign for multiple Dango accounts; the signer is bound to one.
        wallet = _wallet()
        signer = SingleSigner(wallet, _OTHER_ADDRESS)
        assert signer.address == _OTHER_ADDRESS
        assert wallet.address == _DEMO_ADDRESS


class TestQueryUserIndex:
    def test_calls_account_factory_contract(self) -> None:
        """query_user_index posts the right (contract, msg) tuple."""
        info = _info_for(_DEMO_ADDRESS, user_index=42, seen_nonces=[])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        signer.query_user_index(info)
        # Exactly one call, to the account-factory contract, with the
        # externally-tagged `account` variant carrying our address.
        assert info.calls == [
            (Addr(ACCOUNT_FACTORY_CONTRACT), {"account": {"address": _DEMO_ADDRESS}}),
        ]

    def test_returns_owner_field(self) -> None:
        """The response's `owner` field is the user_index."""
        info = _info_for(_DEMO_ADDRESS, user_index=42, seen_nonces=[])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        assert signer.query_user_index(info) == 42

    def test_does_not_mutate_state(self) -> None:
        """Calling query_user_index does not write to self.user_index."""
        info = _info_for(_DEMO_ADDRESS, user_index=42, seen_nonces=[])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        signer.query_user_index(info)
        # The query is a pure observation; mutation only happens via
        # auto_resolve() or explicit assignment.
        assert signer.user_index is None


class TestQueryNextNonce:
    def test_empty_seen_nonces_returns_zero(self) -> None:
        """An account that has never sent a tx has next_nonce=0."""
        info = _info_for(_DEMO_ADDRESS, user_index=0, seen_nonces=[])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        assert signer.query_next_nonce(info) == 0

    def test_nonzero_seen_nonces_returns_max_plus_one(self) -> None:
        """Active account: next_nonce = max(seen) + 1."""
        info = _info_for(_DEMO_ADDRESS, user_index=0, seen_nonces=[0, 1, 4, 5])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        assert signer.query_next_nonce(info) == 6

    def test_does_not_mutate_state(self) -> None:
        """Calling query_next_nonce does not write to self.next_nonce."""
        info = _info_for(_DEMO_ADDRESS, user_index=0, seen_nonces=[3])
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS)
        signer.query_next_nonce(info)
        assert signer.next_nonce is None


class TestAutoResolve:
    def test_populates_both_fields(self) -> None:
        """auto_resolve sets user_index and next_nonce from chain."""
        info = _info_for(_DEMO_ADDRESS, user_index=42, seen_nonces=[10])
        signer = SingleSigner.auto_resolve(_wallet(), _DEMO_ADDRESS, info)
        assert signer.user_index == 42
        # max(seen) + 1 — the next nonce after 10 is 11.
        assert signer.next_nonce == 11
        # Sanity: both queries actually fired.
        assert len(info.calls) == 2


class TestBuildUnsignedTx:
    def test_returns_correct_wire_shape(self) -> None:
        """UnsignedTx has sender, msgs, data, and NO gas_limit."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1, next_nonce=0)
        tx = signer.build_unsigned_tx([], "dango-1")
        # `gas_limit` is set by the caller via `sign_tx`, after they've
        # asked Info.simulate() how much gas to allow. UnsignedTx must
        # NOT have it.
        keys = set(cast(dict[str, Any], tx).keys())
        assert keys == {"sender", "msgs", "data"}

    def test_metadata_carries_chain_state(self) -> None:
        """data field is a Metadata-shaped dict with user_index, chain_id, nonce, expiry."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=42, next_nonce=7)
        tx: UnsignedTx = signer.build_unsigned_tx([], "dango-1")
        metadata = tx["data"]
        assert metadata["user_index"] == 42
        assert metadata["chain_id"] == "dango-1"
        assert metadata["nonce"] == 7
        # `expiry=None` is intentional — the Rust signer also leaves it
        # unset; gas-meter expiry is reserved for future use.
        assert metadata["expiry"] is None

    def test_raises_if_user_index_unresolved(self) -> None:
        """RuntimeError when user_index is None."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, next_nonce=0)
        with pytest.raises(RuntimeError, match="user_index unresolved"):
            signer.build_unsigned_tx([], "dango-1")

    def test_raises_if_next_nonce_unresolved(self) -> None:
        """RuntimeError when next_nonce is None."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1)
        with pytest.raises(RuntimeError, match="next_nonce unresolved"):
            signer.build_unsigned_tx([], "dango-1")


class TestSignTx:
    def test_returns_tx_with_credential(self) -> None:
        """Tx has all UnsignedTx fields plus gas_limit and credential."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1, next_nonce=0)
        tx: Tx = signer.sign_tx([], "dango-1", 100_000)
        keys = set(cast(dict[str, Any], tx).keys())
        assert keys == {"sender", "gas_limit", "msgs", "data", "credential"}
        assert tx["gas_limit"] == 100_000

    def test_credential_is_standard_secp256k1(self) -> None:
        """credential is {"Standard": {"key_hash": <hex>, "signature": {"Secp256k1": <base64>}}}."""
        wallet = _wallet()
        signer = SingleSigner(wallet, _DEMO_ADDRESS, user_index=1, next_nonce=0)
        tx = signer.sign_tx([], "dango-1", 100_000)

        credential = cast(dict[str, Any], tx["credential"])
        assert "Standard" in credential
        standard = credential["Standard"]
        # key_hash is the on-chain identifier the contract uses to look up
        # the pubkey; here we verify it matches the wallet's key_hash.
        assert standard["key_hash"] == wallet.key_hash
        assert "Secp256k1" in standard["signature"]
        # The Secp256k1 envelope carries 64 bytes of (r || s) base64-encoded.
        sig_bytes = base64.b64decode(standard["signature"]["Secp256k1"])
        assert len(sig_bytes) == 64

    def test_increments_nonce_on_success(self) -> None:
        """After sign_tx, self.next_nonce == old_nonce + 1."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1, next_nonce=5)
        signer.sign_tx([], "dango-1", 100_000)
        assert signer.next_nonce == 6

    def test_uses_old_nonce_in_metadata(self) -> None:
        """The signed Tx's data.nonce is the value BEFORE incrementing."""
        # Critical invariant: the tx must carry the nonce we promised when
        # we started signing, not the post-increment value. The chain will
        # reject any tx whose nonce does not match the next-expected slot.
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1, next_nonce=5)
        tx = signer.sign_tx([], "dango-1", 100_000)
        metadata: Metadata = tx["data"]
        assert metadata["nonce"] == 5
        assert signer.next_nonce == 6

    def test_consecutive_signs_use_consecutive_nonces(self) -> None:
        """Two sign_tx calls produce txs with nonces N and N+1."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1, next_nonce=0)
        first = signer.sign_tx([], "dango-1", 100_000)
        second = signer.sign_tx([], "dango-1", 100_000)
        assert first["data"]["nonce"] == 0
        assert second["data"]["nonce"] == 1
        assert signer.next_nonce == 2

    def test_increments_nonce_even_if_signing_throws(self) -> None:
        """A signing failure still advances next_nonce so retries cannot reuse the slot."""

        # We swap in a sabotaged wallet whose `sign` raises. Atomic-safe
        # nonce semantics require the increment to happen *before* signing
        # (mirrors signer.rs:271-272); otherwise a flaky signer + retry
        # loop would replay the same nonce.
        class _BoomWallet:
            address = _DEMO_ADDRESS
            key = cast(Any, {"Secp256k1": "AA=="})
            key_hash = cast(Any, "00" * 32)

            def sign(self, sign_doc: Any) -> Any:
                raise RuntimeError("hardware key disconnected")

        signer = SingleSigner(cast(Any, _BoomWallet()), _DEMO_ADDRESS, user_index=1, next_nonce=5)
        with pytest.raises(RuntimeError, match="hardware"):
            signer.sign_tx([], "dango-1", 100_000)
        assert signer.next_nonce == 6

    def test_sign_receives_signdoc_with_messages_field(self) -> None:
        """The SignDoc handed to wallet.sign uses `messages` (not `msgs`)."""

        # Tx and UnsignedTx use `msgs` but SignDoc uses `messages`. The
        # asymmetry is a Rust serde quirk; mypy catches a mis-spelled key
        # at compile time, but a runtime spy guards against future refactors
        # that reach around the TypedDict (e.g. via cast or **kwargs).
        captured: dict[str, Any] = {}

        class _SpyWallet:
            address = _DEMO_ADDRESS
            key = cast(Any, {"Secp256k1": "AA=="})
            key_hash = cast(Any, "00" * 32)

            def sign(self, sign_doc: Any) -> Any:
                captured["sign_doc"] = sign_doc
                return cast(Any, {"Secp256k1": base64.b64encode(b"\x00" * 64).decode("ascii")})

        signer = SingleSigner(cast(Any, _SpyWallet()), _DEMO_ADDRESS, user_index=1, next_nonce=0)
        signer.sign_tx([], "dango-1", 100_000)
        assert set(captured["sign_doc"].keys()) == {"sender", "gas_limit", "messages", "data"}

    def test_raises_if_user_index_unresolved(self) -> None:
        """RuntimeError when user_index is None."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, next_nonce=0)
        with pytest.raises(RuntimeError, match="user_index unresolved"):
            signer.sign_tx([], "dango-1", 100_000)

    def test_raises_if_next_nonce_unresolved(self) -> None:
        """RuntimeError when next_nonce is None."""
        signer = SingleSigner(_wallet(), _DEMO_ADDRESS, user_index=1)
        with pytest.raises(RuntimeError, match="next_nonce unresolved"):
            signer.sign_tx([], "dango-1", 100_000)

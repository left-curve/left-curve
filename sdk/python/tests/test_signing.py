"""Tests for dango.utils.signing — canonical JSON, Secp256k1 wallet, signing pipeline."""

from __future__ import annotations

import base64
import hashlib
from typing import cast

import pytest
from eth_keys.datatypes import Signature as EthSignature

from dango.utils.signing import (
    Secp256k1Wallet,
    Wallet,
    sign_doc_canonical_json,
    sign_doc_sha256,
)
from dango.utils.types import Addr, Metadata, Nonce, SignDoc, UserIndex

_DEMO_ADDRESS = Addr("0x000000000000000000000000000000000000beef")


def _demo_sign_doc() -> SignDoc:
    # `messages=[]` is fine for canonical-JSON unit tests but would be rejected
    # by Rust's `NonEmpty<Vec<Message>>` contract on the wire.
    return SignDoc(
        sender=_DEMO_ADDRESS,
        gas_limit=100_000,
        messages=[],
        data=Metadata(
            user_index=UserIndex(1),
            chain_id="dango-1",
            nonce=Nonce(0),
            expiry=None,
        ),
    )


class TestCanonicalJson:
    def test_keys_sorted_alphabetically(self) -> None:
        """Top-level object keys are sorted alphabetically regardless of insertion order."""
        # Use a generic dict (not a SignDoc literal) so we can vary insertion
        # order without fighting TypedDict's strict-mode key checks.
        doc: dict[str, object] = {
            "sender": "x",
            "gas_limit": 1,
            "messages": [],
            "data": {},
        }
        out = sign_doc_canonical_json(cast(SignDoc, doc))
        assert out == b'{"data":{},"gas_limit":1,"messages":[],"sender":"x"}'

    def test_no_whitespace_around_separators(self) -> None:
        """Canonical JSON has no spaces around `:` or `,`."""
        doc: dict[str, object] = {"a": 1, "b": 2}
        out = sign_doc_canonical_json(cast(SignDoc, doc))
        assert b": " not in out
        assert b", " not in out

    def test_array_order_preserved(self) -> None:
        """Arrays are NOT sorted (only object keys are)."""
        doc: dict[str, object] = {"messages": [3, 1, 2], "sender": "x"}
        out = sign_doc_canonical_json(cast(SignDoc, doc))
        assert b"[3,1,2]" in out

    def test_nested_keys_sorted_recursively(self) -> None:
        """Nested object keys are also sorted (matches grug sort_all_objects)."""
        doc: dict[str, object] = {"data": {"z": 1, "a": 2}, "sender": "x"}
        out = sign_doc_canonical_json(cast(SignDoc, doc))
        assert out == b'{"data":{"a":2,"z":1},"sender":"x"}'

    def test_utf8_encoding_no_ascii_escapes(self) -> None:
        """Non-ASCII characters are emitted as UTF-8 bytes, not \\uXXXX escapes."""
        # Matches serde_json's default UTF-8 output.
        doc: dict[str, object] = {"chain_id": "dango-é"}
        out = sign_doc_canonical_json(cast(SignDoc, doc))
        assert "é".encode() in out
        assert b"\\u" not in out

    def test_golden_signdoc_byte_string(self) -> None:
        """A real-shape SignDoc encodes to a frozen byte string (regression gate)."""
        # Pins the canonical-JSON contract: object keys sorted alphabetically
        # at every level, no whitespace, integers as numbers, and `None` in
        # `data` (the chain's `Metadata` struct) STRIPPED — the chain uses
        # `serde_with::skip_serializing_none` and our canonical bytes must
        # match what it reconstructs for signature verification. Any silent
        # change to the encoder will break this fixture.
        expected = (
            b'{"data":{"chain_id":"dango-1","nonce":0,"user_index":1},'
            b'"gas_limit":100000,"messages":[],'
            b'"sender":"0x000000000000000000000000000000000000beef"}'
        )
        assert sign_doc_canonical_json(_demo_sign_doc()) == expected


class TestSha256Digest:
    def test_matches_manual_sha256(self) -> None:
        """sign_doc_sha256 equals SHA-256 of the canonical JSON bytes."""
        doc = _demo_sign_doc()
        assert sign_doc_sha256(doc) == hashlib.sha256(sign_doc_canonical_json(doc)).digest()

    def test_digest_is_32_bytes(self) -> None:
        """SHA-256 always produces a 32-byte digest."""
        assert len(sign_doc_sha256(_demo_sign_doc())) == 32


class TestSecp256k1Wallet:
    def test_random_produces_32_byte_secret(self) -> None:
        """random() yields a wallet with a valid 32-byte secret."""
        w = Secp256k1Wallet.random(_DEMO_ADDRESS)
        assert len(w.secret_bytes) == 32

    def test_random_is_actually_random(self) -> None:
        """Two random wallets have different secrets (probabilistic but practically certain)."""
        a = Secp256k1Wallet.random(_DEMO_ADDRESS)
        b = Secp256k1Wallet.random(_DEMO_ADDRESS)
        assert a.secret_bytes != b.secret_bytes

    def test_from_bytes_invalid_length_raises(self) -> None:
        """from_bytes rejects non-32-byte secrets."""
        with pytest.raises(ValueError, match="32 bytes"):
            Secp256k1Wallet.from_bytes(b"\x01" * 16, _DEMO_ADDRESS)

    def test_from_bytes_zero_secret_raises(self) -> None:
        """A zero secret is out of the valid range [1, n-1]."""
        with pytest.raises(ValueError, match="out of range"):
            Secp256k1Wallet.from_bytes(b"\x00" * 32, _DEMO_ADDRESS)

    def test_from_bytes_secret_at_curve_order_raises(self) -> None:
        """A secret >= n is out of range; eth_keys would reject it as well."""
        n_hex = "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"
        with pytest.raises(ValueError, match="out of range"):
            Secp256k1Wallet.from_bytes(bytes.fromhex(n_hex), _DEMO_ADDRESS)

    def test_address_property(self) -> None:
        """Address is the constructor-supplied value, NOT derived from the key."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        assert w.address == _DEMO_ADDRESS

    def test_public_key_is_33_bytes_compressed(self) -> None:
        """Compressed pubkey has the 0x02/0x03 parity byte plus 32-byte x coord."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        assert len(w.public_key_compressed) == 33
        assert w.public_key_compressed[0] in (0x02, 0x03)

    def test_key_wire_shape(self) -> None:
        """Key wire form is `{"secp256k1": "<base64 of 33-byte compressed pubkey>"}`."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        key = w.key
        # The Key TypedDict union declares `secp256k1` as one variant; we
        # narrow via cast to access it without mypy fighting the union.
        secp_key = cast(dict[str, str], key)
        assert "secp256k1" in secp_key
        assert base64.b64decode(secp_key["secp256k1"]) == w.public_key_compressed

    def test_key_hash_is_sha256_of_compressed_pubkey(self) -> None:
        """key_hash is hex(sha256(compressed_pubkey)), uppercase, 64 chars."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        expected = hashlib.sha256(w.public_key_compressed).hexdigest().upper()
        assert w.key_hash == expected
        assert len(w.key_hash) == 64

    def test_satisfies_wallet_protocol(self) -> None:
        """Secp256k1Wallet satisfies the Wallet Protocol via duck typing."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        assert isinstance(w, Wallet)


class TestSign:
    def test_returns_secp256k1_signature_envelope(self) -> None:
        """sign() returns a `{"secp256k1": "<base64 64 bytes>"}` envelope."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        sig = w.sign(_demo_sign_doc())
        secp_sig = cast(dict[str, str], sig)
        assert "secp256k1" in secp_sig
        assert len(base64.b64decode(secp_sig["secp256k1"])) == 64

    def test_signature_is_deterministic(self) -> None:
        """RFC 6979 deterministic-k means the same key signs the same doc identically."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        doc = _demo_sign_doc()
        assert w.sign(doc) == w.sign(doc)

    def test_golden_signature_envelope(self) -> None:
        """Frozen secret + frozen SignDoc produces a frozen signature envelope."""
        # Self-consistency regression gate: pins the entire pipeline (canonical
        # JSON + SHA-256 + RFC-6979 deterministic-k ECDSA + low-S + 64-byte
        # truncation + base64). True Rust-cross-check would require generating
        # the value from `sdk/rust/src/secret.rs`'s Secp256k1::sign_transaction
        # with the same inputs; deferred until we have a fixture binary.
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        sig = cast(dict[str, str], w.sign(_demo_sign_doc()))
        assert sig["secp256k1"] == (
            "Wa2xpNxLKznjA/wpOySinJKOzgHHRQKA4M74zo4pqycceCobIvJD1pSZ9EK7Lfly4V9gOw56WpeuAyDh596Nlg=="
        )

    def test_signature_recovers_to_pubkey(self) -> None:
        """Round-trip: signing then ECDSA-recover returns the wallet's compressed pubkey."""
        w = Secp256k1Wallet.from_bytes(b"\x01" * 32, _DEMO_ADDRESS)
        doc = _demo_sign_doc()
        secp_sig = cast(dict[str, str], w.sign(doc))
        sig_bytes = base64.b64decode(secp_sig["secp256k1"])
        digest = sign_doc_sha256(doc)
        # We dropped the recovery byte during signing, so iterate v=0,1 to
        # find which one recovers to the wallet's pubkey.
        recovered = None
        for v in (0, 1):
            try:
                candidate = EthSignature(sig_bytes + bytes([v])).recover_public_key_from_msg_hash(
                    digest
                )
                if candidate.to_compressed_bytes() == w.public_key_compressed:
                    recovered = candidate
                    break
            except Exception:
                continue
        assert recovered is not None, "neither v=0 nor v=1 recovered to the wallet pubkey"


class TestFromMnemonic:
    # Universal BIP-39 test vector: 12 words `abandon abandon ... about` plus
    # path m/44'/60'/0'/0/0 yields this canonical private key, cross-checked
    # against the Rust k256/bip32 implementation.
    _ABANDON_MNEMONIC = " ".join(["abandon"] * 11 + ["about"])
    _ABANDON_ETH_KEY_HEX = "1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727"

    def test_known_test_vector(self) -> None:
        """The canonical 12-word `abandon ... about` mnemonic derives to the BIP-44 known key."""
        w = Secp256k1Wallet.from_mnemonic(self._ABANDON_MNEMONIC, _DEMO_ADDRESS)
        assert w.secret_bytes.hex() == self._ABANDON_ETH_KEY_HEX

    def test_custom_coin_type(self) -> None:
        """Passing coin_type changes the derivation path and yields a different key."""
        eth = Secp256k1Wallet.from_mnemonic(self._ABANDON_MNEMONIC, _DEMO_ADDRESS, coin_type=60)
        cosmos = Secp256k1Wallet.from_mnemonic(self._ABANDON_MNEMONIC, _DEMO_ADDRESS, coin_type=118)
        assert eth.secret_bytes != cosmos.secret_bytes


class TestFromEthAccount:
    def test_extracts_secret(self) -> None:
        """from_eth_account uses the LocalAccount's underlying secp256k1 secret bytes."""
        from eth_account import Account

        local = Account.from_key(b"\x01" * 32)
        w = Secp256k1Wallet.from_eth_account(local, _DEMO_ADDRESS)
        assert w.secret_bytes == bytes(local.key)

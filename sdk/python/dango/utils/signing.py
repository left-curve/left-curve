"""Wallets, SignDoc canonical JSON, and Secp256k1 signing primitives."""

from __future__ import annotations

import base64
import hashlib
import json
import secrets
from typing import TYPE_CHECKING, Protocol, cast, runtime_checkable

# eth_keys exposes raw secp256k1 with RFC 6979 deterministic-k. The stdlib
# `cryptography` package doesn't ship secp256k1 by default (only NIST curves),
# and pulling in `coincurve` would duplicate functionality already provided by
# eth-account's transitive dep. We import from `eth_keys.datatypes` rather
# than the package root because mypy can't see the top-level `keys` re-export
# under strict mode.
from eth_keys.datatypes import PrivateKey

from dango.utils.types import (
    Addr,
    Binary,
    Hash256,
    Key,
    Signature,
    SignDoc,
)

if TYPE_CHECKING:
    from eth_account.signers.local import LocalAccount


# --- Canonical JSON ----------------------------------------------------------


def sign_doc_canonical_json(sign_doc: SignDoc) -> bytes:
    """Encode a SignDoc as canonical JSON (sorted keys recursively, no whitespace)."""
    # Mirrors `grug::SignData::to_prehash_sign_data`, which calls
    # `to_json_value().sort_all_objects().to_json_vec()`. `sort_keys=True` in
    # json.dumps is recursive on every nested object, matching that contract.
    # `ensure_ascii=False` keeps non-ASCII chars as UTF-8 bytes (no \uXXXX
    # escapes), matching serde_json's default UTF-8 output.
    return json.dumps(
        sign_doc,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def sign_doc_sha256(sign_doc: SignDoc) -> bytes:
    """SHA-256 digest of a SignDoc's canonical JSON; the 32-byte payload that gets signed."""
    # Exposed (not just a private helper) so tests and integration code can
    # verify the digest without re-deriving the canonical-JSON contract.
    return hashlib.sha256(sign_doc_canonical_json(sign_doc)).digest()


# --- Wallet protocol ---------------------------------------------------------


@runtime_checkable
class Wallet(Protocol):
    """Abstract signing identity; future Passkey/Session wallets will satisfy this too."""

    @property
    def address(self) -> Addr: ...

    @property
    def key(self) -> Key: ...

    @property
    def key_hash(self) -> Hash256: ...

    def sign(self, sign_doc: SignDoc) -> Signature: ...


# --- Secp256k1 wallet --------------------------------------------------------


# Ethereum's BIP-44 coin type. Dango's Secp256k1 wallet uses the same default
# so that an Ethereum mnemonic and a Dango Secp256k1 key derived from the same
# words land on the same private key (not the same Dango address — see
# `from_eth_account` for the address-derivation caveat).
_DEFAULT_COIN_TYPE = 60

# secp256k1 curve order. eth_keys rejects secrets >= n but accepts the
# all-zero secret, which produces the point at infinity. The Rust impl
# (k256::SigningKey::from_bytes) rejects zero too, so we add an explicit
# check here to match.
_SECP256K1_CURVE_ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


class Secp256k1Wallet:
    """Holds a 32-byte secp256k1 secret plus the Dango account address it controls."""

    def __init__(self, secret: bytes, address: Addr) -> None:
        if len(secret) != 32:
            raise ValueError(f"secp256k1 secret must be 32 bytes, got {len(secret)}")
        # Validate the secret is in [1, n-1]. eth_keys rejects values >= n
        # but accepts zero, so we explicitly reject zero here to match the
        # Rust k256 behavior.
        secret_int = int.from_bytes(secret, "big")
        if secret_int == 0 or secret_int >= _SECP256K1_CURVE_ORDER:
            raise ValueError("secp256k1 secret out of range [1, n-1]")
        self._private_key = PrivateKey(secret)
        self._address: Addr = address

    @classmethod
    def random(cls, address: Addr) -> Secp256k1Wallet:
        """Generate a new wallet with a CSPRNG-sourced 32-byte secret."""
        return cls(secrets.token_bytes(32), address)

    @classmethod
    def from_bytes(cls, secret: bytes, address: Addr) -> Secp256k1Wallet:
        """Construct a wallet from a raw 32-byte secret."""
        return cls(secret, address)

    @classmethod
    def from_mnemonic(
        cls,
        mnemonic: str,
        address: Addr,
        *,
        coin_type: int = _DEFAULT_COIN_TYPE,
    ) -> Secp256k1Wallet:
        """BIP-39 mnemonic plus path m/44'/{coin_type}'/0'/0/0 to wallet."""
        # Late import keeps the eth_account hot path out of module-load time;
        # users that only pass raw bytes never need to enable the unaudited
        # HD-wallet feature.
        from eth_account import Account

        # eth-account hides BIP-39/BIP-32 behind a feature flag because that
        # impl is documented "unaudited"; we accept the risk for parity with
        # the Rust SDK, which uses the equivalent bip32 crate path.
        Account.enable_unaudited_hdwallet_features()
        # BIP-44 standard derivation path. The Rust impl uses the identical
        # template; see `sdk/rust/src/secret.rs::Secp256k1::from_mnemonic`.
        path = f"m/44'/{coin_type}'/0'/0/0"
        # eth-account's `from_mnemonic` defaults to an empty BIP-39 passphrase,
        # which matches the Cosmos / Terra Station / Keplr convention the Rust
        # SDK explicitly cites — users typically don't set a passphrase.
        local = Account.from_mnemonic(mnemonic, account_path=path)
        return cls(bytes(local.key), address)

    @classmethod
    def from_eth_account(
        cls,
        account: LocalAccount,
        address: Addr,
    ) -> Secp256k1Wallet:
        """Re-use a LocalAccount's secp256k1 secret as a Dango Secp256k1 key."""
        # IMPORTANT: the Dango address derived from this key uses
        # key_tag=1 (Secp256k1), NOT key_tag=2 (Ethereum). It will therefore
        # differ from any Dango account previously activated through an
        # EIP-712 path with the same Ethereum key. Caller must pass the
        # correct Dango address explicitly; this adapter does not derive it.
        return cls(bytes(account.key), address)

    @property
    def address(self) -> Addr:
        """The Dango account address supplied at construction."""
        return self._address

    @property
    def secret_bytes(self) -> bytes:
        """The raw 32-byte secret. Treat as sensitive."""
        return self._private_key.to_bytes()

    @property
    def public_key_compressed(self) -> bytes:
        """The 33-byte compressed public key (1-byte parity + 32-byte x coord)."""
        return self._private_key.public_key.to_compressed_bytes()

    @property
    def key(self) -> Key:
        """Wire-shape key as `{"Secp256k1": "<base64 of 33-byte compressed pubkey>"}`."""
        # `ByteArray<33>` in grug serializes via Base64Encoder (see
        # `grug/types/src/binary.rs`), not hex. Cast through Key (a TypedDict
        # union) — the variant-specific dict literal is the precise type but
        # callers want the union for polymorphic Wallet code.
        encoded = Binary(base64.b64encode(self.public_key_compressed).decode("ascii"))
        return cast(Key, {"Secp256k1": encoded})

    @property
    def key_hash(self) -> Hash256:
        """SHA-256(compressed_pubkey) as uppercase hex; the on-chain key identifier."""
        # Hash256's wire form is uppercase hex (Hash256 standard in grug);
        # the digest itself is sha256 of the *compressed* pubkey bytes.
        digest = hashlib.sha256(self.public_key_compressed).digest()
        return Hash256(digest.hex().upper())

    def sign(self, sign_doc: SignDoc) -> Signature:
        """Produce a Secp256k1 signature over SHA-256(canonical_json(sign_doc))."""
        digest = sign_doc_sha256(sign_doc)
        # eth_keys returns a 65-byte recoverable signature (r || s || v).
        # Dango's `Signature::Secp256k1` stores the 64-byte non-recoverable
        # form (r || s), so we strip the trailing recovery byte. Verification
        # on-chain uses the pubkey from the StandardCredential.key_hash
        # lookup, so recovery isn't needed.
        sig_65 = self._private_key.sign_msg_hash(digest)
        sig_64 = sig_65.to_bytes()[:64]
        encoded = Binary(base64.b64encode(sig_64).decode("ascii"))
        return cast(Signature, {"Secp256k1": encoded})

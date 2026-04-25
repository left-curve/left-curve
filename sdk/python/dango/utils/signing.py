"""Wallets, SignDoc canonical JSON, Secp256k1 signing, and stateful SingleSigner."""

from __future__ import annotations

import base64
import hashlib
import json
import secrets
from typing import TYPE_CHECKING, Any, Protocol, cast, runtime_checkable

# eth_keys exposes raw secp256k1 with RFC 6979 deterministic-k. The stdlib
# `cryptography` package doesn't ship secp256k1 by default (only NIST curves),
# and pulling in `coincurve` would duplicate functionality already provided by
# eth-account's transitive dep. We import from `eth_keys.datatypes` rather
# than the package root because mypy can't see the top-level `keys` re-export
# under strict mode.
from eth_keys.datatypes import PrivateKey

from dango.utils.constants import ACCOUNT_FACTORY_CONTRACT
from dango.utils.types import (
    Addr,
    Binary,
    Credential,
    Hash256,
    Key,
    Message,
    Metadata,
    Nonce,
    Signature,
    SignDoc,
    StandardCredential,
    Tx,
    UnsignedTx,
    UserIndex,
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


# --- SingleSigner ------------------------------------------------------------


# Phase 6 hasn't shipped yet, so we can't import `dango.info.Info` directly
# without a circular import once it does land. A structural Protocol is the
# stdlib answer: SingleSigner depends only on `query_app_smart`, and any
# duck-typed object (real Info, mocks in tests, alternate transports) that
# exposes that method will satisfy this contract. Keeping it module-private
# (leading underscore) signals that callers should pass an `Info`, not import
# the Protocol directly.
class _QueryClient(Protocol):
    """Minimal subset of Info that SingleSigner needs; Info satisfies this structurally."""

    def query_app_smart(
        self,
        contract: Addr,
        msg: dict[str, Any],
        *,
        height: int | None = None,
    ) -> Any: ...


class SingleSigner:
    """Stateful per-account signer; tracks user_index and next_nonce, produces signed Txs."""

    def __init__(
        self,
        wallet: Wallet,
        address: Addr,
        *,
        user_index: int | None = None,
        next_nonce: int | None = None,
    ) -> None:
        # `address` is taken explicitly rather than from `wallet.address` so
        # that the same key can sign for multiple Dango accounts. The Wallet
        # protocol's `address` is a per-instance default, not a binding.
        self.wallet: Wallet = wallet
        self.address: Addr = address
        self.user_index: int | None = user_index
        self.next_nonce: int | None = next_nonce

    @classmethod
    def auto_resolve(
        cls,
        wallet: Wallet,
        address: Addr,
        info: _QueryClient,
    ) -> SingleSigner:
        """Construct a signer and populate user_index and next_nonce by querying the chain."""
        # We take `address` explicitly (not `wallet.address`) because the
        # Wallet protocol's address is advisory: a single key can control
        # multiple accounts, and the SingleSigner is bound to one of them.
        # Resolution order doesn't matter — both queries are independent —
        # but we resolve user_index first since failing there is the more
        # common newbie misconfig (wrong account_factory address, account
        # not yet activated on chain).
        signer = cls(wallet, address)
        signer.user_index = signer.query_user_index(info)
        signer.next_nonce = signer.query_next_nonce(info)
        return signer

    def query_user_index(self, info: _QueryClient) -> int:
        """Look up this address's user_index via the account-factory contract."""
        # `account_factory` is hard-coded from constants rather than accepted
        # as a parameter: chain-specific knowledge belongs at the constants
        # layer, and v1 only supports the canonical Dango deployment. If
        # multi-chain support is needed later, add a chain-aware constants
        # lookup here, not a parameter on every signer call.
        #
        # `QueryAccountRequest` is an externally-tagged enum variant on
        # `dango_account_factory::QueryMsg`, so the wire form is
        # `{"account": {"address": "0x..."}}`. The response is a `User`
        # struct whose `owner` field is the user_index (the User's index in
        # the factory's USERS map). We discard the rest of the response.
        response = info.query_app_smart(
            Addr(ACCOUNT_FACTORY_CONTRACT),
            {"account": {"address": self.address}},
        )
        return int(response["owner"])

    def query_next_nonce(self, info: _QueryClient) -> int:
        """Compute next_nonce from the account's seen-nonces sliding window."""
        # `QuerySeenNoncesRequest` is the externally-tagged variant
        # `{"seen_nonces": {}}` on the per-account contract's QueryMsg. The
        # response is a JSON array of recently-seen nonces (sorted ascending
        # by the contract). Empty array means the account has never sent a
        # transaction, so next_nonce is 0; otherwise it's max(seen) + 1.
        # Mirrors `signer.rs::query_next_nonce`.
        response = info.query_app_smart(
            self.address,
            {"seen_nonces": {}},
        )
        # The Rust side calls `.last()` on the sorted vec, but defensive
        # programming says use `max()`: if the contract ever returns an
        # unsorted array we still get the right answer, and the cost is
        # O(n) on a tiny array.
        seen = cast(list[int], response)
        return max(seen) + 1 if seen else 0

    def build_unsigned_tx(self, messages: list[Message], chain_id: str) -> UnsignedTx:
        """Wrap messages plus Metadata into an UnsignedTx ready to feed to Info.simulate()."""
        # State must be resolved before we can build a tx. We raise
        # RuntimeError (not ValueError) because the inputs are valid; the
        # signer's *state* is incomplete. Encourage callers to be explicit:
        # call query_*() / auto_resolve() rather than have us re-query
        # under the hood (which would silently mask state-management bugs).
        user_index = self._require_user_index()
        next_nonce = self._require_next_nonce()
        return UnsignedTx(
            sender=self.address,
            msgs=messages,
            data=Metadata(
                user_index=UserIndex(user_index),
                chain_id=chain_id,
                nonce=Nonce(next_nonce),
                expiry=None,
            ),
        )

    def sign_tx(self, messages: list[Message], chain_id: str, gas_limit: int) -> Tx:
        """Sign and return a Tx; increments self.next_nonce on success or failure."""
        user_index = self._require_user_index()
        # Snapshot the current nonce *before* incrementing self.next_nonce.
        # This matches the Rust source (signer.rs:271-272) and is atomic-safe:
        # if signing throws, we still advance the local nonce so the caller
        # cannot accidentally reuse the same nonce on retry. The chain
        # rejects duplicate nonces, so optimistic increment is strictly safer
        # than waiting until success.
        nonce = self._require_next_nonce()
        self.next_nonce = nonce + 1

        metadata = Metadata(
            user_index=UserIndex(user_index),
            chain_id=chain_id,
            nonce=Nonce(nonce),
            expiry=None,
        )

        # SignDoc has a slightly different field name (`messages` vs `msgs`)
        # from UnsignedTx/Tx — this is a quirk of the Rust types, faithfully
        # mirrored here. SignDoc is what gets canonical-JSON encoded and
        # SHA-256 hashed; UnsignedTx/Tx is the wire envelope.
        sign_doc = SignDoc(
            sender=self.address,
            gas_limit=gas_limit,
            messages=messages,
            data=metadata,
        )

        signature = self.wallet.sign(sign_doc)
        # `Credential::Standard` is an externally-tagged enum variant on
        # `dango_types::auth::Credential`, hence the `{"Standard": ...}`
        # outer wrapper. The inner `StandardCredential` carries `key_hash`
        # (the on-chain identifier the contract uses to look up the pubkey)
        # and the `signature` envelope itself. Cast through the union since
        # the typed dict literal is a more precise type than Credential.
        credential = cast(
            Credential,
            {
                "Standard": StandardCredential(
                    key_hash=self.wallet.key_hash,
                    signature=signature,
                ),
            },
        )

        return Tx(
            sender=self.address,
            gas_limit=gas_limit,
            msgs=messages,
            data=metadata,
            credential=credential,
        )

    def _require_user_index(self) -> int:
        # State guards live in helpers so the call sites read as ordinary
        # field access. RuntimeError, not ValueError: the *inputs* to
        # build_unsigned_tx / sign_tx are well-formed; the signer itself
        # is in an incomplete state.
        if self.user_index is None:
            raise RuntimeError(
                "user_index unresolved; call query_user_index() or auto_resolve() first",
            )
        return self.user_index

    def _require_next_nonce(self) -> int:
        if self.next_nonce is None:
            raise RuntimeError(
                "next_nonce unresolved; call query_next_nonce() or auto_resolve() first",
            )
        return self.next_nonce

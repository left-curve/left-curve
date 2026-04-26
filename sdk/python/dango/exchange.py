"""Signed-action surface: Exchange wraps the sign + simulate + broadcast pipeline."""

from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING, Any, Final

from dango.api import API
from dango.info import Info
from dango.utils.constants import (
    GAS_OVERHEAD_SECP256K1,
    PERPS_CONTRACT_MAINNET,
    SETTLEMENT_DENOM,
)
from dango.utils.signing import Secp256k1Wallet, SingleSigner, Wallet
from dango.utils.types import Addr, Message, dango_decimal

if TYPE_CHECKING:
    from eth_account.signers.local import LocalAccount


class Exchange(API):
    """Build, sign, and broadcast Dango transactions on behalf of one account."""

    # Fixed gas overhead added on top of the simulated `gas_used` to cover
    # signature verification, which `Info.simulate` deliberately skips
    # (the simulate path does not run the auth pre-handler). 770_000 is
    # the empirically-measured cost for a Secp256k1 verify on Dango;
    # see `GAS_OVERHEAD_SECP256K1` in constants.py for the source-of-
    # truth definition. Exposed as a class attribute so subclasses /
    # tests can override without monkeypatching the constants module.
    DEFAULT_GAS_OVERHEAD: Final[int] = GAS_OVERHEAD_SECP256K1

    def __init__(
        self,
        wallet: Wallet | LocalAccount,
        base_url: str,
        *,
        account_address: Addr,
        user_index: int | None = None,
        next_nonce: int | None = None,
        chain_id: str | None = None,
        timeout: float | None = None,
        info: Info | None = None,
        perps_contract: Addr | None = None,
    ) -> None:
        super().__init__(base_url, timeout=timeout)
        # `Info` is reused for queries (chain_id, user_index, nonce,
        # simulate, broadcast). Tests inject a mock; production code lets
        # us construct one over the same base_url so a single Exchange
        # only opens one HTTP session-equivalent endpoint instead of two.
        self._info: Info = info if info is not None else Info(base_url, timeout=timeout)
        # Wallet adapter: accept either a Wallet (Phase 4 Protocol) or an
        # eth_account.LocalAccount. The latter shows up commonly because
        # HL traders already hold one for EVM chains; we extract its raw
        # secp256k1 secret and wrap as Secp256k1Wallet so the SignDoc is
        # signed with raw secp256k1 over SHA-256 (Dango's KeyType=1
        # path), NOT with EIP-712. We rely on `Wallet` being decorated
        # `@runtime_checkable` (see signing.py); LocalAccount cleanly
        # fails the isinstance because it lacks `sign(SignDoc)` and
        # `key_hash` — so the dispatch is unambiguous.
        wallet_obj: Wallet
        if isinstance(wallet, Wallet):
            wallet_obj = wallet
        else:
            wallet_obj = Secp256k1Wallet.from_eth_account(wallet, account_address)
        # Auto-fetch chain_id if not supplied. The server is the
        # authoritative source — we'd rather pay one extra GraphQL round-
        # trip than ship a stale id from constants and have every signed
        # tx silently rejected. Callers who already know the id (e.g.
        # tests, multi-Exchange sharing a status query) skip this by
        # passing `chain_id=` explicitly.
        if chain_id is None:
            chain_id = str(self._info.query_status()["chainId"])
        self._chain_id: str = chain_id
        # Per the Phase 5 SingleSigner spec, both user_index and
        # next_nonce are auto-resolved when None and respected when set.
        # user_index is stable for the account's lifetime, so caching is
        # safe; next_nonce is normally unsafe to cache across processes
        # but a caller may want to pre-set it (e.g. recovering after a
        # failed broadcast where the nonce already advanced server-side,
        # or driving a deterministic test).
        self._signer: SingleSigner = SingleSigner(
            wallet_obj,
            account_address,
            user_index=user_index,
            next_nonce=next_nonce,
        )
        if user_index is None:
            self._signer.user_index = self._signer.query_user_index(self._info)
        if next_nonce is None:
            self._signer.next_nonce = self._signer.query_next_nonce(self._info)
        # Wrap in `Addr(...)` so the stored field is the typed alias even
        # if the caller passed a plain `str` (the constants are `str`).
        self._perps_contract: Addr = Addr(perps_contract or PERPS_CONTRACT_MAINNET)

    @property
    def address(self) -> Addr:
        """The Dango account address this Exchange transacts as."""
        return self._signer.address

    @property
    def signer(self) -> SingleSigner:
        """The underlying SingleSigner; useful for manual nonce tweaks in tests."""
        return self._signer

    # --- Pipeline ------------------------------------------------------------

    def _send_action(
        self,
        messages: list[Message],
        *,
        funds: dict[str, str] | None = None,  # reserved for Phase 12+; unused here
    ) -> dict[str, Any]:
        """Run a list of Messages through simulate -> sign -> broadcast."""
        # Simulate first to learn the gas cost. The chain rejects under-
        # gas txs, but we don't want to hard-code a guess: simulate
        # returns the exact post-execute gas, then we add a fixed
        # overhead for signature verify (which simulate skips — see the
        # DEFAULT_GAS_OVERHEAD docstring). The Rust SDK does the same
        # math; mirroring it keeps the Python and Rust paths
        # interchangeable.
        unsigned = self._signer.build_unsigned_tx(messages, self._chain_id)
        sim = self._info.simulate(unsigned)
        gas_used = int(sim["gas_used"])
        gas_limit = gas_used + self.DEFAULT_GAS_OVERHEAD
        # `sign_tx` increments the signer's local nonce — see the Rust
        # source signer.rs:271-272 and the Python mirror in
        # signing.py::SingleSigner.sign_tx. We deliberately do NOT
        # decrement on broadcast failure: the chain rejects duplicate
        # nonces, so an optimistic increment is strictly safer than
        # waiting until success and risking a same-nonce retry.
        signed = self._signer.sign_tx(messages, self._chain_id, gas_limit)
        return self._info.broadcast_tx_sync(signed)

    # --- Margin --------------------------------------------------------------

    def deposit_margin(self, amount: int) -> dict[str, Any]:
        """Deposit USDC into the perps margin sub-account; amount is in base units."""
        # `amount` is base units (Uint128) — caller does the human-USD
        # conversion explicitly. 1.50 USDC = 1_500_000 base units (since
        # SETTLEMENT_DECIMALS = 6). This avoids the ambiguity of "is 1.5
        # USDC or 1.5 base units?" that float input would invite.
        #
        # Wire shape (Rust source: `dango/types/src/perps.rs::TraderMsg::Deposit`):
        #
        #   * `funds` is a `Coins` map (`BTreeMap<Denom, Uint128>`); the
        #     Uint128 serializes as a base-10 integer *string*, hence
        #     `str(amount)`.
        #   * `TraderMsg::Deposit` itself only wraps an optional `to`
        #     address; the actual amount comes via the `funds` map.
        #     `to=None` defaults to the sender's own margin sub-account,
        #     which is the only path we expose in v1.
        #   * Enum keys are snake_case because `#[grug::derive(Serde)]`
        #     injects `rename_all = "snake_case"` (see
        #     `grug/macros/src/derive.rs`).
        if isinstance(amount, bool) or not isinstance(amount, int):
            raise TypeError(
                f"deposit_margin amount must be an int (base units), got {type(amount).__name__}",
            )
        if amount <= 0:
            raise ValueError(f"deposit_margin amount must be positive, got {amount}")
        message: Message = {
            "execute": {
                "contract": self._perps_contract,
                "msg": {"trade": {"deposit": {"to": None}}},
                "funds": {SETTLEMENT_DENOM: str(amount)},
            },
        }
        return self._send_action([message])

    def withdraw_margin(self, amount: float | str | Decimal) -> dict[str, Any]:
        """Withdraw USDC from the perps margin sub-account; amount is in USD."""
        # Withdraw goes the OTHER direction from deposit: funds flow OUT
        # of the contract, so the surrounding execute message carries
        # empty funds. The amount lives inside the TraderMsg::Withdraw
        # payload as a `UsdValue` — a 6-decimal USD string, NOT base
        # units. The contract converts USD to settlement-currency base
        # units internally at the current oracle price.
        #
        # This asymmetry mirrors the contract: `deposit_margin` takes
        # base-unit ints (because the wire uses `Coins`/`Uint128`),
        # while `withdraw_margin` takes USD floats (because the wire
        # uses `UsdValue`/6-decimal). `dango_decimal()` formats the
        # latter; the deposit path str()s the int directly.
        message: Message = {
            "execute": {
                "contract": self._perps_contract,
                "msg": {"trade": {"withdraw": {"amount": dango_decimal(amount)}}},
                "funds": {},
            },
        }
        return self._send_action([message])

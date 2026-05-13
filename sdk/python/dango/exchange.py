"""Signed-action surface: Exchange wraps the sign + simulate + broadcast pipeline."""

from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING, Any, Final, Literal, cast

from dango.api import API
from dango.info import Info
from dango.utils.constants import (
    ACCOUNT_FACTORY_CONTRACT,
    GAS_OVERHEAD_SECP256K1,
    PERPS_CONTRACT_MAINNET,
    SETTLEMENT_DENOM,
)
from dango.utils.signing import Secp256k1Wallet, SingleSigner, Wallet
from dango.utils.types import (
    Addr,
    AllForPair,
    CancelConditionalOrderRequest,
    CancelConditionalSpec,
    CancelOrderRequest,
    ChildOrder,
    ClientOrderIdRef,
    ConditionalOrderRef,
    Message,
    OrderId,
    OrderKind,
    PairId,
    Quantity,
    SubmitAction,
    SubmitOrCancelAction,
    SubmitOrCancelOrderRequest,
    SubmitOrderRequest,
    TimeInForce,
    TriggerDirection,
    dango_decimal,
)

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

    # --- Orders --------------------------------------------------------------

    def _build_submit_order_wire(
        self,
        pair_id: PairId,
        size: float | int | str | Decimal,
        kind: OrderKind,
        *,
        reduce_only: bool,
        tp: ChildOrder | None,
        sl: ChildOrder | None,
    ) -> SubmitOrderRequest:
        """Construct the wire-shape SubmitOrderRequest dict; rejects size==0."""

        # Sign convention: positive = buy, negative = sell. Zero is
        # meaningless (the contract would reject it; we fail fast
        # client-side for a friendlier error). `dango_decimal` already
        # rejects NaN/Inf inputs, so we don't pre-check those here.
        size_str = dango_decimal(size)

        # `dango_decimal` always pads to 6 dp, so a zero check on the
        # Decimal-equivalent form is unambiguous regardless of input
        # type (`0`, `0.0`, `"0"`, `Decimal("0")` all collapse to
        # `"0.000000"`).
        if Decimal(size_str) == 0:
            raise ValueError("order size must be non-zero (positive=buy, negative=sell)")

        # The TypedDict enforces snake_case keys at type-check time;
        # we also rely on grug-derived enums serializing variant tags
        # as snake_case (`market`/`limit`) — `rename_all = "snake_case"`
        # is injected by `grug/macros/src/derive.rs:93`.
        return SubmitOrderRequest(
            pair_id=pair_id,
            size=cast("Quantity", size_str),
            kind=kind,
            reduce_only=reduce_only,
            tp=tp,
            sl=sl,
        )

    def _build_cancel_order_wire(
        self,
        spec: OrderId | ClientOrderIdRef | Literal["all"],
    ) -> CancelOrderRequest:
        """Construct the wire-shape CancelOrderRequest from a user-facing spec."""

        # The order of these branches matters: `OrderId` is
        # `NewType("OrderId", str)` — at runtime it's a plain `str`,
        # which would also match `spec == "all"`. So we test for the
        # literal `"all"` FIRST, then the dataclass wrapper, and only
        # then fall through to the OrderId path. Reversing this order
        # would silently route `cancel_order("all")` through the
        # `{"one": "all"}` branch, which the contract would reject.
        if spec == "all":
            return "all"

        if isinstance(spec, ClientOrderIdRef):
            # ClientOrderId is `Uint64` on the wire = base-10 decimal
            # *string*. We accept an `int` for ergonomics (the user
            # types `ClientOrderIdRef(value=7)`, not `"7"`) and
            # stringify here. Negative values are caller error and
            # would be rejected by the chain; we don't second-guess.
            return cast("CancelOrderRequest", {"one_by_client_order_id": str(spec.value)})

        # Treat any remaining value as an OrderId (a Uint64 string).
        # We don't `isinstance(spec, str)`-guard because the type
        # checker has already narrowed it to `OrderId` (= str) here;
        # adding a runtime check would only obscure that.
        return cast("CancelOrderRequest", {"one": str(spec)})

    def _wrap_perps_execute(self, key: str, inner: dict[str, Any]) -> Message:
        """Wrap an inner payload under `{<key>: inner}` and target the perps contract."""

        # ExecuteMsg is an externally-tagged enum with four variants —
        # `Trade`, `Vault`, `Referral`, `Maintain` (see
        # `dango/types/src/perps.rs::ExecuteMsg`) — so the dispatch key is
        # the only thing that varies between methods. Every action that
        # goes through this helper carries no funds: trades draw from
        # existing margin, vault liquidity is debited from the user's
        # margin (NOT attached as funds), and the referral / maintain
        # variants trivially carry nothing. Only `deposit_margin` uses a
        # non-empty funds map and so emits its execute wrapper inline
        # rather than via this helper.
        return {
            "execute": {
                "contract": self._perps_contract,
                "msg": {key: inner},
                "funds": {},
            },
        }

    def submit_order(
        self,
        pair_id: PairId,
        size: float | int | str | Decimal,
        kind: OrderKind,
        *,
        reduce_only: bool = False,
        tp: ChildOrder | None = None,
        sl: ChildOrder | None = None,
    ) -> dict[str, Any]:
        """Place a single perps order; size is signed (+ buy / − sell)."""

        request = self._build_submit_order_wire(
            pair_id,
            size,
            kind,
            reduce_only=reduce_only,
            tp=tp,
            sl=sl,
        )

        return self._send_action([self._wrap_perps_execute("trade", {"submit_order": request})])

    def cancel_order(
        self,
        spec: OrderId | ClientOrderIdRef | Literal["all"],
    ) -> dict[str, Any]:
        """Cancel by chain OrderId, ClientOrderIdRef, or 'all' for every open order."""

        # The CancelOrderRequest wire form is an externally-tagged
        # enum, which serde encodes as either a single-key sub-object
        # (`One`/`OneByClientOrderId`) or a bare string (`All`). The
        # helper picks the right shape; we just hand the result to
        # `_wrap_perps_execute`.
        return self._send_action(
            [
                self._wrap_perps_execute(
                    "trade",
                    {"cancel_order": self._build_cancel_order_wire(spec)},
                ),
            ],
        )

    def batch_update_orders(
        self,
        actions: list[SubmitOrCancelAction],
    ) -> dict[str, Any]:
        """Submit and/or cancel multiple orders atomically in one transaction."""

        # The contract enforces `1 <= len <= max_action_batch_size`
        # (governance-tunable, fixture default 5). We only enforce
        # non-empty client-side; an over-sized batch is rejected by
        # the chain rather than the SDK so we don't have to track
        # governance changes locally.
        if not actions:
            raise ValueError("batch_update_orders requires at least one action")

        wire: list[SubmitOrCancelOrderRequest] = []

        for action in actions:
            if isinstance(action, SubmitAction):
                wire.append(
                    cast(
                        "SubmitOrCancelOrderRequest",
                        {
                            "submit": self._build_submit_order_wire(
                                action.pair_id,
                                action.size,
                                action.kind,
                                reduce_only=action.reduce_only,
                                tp=action.tp,
                                sl=action.sl,
                            ),
                        },
                    ),
                )
            else:
                # `CancelAction` is the only other variant of the
                # `SubmitOrCancelAction` union, so this branch is
                # exhaustive — but we don't `assert isinstance(...)`
                # because mypy already narrows `action` to
                # `CancelAction` here.
                wire.append(
                    cast(
                        "SubmitOrCancelOrderRequest",
                        {"cancel": self._build_cancel_order_wire(action.spec)},
                    ),
                )

        return self._send_action(
            [self._wrap_perps_execute("trade", {"batch_update_orders": wire})],
        )

    # --- Convenience helpers -------------------------------------------------

    def submit_market_order(
        self,
        pair_id: PairId,
        size: float | int | str | Decimal,
        *,
        max_slippage: float | str | Decimal = 0.01,
        reduce_only: bool = False,
        tp: ChildOrder | None = None,
        sl: ChildOrder | None = None,
    ) -> dict[str, Any]:
        """Place a market order with a slippage cap (default 1%)."""

        # `max_slippage` is a `Dimensionless` — same 6-decimal string
        # encoding as USD/quantity values. Passing 0.01 = 1%.
        kind = cast(
            "OrderKind",
            {"market": {"max_slippage": dango_decimal(max_slippage)}},
        )

        return self.submit_order(
            pair_id,
            size,
            kind,
            reduce_only=reduce_only,
            tp=tp,
            sl=sl,
        )

    def submit_limit_order(
        self,
        pair_id: PairId,
        size: float | int | str | Decimal,
        limit_price: float | str | Decimal,
        *,
        time_in_force: TimeInForce = TimeInForce.GTC,
        client_order_id: int | None = None,
        reduce_only: bool = False,
        tp: ChildOrder | None = None,
        sl: ChildOrder | None = None,
    ) -> dict[str, Any]:
        """Place a limit order; defaults to GTC and no client-side id."""

        # Store `time_in_force.value` rather than the enum itself so
        # downstream `json.dumps` and equality assertions both treat
        # it as a plain str ("GTC"/"IOC"/"POST"). `StrEnum` would
        # round-trip through `json.dumps` correctly anyway, but the
        # GraphQL HTTP layer doesn't always, and identity assertions
        # in tests need the unwrapped value.
        limit_payload: dict[str, Any] = {
            "limit_price": dango_decimal(limit_price),
            "time_in_force": time_in_force.value,
            "client_order_id": str(client_order_id) if client_order_id is not None else None,
        }

        kind = cast("OrderKind", {"limit": limit_payload})

        return self.submit_order(
            pair_id,
            size,
            kind,
            reduce_only=reduce_only,
            tp=tp,
            sl=sl,
        )

    # --- Conditional orders (TP/SL) ------------------------------------------

    def _build_cancel_conditional_wire(
        self,
        spec: CancelConditionalSpec,
    ) -> CancelConditionalOrderRequest:
        """Construct the wire-shape CancelConditionalOrderRequest from a user-facing spec."""

        # Mirror `_build_cancel_order_wire`: bare-string variant first,
        # dataclass variants next, defensive raise for off-types.
        if spec == "all":
            return "all"

        if isinstance(spec, ConditionalOrderRef):
            return cast(
                "CancelConditionalOrderRequest",
                {
                    "one": {
                        "pair_id": spec.pair_id,
                        "trigger_direction": spec.trigger_direction.value,
                    },
                },
            )

        if isinstance(spec, AllForPair):
            return cast(
                "CancelConditionalOrderRequest",
                {"all_for_pair": {"pair_id": spec.pair_id}},
            )

        # Defensive runtime guard for callers that bypass the type
        # checker. Mirrors the friendly-error pattern elsewhere in
        # this module (cf. `deposit_margin`'s type check).
        raise TypeError(  # pragma: no cover - static types prevent reaching this
            f"unsupported cancel_conditional_order spec: {type(spec).__name__}",
        )

    def submit_conditional_order(
        self,
        pair_id: PairId,
        size: float | int | str | Decimal | None,
        trigger_price: float | int | str | Decimal,
        trigger_direction: TriggerDirection,
        max_slippage: float | int | str | Decimal,
    ) -> dict[str, Any]:
        """Place a conditional (TP/SL) order; reduce-only is implicit. size=None closes all."""

        # Per the Rust comment on TraderMsg::SubmitConditionalOrder, the
        # caller is responsible for the size sign: negative closes a
        # long (sells), positive closes a short (buys). `None` means
        # "close the entire position at trigger time" — distinct from
        # zero (which is ambiguous and rejected). Reduce-only is NOT a
        # parameter because conditional orders are always reduce-only
        # by construction.
        size_str: str | None
        if size is None:
            size_str = None
        else:
            size_str = dango_decimal(size)
            # Same zero-guard rationale as `_build_submit_order_wire`:
            # `dango_decimal` collapses every zero-equivalent input to
            # `"0.000000"`, so the Decimal compare is unambiguous.
            if Decimal(size_str) == 0:
                raise ValueError(
                    "conditional order size must be non-zero or None"
                    " (None = close entire position)",
                )

        # Snake_case keys match the contract; `.value` unwraps the
        # StrEnum to a plain str so `json.dumps` doesn't emit a
        # `"TriggerDirection.ABOVE"` literal.
        inner: dict[str, Any] = {
            "submit_conditional_order": {
                "pair_id": pair_id,
                "size": size_str,
                "trigger_price": dango_decimal(trigger_price),
                "trigger_direction": trigger_direction.value,
                "max_slippage": dango_decimal(max_slippage),
            },
        }

        return self._send_action([self._wrap_perps_execute("trade", inner)])

    def cancel_conditional_order(
        self,
        spec: CancelConditionalSpec,
    ) -> dict[str, Any]:
        """Cancel a conditional order by ref, all-for-pair, or 'all' for every CO."""

        # Mirrors `cancel_order`: the helper handles the three
        # externally-tagged variants (bare string `"all"` / `{"one":
        # {...}}` / `{"all_for_pair": {...}}`); we just wrap and send.
        return self._send_action(
            [
                self._wrap_perps_execute(
                    "trade",
                    {"cancel_conditional_order": self._build_cancel_conditional_wire(spec)},
                ),
            ],
        )

    # --- Vault ---------------------------------------------------------------

    def add_liquidity(
        self,
        amount: float | int | str | Decimal,
        *,
        min_shares_to_mint: int | None = None,
    ) -> dict[str, Any]:
        """Transfer USD margin into the counterparty vault, minting LP shares."""

        # Wire shape (Rust source: `dango/types/src/perps.rs::VaultMsg::AddLiquidity`):
        #
        #   * `amount` is a `UsdValue` — 6-decimal fixed-point string,
        #     e.g. "1000.000000". `dango_decimal` produces this canonical
        #     form. The amount is debited from the caller's existing
        #     trading margin, NOT attached as `funds` on the execute
        #     message — vault deposits flow inside the contract, not
        #     across the wallet boundary.
        #   * `min_shares_to_mint` is an `Option<Uint128>`: `None` →
        #     JSON null on the wire (= no slippage protection),
        #     otherwise a base-10 integer string of the share count.
        #     Passing `0` would also disable the guard but is wasteful;
        #     callers who want a guard should pick a positive value.
        #
        # Reject `amount <= 0` client-side: zero shares of LP make no
        # sense, and a negative amount is meaningless. The chain would
        # also reject these but failing early surfaces a friendlier
        # error.
        amount_str = dango_decimal(amount)
        if Decimal(amount_str) <= 0:
            raise ValueError(f"add_liquidity amount must be positive, got {amount!r}")

        # `min_shares_to_mint` is `int | None`. Bool is an int subclass
        # in Python, so we filter it BEFORE the int branch — otherwise
        # `add_liquidity(amount, min_shares_to_mint=True)` would
        # silently coerce to "1" and pass the negative-check.
        min_shares_str: str | None
        if min_shares_to_mint is None:
            min_shares_str = None
        elif isinstance(min_shares_to_mint, bool) or not isinstance(min_shares_to_mint, int):
            raise TypeError(
                "add_liquidity min_shares_to_mint must be an int or None, "
                f"got {type(min_shares_to_mint).__name__}",
            )
        elif min_shares_to_mint < 0:
            raise ValueError(
                f"add_liquidity min_shares_to_mint must be non-negative, got {min_shares_to_mint}",
            )
        else:
            min_shares_str = str(min_shares_to_mint)

        inner = {
            "add_liquidity": {
                "amount": amount_str,
                "min_shares_to_mint": min_shares_str,
            },
        }

        return self._send_action([self._wrap_perps_execute("vault", inner)])

    def remove_liquidity(self, shares_to_burn: int) -> dict[str, Any]:
        """Burn LP shares to schedule a vault withdrawal (subject to cooldown)."""

        # Wire shape (Rust source: `dango/types/src/perps.rs::VaultMsg::RemoveLiquidity`):
        #
        #   * `shares_to_burn` is `Uint128` = base-10 integer string.
        #
        # Mirror `deposit_margin`'s int validation: bool is an int subtype,
        # so reject it BEFORE the int branch; then enforce strictly
        # positive. The contract would also reject zero/negative but the
        # client-side check produces a friendlier error.
        if isinstance(shares_to_burn, bool) or not isinstance(shares_to_burn, int):
            raise TypeError(
                "remove_liquidity shares_to_burn must be an int, "
                f"got {type(shares_to_burn).__name__}",
            )

        if shares_to_burn <= 0:
            raise ValueError(
                f"remove_liquidity shares_to_burn must be positive, got {shares_to_burn}",
            )

        inner = {"remove_liquidity": {"shares_to_burn": str(shares_to_burn)}}

        return self._send_action([self._wrap_perps_execute("vault", inner)])

    # --- Referrals -----------------------------------------------------------

    def set_referral(self, referrer: int | str) -> dict[str, Any]:
        """Bind the signer as a referee of `referrer` (user_index or username)."""

        # Wire shape (Rust source: `dango/types/src/perps.rs::ReferralMsg::SetReferral`):
        #
        #   * `referrer` is a `UserIndex` (u32) — JSON number.
        #   * `referee` is also a `UserIndex`; we auto-fill it from the
        #     signer's resolved index. By the time this method is
        #     callable the Exchange constructor has already populated
        #     `self._signer.user_index` (auto-fetched if not supplied
        #     to `__init__`), so the value is never None here.
        #
        # The Python API takes `int | str`: int is used as the
        # user_index directly, str triggers a username lookup against
        # the account-factory contract. The lookup is intentionally
        # not cached — usernames are theoretically rebindable per the
        # contract docs, and a fresh query per call costs one cheap
        # round-trip.
        referrer_index: int

        if isinstance(referrer, bool):
            # Reject bool first: it is an int subclass and would
            # otherwise silently route through the int branch as 0/1.
            raise TypeError("set_referral referrer must not be a bool")

        if isinstance(referrer, int):
            if referrer < 0:
                raise ValueError(
                    f"set_referral referrer index must be non-negative, got {referrer}"
                )
            referrer_index = referrer
        elif isinstance(referrer, str):
            if referrer == "":
                raise ValueError("set_referral referrer username must be non-empty")
            # Account-factory `QueryMsg::User(UserIndexOrName::Name(_))`
            # — wire form `{"user": {"name": "<username>"}}` — returns a
            # `User` struct (`dango/types/src/account_factory/msg.rs`).
            # We only need `index` from the response; the rest is
            # discarded. Don't replicate the chain's Username regex
            # client-side — let the chain reject malformed inputs and
            # surface that error verbatim.
            response = self._info.query_app_smart(
                Addr(ACCOUNT_FACTORY_CONTRACT),
                {"user": {"name": referrer}},
            )
            referrer_index = int(response["index"])
        else:
            raise TypeError(
                f"set_referral referrer must be int or str, got {type(referrer).__name__}",
            )

        # `_require_user_index` raises a clear RuntimeError if the
        # signer was constructed without a user_index AND the auto-
        # resolution path was bypassed. We use it instead of `assert`
        # because `python -O` strips asserts, which would let a None
        # propagate into the wire dict and produce an invalid
        # JSON-number-typed `referee` field.
        referee_index = self._signer._require_user_index()

        inner = {
            "set_referral": {
                "referrer": referrer_index,
                "referee": referee_index,
            },
        }

        return self._send_action([self._wrap_perps_execute("referral", inner)])

    # --- Liquidation ---------------------------------------------------------

    def liquidate(self, user: Addr) -> dict[str, Any]:
        """Force-close an underwater user's positions (permissionless)."""

        # Wire shape (Rust source: `dango/types/src/perps.rs::MaintainerMsg::Liquidate`):
        #
        #   * `user` is the target trader's account `Addr`. The contract
        #     handler at `dango/perps/src/maintain/liquidate.rs` does
        #     NOT check the caller's identity — anyone can submit this
        #     message for any trader. The contract itself decides
        #     whether the target is actually liquidatable based on
        #     equity vs. maintenance margin.
        #
        # No client-side validation beyond accepting the typed Addr —
        # the chain's liquidatable-or-not check is the authoritative
        # one, so we don't replicate it here.
        inner = {"liquidate": {"user": user}}

        return self._send_action([self._wrap_perps_execute("maintain", inner)])

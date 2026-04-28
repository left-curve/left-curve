"""HL-shaped ``Exchange`` wrapper that translates HL-style signed actions to native Dango calls.

This module is the write-side translator for the Hyperliquid-compat layer
(Phase 17). The public class :class:`Exchange` mirrors HL's
``hyperliquid.exchange.Exchange`` constructor and method signatures so HL
traders can swap their import statement and keep going.

Design goals:

* HL camelCase wire shapes are preserved verbatim on the *response* envelopes
  (``oid``, ``totalSz``, ``avgPx``, ``status``). All decimal strings on the
  wire pass through :func:`dango_decimal_to_hl_str` so trailing zeros are
  stripped.
* HL-only or Dango-gap methods raise :class:`NotImplementedError` with a
  one-line reason. We do NOT silently no-op — the goal is loud failure so
  callers can route around the gap explicitly.
* Methods that produce a tx return HL's status envelope:
  ``{"status": "ok", "response": {"type": ..., "data": {"statuses": [...]}}}``
  built with the Phase 15 helpers in :mod:`dango.hyperliquid_compatibility.types`.

Cloid asymmetry warning
-----------------------

HL's ``Cloid`` is 16 bytes; Dango's ``ClientOrderId`` is a Uint64. We hash
the cloid down via ``Cloid.to_uint64()`` (deterministic SHA-256 prefix). So
the cloid you see in a Dango response (extracted from indexer events) is
*not* the original 16-byte cloid you sent — see the module docstring of
``dango.hyperliquid_compatibility.types`` for the full rationale. Callers who
need round-trip cloid identity must keep their own mapping.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Final, cast

from dango.exchange import Exchange as NativeExchange
from dango.hyperliquid_compatibility.types import (
    Cloid,
    Grouping,
    HlStatusEntry,
    OrderRequest,
    OrderType,
    dango_decimal_to_hl_str,
    hl_resting_entry,
    hl_status_envelope,
)
from dango.utils.types import (
    Addr,
    CancelAction,
    ClientOrderIdRef,
    OrderId,
    OrderKind,
    PairId,
    SubmitAction,
    SubmitOrCancelAction,
    TimeInForce,
)

if TYPE_CHECKING:
    from collections.abc import Iterable

    from eth_account.signers.local import LocalAccount

    from dango.hyperliquid_compatibility.info import Info as HlInfo
    from dango.utils.signing import Wallet


# --- Internal helpers -------------------------------------------------------


# HL "Alo" (add-liquidity-only / post-only) maps to Dango's POST time-in-
# force. The Dango chain rejects an Alo limit order that would cross the
# book at submission, matching HL semantics. GTC and IOC are 1:1.
_HL_TIF_TO_DANGO: Final[dict[str, TimeInForce]] = {
    "Gtc": TimeInForce.GTC,
    "Ioc": TimeInForce.IOC,
    "Alo": TimeInForce.POST,
}


def _hl_tif_to_dango(tif: str) -> TimeInForce:
    """Translate HL's TIF string ("Gtc"/"Ioc"/"Alo") to the Dango enum."""
    # Listed-then-raise so a typo in caller code (e.g. lowercase "ioc") fails
    # loudly rather than silently routing through the default.
    try:
        return _HL_TIF_TO_DANGO[tif]
    except KeyError as exc:
        supported = ", ".join(sorted(_HL_TIF_TO_DANGO))
        raise ValueError(
            f"unsupported HL time_in_force {tif!r}; supported: {supported}",
        ) from exc


def _hl_order_type_to_dango_kind(
    order_type: OrderType,
    limit_px: float,
    cloid: Cloid | None,
) -> OrderKind:
    """Translate an HL ``OrderType`` dict into a native Dango ``OrderKind``.

    Only the ``{"limit": {"tif": ...}}`` branch is implemented in Phase 17.
    The ``{"trigger": ...}`` branch routes to a different native call
    (``submit_conditional_order``) and is structurally different enough that
    we raise ``NotImplementedError`` with a clear message rather than guess.
    """
    # Dispatch on the first (and only) key. HL's `OrderType` is a TypedDict
    # with `total=False`, so we can't rely on `.get(...)` returning None
    # reliably for the unset variant — instead branch on key presence.
    if "limit" in order_type:
        limit = order_type["limit"]
        tif = _hl_tif_to_dango(limit["tif"])
        # `dango_decimal` is performed inside the native Exchange, so we
        # forward `limit_px` as a plain float and let the native side
        # canonicalize. `client_order_id` is the Uint64 form of the HL
        # cloid; see the asymmetry warning in the module docstring.
        client_order_id = str(cloid.to_uint64()) if cloid is not None else None
        return cast(
            "OrderKind",
            {
                "limit": {
                    "limit_price": limit_px,
                    "time_in_force": tif.value,
                    "client_order_id": client_order_id,
                },
            },
        )
    if "trigger" in order_type:
        # The HL trigger order packages a parent (limit or market) order
        # together with a trigger condition. Native Dango splits this into
        # `submit_conditional_order` (which is reduce-only by construction
        # and takes a `trigger_direction` derived from the
        # tpsl/parent-side combination). The mapping is non-trivial — see
        # the Phase 17 spec for the full rationale — so we defer this
        # translation rather than ship a half-correct shim.
        raise NotImplementedError(
            "HL trigger orders (TP/SL) are deferred — "
            "use the native Exchange.submit_conditional_order until this is implemented",
        )
    raise ValueError(
        f"unsupported HL OrderType (must contain 'limit' or 'trigger'): {order_type!r}",
    )


def _signed_size(is_buy: bool, sz: float) -> float:
    """Convert HL's (is_buy, positive sz) to a Dango signed size (+ buy / − sell)."""
    # HL exposes side as a separate `is_buy` flag with a positive `sz`.
    # Dango encodes side in the sign of `size`. The translation is just
    # `+sz` for buys, `-sz` for sells.
    return sz if is_buy else -sz


def _build_submit_action(
    pair_id: PairId,
    is_buy: bool,
    sz: float,
    order_kind: OrderKind,
    *,
    reduce_only: bool,
) -> SubmitAction:
    """Construct a native ``SubmitAction`` from HL-style parts."""
    return SubmitAction(
        pair_id=pair_id,
        size=_signed_size(is_buy, sz),
        kind=order_kind,
        reduce_only=reduce_only,
    )


def _native_outcome_to_resting_envelope(
    outcome: dict[str, Any],
    *,
    response_type: str,
    expected_count: int,
) -> dict[str, Any]:
    """Wrap a native broadcast outcome into HL's resting status envelope.

    The native broadcast outcome from ``Exchange._send_action`` is the
    BroadcastTxOutcome dict (``check_tx`` / ``deliver_tx`` envelope, or in
    test fakes ``{"code": 0, "hash": ..., "events": [...]}``). For Phase 17
    we treat the outcome opaquely: if it carries an explicit error signal
    (``check_tx.code != 0`` or top-level ``code != 0`` or an ``err`` key),
    we return an err envelope; otherwise we synthesize a list of
    ``"resting"`` entries — one per submitted order.

    Why "resting" by default: parsing the chain-level events out of a
    Dango broadcast outcome to recover ``OrderFilled`` / ``OrderPersisted``
    payloads is structurally complex (the events live nested inside
    ``check_tx.events.msgs_and_backrun.msgs[*]`` with multiple
    ``CommitmentStatus`` / ``EventStatus`` wrapper layers, and the tx
    can include cron events too). Rather than ship a fragile partial
    parser, we surface the conservative outcome (orders accepted into
    the book) and let callers query the indexer for fills via the HL
    ``Info.user_fills`` reshape. A future refinement can plumb event
    parsing through once the chain-side event shapes stabilize.
    """
    error_message = _extract_error_message(outcome)
    if error_message is not None:
        return hl_status_envelope(response_type=response_type, error=error_message)
    # No usable oid information in the outcome — the chain assigns oids
    # and surfaces them via indexer events, not via the broadcast envelope.
    # Emit a `resting` entry with `oid=0` per submitted order. HL traders
    # who need the real oid should subscribe to `orderUpdates` (Phase 16)
    # or query `historical_orders` after the broadcast settles.
    statuses: list[HlStatusEntry] = [hl_resting_entry(0) for _ in range(expected_count)]
    return hl_status_envelope(response_type=response_type, statuses=statuses)


def _extract_error_message(outcome: dict[str, Any]) -> str | None:
    """Pull a human-readable error string out of a broadcast outcome, or None.

    Dango's broadcast outcome can carry an error in several places depending
    on which stage of the pipeline rejected the tx. We check, in order:
    top-level ``error`` / ``err``, ``check_tx.error`` / ``check_tx.code``,
    and ``result.err`` / non-zero ``code``. None of these are
    type-system-guaranteed; the chain emits whichever subset is relevant.
    """
    # Top-level error string: matches the `_helpers.FakeInfo.broadcast_tx_sync`
    # success shape (no `error` key) and the canonical err shape downstream
    # consumers may inject.
    if isinstance(outcome.get("error"), str):
        return cast("str", outcome["error"])
    if isinstance(outcome.get("err"), str):
        return cast("str", outcome["err"])
    # `check_tx` is the ABCI CheckTx outcome; `code != 0` means the tx was
    # rejected at the gateway. The `error` field carries the message.
    check_tx = outcome.get("check_tx")
    if isinstance(check_tx, dict):
        if isinstance(check_tx.get("error"), str):
            return cast("str", check_tx["error"])
        code = check_tx.get("code")
        if isinstance(code, int) and code != 0:
            return f"check_tx failed with code {code}"
    # Top-level `code` is what the FakeInfo / our own tests emit.
    code = outcome.get("code")
    if isinstance(code, int) and code != 0:
        return f"tx failed with code {code}"
    # `result.err` is the GenericResult variant Dango emits when the tx
    # was simulated successfully but the on-chain handler reverted.
    result = outcome.get("result")
    if isinstance(result, dict):
        if isinstance(result.get("err"), str):
            return cast("str", result["err"])
        if isinstance(result.get("Err"), str):
            return cast("str", result["Err"])
    return None


def _native_outcome_to_cancel_envelope(
    outcome: dict[str, Any],
    *,
    response_type: str,
    expected_count: int,
) -> dict[str, Any]:
    """Wrap a native broadcast outcome into HL's cancel status envelope.

    HL distinguishes ``cancel`` (by oid) from ``cancelByCloid`` in
    ``response.type``; pass the appropriate value. Per-entry status is
    ``{"status": "success"}`` regardless.
    """
    error_message = _extract_error_message(outcome)
    if error_message is not None:
        return hl_status_envelope(response_type=response_type, error=error_message)
    # Emit a `{"status": "success"}` per cancel request. We can't tell
    # from the outcome alone whether the cancel actually matched an
    # open order (the chain may have already filled or removed it); HL
    # traders inspecting individual statuses should treat this as
    # "the tx itself was accepted" rather than "this specific cancel hit".
    statuses: list[HlStatusEntry] = [
        cast("HlStatusEntry", {"status": "success"}) for _ in range(expected_count)
    ]
    return hl_status_envelope(response_type=response_type, statuses=statuses)


# --- The public Exchange class ---------------------------------------------


class Exchange:
    """HL-shaped facade over the native :class:`dango.exchange.Exchange`.

    Construction mirrors HL's signature:

    .. code-block:: python

        ex = Exchange(wallet, base_url="https://...", account_address="0x...")

    The wrapper holds a native ``Exchange`` for signed actions and an HL-
    shaped ``Info`` for read calls (used by ``market_close`` to look up the
    user's position).
    """

    # HL's class attribute, kept for API parity. Callers can override at
    # the call site via the `slippage=` kwarg.
    DEFAULT_SLIPPAGE: Final[float] = 0.05

    def __init__(
        self,
        wallet: Wallet | LocalAccount,
        base_url: str | None = None,
        meta: Any = None,
        vault_address: str | None = None,
        account_address: str | None = None,
        spot_meta: Any = None,
        perp_dexs: list[str] | None = None,
        timeout: float | None = None,
    ) -> None:
        # `vault_address`: HL routes vault-account txs through this address.
        # Dango has no vault-account abstraction at the wallet level (vault
        # liquidity is debited from the user's margin via `add_liquidity` /
        # `remove_liquidity` on the native Exchange). We accept the kwarg
        # for signature parity but warn-via-error if a non-None value is
        # passed — silently ignoring it would route trades to the wrong
        # account in HL traders' minds.
        if vault_address is not None:
            raise NotImplementedError(
                "vault_address is HL-specific — Dango exposes vault liquidity "
                "via add_liquidity/remove_liquidity on the native Exchange; "
                "trades are always signed by the wallet's own account",
            )
        # `spot_meta`: HL traders pass a preloaded SpotMeta to skip the
        # initial fetch. Dango is perps-only, so a non-None value indicates
        # the caller is mis-using the wrapper; raise to surface that.
        if spot_meta is not None:
            raise NotImplementedError(
                "spot_meta is not supported — Dango is perps-only",
            )
        # `account_address` is required for the native Exchange (it
        # signs as that account). HL allows None and defaults to the
        # wallet's own EVM address; Dango has its own account model
        # where the wallet (key) and account (storage) are decoupled.
        # We require an explicit account_address here rather than
        # silently defaulting to a derived one that would be wrong.
        if account_address is None:
            raise ValueError(
                "account_address is required for the Dango HL-compat Exchange; "
                "Dango decouples the signing key from the account address",
            )
        # `base_url` defaults to the local URL when None — HL's default is
        # the production mainnet, but Dango has no canonical default and we
        # don't want to silently point at a real chain. Prefer LOCAL.
        if base_url is None:
            from dango.utils.constants import LOCAL_API_URL

            base_url = LOCAL_API_URL
        self.base_url: str = base_url
        self.wallet: Wallet | LocalAccount = wallet
        self.account_address: str = account_address
        self.vault_address: str | None = None
        # `expires_after` is recorded but not currently threaded through the
        # native sign path. See `set_expires_after` for the WHY-comment.
        self.expires_after: int | None = None
        # The native Exchange does the actual signing + simulation +
        # broadcast. We hand it the wallet and account_address verbatim;
        # constructor-level chain_id and nonce auto-resolution happen
        # inside it.
        self._native: NativeExchange = NativeExchange(
            wallet,
            base_url,
            account_address=Addr(account_address),
            timeout=timeout,
        )
        # HL's Exchange holds a `self.info` for things like `market_close`
        # (which reads the user's positions). We embed an HL-shaped Info
        # over the same base_url with `skip_ws=True` because the Exchange
        # never needs websocket subscriptions.
        from dango.hyperliquid_compatibility.info import Info

        self.info: HlInfo = Info(
            base_url=base_url,
            meta=meta,
            perp_dexs=perp_dexs,
            timeout=timeout,
            skip_ws=True,
        )

    # --- Single-order entry points ----------------------------------------

    def order(
        self,
        name: str,
        is_buy: bool,
        sz: float,
        limit_px: float,
        order_type: OrderType,
        reduce_only: bool = False,
        cloid: Cloid | None = None,
        builder: Any = None,
    ) -> dict[str, Any]:
        """Place a single HL-style order. Routes to the native ``submit_order``."""
        # Builder: HL has a fee-share marketplace (the "builder fee" model)
        # where third-party UIs can take a cut. Dango has no analog —
        # raise rather than silently drop the parameter and let HL traders
        # think they're paying their UI builder.
        if builder is not None:
            raise NotImplementedError("Dango has no builder fee marketplace")
        return self.bulk_orders(
            [
                cast(
                    "OrderRequest",
                    {
                        "coin": name,
                        "is_buy": is_buy,
                        "sz": sz,
                        "limit_px": limit_px,
                        "order_type": order_type,
                        "reduce_only": reduce_only,
                        "cloid": cloid,
                    },
                ),
            ],
        )

    def bulk_orders(
        self,
        order_requests: Iterable[OrderRequest],
        *,
        builder: Any = None,
        grouping: Grouping = "na",
    ) -> dict[str, Any]:
        """Place multiple HL-style orders in one batched native call."""
        if builder is not None:
            raise NotImplementedError("Dango has no builder fee marketplace")
        if grouping != "na":
            # `normalTpsl` and `positionTpsl` group parent + child orders
            # under HL's TP/SL attachment semantics. Translating to native
            # `submit_order(tp=..., sl=...)` requires extracting the trigger
            # children out of the request list and pairing them with parents
            # — a non-trivial reshape. We defer.
            raise NotImplementedError(
                f"HL grouping={grouping!r} not yet supported — "
                "use grouping='na' (default) and submit TP/SL via the native "
                "Exchange.submit_conditional_order until this is implemented",
            )
        actions: list[SubmitOrCancelAction] = []
        request_list = list(order_requests)
        for req in request_list:
            pair_id = self.info.name_to_pair(req["coin"])
            cloid = req.get("cloid")
            order_kind = _hl_order_type_to_dango_kind(
                req["order_type"],
                req["limit_px"],
                cloid,
            )
            actions.append(
                _build_submit_action(
                    pair_id,
                    req["is_buy"],
                    req["sz"],
                    order_kind,
                    reduce_only=req.get("reduce_only", False),
                ),
            )
        if not actions:
            raise ValueError("bulk_orders requires at least one order request")
        # Single-order shortcut: avoid the batch overhead by calling
        # submit_order directly. Both paths share the same response
        # envelope shape so callers can treat them interchangeably.
        if len(actions) == 1:
            action = actions[0]
            assert isinstance(action, SubmitAction)  # narrowing for mypy
            outcome = self._native.submit_order(
                action.pair_id,
                action.size,
                action.kind,
                reduce_only=action.reduce_only,
            )
        else:
            outcome = self._native.batch_update_orders(actions)
        return _native_outcome_to_resting_envelope(
            outcome,
            response_type="order",
            expected_count=len(actions),
        )

    # --- Cancel ----------------------------------------------------------

    def cancel(self, name: str, oid: int) -> dict[str, Any]:
        """Cancel one open order by chain ``oid``. ``name`` is verified for parity."""
        # `name` is HL's coin name; we resolve it to a pair_id even though
        # native cancel only needs the oid — this gives a friendlier error
        # if the caller passes a typo (KeyError on lookup) instead of
        # silently sending a cancel for an oid that may belong to a
        # different pair.
        _ = self.info.name_to_pair(name)
        outcome = self._native.cancel_order(OrderId(str(oid)))
        return _native_outcome_to_cancel_envelope(
            outcome,
            response_type="cancel",
            expected_count=1,
        )

    def bulk_cancel(self, cancel_requests: Iterable[dict[str, Any]]) -> dict[str, Any]:
        """Cancel multiple open orders by chain ``oid`` in one batched native call."""
        actions: list[SubmitOrCancelAction] = []
        for req in cancel_requests:
            # Verify each `coin` so a typo fails loudly here rather than
            # silently dispatching the cancel under an unknown pair.
            _ = self.info.name_to_pair(req["coin"])
            actions.append(CancelAction(spec=OrderId(str(req["oid"]))))
        if not actions:
            raise ValueError("bulk_cancel requires at least one cancel request")
        outcome = self._native.batch_update_orders(actions)
        return _native_outcome_to_cancel_envelope(
            outcome,
            response_type="cancel",
            expected_count=len(actions),
        )

    def cancel_by_cloid(self, name: str, cloid: Cloid) -> dict[str, Any]:
        """Cancel one open order by ``cloid``. Hashes the 16-byte HL cloid to Uint64."""
        _ = self.info.name_to_pair(name)
        outcome = self._native.cancel_order(ClientOrderIdRef(value=cloid.to_uint64()))
        # `response_type="cancelByCloid"` matches HL's action type so traders
        # whose dispatch branches on `result["response"]["type"]` keep working.
        return _native_outcome_to_cancel_envelope(
            outcome,
            response_type="cancelByCloid",
            expected_count=1,
        )

    def bulk_cancel_by_cloid(self, cancel_requests: Iterable[dict[str, Any]]) -> dict[str, Any]:
        """Cancel multiple open orders by ``cloid`` in one batched native call."""
        actions: list[SubmitOrCancelAction] = []
        for req in cancel_requests:
            _ = self.info.name_to_pair(req["coin"])
            cloid = req["cloid"]
            if not isinstance(cloid, Cloid):
                raise TypeError(
                    f"each cancel request must carry a Cloid, got {type(cloid).__name__}",
                )
            actions.append(CancelAction(spec=ClientOrderIdRef(value=cloid.to_uint64())))
        if not actions:
            raise ValueError("bulk_cancel_by_cloid requires at least one cancel request")
        outcome = self._native.batch_update_orders(actions)
        return _native_outcome_to_cancel_envelope(
            outcome,
            response_type="cancelByCloid",
            expected_count=len(actions),
        )

    # --- Modify ----------------------------------------------------------

    def modify_order(
        self,
        oid: int | Cloid,
        name: str,
        is_buy: bool,
        sz: float,
        limit_px: float,
        order_type: OrderType,
        reduce_only: bool = False,
        cloid: Cloid | None = None,
    ) -> dict[str, Any]:
        """Atomic cancel + replace: emulated as a single batched action.

        Per the API design notes, Dango has no first-class "modify" message —
        the canonical pattern is an atomic cancel+submit pair via
        ``batch_update_orders``. ``oid`` (the order to cancel) is independent
        from ``cloid`` (the new client-order-id for the resubmitted order).
        """
        return self.bulk_modify_orders_new(
            [
                {
                    "oid": oid,
                    "order": cast(
                        "OrderRequest",
                        {
                            "coin": name,
                            "is_buy": is_buy,
                            "sz": sz,
                            "limit_px": limit_px,
                            "order_type": order_type,
                            "reduce_only": reduce_only,
                            "cloid": cloid,
                        },
                    ),
                },
            ],
        )

    def bulk_modify_orders_new(
        self,
        modify_requests: Iterable[dict[str, Any]],
    ) -> dict[str, Any]:
        """Batch a list of cancel+submit pairs into one ``batch_update_orders`` call."""
        actions: list[SubmitOrCancelAction] = []
        request_list = list(modify_requests)
        for req in request_list:
            oid = req["oid"]
            order_req = cast("OrderRequest", req["order"])
            # Cancel side: `oid` may be an int (chain order id) or a Cloid
            # (the client-assigned id used to bind the original order).
            # Native takes either via different OrderId / ClientOrderIdRef
            # specs; we route accordingly.
            if isinstance(oid, Cloid):
                cancel_spec: OrderId | ClientOrderIdRef = ClientOrderIdRef(
                    value=oid.to_uint64(),
                )
            else:
                cancel_spec = OrderId(str(oid))
            actions.append(CancelAction(spec=cancel_spec))
            # Submit side: same path as bulk_orders.
            pair_id = self.info.name_to_pair(order_req["coin"])
            new_cloid = order_req.get("cloid")
            order_kind = _hl_order_type_to_dango_kind(
                order_req["order_type"],
                order_req["limit_px"],
                new_cloid,
            )
            actions.append(
                _build_submit_action(
                    pair_id,
                    order_req["is_buy"],
                    order_req["sz"],
                    order_kind,
                    reduce_only=order_req.get("reduce_only", False),
                ),
            )
        if not actions:
            raise ValueError("bulk_modify_orders_new requires at least one modify request")
        outcome = self._native.batch_update_orders(actions)
        # `response_type="batchModify"` matches HL's action type so traders'
        # dispatch on `result["response"]["type"]` keeps working. The per-
        # entry shape is the same as a regular order response (one entry
        # per submit; the cancel half is implicit in modify semantics).
        return _native_outcome_to_resting_envelope(
            outcome,
            response_type="batchModify",
            expected_count=len(request_list),
        )

    # --- Market open / close ---------------------------------------------

    def market_open(
        self,
        name: str,
        is_buy: bool,
        sz: float,
        *,
        px: float | None = None,
        slippage: float = DEFAULT_SLIPPAGE,
        cloid: Cloid | None = None,
        builder: Any = None,
    ) -> dict[str, Any]:
        """Place a market order with a slippage cap. ``px`` is ignored."""
        # `px` is HL's "current oracle price ± slippage = limit price" hint.
        # Dango's market order computes its own slippage band internally
        # against the contract's mark price, so HL's `px` is redundant
        # (and potentially wrong if HL's mid drifted from Dango's mark).
        # Document the divergence rather than pretend we use it.
        _ = px
        if builder is not None:
            raise NotImplementedError("Dango has no builder fee marketplace")
        # `cloid` doesn't flow into a native market order: Dango's
        # `submit_market_order` doesn't take a client_order_id. If a
        # caller passes one, raise so they don't silently lose it.
        if cloid is not None:
            raise NotImplementedError(
                "cloid on market orders is not supported — "
                "Dango's submit_market_order does not accept a client_order_id; "
                "use a limit order with tif='Ioc' as a market-equivalent if cloid is required",
            )
        pair_id = self.info.name_to_pair(name)
        outcome = self._native.submit_market_order(
            pair_id,
            _signed_size(is_buy, sz),
            max_slippage=slippage,
        )
        return _native_outcome_to_resting_envelope(
            outcome,
            response_type="order",
            expected_count=1,
        )

    def market_close(
        self,
        coin: str,
        *,
        sz: float | None = None,
        px: float | None = None,
        slippage: float = DEFAULT_SLIPPAGE,
        cloid: Cloid | None = None,
        builder: Any = None,
    ) -> dict[str, Any]:
        """Reduce-only market order to close the position in ``coin``."""
        _ = px  # See market_open — Dango ignores HL's px hint.
        if builder is not None:
            raise NotImplementedError("Dango has no builder fee marketplace")
        if cloid is not None:
            raise NotImplementedError(
                "cloid on market orders is not supported — "
                "Dango's submit_market_order does not accept a client_order_id",
            )
        # Read the user's position in `coin` to determine close direction
        # and (if not specified) close size. HL's market_close reads
        # `assetPositions` for this; we use the same path via the embedded
        # HL Info wrapper.
        state = self.info.user_state(self.account_address)
        positions = state.get("assetPositions", [])
        target_position: dict[str, Any] | None = None
        for entry in positions:
            inner = entry.get("position", {})
            if inner.get("coin") == coin:
                target_position = inner
                break
        if target_position is None:
            return hl_status_envelope(
                response_type="order",
                error=f"no open position in {coin!r} to close",
            )
        # `szi` is HL's signed size: positive = long, negative = short.
        # Closing a long means selling (negative size); closing a short
        # means buying (positive size). The signed close size is the
        # OPPOSITE of `szi`.
        szi_str = target_position.get("szi", "0")
        try:
            szi = float(szi_str)
        except ValueError, TypeError:
            szi = 0.0
        if szi == 0:
            return hl_status_envelope(
                response_type="order",
                error=f"position in {coin!r} has zero size",
            )
        # Default `sz = abs(szi)` if not specified (HL semantics: close
        # the entire position). Caller-supplied `sz` is always positive
        # (HL convention); we apply the sign based on position direction.
        close_size_abs = sz if sz is not None else abs(szi)
        close_signed_size = -close_size_abs if szi > 0 else close_size_abs
        pair_id = self.info.name_to_pair(coin)
        outcome = self._native.submit_market_order(
            pair_id,
            close_signed_size,
            max_slippage=slippage,
            reduce_only=True,
        )
        return _native_outcome_to_resting_envelope(
            outcome,
            response_type="order",
            expected_count=1,
        )

    # --- Referral --------------------------------------------------------

    def set_referrer(self, code: str) -> dict[str, Any]:
        """Bind the signer as a referee of ``code`` (HL username form)."""
        # HL's `set_referrer` takes a string code (username). Native
        # `set_referral` accepts `int | str`; we forward the str directly
        # so the username-lookup happens chain-side.
        outcome = self._native.set_referral(code)
        # HL's response uses `type="setReferrer"`. We don't have per-entry
        # statuses to populate (the action either succeeded or failed); a
        # success is represented by an empty statuses list, an error by an
        # err envelope.
        error_message = _extract_error_message(outcome)
        if error_message is not None:
            return hl_status_envelope(response_type="setReferrer", error=error_message)
        return hl_status_envelope(response_type="setReferrer", statuses=[])

    # --- Signing-time hints -----------------------------------------------

    def set_expires_after(self, expires_after: int | None) -> None:
        """Record an HL ``expiresAfter`` ms hint to attach to subsequent txs.

        Phase 17 stores the value but does NOT yet thread it through to the
        native sign path: ``Metadata.expiry`` is typed in Phase 5 but the
        native ``Exchange`` constructor doesn't currently expose a per-tx
        expiry override. Wiring this end-to-end is a follow-up item; for
        now this is a no-op-with-state-storage so HL traders' code that
        calls it doesn't crash.
        """
        # Stored on `self` for future plumbing (see docstring) — currently
        # not consulted by any code path. We intentionally don't add a
        # warning-on-read because callers will discover the gap when an
        # expired tx is accepted (which won't happen until the wiring lands).
        self.expires_after = expires_after

    # --- NotImplementedError stubs ----------------------------------------
    #
    # The methods below either have no Dango analog (hard gaps) or were
    # explicitly deferred out of Phase 17 (Phase-17-deferred). Each stub
    # carries a one-line reason. We do NOT silently no-op — HL traders
    # whose code calls these need a loud failure to route around the gap.

    def update_leverage(
        self,
        leverage: int,
        name: str,
        is_cross: bool = True,
    ) -> Any:
        raise NotImplementedError(
            "Dango is cross-margin only; per-asset leverage cannot be set. "
            "Margin requirement is determined by `pair_param.initial_margin_ratio`.",
        )

    def update_isolated_margin(self, amount: float, name: str) -> Any:
        raise NotImplementedError("Dango has no isolated margin")

    def schedule_cancel(self, time: int | None) -> Any:
        raise NotImplementedError("Dango has no scheduled cancellation")

    def usd_class_transfer(self, amount: float, to_perp: bool) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot/perp split")

    def send_asset(
        self,
        destination: str,
        source_dex: str,
        destination_dex: str,
        token: str,
        amount: float,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot/perp split")

    def vault_usd_transfer(
        self,
        vault_address: str,
        is_deposit: bool,
        usd: int,
    ) -> Any:
        raise NotImplementedError(
            "Use add_liquidity / remove_liquidity on the native Exchange instead",
        )

    def sub_account_transfer(
        self,
        sub_account_user: str,
        is_deposit: bool,
        usd: int,
    ) -> Any:
        raise NotImplementedError(
            "deferred — needs Grug bank-message wrapping",
        )

    def sub_account_spot_transfer(
        self,
        sub_account_user: str,
        is_deposit: bool,
        token: str,
        amount: float,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot transfers")

    def approve_builder_fee(self, builder: str, max_fee_rate: str) -> Any:
        raise NotImplementedError("Dango has no builder fee marketplace")

    def convert_to_multi_sig_user(
        self,
        authorized_users: list[str],
        threshold: int,
    ) -> Any:
        raise NotImplementedError("Dango multi-sig is not exposed via the perps SDK")

    def multi_sig(
        self,
        multi_sig_user: str,
        inner_action: Any,
        signatures: Any,
        nonce: int,
        vault_address: str | None = None,
    ) -> Any:
        raise NotImplementedError("Dango multi-sig is not exposed via the perps SDK")

    def create_sub_account(self, name: str) -> Any:
        raise NotImplementedError(
            "deferred — needs account-factory register_account wrapping",
        )

    def usd_transfer(self, amount: float, destination: str) -> Any:
        raise NotImplementedError(
            "deferred — needs Grug bank-message wrapping",
        )

    def withdraw_from_bridge(self, amount: float, destination: str) -> Any:
        raise NotImplementedError(
            "deferred — needs Hyperlane warp transfer",
        )

    def spot_transfer(self, amount: float, destination: str, token: str) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot transfers")

    def approve_agent(self, name: str | None = None) -> Any:
        raise NotImplementedError(
            "deferred — Dango session credentials need separate API",
        )

    def agent_enable_dex_abstraction(self) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def agent_set_abstraction(self, abstraction: Any) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def user_dex_abstraction(self, user: str, enabled: bool) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def user_set_abstraction(self, user: str, abstraction: Any) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def token_delegate(self, validator: str, wei: int, is_undelegate: bool) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def use_big_blocks(self, enable: bool) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def c_signer_unjail_self(self) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def c_signer_jail_self(self) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def c_validator_register(
        self,
        node_ip: str,
        name: str,
        description: str,
        delegations_disabled: bool,
        commission_bps: int,
        signer: str,
        unjailed: bool,
        initial_wei: int,
    ) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def c_validator_change_profile(
        self,
        node_ip: str | None,
        name: str | None,
        description: str | None,
        unjailed: bool,
        disable_delegations: bool | None,
        commission_bps: int | None,
        signer: str | None,
    ) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def c_validator_unregister(self) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def noop(self, nonce: int) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def gossip_priority_bid(self, slot_id: int, ip: str, max_gas: int) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def spot_deploy_register_token(
        self,
        token_name: str,
        sz_decimals: int,
        wei_decimals: int,
        max_gas: int,
        full_name: str,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_user_genesis(
        self,
        token: int,
        user_and_wei: list[tuple[str, str]],
        existing_token_and_wei: list[tuple[int, str]],
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_enable_freeze_privilege(self, token: int) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_freeze_user(self, token: int, user: str, freeze: bool) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_revoke_freeze_privilege(self, token: int) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_enable_quote_token(self, token: int) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_token_action_inner(self, variant: str, token: int) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_genesis(
        self,
        token: int,
        max_supply: str,
        no_hyperliquidity: bool,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_register_spot(self, base_token: int, quote_token: int) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_register_hyperliquidity(
        self,
        spot: int,
        start_px: float,
        order_sz: float,
        n_orders: int,
        n_seeded_levels: int | None,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def spot_deploy_set_deployer_trading_fee_share(
        self,
        token: int,
        share: str,
    ) -> Any:
        raise NotImplementedError("Dango is perps-only; no spot deploys")

    def perp_deploy_register_asset(
        self,
        dex: str,
        max_gas: int | None,
        coin: str,
        sz_decimals: int,
        oracle_px: str,
        margin_table_id: int,
        only_isolated: bool,
        schema: Any,
    ) -> Any:
        raise NotImplementedError("Dango has no permissionless perp deploys")

    def perp_deploy_set_oracle(
        self,
        dex: str,
        oracle_pxs: dict[str, str],
        all_mark_pxs: list[dict[str, str]],
        external_perp_pxs: dict[str, str],
    ) -> Any:
        raise NotImplementedError("Dango has no permissionless perp deploys")


# Re-export the `dango_decimal_to_hl_str` symbol so callers that follow
# the HL pattern of `from .exchange import ...` for the wire helpers find
# them in the expected place. Mirrors the Phase 16 layout where the
# `Info` module re-exports `dango_decimal_to_hl_str`.
__all__ = [
    "Exchange",
    "dango_decimal_to_hl_str",
]

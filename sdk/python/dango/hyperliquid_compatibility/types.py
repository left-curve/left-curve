"""HL-shaped TypedDicts and helpers used by the Hyperliquid-compat layer.

This module mirrors the public types exported by the upstream
``hyperliquid`` Python SDK (``hyperliquid/utils/types.py`` and the type
defs section of ``hyperliquid/utils/signing.py``) so HL traders can
import them by name with no source-level changes:

    from dango.hyperliquid_compatibility.types import OrderRequest, Cloid

The shapes are byte-compatible with the HL wire format (camelCase keys
preserved as-is). The syntax is modernised (PEP 604/585) since the rest
of the SDK targets Python 3.14, but every TypedDict / type-alias name
matches HL's exactly.

Asymmetry warning for ``Cloid``
-------------------------------

HL's ``Cloid`` is a 16-byte (128-bit) hex value. Dango's
``ClientOrderId`` is a ``Uint64`` (64 bits). To preserve HL input
parity, ``Cloid`` accepts the same 16-byte hex string HL expects, but
when the SDK forwards it to the Dango contract it deterministically
hashes the cloid down to a Uint64 via ``Cloid.to_uint64()`` (SHA-256 of
the lowercase 0x-prefixed hex bytes, first 8 bytes big-endian).

This means **the cloid you see in responses is NOT the cloid you sent**
- responses carry the Uint64 the contract recorded, not the original
16-byte HL cloid. Callers who rely on round-tripping cloids must keep
their own mapping from HL cloid -> Uint64 (e.g. by calling
``Cloid.to_uint64()`` themselves before submitting and matching against
the response). The hash is deterministic, so the same HL cloid always
maps to the same Uint64.
"""

from __future__ import annotations

import hashlib
from decimal import Decimal
from typing import Any, Literal, NotRequired, TypedDict

# --- Cloid -------------------------------------------------------------------
# Defined first so other TypedDicts can reference it without forward
# strings — `from __future__ import annotations` already makes every
# annotation lazy, so we don't need quoted forward refs.


class Cloid:
    """HL-compatible 16-byte hex client-order id; hashes to Uint64 for Dango.

    Round-trip warning: the cloid in Dango responses is the Uint64
    produced by ``to_uint64()``, not the 16-byte hex you constructed
    this with. See the module docstring for the full rationale.
    """

    def __init__(self, raw_cloid: str) -> None:
        self._raw_cloid: str = raw_cloid
        self._validate()

    def _validate(self) -> None:
        if not self._raw_cloid[:2] == "0x":
            raise TypeError("cloid is not a hex string")
        if not len(self._raw_cloid[2:]) == 32:
            raise TypeError("cloid is not 16 bytes")

    def __str__(self) -> str:
        return str(self._raw_cloid)

    def __repr__(self) -> str:
        return str(self._raw_cloid)

    @staticmethod
    def from_int(cloid: int) -> Cloid:
        return Cloid(f"{cloid:#034x}")

    @staticmethod
    def from_str(cloid: str) -> Cloid:
        return Cloid(cloid)

    def to_raw(self) -> str:
        return self._raw_cloid

    def to_uint64(self) -> int:
        """SHA-256-derived ``Uint64`` the Dango contract uses as ``ClientOrderId``.

        Hash policy: SHA-256 over the lowercase 0x-prefixed hex bytes,
        first 8 bytes decoded big-endian. Deterministic — the same
        ``Cloid`` always maps to the same ``Uint64`` — but lossy: an
        HL cloid carries 128 bits of entropy and a Dango
        ``ClientOrderId`` is only 64 bits, so two different cloids
        could in principle collide here (probability ~2**-32 over
        ~4e9 cloids; negligible for any realistic trading workflow).
        """
        digest = hashlib.sha256(self._raw_cloid.lower().encode("ascii")).digest()
        return int.from_bytes(digest[:8], byteorder="big")


# --- Domain types (HL `utils/types.py`) -------------------------------------


class AssetInfo(TypedDict):
    name: str
    szDecimals: int  # noqa: N815


class Meta(TypedDict):
    universe: list[AssetInfo]


Side = Literal["A", "B"]
SIDES: list[Side] = ["A", "B"]


class SpotAssetInfo(TypedDict):
    name: str
    tokens: list[int]
    index: int
    isCanonical: bool  # noqa: N815


class SpotTokenInfo(TypedDict):
    name: str
    szDecimals: int  # noqa: N815
    weiDecimals: int  # noqa: N815
    index: int
    tokenId: str  # noqa: N815
    isCanonical: bool  # noqa: N815
    evmContract: str | None  # noqa: N815
    fullName: str | None  # noqa: N815


class SpotMeta(TypedDict):
    universe: list[SpotAssetInfo]
    tokens: list[SpotTokenInfo]


class SpotAssetCtx(TypedDict):
    dayNtlVlm: str  # noqa: N815
    markPx: str  # noqa: N815
    midPx: str | None  # noqa: N815
    prevDayPx: str  # noqa: N815
    circulatingSupply: str  # noqa: N815
    coin: str


SpotMetaAndAssetCtxs = tuple[SpotMeta, list[SpotAssetCtx]]


# --- Subscriptions (HL `utils/types.py`) ------------------------------------
#
# These TypedDicts use the field name `type`, which shadows the builtin
# `type` inside the class body. That's harmless for TypedDicts (they have
# no methods), so we use class form for readability. Ruff's `UP013` would
# convert functional form anyway.


class AllMidsSubscription(TypedDict):
    type: Literal["allMids"]


class BboSubscription(TypedDict):
    type: Literal["bbo"]
    coin: str


class L2BookSubscription(TypedDict):
    type: Literal["l2Book"]
    coin: str


class TradesSubscription(TypedDict):
    type: Literal["trades"]
    coin: str


class UserEventsSubscription(TypedDict):
    type: Literal["userEvents"]
    user: str


class UserFillsSubscription(TypedDict):
    type: Literal["userFills"]
    user: str


class CandleSubscription(TypedDict):
    type: Literal["candle"]
    coin: str
    interval: str


class OrderUpdatesSubscription(TypedDict):
    type: Literal["orderUpdates"]
    user: str


class UserFundingsSubscription(TypedDict):
    type: Literal["userFundings"]
    user: str


class UserNonFundingLedgerUpdatesSubscription(TypedDict):
    type: Literal["userNonFundingLedgerUpdates"]
    user: str


class WebData2Subscription(TypedDict):
    type: Literal["webData2"]
    user: str


class ActiveAssetCtxSubscription(TypedDict):
    type: Literal["activeAssetCtx"]
    coin: str


class ActiveAssetDataSubscription(TypedDict):
    type: Literal["activeAssetData"]
    user: str
    coin: str


Subscription = (
    AllMidsSubscription
    | BboSubscription
    | L2BookSubscription
    | TradesSubscription
    | UserEventsSubscription
    | UserFillsSubscription
    | CandleSubscription
    | OrderUpdatesSubscription
    | UserFundingsSubscription
    | UserNonFundingLedgerUpdatesSubscription
    | WebData2Subscription
    | ActiveAssetCtxSubscription
    | ActiveAssetDataSubscription
)


# --- WebSocket data and message envelopes -----------------------------------


class AllMidsData(TypedDict):
    mids: dict[str, str]


class AllMidsMsg(TypedDict):
    channel: Literal["allMids"]
    data: AllMidsData


class L2Level(TypedDict):
    px: str
    sz: str
    n: int


class L2BookData(TypedDict):
    coin: str
    levels: tuple[list[L2Level], list[L2Level]]
    time: int


class L2BookMsg(TypedDict):
    channel: Literal["l2Book"]
    data: L2BookData


class BboData(TypedDict):
    coin: str
    time: int
    bbo: tuple[L2Level | None, L2Level | None]


class BboMsg(TypedDict):
    channel: Literal["bbo"]
    data: BboData


class PongMsg(TypedDict):
    channel: Literal["pong"]


class Trade(TypedDict):
    coin: str
    side: Side
    px: str
    sz: int
    hash: str
    time: int


class CrossLeverage(TypedDict):
    type: Literal["cross"]
    value: int


class IsolatedLeverage(TypedDict):
    type: Literal["isolated"]
    value: int
    rawUsd: str  # noqa: N815


Leverage = CrossLeverage | IsolatedLeverage


class TradesMsg(TypedDict):
    channel: Literal["trades"]
    data: list[Trade]


class PerpAssetCtx(TypedDict):
    funding: str
    openInterest: str  # noqa: N815
    prevDayPx: str  # noqa: N815
    dayNtlVlm: str  # noqa: N815
    premium: str
    oraclePx: str  # noqa: N815
    markPx: str  # noqa: N815
    midPx: str | None  # noqa: N815
    impactPxs: tuple[str, str] | None  # noqa: N815
    dayBaseVlm: str  # noqa: N815


class ActiveAssetCtx(TypedDict):
    coin: str
    ctx: PerpAssetCtx


class ActiveSpotAssetCtx(TypedDict):
    coin: str
    ctx: SpotAssetCtx


class ActiveAssetCtxMsg(TypedDict):
    channel: Literal["activeAssetCtx"]
    data: ActiveAssetCtx


class ActiveSpotAssetCtxMsg(TypedDict):
    channel: Literal["activeSpotAssetCtx"]
    data: ActiveSpotAssetCtx


class ActiveAssetData(TypedDict):
    user: str
    coin: str
    leverage: Leverage
    maxTradeSzs: tuple[str, str]  # noqa: N815
    availableToTrade: tuple[str, str]  # noqa: N815
    markPx: str  # noqa: N815


class ActiveAssetDataMsg(TypedDict):
    channel: Literal["activeAssetData"]
    data: ActiveAssetData


class Fill(TypedDict):
    coin: str
    px: str
    sz: str
    side: Side
    time: int
    startPosition: str  # noqa: N815
    dir: str
    closedPnl: str  # noqa: N815
    hash: str
    oid: int
    crossed: bool
    fee: str
    tid: int
    feeToken: str  # noqa: N815


# `total=False` mirrors HL: `UserEventsData` may carry just `{"fills": ...}`
# or be empty depending on the event subtype.
class UserEventsData(TypedDict, total=False):
    fills: list[Fill]


class UserEventsMsg(TypedDict):
    channel: Literal["user"]
    data: UserEventsData


class UserFillsData(TypedDict):
    user: str
    isSnapshot: bool  # noqa: N815
    fills: list[Fill]


class UserFillsMsg(TypedDict):
    channel: Literal["userFills"]
    data: UserFillsData


# HL groups several heterogeneous channels under a catch-all envelope; the
# `data` payload is intentionally `Any` because the shape varies per channel.
class OtherWsMsg(TypedDict, total=False):
    channel: Literal[
        "candle",
        "orderUpdates",
        "userFundings",
        "userNonFundingLedgerUpdates",
        "webData2",
    ]
    data: Any


WsMsg = (
    AllMidsMsg
    | BboMsg
    | L2BookMsg
    | TradesMsg
    | UserEventsMsg
    | PongMsg
    | UserFillsMsg
    | OtherWsMsg
    | ActiveAssetCtxMsg
    | ActiveSpotAssetCtxMsg
    | ActiveAssetDataMsg
)


# --- Builder / abstraction / dex schema -------------------------------------


class BuilderInfo(TypedDict):
    # `b` is the builder's public address; `f` is the fee in tenths of basis
    # points (10 == 1 bp).
    b: str
    f: int


Abstraction = Literal["unifiedAccount", "portfolioMargin", "disabled"]
AgentAbstraction = Literal["u", "p", "i"]


class PerpDexSchemaInput(TypedDict):
    fullName: str  # noqa: N815
    collateralToken: int  # noqa: N815
    oracleUpdater: str | None  # noqa: N815


# --- Order types (HL `utils/signing.py`) ------------------------------------

Tif = Literal["Alo", "Ioc", "Gtc"]
Tpsl = Literal["tp", "sl"]


class LimitOrderType(TypedDict):
    tif: Tif


class TriggerOrderType(TypedDict):
    triggerPx: float  # noqa: N815
    isMarket: bool  # noqa: N815
    tpsl: Tpsl


class TriggerOrderTypeWire(TypedDict):
    triggerPx: str  # noqa: N815
    isMarket: bool  # noqa: N815
    tpsl: Tpsl


# `total=False` because HL allows `OrderType` to carry exactly one of
# `limit` / `trigger` (never both, never neither in practice — but the
# TypedDict can't express that constraint).
class OrderType(TypedDict, total=False):
    limit: LimitOrderType
    trigger: TriggerOrderType


class OrderTypeWire(TypedDict, total=False):
    limit: LimitOrderType
    trigger: TriggerOrderTypeWire


# `OrderRequest` uses snake_case keys (`is_buy`, `limit_px`, `order_type`,
# `reduce_only`) because that's what HL exposes at the Python API level —
# the wire-shape (camelCase, single-letter keys) is `OrderWire` below.
class OrderRequest(TypedDict, total=False):
    coin: str
    is_buy: bool
    sz: float
    limit_px: float
    order_type: OrderType
    reduce_only: bool
    cloid: NotRequired[Cloid | None]


OidOrCloid = int | Cloid


class ModifyRequest(TypedDict, total=False):
    oid: int | Cloid
    order: OrderRequest


class CancelRequest(TypedDict):
    coin: str
    oid: int


class CancelByCloidRequest(TypedDict):
    coin: str
    cloid: Cloid


class PriorityGrouping(TypedDict):
    p: int


Grouping = Literal["na", "normalTpsl", "positionTpsl"] | PriorityGrouping


class Order(TypedDict):
    asset: int
    isBuy: bool  # noqa: N815
    limitPx: float  # noqa: N815
    sz: float
    reduceOnly: bool  # noqa: N815
    cloid: Cloid | None


# Single-letter keys are the HL wire shape — DO NOT rename to snake_case.
class OrderWire(TypedDict):
    a: int
    b: bool
    p: str
    s: str
    r: bool
    t: OrderTypeWire
    c: NotRequired[str | None]


class ModifyWire(TypedDict):
    oid: int
    order: OrderWire


class ScheduleCancelAction(TypedDict):
    type: Literal["scheduleCancel"]
    time: NotRequired[int | None]


# --- HL wire helpers --------------------------------------------------------


def dango_decimal_to_hl_str(x: str) -> str:
    """Strip trailing zeros from a Dango decimal string for HL wire shape.

    Dango canonicalises numbers as fixed-decimal strings (e.g. ``"1.230000"``)
    via ``dango_decimal``. HL's wire shape strips trailing zeros and drops
    a bare decimal point if no fractional part remains (e.g. ``"1.23"``,
    ``"1"``). This helper performs that conversion.

    Examples:
        ``"1.230000"`` -> ``"1.23"``
        ``"1.000000"`` -> ``"1"``
        ``"0.000000"`` -> ``"0"``
        ``"-1.500000"`` -> ``"-1.5"``
        ``"5"`` -> ``"5"``
    """
    # `Decimal.normalize()` preserves the sign of zero (`Decimal("-0.000000")`
    # normalizes to `Decimal("-0")`), so a separate zero short-circuit is
    # required to collapse `"-0"` → `"0"`.
    d = Decimal(x)
    if d.is_zero():
        return "0"
    # `Decimal.normalize()` returns scientific notation for whole numbers
    # >= 10 (e.g. `"10"` becomes `Decimal("1E+1")`), but `format(d, "f")`
    # always produces fixed-point output, so this composition is safe.
    return format(d.normalize(), "f")


# Per-status entry shape factories. Phase 17 (HL exchange) calls these from
# its translator; Phase 15 only provides them so the shapes live in one place.
HlRestingEntry = dict[str, dict[str, int]]
HlFilledEntry = dict[str, dict[str, str | int]]
HlErrorEntry = dict[str, str]
HlStatusEntry = HlRestingEntry | HlFilledEntry | HlErrorEntry


def hl_resting_entry(oid: int) -> HlRestingEntry:
    """One HL ``"resting"`` status entry: the order is on the book."""
    return {"resting": {"oid": oid}}


def hl_filled_entry(*, total_sz: str, avg_px: str, oid: int) -> HlFilledEntry:
    """One HL ``"filled"`` status entry: the order matched."""
    return {"filled": {"totalSz": total_sz, "avgPx": avg_px, "oid": oid}}


def hl_error_entry(message: str) -> HlErrorEntry:
    """One HL per-order error status entry."""
    return {"error": message}


def hl_status_envelope(
    *,
    response_type: str,
    statuses: list[HlStatusEntry] | None = None,
    error: str | None = None,
) -> dict[str, Any]:
    """Wrap a Dango outcome into HL's status envelope.

    If ``error`` is set, the err shape is returned:
    ``{"status": "err", "response": <error>}``. Otherwise the ok shape
    is returned with ``response.type`` set to ``response_type`` and
    ``response.data.statuses`` to the supplied list (or ``[]``).

    HL never raises on logical errors; callers inspect ``result["status"]``.
    """
    if error is not None:
        return {"status": "err", "response": error}
    return {
        "status": "ok",
        "response": {
            "type": response_type,
            "data": {"statuses": statuses or []},
        },
    }

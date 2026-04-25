"""Typed wire shapes and dango_decimal helper used across the SDK."""

from __future__ import annotations

from decimal import Decimal, InvalidOperation
from enum import StrEnum
from typing import Final, Literal, NewType, NotRequired, TypedDict

# --- Numeric helper ----------------------------------------------------------

_DEFAULT_MAX_PLACES: Final[int] = 6


def dango_decimal(x: float | int | str | Decimal, max_places: int = _DEFAULT_MAX_PLACES) -> str:
    """Return the canonical fixed-decimal string form of x; raise if precision lost."""
    if isinstance(x, bool) or not isinstance(x, int | float | str | Decimal):
        raise TypeError(f"unsupported type for dango_decimal: {type(x).__name__}")

    if isinstance(x, float):
        if x != x or x in (float("inf"), float("-inf")):
            raise ValueError(f"dango_decimal does not accept non-finite floats: {x!r}")
        d = Decimal(str(x))
    elif isinstance(x, str):
        try:
            d = Decimal(x)
        except InvalidOperation as exc:
            raise ValueError(f"invalid decimal string: {x!r}") from exc
    elif isinstance(x, int):
        d = Decimal(x)
    else:
        d = x

    if not d.is_finite():
        raise ValueError(f"dango_decimal does not accept non-finite Decimals: {x!r}")

    exponent = d.as_tuple().exponent
    if isinstance(exponent, int) and exponent < -max_places:
        raise ValueError(f"value {x!r} requires more than {max_places} decimal places of precision")

    return f"{d:.{max_places}f}"


# --- Identifier aliases ------------------------------------------------------

Addr = NewType("Addr", str)
Hash256 = NewType("Hash256", str)
Binary = NewType("Binary", str)
PairId = NewType("PairId", str)
OrderId = NewType("OrderId", str)
ConditionalOrderId = NewType("ConditionalOrderId", str)
FillId = NewType("FillId", str)
ClientOrderId = NewType("ClientOrderId", str)
UserIndex = NewType("UserIndex", int)
Nonce = NewType("Nonce", int)

Dimensionless = NewType("Dimensionless", str)
Quantity = NewType("Quantity", str)
UsdValue = NewType("UsdValue", str)
UsdPrice = NewType("UsdPrice", str)
FundingPerUnit = NewType("FundingPerUnit", str)
FundingRate = NewType("FundingRate", str)
Days = NewType("Days", str)

Timestamp = NewType("Timestamp", str)
Duration = NewType("Duration", str)

Uint64 = NewType("Uint64", str)
Uint128 = NewType("Uint128", str)

Referrer = NewType("Referrer", int)
Referee = NewType("Referee", int)
FeeShareRatio = NewType("FeeShareRatio", str)
CommissionRate = NewType("CommissionRate", str)


# --- Enums -------------------------------------------------------------------


class TimeInForce(StrEnum):
    GTC = "GTC"
    IOC = "IOC"
    POST = "POST"


class TriggerDirection(StrEnum):
    ABOVE = "Above"
    BELOW = "Below"


class CandleInterval(StrEnum):
    """GraphQL indexer enum form; the contract's strum-Display uses `1s`/`1m`/... separately."""

    ONE_SECOND = "ONE_SECOND"
    ONE_MINUTE = "ONE_MINUTE"
    FIVE_MINUTES = "FIVE_MINUTES"
    FIFTEEN_MINUTES = "FIFTEEN_MINUTES"
    ONE_HOUR = "ONE_HOUR"
    FOUR_HOURS = "FOUR_HOURS"
    ONE_DAY = "ONE_DAY"
    ONE_WEEK = "ONE_WEEK"


class ReasonForOrderRemoval(StrEnum):
    FILLED = "Filled"
    CANCELED = "Canceled"
    POSITION_CLOSED = "PositionClosed"
    SELF_TRADE_PREVENTION = "SelfTradePrevention"
    LIQUIDATED = "Liquidated"
    DELEVERAGED = "Deleveraged"
    SLIPPAGE_EXCEEDED = "SlippageExceeded"
    PRICE_BAND_VIOLATION = "PriceBandViolation"
    SLIPPAGE_CAP_TIGHTENED = "SlippageCapTightened"


class KeyType(StrEnum):
    SECP256R1 = "Secp256r1"
    SECP256K1 = "Secp256k1"
    ETHEREUM = "Ethereum"


class AccountStatus(StrEnum):
    INACTIVE = "Inactive"
    ACTIVE = "Active"
    FROZEN = "Frozen"


# --- Auth: Key / Signature / Credential primitives ---------------------------


class _KeySecp256r1(TypedDict):
    Secp256r1: Binary


class _KeySecp256k1(TypedDict):
    Secp256k1: Binary


class _KeyEthereum(TypedDict):
    Ethereum: Addr


Key = _KeySecp256r1 | _KeySecp256k1 | _KeyEthereum


ClientData = TypedDict(  # noqa: UP013
    "ClientData",
    {
        "type": str,
        "challenge": str,
        "origin": str,
        "crossOrigin": NotRequired[bool | None],
    },
)


class PasskeySignature(TypedDict):
    authenticator_data: Binary
    client_data: Binary
    sig: Binary


class Eip712Signature(TypedDict):
    typed_data: Binary
    sig: Binary


class _SignaturePasskey(TypedDict):
    Passkey: PasskeySignature


class _SignatureSecp256k1(TypedDict):
    Secp256k1: Binary


class _SignatureEip712(TypedDict):
    Eip712: Eip712Signature


Signature = _SignaturePasskey | _SignatureSecp256k1 | _SignatureEip712


class StandardCredential(TypedDict):
    key_hash: Hash256
    signature: Signature


class SessionInfo(TypedDict):
    chain_id: str
    session_key: Binary
    expire_at: Timestamp


class SessionCredential(TypedDict):
    session_info: SessionInfo
    session_signature: Binary
    authorization: StandardCredential


class _CredentialStandard(TypedDict):
    Standard: StandardCredential


class _CredentialSession(TypedDict):
    Session: SessionCredential


Credential = _CredentialStandard | _CredentialSession


class Metadata(TypedDict):
    user_index: UserIndex
    chain_id: str
    nonce: Nonce
    expiry: Timestamp | None


# --- Tx primitives -----------------------------------------------------------


Message = dict[str, object]


class SignDoc(TypedDict):
    sender: Addr
    gas_limit: int
    messages: list[Message]
    data: Metadata


class UnsignedTx(TypedDict):
    sender: Addr
    msgs: list[Message]
    data: Metadata


class Tx(TypedDict):
    sender: Addr
    gas_limit: int
    msgs: list[Message]
    data: Metadata
    credential: Credential


# --- Order primitives --------------------------------------------------------


class _MarketPayload(TypedDict):
    max_slippage: Dimensionless


class MarketKind(TypedDict):
    Market: _MarketPayload


class _LimitPayload(TypedDict):
    limit_price: UsdPrice
    time_in_force: NotRequired[TimeInForce]
    client_order_id: NotRequired[ClientOrderId | None]


class LimitKind(TypedDict):
    Limit: _LimitPayload


OrderKind = MarketKind | LimitKind


class ChildOrder(TypedDict):
    trigger_price: UsdPrice
    max_slippage: Dimensionless
    size: Quantity | None


class SubmitOrderRequest(TypedDict):
    pair_id: PairId
    size: Quantity
    kind: OrderKind
    reduce_only: bool
    tp: ChildOrder | None
    sl: ChildOrder | None


class _CancelOne(TypedDict):
    One: OrderId


class _CancelOneByClientOrderId(TypedDict):
    OneByClientOrderId: ClientOrderId


CancelOrderRequest = _CancelOne | _CancelOneByClientOrderId | Literal["All"]


class _SubmitOrCancelSubmit(TypedDict):
    Submit: SubmitOrderRequest


class _SubmitOrCancelCancel(TypedDict):
    Cancel: CancelOrderRequest


SubmitOrCancelOrderRequest = _SubmitOrCancelSubmit | _SubmitOrCancelCancel


class _CancelConditionalOnePayload(TypedDict):
    pair_id: PairId
    trigger_direction: TriggerDirection


class _CancelConditionalOne(TypedDict):
    One: _CancelConditionalOnePayload


class _CancelConditionalAllForPairPayload(TypedDict):
    pair_id: PairId


class _CancelConditionalAllForPair(TypedDict):
    AllForPair: _CancelConditionalAllForPairPayload


CancelConditionalOrderRequest = (
    _CancelConditionalOne | _CancelConditionalAllForPair | Literal["All"]
)


class ConditionalOrder(TypedDict):
    order_id: ConditionalOrderId
    size: Quantity | None
    trigger_price: UsdPrice
    max_slippage: Dimensionless


# --- Position / user state ---------------------------------------------------


class Position(TypedDict):
    size: Quantity
    entry_price: UsdPrice
    entry_funding_per_unit: FundingPerUnit
    conditional_order_above: ConditionalOrder | None
    conditional_order_below: ConditionalOrder | None


class PositionExtended(TypedDict):
    size: Quantity
    entry_price: UsdPrice
    entry_funding_per_unit: FundingPerUnit
    conditional_order_above: ConditionalOrder | None
    conditional_order_below: ConditionalOrder | None
    unrealized_pnl: UsdValue | None
    unrealized_funding: UsdValue | None
    liquidation_price: UsdPrice | None


class Unlock(TypedDict):
    end_time: Timestamp
    amount_to_release: UsdValue


class UserState(TypedDict):
    margin: UsdValue
    vault_shares: Uint128
    positions: dict[PairId, Position]
    unlocks: list[Unlock]
    reserved_margin: UsdValue
    open_order_count: int


class UserStateExtended(TypedDict):
    margin: UsdValue
    vault_shares: Uint128
    unlocks: list[Unlock]
    reserved_margin: UsdValue
    open_order_count: int
    equity: UsdValue | None
    available_margin: UsdValue | None
    maintenance_margin: UsdValue | None
    positions: dict[PairId, PositionExtended]


# --- Pair / market data ------------------------------------------------------


class RateSchedule(TypedDict):
    base: Dimensionless
    tiers: dict[UsdValue, Dimensionless]


class ReferrerSettings(TypedDict):
    commission_rate: CommissionRate
    share_ratio: FeeShareRatio


class RefereeStats(TypedDict):
    registered_at: Timestamp
    volume: UsdValue
    commission_earned: UsdValue
    last_day_active: Timestamp


class Param(TypedDict):
    max_unlocks: int
    max_open_orders: int
    maker_fee_rates: RateSchedule
    taker_fee_rates: RateSchedule
    protocol_fee_rate: Dimensionless
    liquidation_fee_rate: Dimensionless
    liquidation_buffer_ratio: Dimensionless
    funding_period: Duration
    vault_total_weight: Dimensionless
    vault_cooldown_period: Duration
    referral_active: bool
    min_referrer_volume: UsdValue
    referrer_commission_rates: RateSchedule
    vault_deposit_cap: UsdValue | None
    max_action_batch_size: int


class State(TypedDict):
    last_funding_time: Timestamp
    vault_share_supply: Uint128
    insurance_fund: UsdValue
    treasury: UsdValue


class PairParam(TypedDict):
    tick_size: UsdPrice
    min_order_size: UsdValue
    max_limit_price_deviation: Dimensionless
    max_market_slippage: Dimensionless
    max_abs_oi: Quantity
    max_abs_funding_rate: FundingRate
    initial_margin_ratio: Dimensionless
    maintenance_margin_ratio: Dimensionless
    impact_size: UsdValue
    vault_liquidity_weight: Dimensionless
    vault_half_spread: Dimensionless
    vault_max_quote_size: Quantity
    vault_size_skew_factor: Dimensionless
    vault_spread_skew_factor: Dimensionless
    vault_max_skew_size: Quantity
    funding_rate_multiplier: Dimensionless
    bucket_sizes: list[UsdPrice]


class PairState(TypedDict):
    long_oi: Quantity
    short_oi: Quantity
    funding_per_unit: FundingPerUnit
    funding_rate: FundingRate


class LiquidityDepth(TypedDict):
    size: Quantity
    notional: UsdValue


class LiquidityDepthResponse(TypedDict):
    bids: dict[UsdPrice, LiquidityDepth]
    asks: dict[UsdPrice, LiquidityDepth]


class QueryOrderResponse(TypedDict):
    user: Addr
    pair_id: PairId
    size: Quantity
    limit_price: UsdPrice
    reduce_only: bool
    reserved_margin: UsdValue
    created_at: Timestamp


class QueryOrdersByUserResponseItem(TypedDict):
    pair_id: PairId
    size: Quantity
    limit_price: UsdPrice
    reduce_only: bool
    reserved_margin: UsdValue
    created_at: Timestamp


class UserReferralData(TypedDict):
    volume: UsdValue
    commission_shared_by_referrer: UsdValue
    referee_count: int
    referees_volume: UsdValue
    commission_earned_from_referees: UsdValue
    cumulative_daily_active_referees: int
    cumulative_global_active_referees: int


# --- Events ------------------------------------------------------------------


class Deposited(TypedDict):
    user: Addr
    amount: UsdValue


class Withdrew(TypedDict):
    user: Addr
    amount: UsdValue


class LiquidityAdded(TypedDict):
    user: Addr
    amount: UsdValue
    shares_minted: Uint128


class LiquidityUnlocking(TypedDict):
    user: Addr
    amount: UsdValue
    shares_burned: Uint128
    end_time: Timestamp


class LiquidityReleased(TypedDict):
    user: Addr
    amount: UsdValue


class OrderFilled(TypedDict):
    order_id: OrderId
    pair_id: PairId
    user: Addr
    fill_price: UsdPrice
    fill_size: Quantity
    closing_size: Quantity
    opening_size: Quantity
    realized_pnl: UsdValue
    fee: UsdValue
    client_order_id: ClientOrderId | None
    fill_id: FillId | None
    is_maker: bool | None


class OrderPersisted(TypedDict):
    order_id: OrderId
    pair_id: PairId
    user: Addr
    limit_price: UsdPrice
    size: Quantity
    client_order_id: ClientOrderId | None


class OrderRemoved(TypedDict):
    order_id: OrderId
    pair_id: PairId
    user: Addr
    reason: ReasonForOrderRemoval
    client_order_id: ClientOrderId | None


class ConditionalOrderPlaced(TypedDict):
    pair_id: PairId
    user: Addr
    trigger_price: UsdPrice
    trigger_direction: TriggerDirection
    size: Quantity | None
    max_slippage: Dimensionless


class ConditionalOrderTriggered(TypedDict):
    pair_id: PairId
    user: Addr
    trigger_price: UsdPrice
    trigger_direction: TriggerDirection
    oracle_price: UsdPrice


class ConditionalOrderRemoved(TypedDict):
    pair_id: PairId
    user: Addr
    trigger_direction: TriggerDirection
    reason: ReasonForOrderRemoval


class Liquidated(TypedDict):
    user: Addr
    pair_id: PairId
    adl_size: Quantity
    adl_price: UsdPrice | None
    adl_realized_pnl: UsdValue


class Deleveraged(TypedDict):
    user: Addr
    pair_id: PairId
    closing_size: Quantity
    fill_price: UsdPrice
    realized_pnl: UsdValue


class BadDebtCovered(TypedDict):
    liquidated_user: Addr
    amount: UsdValue
    insurance_fund_remaining: UsdValue


class FeeDistributed(TypedDict):
    payer: UserIndex
    payer_addr: Addr
    protocol_fee: UsdValue
    vault_fee: UsdValue
    commissions: list[UsdValue]


class ReferralSet(TypedDict):
    referrer: UserIndex
    referee: UserIndex

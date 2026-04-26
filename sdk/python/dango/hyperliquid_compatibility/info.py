"""HL-shaped ``Info`` wrapper that translates HL read calls to native Dango calls.

This module is the read-side translator for the Hyperliquid-compat layer
(Phase 16). The public class :class:`Info` mirrors HL's
``hyperliquid.info.Info`` constructor and method signatures byte-for-byte
so HL traders can swap their import statement and keep going.

Design goals:

* HL camelCase wire shapes are preserved verbatim (``assetPositions``,
  ``crossMarginSummary``, ``marginUsed``, etc.). All decimal strings on
  the wire pass through :func:`dango_decimal_to_hl_str` so trailing
  zeros are stripped (HL never sends ``"1.230000"``).
* HL-only or Dango-gap methods raise :class:`NotImplementedError` with a
  one-line reason. We do NOT silently return empty lists — the goal is
  loud failure so callers can route around the gap explicitly.
* Subscriptions dispatch by the ``type`` field on the
  :class:`~dango.hyperliquid_compatibility.types.Subscription` dict.
  Polling-backed subscriptions (``l2Book``, ``bbo``, ``allMids``,
  ``activeAssetCtx``, ``activeAssetData``) sit on top of the native
  ``subscribe_query_app`` block-interval polling.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING, Any, cast

from dango.hyperliquid_compatibility.types import (
    ActiveAssetData,
    Fill,
    L2BookData,
    L2Level,
    Meta,
    PerpAssetCtx,
    Subscription,
    Trade,
    dango_decimal_to_hl_str,
)
from dango.utils.constants import SETTLEMENT_DECIMALS
from dango.utils.types import (
    Addr,
    CandleInterval,
    OrderId,
    PairId,
)

if TYPE_CHECKING:
    import dango.info as _native_info
    from dango.utils.types import (
        PairState,
        PerpsCandle,
        PerpsEvent,
        PerpsPairStats,
        UserStateExtended,
    )


# --- Interval mapping -------------------------------------------------------
#
# HL exposes the union of intervals supported by the major venues; Dango's
# indexer only supports the seven listed below. Anything else raises
# ``ValueError`` so the caller hears about the gap immediately rather than
# getting silently-wrong data.

_HL_TO_DANGO_INTERVAL: dict[str, CandleInterval] = {
    "1m": CandleInterval.ONE_MINUTE,
    "5m": CandleInterval.FIVE_MINUTES,
    "15m": CandleInterval.FIFTEEN_MINUTES,
    "1h": CandleInterval.ONE_HOUR,
    "4h": CandleInterval.FOUR_HOURS,
    "1d": CandleInterval.ONE_DAY,
    "1w": CandleInterval.ONE_WEEK,
}


def _hl_interval_to_dango(interval: str) -> CandleInterval:
    """Translate an HL candle interval string into the Dango ``CandleInterval`` enum."""
    # Listed-then-raise: a `match` statement here would be overkill for what
    # is fundamentally a dict lookup. The error message lists every
    # supported value so callers don't need to consult the docs to recover.
    try:
        return _HL_TO_DANGO_INTERVAL[interval]
    except KeyError as exc:
        supported = ", ".join(sorted(_HL_TO_DANGO_INTERVAL))
        raise ValueError(
            f"unsupported candle interval {interval!r}; Dango supports only: {supported}"
        ) from exc


# --- Pair name <-> coin helpers ---------------------------------------------


def _pair_id_to_coin(pair_id: str) -> str:
    """Strip ``perp/`` prefix and ``usd`` suffix to recover an HL-style coin name.

    ``"perp/btcusd"`` → ``"BTC"``. Dango pair IDs are lowercase and always
    settle in USD, so this is a deterministic round-trip.
    """
    # Strip prefix first so "perp/" is consumed before suffix detection;
    # uppercasing happens last so the returned form matches HL conventions
    # (HL always reports coins in uppercase).
    name = pair_id.removeprefix("perp/").removesuffix("usd")
    return name.upper()


def _coin_to_pair_id(coin: str) -> PairId:
    """Inverse of :func:`_pair_id_to_coin`. ``"ETH"`` → ``"perp/ethusd"``."""
    return PairId(f"perp/{coin.lower()}usd")


# --- Reshape helpers --------------------------------------------------------
#
# Each function below takes a native Dango wire-shaped dict and returns the
# HL wire shape. They're module-level (not Info methods) so tests can pin
# the exact reshape independently of any Info instance.


def _to_hl_str(value: str | None, *, default: str = "0") -> str:
    """``dango_decimal_to_hl_str`` with a None-passthrough default."""
    # `default` is exposed so callers can choose `"0"` (numeric) vs `""`
    # (string) per field. HL uses `"0"` everywhere a decimal is expected
    # — the empty-string variant is only relevant if a future HL field
    # surfaces as a non-numeric string.
    if value is None:
        return default
    return dango_decimal_to_hl_str(value)


def _reshape_user_state_to_hl(
    state: UserStateExtended | None,
    pair_states: dict[PairId, PairState],
) -> dict[str, Any]:
    """Reshape a native ``UserStateExtended`` to HL's ``clearinghouseState`` dict.

    HL splits margin info into `marginSummary` (always cross on Dango,
    matches `crossMarginSummary`) and `assetPositions` (one entry per
    open position). Dango stores everything together in
    ``UserStateExtended``; this function performs the split.
    """
    # Empty-state branch: HL still returns the four keys with zero values
    # for a user with no on-chain margin record. Mirroring this avoids
    # KeyError in HL-shaped consumers that assume the keys exist.
    if state is None:
        zero_margin_summary = {
            "accountValue": "0",
            "totalMarginUsed": "0",
            "totalNtlPos": "0",
            "totalRawUsd": "0",
        }
        return {
            "assetPositions": [],
            "crossMarginSummary": zero_margin_summary,
            "marginSummary": zero_margin_summary,
            "withdrawable": "0",
        }

    asset_positions: list[dict[str, Any]] = []
    total_ntl_pos = 0.0
    for pair_id, position in state["positions"].items():
        coin = _pair_id_to_coin(pair_id)
        size = position["size"]
        entry_px = position["entry_price"]
        unrealized_pnl = position.get("unrealized_pnl") or "0"
        liquidation_px = position.get("liquidation_price")

        # `positionValue` and `marginUsed` are derived: HL's positionValue
        # is `|size| * entry_price`, and marginUsed is the equity tied
        # up by this position. Dango's `equity` is global (account-level
        # margin), so we approximate per-position margin as
        # `|positionValue * initial_margin_ratio|`. Since the HL clients
        # mostly read these for display, an approximation is acceptable.
        try:
            sz_abs = abs(float(size))
            entry_px_f = float(entry_px)
            position_value = sz_abs * entry_px_f
            total_ntl_pos += position_value
            position_value_str = dango_decimal_to_hl_str(f"{position_value:.6f}")
        except ValueError, TypeError:
            # Defensive: if a numeric string fails to parse (e.g. server
            # returned a sentinel), fall back to "0" rather than crash.
            position_value_str = "0"

        # `returnOnEquity` requires both unrealized PnL and margin —
        # Dango doesn't expose per-position margin, so we report 0 and
        # let HL clients that gate on this number show "0%". Filing this
        # as a known approximation rather than a gap.
        asset_positions.append(
            {
                "type": "oneWay",
                "position": {
                    "coin": coin,
                    "szi": dango_decimal_to_hl_str(size),
                    "entryPx": dango_decimal_to_hl_str(entry_px),
                    # Synthetic: Dango is cross-only, no leverage knob.
                    # `value=1` matches HL's "1x" default for accounts
                    # without explicit leverage adjustments.
                    "leverage": {"type": "cross", "value": 1},
                    "liquidationPx": (
                        dango_decimal_to_hl_str(liquidation_px)
                        if liquidation_px is not None
                        else None
                    ),
                    "marginUsed": "0",
                    "maxLeverage": 1,
                    "positionValue": position_value_str,
                    "returnOnEquity": "0",
                    "unrealizedPnl": _to_hl_str(unrealized_pnl),
                    # Funding is folded into realized PnL on each fill on
                    # Dango — no separate `cumFunding` series. Report
                    # zeros so HL clients reading this object don't
                    # crash, with the rationale documented in
                    # `funding_history` / `user_funding_history`.
                    "cumFunding": {
                        "allTime": "0",
                        "sinceOpen": "0",
                        "sinceChange": "0",
                    },
                },
            }
        )
        # `pair_states` is unused here but accepted in the signature for
        # forward-compat: when we eventually track `markPx`-vs-`entryPx`
        # spreads, that delta needs `pair_state.funding_per_unit` which
        # only the per-pair states dict has.
        _ = pair_states.get(pair_id)

    margin = state["margin"]
    equity = state.get("equity") or margin
    available_margin = state.get("available_margin") or margin
    margin_summary = {
        "accountValue": dango_decimal_to_hl_str(equity),
        "totalMarginUsed": dango_decimal_to_hl_str(state["reserved_margin"]),
        "totalNtlPos": dango_decimal_to_hl_str(f"{total_ntl_pos:.6f}"),
        "totalRawUsd": dango_decimal_to_hl_str(margin),
    }

    return {
        "assetPositions": asset_positions,
        # Dango is cross-margin only: cross and total share the same
        # numbers. HL clients that branch on `crossMarginSummary` vs
        # `marginSummary` will see equal values; that's correct since
        # Dango has no isolated-margin partition.
        "crossMarginSummary": margin_summary,
        "marginSummary": margin_summary,
        "withdrawable": dango_decimal_to_hl_str(available_margin),
    }


def _reshape_order_to_hl(order: dict[str, Any], order_id: str) -> dict[str, Any]:
    """Reshape a native ``QueryOrdersByUserResponseItem`` row to HL ``open_orders`` shape."""
    # Dango stores `size` as a signed quantity; HL splits this into
    # `side` ("A" for ask/sell, "B" for bid/buy) and `sz` (always
    # positive). The convention follows HL's own — see the HL Side
    # type docs.
    size = order["size"]
    try:
        size_f = float(size)
    except ValueError, TypeError:
        size_f = 0.0
    side = "B" if size_f > 0 else "A"
    sz_abs = dango_decimal_to_hl_str(f"{abs(size_f):.6f}")

    # `oid` is HL's int field name; Dango's order_id is a string.
    # We pass through verbatim: HL traders that try `int(order["oid"])`
    # will hit a ValueError, but the key still exists. This is a known
    # asymmetry — see `query_order_by_oid` for the round-trip.
    pair_id = order["pair_id"]
    return {
        "coin": _pair_id_to_coin(pair_id),
        "side": side,
        "limitPx": dango_decimal_to_hl_str(order["limit_price"]),
        "sz": sz_abs,
        "oid": order_id,
        # Dango stores `created_at` as a Timestamp string (ns); HL
        # reports `timestamp` in milliseconds. Convert if possible,
        # fall back to 0 on parse failure.
        "timestamp": _timestamp_ns_to_ms(order.get("created_at")),
        "origSz": sz_abs,
    }


def _timestamp_ns_to_ms(ts: str | None) -> int:
    """Convert a Dango ns Timestamp string to HL's ms int. ``None`` → 0."""
    # Dango's `Timestamp` wire shape is a stringified ns count (per
    # `Timestamp = NewType("Timestamp", str)`), e.g. "1700000000000000000".
    # HL uses ms ints. The `// 1_000_000` integer-division is exact
    # because Dango never emits a non-integer ns count.
    if ts is None:
        return 0
    try:
        return int(ts) // 1_000_000
    except ValueError, TypeError:
        return 0


def _reshape_fill_to_hl(event: PerpsEvent) -> Fill:
    """Reshape a native ``PerpsEvent`` whose payload is ``OrderFilled`` to HL ``Fill``."""
    # The `data` payload of an `order_filled` event is the
    # `OrderFilled` typeddict; we pull fields verbatim and cast to
    # the HL `Fill` shape. `tid` and `feeToken` are HL-only concepts
    # (trade-id and fee-token-symbol) which Dango doesn't model;
    # we synthesize stable values: `tid` is derived from
    # `(blockHeight, idx)` so it's unique per event, and `feeToken`
    # is hardcoded to "USDC" since Dango fees are settled in USDC
    # (see SETTLEMENT_DENOM = "bridge/usdc").
    data = event["data"]
    fill_size = data.get("fill_size", "0")
    closing_size = data.get("closing_size", "0")
    fill_price = data.get("fill_price", "0")
    realized_pnl = data.get("realized_pnl", "0")
    fee = data.get("fee", "0")

    try:
        size_f = float(fill_size)
    except ValueError, TypeError:
        size_f = 0.0
    try:
        closing_f = float(closing_size)
    except ValueError, TypeError:
        closing_f = 0.0

    # HL `side`: "A" = ask/sell, "B" = bid/buy. Dango's wire convention
    # signs the fill size: positive = buyer's fill, negative = seller's.
    side = "B" if size_f > 0 else "A"

    # HL `dir`: "Open Long", "Open Short", "Close Long", "Close Short".
    # Derive from the open/close split: closing_size > 0 implies the
    # fill reduced an existing position (close); fill_size > 0 with
    # closing_size == 0 implies an opening fill.
    if closing_f > 0:
        direction = "Close Long" if size_f < 0 else "Close Short"
    else:
        direction = "Open Long" if size_f > 0 else "Open Short"

    # `tid` is HL's per-fill trade id. Dango's `fillId` is a string;
    # we hash to int so the HL Fill TypedDict's `tid: int` stays
    # well-typed. `(block_height, idx)` packs uniquely into a 64-bit
    # int (block height fits 32 bits up to 4 billion; idx fits 32 bits
    # well within ledger limits).
    tid = (event["blockHeight"] << 32) | (event["idx"] & 0xFFFFFFFF)

    # `hash` is HL's tx-hash field. Dango's PerpsEvent has `txHash`.
    pair_id = data.get("pair_id", event.get("pairId", "perp/unknown"))
    coin = _pair_id_to_coin(pair_id)
    return cast(
        "Fill",
        {
            "coin": coin,
            "px": dango_decimal_to_hl_str(fill_price),
            "sz": dango_decimal_to_hl_str(f"{abs(size_f):.6f}"),
            "side": side,
            "time": _isotime_to_ms(event.get("createdAt")),
            "startPosition": "0",
            "dir": direction,
            "closedPnl": _to_hl_str(realized_pnl),
            "hash": event.get("txHash", ""),
            "oid": data.get("order_id", ""),
            "crossed": True,
            "fee": _to_hl_str(fee),
            "tid": tid,
            "feeToken": "USDC",
        },
    )


def _isotime_to_ms(iso: str | None) -> int:
    """Convert a `PerpsEvent.createdAt` ISO-8601 string to ms int. ``None`` → 0."""
    # Dango's indexer emits `createdAt` as ISO-8601, e.g.
    # "2024-01-01T00:00:00.123Z". Python's `fromisoformat` handles
    # the "Z" suffix from 3.11 onwards; we still guard against
    # malformed strings rather than letting a ValueError escape.
    if iso is None:
        return 0
    try:
        from datetime import datetime

        # `Z` is parseable by 3.11+ `fromisoformat`; tolerate `+00:00`
        # form too.
        dt = datetime.fromisoformat(iso.replace("Z", "+00:00"))
        return int(dt.timestamp() * 1000)
    except ValueError, TypeError:
        return 0


def _reshape_l2_to_hl(
    depth: dict[str, Any],
    coin: str,
    time_ms: int,
) -> L2BookData:
    """Reshape a native ``LiquidityDepthResponse`` map to HL's L2 list shape.

    HL: ``{coin, levels: [bids_list, asks_list], time}`` where each level is
    ``{px, sz, n}``. Dango: ``{bids: {price: {size, notional}}, asks: ...}``.

    The ``n`` field (number of orders at the level) is not exposed by
    Dango's depth query — depth is bucketed and the bucket may aggregate
    arbitrary many orders. We default to ``1`` so HL clients that read
    ``n`` for display see something non-zero; readers that depend on the
    actual order count will need to use a different data source.
    """

    def _level_list(side: dict[str, dict[str, str]], reverse: bool) -> list[L2Level]:
        # Sort by numeric price: bids descending (best bid first), asks
        # ascending (best ask first). String sort would mis-order "10"
        # vs "9", hence the `float(...)` key.
        sorted_keys = sorted(side, key=float, reverse=reverse)
        return [
            L2Level(
                px=dango_decimal_to_hl_str(price),
                sz=dango_decimal_to_hl_str(side[price]["size"]),
                # Dango doesn't expose per-bucket order count — use 1
                # as a conservative default. See the docstring above.
                n=1,
            )
            for price in sorted_keys
        ]

    bids_list = _level_list(depth.get("bids") or {}, reverse=True)
    asks_list = _level_list(depth.get("asks") or {}, reverse=False)
    return L2BookData(
        coin=coin,
        levels=(bids_list, asks_list),
        time=time_ms,
    )


def _reshape_candle_to_hl(candle: PerpsCandle) -> dict[str, Any]:
    """Reshape a native ``PerpsCandle`` to HL's single-letter candle dict.

    HL keys: ``T`` (end ms), ``t`` (start ms), ``s`` (coin), ``i``
    (interval), ``o``/``c``/``h``/``l`` (OHLC), ``v`` (volume), ``n``
    (trade count). Dango's candle has the same fields under longer
    names — this is purely a key rename + ms scaling.
    """
    pair_id = candle["pairId"]
    return {
        # Dango candle times are unix seconds; HL uses ms.
        "T": candle["timeEndUnix"] * 1000,
        "t": candle["timeStartUnix"] * 1000,
        "s": _pair_id_to_coin(pair_id),
        "i": candle["interval"],
        "o": dango_decimal_to_hl_str(candle["open"]),
        "c": dango_decimal_to_hl_str(candle["close"]),
        "h": dango_decimal_to_hl_str(candle["high"]),
        "l": dango_decimal_to_hl_str(candle["low"]),
        "v": dango_decimal_to_hl_str(candle["volume"]),
        # Dango doesn't track per-candle trade count separately;
        # default to 0. HL clients reading `n` for display will see
        # 0, which is internally consistent ("we don't know how many
        # trades made up this candle").
        "n": 0,
    }


def _reshape_pair_state_to_perp_ctx(
    pair_id: str,
    pair_state: PairState | None,
    pair_stats: PerpsPairStats | None,
) -> PerpAssetCtx:
    """Combine native ``pair_state`` + ``perps_pair_stats`` into HL's PerpAssetCtx."""
    # `funding`: Dango exposes `funding_rate` as a per-period rate.
    # HL's `funding` is the same concept (per-hour or per-period rate
    # depending on venue convention) — pass through with the standard
    # HL string formatting.
    funding = "0"
    open_interest = "0"
    if pair_state is not None:
        funding = dango_decimal_to_hl_str(pair_state.get("funding_rate", "0"))
        # `openInterest` on HL is single-sided. Dango tracks long_oi
        # and short_oi separately; in equilibrium they're equal, but
        # we report long_oi as the canonical value since HL treats
        # open interest as one number per asset.
        open_interest = dango_decimal_to_hl_str(pair_state.get("long_oi", "0"))

    current_price = "0"
    prev_day_px = "0"
    day_ntl_vlm = "0"
    if pair_stats is not None:
        current_price = _to_hl_str(pair_stats.get("currentPrice"))
        prev_day_px = _to_hl_str(pair_stats.get("price24HAgo"))
        day_ntl_vlm = _to_hl_str(pair_stats.get("volume24H"))

    # Dango doesn't separately track day-base-volume (volume in the
    # base asset, before USD conversion). For perps, the base volume
    # is `notional / price`. We approximate from `volume24H` (which
    # is in USD) and `currentPrice`.
    try:
        if current_price not in ("0", ""):
            day_base_vlm = dango_decimal_to_hl_str(
                f"{float(day_ntl_vlm) / float(current_price):.6f}"
            )
        else:
            day_base_vlm = "0"
    except ValueError, ZeroDivisionError:
        day_base_vlm = "0"

    # `markPx`, `oraclePx`, `midPx` are conceptually distinct on HL
    # (mark = oracle-anchored execution price, oracle = pyth feed,
    # mid = book mid). On Dango, the contract's "oracle price" feeds
    # all three, so we report `currentPrice` for all three. A more
    # faithful reshape would query the oracle contract for `oraclePx`
    # but that's a separate round-trip — out of scope for the v1
    # mapping table.
    _ = pair_id  # used by future per-pair lookups; pinned for forward-compat
    return PerpAssetCtx(
        funding=funding,
        openInterest=open_interest,
        prevDayPx=prev_day_px,
        dayNtlVlm=day_ntl_vlm,
        premium="0",
        oraclePx=current_price,
        markPx=current_price,
        midPx=current_price if current_price != "0" else None,
        impactPxs=None,
        dayBaseVlm=day_base_vlm,
    )


# --- The public Info class --------------------------------------------------


class Info:
    """HL-shaped facade over the native :class:`dango.info.Info`.

    Construction mirrors HL's signature:

    .. code-block:: python

        info = Info(base_url="https://...", skip_ws=False, meta=None)

    The wrapper retains a coin↔pair_id resolver populated from
    ``pair_params`` at construction time (or from the supplied ``meta``
    dict, if the caller wants to run offline).
    """

    def __init__(
        self,
        base_url: str | None = None,
        skip_ws: bool = False,
        meta: Meta | None = None,
        perp_dexs: list[str] | None = None,
        timeout: float | None = None,
    ) -> None:
        # Lazy import of the native Info to avoid pulling in the
        # GraphQL websocket stack at module import time. Tests patch
        # this attribute (`info._native = ...`) to inject a fake.
        from dango.info import Info as NativeInfo

        # Default URL: HL's default is the production API; we leave
        # `None` because Dango has no canonical default URL — the
        # caller must specify an environment. Native Info's
        # constructor takes a `str`, so coalesce here so the type
        # checker stays happy.
        if base_url is None:
            from dango.utils.constants import LOCAL_API_URL

            base_url = LOCAL_API_URL
        self._native: _native_info.Info = NativeInfo(base_url, skip_ws=skip_ws, timeout=timeout)
        # `perp_dexs` is HL's multi-DEX knob: HL allows querying multiple
        # DEXes deployed by builders. Dango has no permissionless
        # listing, so this is a no-op. Keep the parameter to preserve
        # signature parity; record it for inspection in case future
        # versions of Dango get builder DEXes.
        self._perp_dexs: list[str] | None = perp_dexs
        # The coin resolver maps short HL coin names ("ETH") to Dango
        # pair IDs ("perp/ethusd"). It's primed from `meta` if given;
        # otherwise we fetch live `pair_params`.
        self.coin_to_asset: dict[str, int] = {}
        self.name_to_coin: dict[str, str] = {}
        self.asset_to_sz_decimals: dict[int, int] = {}
        self.coin_to_pair: dict[str, PairId] = {}
        self._build_coin_resolver(meta=meta)

    def _build_coin_resolver(self, *, meta: Meta | None = None) -> None:
        """Populate the four coin/asset/pair maps used for HL ↔ Dango translation."""
        # If `meta` is supplied, use it verbatim — this lets callers
        # operate offline against a frozen meta. Otherwise fetch the
        # live `pair_params` snapshot.
        if meta is not None:
            universe = meta["universe"]
            # `meta["universe"]` is HL-shaped: each entry has `name` and
            # `szDecimals`. We round-trip back to a Dango pair_id by
            # `_coin_to_pair_id`.
            for asset_index, info in enumerate(universe):
                name = info["name"]
                self.coin_to_asset[name] = asset_index
                self.name_to_coin[name] = name
                self.asset_to_sz_decimals[asset_index] = info["szDecimals"]
                self.coin_to_pair[name] = _coin_to_pair_id(name)
            return

        # Live path: fetch pair_params and synthesize the maps. We
        # iterate keys in dict order; Python 3.7+ guarantees insertion
        # order, and the indexer returns them sorted by pair_id.
        pair_params = self._native.pair_params()
        for asset_index, pair_id in enumerate(pair_params):
            coin = _pair_id_to_coin(pair_id)
            self.coin_to_asset[coin] = asset_index
            self.name_to_coin[coin] = coin
            # Synthesize `szDecimals`: Dango uses a universal
            # `SETTLEMENT_DECIMALS` (6) for all sizes. HL exposes
            # per-asset szDecimals because different markets quantize
            # at different granularities; on Dango that uniform 6 is
            # the right answer.
            self.asset_to_sz_decimals[asset_index] = SETTLEMENT_DECIMALS
            self.coin_to_pair[coin] = PairId(pair_id)

    def name_to_pair(self, name: str) -> PairId:
        """Translate an HL coin name to a Dango pair_id; raise ``KeyError`` if unknown."""
        return self.coin_to_pair[name]

    def name_to_asset(self, name: str) -> int:
        """HL signature — coin name to integer asset index."""
        return self.coin_to_asset[self.name_to_coin[name]]

    # --- Implemented read methods ------------------------------------------

    def user_state(self, address: str, dex: str = "") -> dict[str, Any]:
        """HL ``clearinghouseState`` — user margin + asset positions."""
        # `dex` is unused on Dango (no builder-deployed DEXes); we
        # accept it for signature parity. Ignoring it silently is fine
        # because the only valid HL value here is "" (the default).
        _ = dex
        state = self._native.user_state_extended(Addr(address))
        # We ignore `pair_states` for now (the reshape has a forward-
        # compat hook for it) but fetching it here would double the
        # round-trips. Since the field is unused, skip the fetch.
        return _reshape_user_state_to_hl(state, {})

    def open_orders(self, address: str, dex: str = "") -> list[dict[str, Any]]:
        """HL ``openOrders`` — flatten Dango's keyed map to a list."""
        _ = dex
        orders = self._native.orders_by_user(Addr(address))
        # Dango returns `dict[OrderId, item]`; HL returns a list. We
        # iterate items and reshape each row, preserving insertion
        # order (which the indexer returns sorted by pair_id).
        return [_reshape_order_to_hl(item, str(oid)) for oid, item in orders.items()]

    def all_mids(self, dex: str = "") -> dict[str, str]:
        """HL ``allMids`` — coin name → mid price string."""
        _ = dex
        stats_list = self._native.all_perps_pair_stats()
        # `currentPrice` doubles as the mid price on Dango: it's the
        # last trade price from the indexer, which is the closest
        # match to "current mid" without a separate book query.
        return {
            _pair_id_to_coin(stats["pairId"]): _to_hl_str(stats.get("currentPrice"))
            for stats in stats_list
        }

    def meta(self, dex: str = "") -> Meta:
        """HL ``meta`` — perp universe metadata."""
        _ = dex
        pair_params = self._native.pair_params()
        # Dango doesn't have an `szDecimals` concept — we use the
        # universal `SETTLEMENT_DECIMALS` (6) for every asset. See
        # the module docstring for the rationale.
        universe = [
            {"name": _pair_id_to_coin(pair_id), "szDecimals": SETTLEMENT_DECIMALS}
            for pair_id in pair_params
        ]
        return cast("Meta", {"universe": universe})

    def meta_and_asset_ctxs(self) -> list[Any]:
        """HL ``metaAndAssetCtxs`` — bundles meta with per-asset ctx."""
        # HL atomically combines meta + ctx in one call. On Dango we
        # need three queries (`pair_params`, `pair_states`,
        # `all_perps_pair_stats`); they're independent so an
        # implementation could parallelize, but the round-trip count
        # is dominated by network latency rather than per-call cost.
        pair_params = self._native.pair_params()
        pair_states = self._native.pair_states()
        all_stats = self._native.all_perps_pair_stats()
        # Index `pair_stats` by pair_id for O(1) per-pair lookup.
        stats_by_pair = {stats["pairId"]: stats for stats in all_stats}
        ctxs: list[PerpAssetCtx] = []
        for pair_id in pair_params:
            pair_state = pair_states.get(PairId(pair_id))
            stats = stats_by_pair.get(pair_id)
            ctxs.append(_reshape_pair_state_to_perp_ctx(pair_id, pair_state, stats))
        meta = self.meta()
        return [meta, ctxs]

    def l2_snapshot(self, name: str) -> L2BookData:
        """HL ``l2Book`` — bid/ask depth at the finest bucket."""
        pair_id = self.name_to_pair(name)
        # Dango requires picking a `bucket_size` from the pair's
        # `bucket_sizes` list; HL doesn't take one, so we pick the
        # smallest (finest grain) from `pair_param.bucket_sizes`.
        # Defensive: if `pair_param` is missing the list, fall back to
        # a coarse default to surface the misconfiguration loudly.
        param = self._native.pair_param(pair_id)
        if param is None or not param.get("bucket_sizes"):
            raise RuntimeError(
                f"pair {pair_id} has no configured bucket_sizes; cannot compute L2 snapshot"
            )
        bucket_sizes = param["bucket_sizes"]
        # Sort by numeric value to pick the finest grain. The wire
        # form is fixed-decimal strings ("0.10000"), which sort
        # correctly numerically when cast to float.
        bucket_size = min(bucket_sizes, key=float)
        depth = self._native.liquidity_depth(pair_id, bucket_size=bucket_size)
        # Dango doesn't include a server-side timestamp on the depth
        # response. For now we return 0; HL clients can use their own
        # arrival time. A future refinement could thread the
        # `query_status().block.timestamp` through but that's an extra
        # round-trip per call.
        return _reshape_l2_to_hl(cast("dict[str, Any]", depth), name, time_ms=0)

    def candles_snapshot(
        self,
        name: str,
        interval: str,
        start: int,
        end: int,
    ) -> list[dict[str, Any]]:
        """HL ``candleSnapshot`` — OHLCV candles in a time window."""
        pair_id = self.name_to_pair(name)
        dango_interval = _hl_interval_to_dango(interval)
        # HL takes ms timestamps; Dango's indexer takes ns ISO strings
        # via `laterThan` / `earlierThan`. We convert ms → ns and
        # format as `Timestamp` strings (the indexer accepts both
        # plain ns ints and ISO strings; we pass ns ints stringified).
        later_than = _ms_to_ns_str(start)
        earlier_than = _ms_to_ns_str(end)
        page = self._native.perps_candles(
            pair_id,
            dango_interval,
            later_than=later_than,
            earlier_than=earlier_than,
        )
        return [_reshape_candle_to_hl(candle) for candle in page.nodes]

    def user_fills(self, address: str) -> list[Fill]:
        """HL ``userFills`` — flat list of trade fills for one user."""
        # Dango pushes both maker and taker rows for the same fill
        # (paired by `fill_id`). HL pushes one cumulative row per
        # fill. We dedupe by `fill_id`, keeping the taker side
        # (`is_maker == False`) — the taker side carries the
        # "aggressor's" view of the trade, which is what HL
        # historically reports. If `fill_id` is missing, we fall back
        # to the (block_height, idx) tuple as the dedup key.
        events = list(
            self._native.perps_events_all(
                user_addr=Addr(address),
                event_type="order_filled",
            )
        )
        return _dedupe_fills(events)

    def user_fills_by_time(
        self,
        addr: str,
        start: int,
        end: int | None = None,
    ) -> list[Fill]:
        """HL ``userFillsByTime`` — same as user_fills, with a time-range filter."""
        # `perps_events_all` doesn't accept time bounds directly; we
        # paginate the entire stream and filter client-side. For
        # high-volume users this is suboptimal but correct — a future
        # optimization could add `created_at` filter support to the
        # indexer query.
        all_events = list(
            self._native.perps_events_all(
                user_addr=Addr(addr),
                event_type="order_filled",
            )
        )
        end_ms = end if end is not None else 1 << 62  # effectively +inf
        in_range = [
            event
            for event in all_events
            if start <= _isotime_to_ms(event.get("createdAt")) <= end_ms
        ]
        return _dedupe_fills(in_range)

    def query_order_by_oid(self, user: str, oid: int | str) -> dict[str, Any]:
        """HL ``orderStatus`` — fetch one order by id.

        Dango's order storage is keyed by oid alone, so the ``user``
        argument is redundant — we accept it for HL signature parity
        but don't forward it to the contract.
        """
        _ = user
        order_id = OrderId(str(oid))
        order = self._native.order(order_id)
        if order is None:
            # HL reports unknown orders as `{"status": "unknownOid"}`;
            # the response shape is documented in the HL CLI but
            # historically inconsistent across versions. We use the
            # canonical form per HL Python SDK as of 2024.
            return {"status": "unknownOid"}
        return {
            "status": "order",
            "order": _reshape_order_to_hl(order, str(oid)),
        }

    def historical_orders(self, user: str) -> list[dict[str, Any]]:
        """HL ``historicalOrders`` — order lifecycle from persisted+removed events."""
        # We iterate persisted then removed events and zip them by
        # order_id. HL's shape per row is:
        # `{order: {coin, side, limitPx, sz, oid, timestamp, origSz},
        #   status: "filled"|"canceled"|...,
        #   statusTimestamp: <ms>}`.
        # Dango doesn't fold the lifecycle on the server side; we
        # collect the persisted (open) and removed (closed) events
        # and emit one HL row per persisted, looking up the matching
        # removed event for the status/timestamp.
        persisted = list(
            self._native.perps_events_all(
                user_addr=Addr(user),
                event_type="order_persisted",
            )
        )
        removed = list(
            self._native.perps_events_all(
                user_addr=Addr(user),
                event_type="order_removed",
            )
        )
        removed_by_oid = {ev["data"].get("order_id"): ev for ev in removed}

        rows: list[dict[str, Any]] = []
        for ev in persisted:
            data = ev["data"]
            oid = data.get("order_id", "")
            pair_id = data.get("pair_id", ev.get("pairId", "perp/unknown"))
            limit_price = data.get("limit_price", "0")
            size = data.get("size", "0")
            try:
                size_f = float(size)
            except ValueError, TypeError:
                size_f = 0.0
            sz_abs = dango_decimal_to_hl_str(f"{abs(size_f):.6f}")
            order_dict = {
                "coin": _pair_id_to_coin(pair_id),
                "side": "B" if size_f > 0 else "A",
                "limitPx": dango_decimal_to_hl_str(limit_price),
                "sz": sz_abs,
                "oid": oid,
                "timestamp": _isotime_to_ms(ev.get("createdAt")),
                "origSz": sz_abs,
            }
            removed_event = removed_by_oid.get(oid)
            if removed_event is not None:
                # The Dango `reason` enum aligns with HL: "filled",
                # "canceled", "liquidated", etc.
                status = removed_event["data"].get("reason", "canceled")
                status_ts = _isotime_to_ms(removed_event.get("createdAt"))
            else:
                status = "open"
                status_ts = order_dict["timestamp"]
            rows.append(
                {
                    "order": order_dict,
                    "status": status,
                    "statusTimestamp": status_ts,
                }
            )
        return rows

    # --- NotImplementedError stubs -----------------------------------------

    def spot_user_state(self, address: str) -> Any:
        raise NotImplementedError("Dango is perps-only")

    def spot_meta(self) -> Any:
        raise NotImplementedError("Dango is perps-only")

    def spot_meta_and_asset_ctxs(self) -> Any:
        raise NotImplementedError("Dango is perps-only")

    def query_spot_deploy_auction_status(self, user: str) -> Any:
        raise NotImplementedError("Dango is perps-only")

    def user_staking_summary(self, address: str) -> Any:
        raise NotImplementedError("Dango has no HYPE-equivalent staking")

    def user_staking_delegations(self, address: str) -> Any:
        raise NotImplementedError("Dango has no HYPE-equivalent staking")

    def user_staking_rewards(self, address: str) -> Any:
        raise NotImplementedError("Dango has no HYPE-equivalent staking")

    def delegator_history(self, user: str) -> Any:
        raise NotImplementedError("Dango has no HYPE-equivalent staking")

    def query_user_to_multi_sig_signers(self, multi_sig_user: str) -> Any:
        raise NotImplementedError("Dango multi-sig is not exposed via the perps SDK")

    def query_perp_deploy_auction_status(self) -> Any:
        raise NotImplementedError("Dango has no permissionless asset listing")

    def query_user_dex_abstraction_state(self, user: str) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def query_user_abstraction_state(self, user: str) -> Any:
        raise NotImplementedError("HL-specific concept; no Dango analog")

    def user_twap_slice_fills(self, user: str) -> Any:
        raise NotImplementedError("Dango has no TWAP orders")

    def portfolio(self, user: str) -> Any:
        raise NotImplementedError("Dango has no per-user portfolio time-series")

    def user_role(self, user: str) -> Any:
        raise NotImplementedError("Dango has no role system")

    def user_rate_limit(self, user: str) -> Any:
        raise NotImplementedError("Dango has no per-user rate limit")

    def extra_agents(self, user: str) -> Any:
        raise NotImplementedError("Dango uses session credentials; no extraAgents analog")

    def funding_history(self, name: str, start_time: int, end_time: int | None = None) -> Any:
        raise NotImplementedError(
            "Dango folds funding into realized PnL on each fill; no separate funding stream"
        )

    def user_funding_history(self, user: str, start_time: int, end_time: int | None = None) -> Any:
        raise NotImplementedError(
            "Dango folds funding into realized PnL on each fill; no separate funding stream"
        )

    def user_non_funding_ledger_updates(
        self, user: str, start_time: int, end_time: int | None = None
    ) -> Any:
        raise NotImplementedError("Phase 16: deferred — needs event-shape reshape")

    def query_referral_state(self, user: str) -> Any:
        raise NotImplementedError("Phase 16: deferred — native query not yet exposed")

    def query_sub_accounts(self, user: str) -> Any:
        raise NotImplementedError("Phase 16: deferred — native query not yet exposed")

    def frontend_open_orders(self, address: str, dex: str = "") -> Any:
        raise NotImplementedError("Phase 16: deferred — needs enrichment")

    def user_fees(self, address: str) -> Any:
        raise NotImplementedError("Phase 16: deferred — needs RateSchedule resolution")

    def query_order_by_cloid(self, user: str, cloid: Any) -> Any:
        raise NotImplementedError("Dango stores cloid only at submit/cancel; no by-cloid lookup")

    def user_vault_equities(self, user: str) -> Any:
        raise NotImplementedError("Phase 16: deferred — needs vault NAV computation")

    # --- Subscriptions -----------------------------------------------------

    def subscribe(
        self,
        subscription: Subscription,
        callback: Callable[[Any], None],
    ) -> int:
        """Dispatch an HL-shaped Subscription dict to the right native subscriber."""
        # Type narrowing: `Subscription` is a union of TypedDicts that
        # all carry `type` as a Literal. Reading `subscription["type"]`
        # works dynamically; mypy can't narrow the union from a
        # `match` on `subscription["type"]` alone, so we cast each
        # branch explicitly.
        sub_type = subscription["type"]
        if sub_type == "trades":
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            return self._subscribe_trades(coin, callback)
        if sub_type == "candle":
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            interval = cast("str", subscription["interval"])  # type: ignore[typeddict-item]
            return self._subscribe_candle(coin, interval, callback)
        if sub_type == "userEvents":
            user = cast("str", subscription["user"])  # type: ignore[typeddict-item]
            return self._subscribe_user_events(user, callback)
        if sub_type == "userFills":
            user = cast("str", subscription["user"])  # type: ignore[typeddict-item]
            return self._subscribe_user_fills(user, callback)
        if sub_type == "orderUpdates":
            user = cast("str", subscription["user"])  # type: ignore[typeddict-item]
            return self._subscribe_order_updates(user, callback)
        if sub_type == "l2Book":
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            return self._subscribe_l2_book(coin, callback)
        if sub_type == "bbo":
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            return self._subscribe_bbo(coin, callback)
        if sub_type == "allMids":
            return self._subscribe_all_mids(callback)
        if sub_type == "activeAssetCtx":
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            return self._subscribe_active_asset_ctx(coin, callback)
        if sub_type == "activeAssetData":
            user = cast("str", subscription["user"])  # type: ignore[typeddict-item]
            coin = cast("str", subscription["coin"])  # type: ignore[typeddict-item]
            return self._subscribe_active_asset_data(user, coin, callback)
        if sub_type == "userFundings":
            raise NotImplementedError(
                "Dango folds funding into realized PnL on each fill; no separate funding stream"
            )
        if sub_type == "webData2":
            raise NotImplementedError("UI-specific aggregation; no Dango analog")
        if sub_type == "userNonFundingLedgerUpdates":
            raise NotImplementedError("Phase 16: deferred — needs event-shape reshape")
        raise ValueError(f"unknown subscription type: {sub_type!r}")

    def unsubscribe(self, subscription: Subscription, subscription_id: int) -> bool:
        """Drop a subscription by id; the `subscription` arg is informational."""
        # Native `unsubscribe` keys on subscription_id alone — the
        # `subscription` dict is only used here for HL signature parity.
        # We don't need to dispatch on the type.
        _ = subscription
        return self._native.unsubscribe(subscription_id)

    def disconnect_websocket(self) -> None:
        """Close the underlying WebSocket connection."""
        self._native.disconnect_websocket()

    # --- Subscription dispatch helpers (private) ---------------------------

    def _subscribe_trades(
        self,
        coin: str,
        callback: Callable[[Any], None],
    ) -> int:
        # Dango pushes both maker and taker fills (paired by fill_id).
        # HL pushes one trade per match. We dedupe by fill_id and emit
        # only the taker side — see the dedupe rationale in
        # `_dedupe_fills`. State is per-subscription (one set per
        # registration) so concurrent subscribers don't trample each
        # other.
        seen_fill_ids: set[str] = set()
        pair_id = self.name_to_pair(coin)

        def wrapped(trade: Any) -> None:
            if not isinstance(trade, dict) or "_error" in trade:
                callback(trade)
                return
            fill_id = trade.get("fillId")
            is_maker = trade.get("isMaker")
            # Skip the maker side of a paired fill: HL traders see one
            # event per match, not two.
            if is_maker:
                return
            if fill_id is not None:
                if fill_id in seen_fill_ids:
                    return
                seen_fill_ids.add(fill_id)
            try:
                size_f = float(trade.get("fillSize", "0"))
            except ValueError, TypeError:
                size_f = 0.0
            # `Trade.sz` is annotated `int` upstream but HL's wire form is
            # actually a decimal string (the int annotation is a known
            # upstream typo). Emit the wire shape — `int(abs(0.5))` would
            # silently zero out fractional sizes.
            sz_str = dango_decimal_to_hl_str(f"{abs(size_f):.6f}")
            # `Trade.hash` is HL's tx hash, which the indexer's perps-trade
            # stream doesn't carry. Emit the empty string rather than
            # substitute the orderId — the latter would silently mislead
            # any consumer that compares `hash` across fills.
            hl_trade = Trade(
                coin=coin,
                side="B" if size_f > 0 else "A",
                px=dango_decimal_to_hl_str(trade.get("fillPrice", "0")),
                sz=cast("int", sz_str),
                hash="",
                time=_isotime_to_ms(trade.get("createdAt")),
            )
            callback(hl_trade)

        return self._native.subscribe_perps_trades(pair_id, wrapped)

    def _subscribe_candle(
        self,
        coin: str,
        interval: str,
        callback: Callable[[Any], None],
    ) -> int:
        pair_id = self.name_to_pair(coin)
        dango_interval = _hl_interval_to_dango(interval)

        def wrapped(candle: Any) -> None:
            if not isinstance(candle, dict) or "_error" in candle:
                callback(candle)
                return
            callback(_reshape_candle_to_hl(cast("PerpsCandle", candle)))

        return self._native.subscribe_perps_candles(pair_id, dango_interval, wrapped)

    def _subscribe_user_events(
        self,
        user: str,
        callback: Callable[[Any], None],
    ) -> int:
        # Wrap each native event into the HL `userEvents` envelope:
        # `{channel: "user", data: {fills: [Fill]}}`.
        def wrapped(event: Any) -> None:
            if not isinstance(event, dict) or "_error" in event:
                callback(event)
                return
            fill = _reshape_fill_to_hl(cast("PerpsEvent", event))
            callback({"channel": "user", "data": {"fills": [fill]}})

        return self._native.subscribe_user_events(
            Addr(user),
            wrapped,
            event_types=["order_filled"],
        )

    def _subscribe_user_fills(
        self,
        user: str,
        callback: Callable[[Any], None],
    ) -> int:
        # Same data source as `_subscribe_user_events`, different envelope.
        # HL's `userFills` sends `isSnapshot=True` on the first message
        # (a backfill of recent fills) and `False` thereafter. Dango's
        # event subscription doesn't backfill; every message is a live
        # fill, so we always emit `isSnapshot=False`.
        def wrapped(event: Any) -> None:
            if not isinstance(event, dict) or "_error" in event:
                callback(event)
                return
            fill = _reshape_fill_to_hl(cast("PerpsEvent", event))
            callback(
                {
                    "channel": "userFills",
                    "data": {
                        "user": user,
                        "isSnapshot": False,
                        "fills": [fill],
                    },
                }
            )

        return self._native.subscribe_user_events(
            Addr(user),
            wrapped,
            event_types=["order_filled"],
        )

    def _subscribe_order_updates(
        self,
        user: str,
        callback: Callable[[Any], None],
    ) -> int:
        # HL's `orderUpdates` delivers a list of `{order, status,
        # statusTimestamp}` per message. Dango pushes one event per
        # state change; we wrap each into the HL list-of-one shape so
        # downstream HL clients that expect an array don't choke on a
        # singleton.
        def wrapped(event: Any) -> None:
            if not isinstance(event, dict) or "_error" in event:
                callback(event)
                return
            data = event.get("data", {})
            event_type = event.get("eventType", "")
            order_id = data.get("order_id", "")
            pair_id = data.get("pair_id", event.get("pairId", "perp/unknown"))
            limit_price = data.get("limit_price", "0")
            size = data.get("size", "0")
            try:
                size_f = float(size)
            except ValueError, TypeError:
                size_f = 0.0
            sz_abs = dango_decimal_to_hl_str(f"{abs(size_f):.6f}")
            order = {
                "coin": _pair_id_to_coin(pair_id),
                "side": "B" if size_f > 0 else "A",
                "limitPx": dango_decimal_to_hl_str(limit_price),
                "sz": sz_abs,
                "oid": order_id,
                "timestamp": _isotime_to_ms(event.get("createdAt")),
                "origSz": sz_abs,
            }
            if event_type == "order_persisted":
                status = "open"
            elif event_type == "order_removed":
                status = data.get("reason", "canceled")
            else:
                status = "unknown"
            callback(
                [
                    {
                        "order": order,
                        "status": status,
                        "statusTimestamp": _isotime_to_ms(event.get("createdAt")),
                    }
                ]
            )

        return self._native.subscribe_user_events(
            Addr(user),
            wrapped,
            event_types=["order_persisted", "order_removed"],
        )

    def _subscribe_l2_book(
        self,
        coin: str,
        callback: Callable[[Any], None],
    ) -> int:
        # No native L2 stream — poll `liquidity_depth` every block via
        # `subscribe_query_app`. `block_interval=1` means once per
        # block (~1 second on Dango).
        pair_id = self.name_to_pair(coin)
        param = self._native.pair_param(pair_id)
        if param is None or not param.get("bucket_sizes"):
            raise RuntimeError(
                f"pair {pair_id} has no configured bucket_sizes; cannot stream L2 snapshot"
            )
        bucket_size = min(param["bucket_sizes"], key=float)
        request = {
            "wasm_smart": {
                "contract": self._native.perps_contract,
                "msg": {
                    "liquidity_depth": {
                        "pair_id": pair_id,
                        "bucket_size": bucket_size,
                        "limit": None,
                    }
                },
            }
        }

        def wrapped(payload: Any) -> None:
            if not isinstance(payload, dict) or "_error" in payload:
                callback(payload)
                return
            depth = payload.get("response") or {}
            time_ms = (payload.get("blockHeight") or 0) * 1000
            callback(_reshape_l2_to_hl(depth, coin, time_ms=time_ms))

        return self._native.subscribe_query_app(request, wrapped, block_interval=1)

    def _subscribe_bbo(
        self,
        coin: str,
        callback: Callable[[Any], None],
    ) -> int:
        # Best bid/offer = top-of-book. Same polling pattern as L2 but
        # with `limit=1` (only fetch the top level on each side).
        pair_id = self.name_to_pair(coin)
        param = self._native.pair_param(pair_id)
        if param is None or not param.get("bucket_sizes"):
            raise RuntimeError(f"pair {pair_id} has no configured bucket_sizes; cannot stream BBO")
        bucket_size = min(param["bucket_sizes"], key=float)
        request = {
            "wasm_smart": {
                "contract": self._native.perps_contract,
                "msg": {
                    "liquidity_depth": {
                        "pair_id": pair_id,
                        "bucket_size": bucket_size,
                        "limit": 1,
                    }
                },
            }
        }

        def wrapped(payload: Any) -> None:
            if not isinstance(payload, dict) or "_error" in payload:
                callback(payload)
                return
            depth = payload.get("response") or {}
            book = _reshape_l2_to_hl(depth, coin, time_ms=0)
            best_bid = book["levels"][0][0] if book["levels"][0] else None
            best_ask = book["levels"][1][0] if book["levels"][1] else None
            time_ms = (payload.get("blockHeight") or 0) * 1000
            callback({"coin": coin, "time": time_ms, "bbo": (best_bid, best_ask)})

        return self._native.subscribe_query_app(request, wrapped, block_interval=1)

    def _subscribe_all_mids(
        self,
        callback: Callable[[Any], None],
    ) -> int:
        # `all_perps_pair_stats` is an indexer query rather than a
        # contract query, but `subscribe_query_app` only polls
        # contract `query_app` requests — we can't poll the indexer
        # via that hook. As a pragmatic fallback we poll
        # `pair_states` (which contains the current oracle price
        # implicitly via `funding_per_unit` and `oi`) — but the
        # mid-price field comes from `currentPrice` on
        # `PerpsPairStats`. For now we raise to flag the missing
        # plumbing rather than emit a partial stream that omits the
        # mid prices.
        _ = callback
        raise NotImplementedError(
            "allMids subscription not yet implemented — needs indexer-side "
            "polling support beyond subscribe_query_app"
        )

    def _subscribe_active_asset_ctx(
        self,
        coin: str,
        callback: Callable[[Any], None],
    ) -> int:
        # Poll `pair_state` per block. The PerpAssetCtx reshape needs
        # `pair_stats` too (for `currentPrice`/`prevDayPx`); since
        # those come from a separate indexer query, we synthesize a
        # ctx from `pair_state` alone and leave price fields zeroed.
        # An HL client reading `markPx`/`oraclePx` will get "0" until
        # we wire up a parallel pair_stats poll.
        pair_id = self.name_to_pair(coin)
        request = {
            "wasm_smart": {
                "contract": self._native.perps_contract,
                "msg": {"pair_state": {"pair_id": pair_id}},
            }
        }

        def wrapped(payload: Any) -> None:
            if not isinstance(payload, dict) or "_error" in payload:
                callback(payload)
                return
            pair_state = payload.get("response")
            ctx = _reshape_pair_state_to_perp_ctx(pair_id, pair_state, None)
            callback({"coin": coin, "ctx": ctx})

        return self._native.subscribe_query_app(request, wrapped, block_interval=1)

    def _subscribe_active_asset_data(
        self,
        user: str,
        coin: str,
        callback: Callable[[Any], None],
    ) -> int:
        # `activeAssetData` combines per-user state with per-pair
        # state. `subscribe_query_app` polls one request at a time,
        # so we use `multi` (API §1.4) to bundle the two queries.
        pair_id = self.name_to_pair(coin)
        request = {
            "multi": [
                {
                    "wasm_smart": {
                        "contract": self._native.perps_contract,
                        "msg": {
                            "user_state_extended": {
                                "user": user,
                                "include_equity": True,
                                "include_available_margin": True,
                                "include_maintenance_margin": True,
                                "include_unrealized_pnl": True,
                                "include_unrealized_funding": True,
                                "include_liquidation_price": False,
                            }
                        },
                    }
                },
                {
                    "wasm_smart": {
                        "contract": self._native.perps_contract,
                        "msg": {"pair_state": {"pair_id": pair_id}},
                    }
                },
            ]
        }

        def wrapped(payload: Any) -> None:
            if not isinstance(payload, dict) or "_error" in payload:
                callback(payload)
                return
            multi = payload.get("response", {}).get("multi") or []
            user_state = multi[0].get("Ok") if multi else None
            available = _to_hl_str(user_state.get("available_margin")) if user_state else "0"
            # `markPx` should be a price; `pair_state.funding_per_unit` is a
            # per-unit funding accrual (~0.0001), not a price (~60_000), so
            # sourcing markPx from it would mislead any HL client that
            # displays it. Until we plumb a parallel `pair_stats` poll into
            # this subscription, leave markPx zeroed and document the gap.
            data = ActiveAssetData(
                user=user,
                coin=coin,
                # Synthetic: Dango is cross-only, no leverage knob;
                # see `_reshape_user_state_to_hl` for the same
                # pattern. `value=1` matches the position-side default.
                leverage={"type": "cross", "value": 1},
                # `maxTradeSzs` and `availableToTrade` are HL
                # convention pairs (long-side, short-side). On
                # Dango the available margin is direction-agnostic
                # because cross-margin pools across positions, so
                # we report the same value twice.
                maxTradeSzs=(available, available),
                availableToTrade=(available, available),
                markPx="0",
            )
            callback({"coin": coin, "user": user, "ctx": data})

        return self._native.subscribe_query_app(request, wrapped, block_interval=1)


# --- Module-level helpers (private) ----------------------------------------


def _ms_to_ns_str(ms: int) -> str:
    """Convert ms int → ns string for Dango's indexer Timestamp wire shape."""
    # Dango's Timestamp is a stringified ns count. The indexer
    # accepts both `laterThan: "1700000000000000000"` (ns string)
    # and `laterThan: "2024-01-01T00:00:00Z"` (ISO). We use ns
    # strings here because they round-trip cleanly with the
    # ms inputs HL traders pass in.
    return str(ms * 1_000_000)


def _dedupe_fills(events: list[PerpsEvent]) -> list[Fill]:
    """Reshape filled-event list to HL Fills, dropping the maker side of paired rows."""
    # Dango writes both the maker's and the taker's view of every
    # fill (paired by `fill_id`). HL emits one row per match. We
    # prefer the taker side (`is_maker == False`); if both are
    # present we drop the maker. If only one side is present (e.g.
    # because the indexer filtered events differently), we keep it.
    seen: dict[str | None, PerpsEvent] = {}
    for ev in events:
        data = ev["data"]
        fill_id = data.get("fill_id")
        is_maker = data.get("is_maker")
        # Use a synthetic key for events without `fill_id`:
        # (block_height, idx) is unique per indexer row.
        key = fill_id if fill_id is not None else f"{ev['blockHeight']}/{ev['idx']}"
        existing = seen.get(key)
        if existing is None:
            seen[key] = ev
        else:
            # Prefer the taker side. If the existing one is maker
            # and the new one is taker, replace; otherwise keep.
            existing_is_maker = existing["data"].get("is_maker")
            if existing_is_maker and not is_maker:
                seen[key] = ev
    return [_reshape_fill_to_hl(ev) for ev in seen.values()]

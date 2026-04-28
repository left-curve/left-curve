"""HL-compat API: walk every implemented read-only query against mainnet.

Each query is its own function so the script reads top-to-bottom as a
table of contents; ``main`` calls them sequentially. The user-keyed
queries target the perps contract address — the counterparty vault — so
the example is fully runnable with no ``.env`` and reflects live
on-chain positions.

Methods that raise ``NotImplementedError`` on Dango (spot_*, staking_*,
funding_history, etc.) are intentionally omitted; see the docstrings on
each stub in ``dango.hyperliquid_compatibility.info`` for the reasons.
"""

from __future__ import annotations

import json
import time

import example_utils_hl as example_utils

from dango.hyperliquid_compatibility import constants
from dango.hyperliquid_compatibility.info import Info
from dango.utils.constants import PERPS_CONTRACT_MAINNET

# Counterparty vault — its on-chain account always carries live positions
# on mainnet, so we use it as the user-keyed query target. HL methods
# take address as a plain ``str`` (not the typed ``Addr``).
_VAULT_ADDR: str = PERPS_CONTRACT_MAINNET


def _print(label: str, value: object) -> None:
    """Print one section header + a truncated JSON dump of the result."""
    # 500-char cap keeps `meta_and_asset_ctxs` and `historical_orders`
    # from drowning the rest of the output; the goal is to confirm shape,
    # not to dump full state.
    print(f"\n--- {label} ---")
    print(json.dumps(value, indent=2, default=str)[:500])


def meta(info: Info) -> None:
    """HL universe metadata: ``{universe: [{name, szDecimals}, ...]}``."""
    _print("meta", info.meta())


def meta_and_asset_ctxs(info: Info) -> None:
    """``[meta, ctxs]`` — static metadata plus per-asset live ctx (funding/markPx/OI)."""
    _print("meta_and_asset_ctxs", info.meta_and_asset_ctxs())


def all_mids(info: Info) -> None:
    """``{coin: midPriceStr}`` — current mid for each listed coin."""
    _print("all_mids", info.all_mids())


def l2_snapshot(info: Info, coin: str) -> None:
    """L2 order book: ``{coin, levels: [bids, asks], time}``."""
    _print(f"l2_snapshot[{coin}]", info.l2_snapshot(coin))


def candles_snapshot(info: Info, coin: str, interval: str) -> None:
    """OHLCV candles for the last 5 minutes, HL ``{T,t,s,i,o,c,h,l,v,n}`` shape."""
    now_ms = int(time.time() * 1000)
    five_minutes_ms = 5 * 60 * 1000
    _print(
        f"candles_snapshot[{coin}, {interval}, last 5 min]",
        info.candles_snapshot(coin, interval, now_ms - five_minutes_ms, now_ms),
    )


def user_state(info: Info, user: str) -> None:
    """HL `clearinghouseState`: assetPositions, marginSummary, withdrawable."""
    _print(f"user_state[{user}]", info.user_state(user))


def open_orders(info: Info, user: str) -> list[dict[str, object]]:
    """Resting orders for one user, HL flat-list shape."""
    orders = info.open_orders(user)
    _print(f"open_orders[{user}]", orders)
    return orders


def user_fills_by_time(info: Info, user: str) -> None:
    """Fills in the last minute for one user.

    The bare ``user_fills(addr)`` paginates the user's entire fill history
    client-side; on the perps vault (which is the counterparty for every
    trade) that's tens of thousands of records and takes minutes. The
    time-bounded variant is the practical choice for a live example —
    we use the last 60 seconds.
    """
    now_ms = int(time.time() * 1000)
    one_minute_ms = 60 * 1000
    _print(
        f"user_fills_by_time[{user}, last minute]",
        info.user_fills_by_time(user, now_ms - one_minute_ms, now_ms),
    )


def query_order_by_oid(info: Info, user: str, oid: int | str) -> None:
    """Single-order lookup by chain OrderId."""
    _print(f"query_order_by_oid[{oid}]", info.query_order_by_oid(user, oid))


def main() -> None:
    info = example_utils.setup_read_only(constants.MAINNET_API_URL, skip_ws=True)

    # Public queries that require no user.
    meta(info)
    meta_and_asset_ctxs(info)
    all_mids(info)
    l2_snapshot(info, "BTC")
    candles_snapshot(info, "BTC", "1m")

    # User-keyed queries against the perps vault — always populated on
    # mainnet. We skip the unbounded ``user_fills`` and ``historical_orders``
    # variants here: against an active vault they paginate tens of
    # thousands of records and take minutes. ``user_fills_by_time`` with a
    # short window is the practical equivalent for a live demo.
    user_state(info, _VAULT_ADDR)
    user_fills_by_time(info, _VAULT_ADDR)
    orders = open_orders(info, _VAULT_ADDR)
    if orders:
        query_order_by_oid(info, _VAULT_ADDR, orders[0]["oid"])


if __name__ == "__main__":
    main()

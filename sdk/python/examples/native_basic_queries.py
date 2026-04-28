"""Native Dango API: walk every read-only HTTP query against mainnet.

Each query is its own function so the script reads top-to-bottom as a
table of contents; ``main`` calls them sequentially. The user-keyed
queries (``user_state``, ``orders_by_user``, ``volume``, ``order``) target
the perps contract address — the counterparty vault — so the example is
fully runnable with no ``.env`` and reflects live on-chain positions.
"""

from __future__ import annotations

import json
from typing import Any

import example_utils

from dango.info import Info
from dango.utils.constants import MAINNET_API_URL, PERPS_CONTRACT_MAINNET
from dango.utils.types import Addr, CandleInterval, OrderId, PairId

# Counterparty vault — its on-chain account always carries live positions
# on mainnet, so we use it as the user-keyed query target.
_VAULT_ADDR = Addr(PERPS_CONTRACT_MAINNET)


def _print(label: str, value: object) -> None:
    """Print one section header + a truncated JSON dump of the result."""
    # 500-char cap keeps `all_perps_pair_stats` and friends from drowning
    # the rest of the output; the goal is to confirm shape, not to dump
    # full state.
    print(f"\n--- {label} ---")
    print(json.dumps(value, indent=2, default=str)[:500])


def query_status(info: Info) -> None:
    """Chain id + latest block — sanity check the connection."""
    _print("query_status", info.query_status())


def perps_param(info: Info) -> None:
    """Global perps params (fee schedules, batch size, etc.)."""
    _print("perps_param", info.perps_param())


def perps_state(info: Info) -> None:
    """Global perps state (insurance fund, treasury, vault share supply)."""
    _print("perps_state", info.perps_state())


def pair_params(info: Info) -> dict[PairId, Any]:
    """Per-pair params (tick_size, bucket_sizes, IM/MM ratios, ...).

    Returned for reuse by downstream queries that need a sample pair.
    """
    pairs = info.pair_params()
    _print("pair_params", pairs)
    return pairs


def pair_states(info: Info) -> None:
    """Per-pair runtime state (open interest, funding accumulator, ...)."""
    _print("pair_states", info.pair_states())


def pair_param_one(info: Info, pair_id: PairId) -> None:
    """Single-pair param lookup; returns ``None`` for unknown pairs."""
    _print(f"pair_param[{pair_id}]", info.pair_param(pair_id))


def pair_state_one(info: Info, pair_id: PairId) -> None:
    """Single-pair runtime state."""
    _print(f"pair_state[{pair_id}]", info.pair_state(pair_id))


def liquidity_depth(info: Info, pair_id: PairId, bucket_size: str) -> None:
    """Aggregated bid/ask depth bucketed by the given price tick."""
    # `limit=5` keeps the dump small. Real consumers typically pull the
    # whole book via `limit=None`.
    _print(
        f"liquidity_depth[{pair_id}, bucket={bucket_size}]",
        info.liquidity_depth(pair_id, bucket_size=bucket_size, limit=5),
    )


def perps_pair_stats(info: Info, pair_id: PairId) -> None:
    """24h indexer-side stats for one pair (volume, OHLC, currentPrice)."""
    _print(f"perps_pair_stats[{pair_id}]", info.perps_pair_stats(pair_id))


def all_perps_pair_stats(info: Info) -> None:
    """24h stats for every listed pair."""
    _print("all_perps_pair_stats", info.all_perps_pair_stats())


def perps_candles(info: Info, pair_id: PairId) -> None:
    """Cursor-paginated OHLCV candles; we only fetch the first three."""
    page = info.perps_candles(pair_id, CandleInterval.ONE_MINUTE, first=3)
    _print(f"perps_candles[{pair_id}, ONE_MINUTE, first=3]", page.nodes)


def perps_events(info: Info) -> None:
    """Cursor-paginated indexer events; first three across all pairs."""
    page = info.perps_events(first=3)
    _print("perps_events[first=3]", page.nodes)


def user_state(info: Info, user: Addr) -> None:
    """Perps margin sub-account state: margin, positions, vault shares."""
    _print(f"user_state[{user}]", info.user_state(user))


def user_state_extended(info: Info, user: Addr) -> None:
    """User state with computed equity / available margin / liquidation prices."""
    _print(f"user_state_extended[{user}]", info.user_state_extended(user))


def orders_by_user(info: Info, user: Addr) -> dict[OrderId, Any]:
    """All resting orders for one user, keyed by chain OrderId."""
    orders = info.orders_by_user(user)
    _print(f"orders_by_user[{user}]", orders)
    return orders


def volume(info: Info, user: Addr) -> None:
    """Lifetime USD trading volume for one user (used for fee-tier resolution)."""
    _print(f"volume[{user}]", info.volume(user))


def order_one(info: Info, order_id: OrderId) -> None:
    """Single-order lookup by chain OrderId; returns ``None`` for unknown ids."""
    _print(f"order[{order_id}]", info.order(order_id))


def main() -> None:
    info = example_utils.setup_read_only(MAINNET_API_URL, skip_ws=True, perps_contract=_VAULT_ADDR)

    # Public queries that require no inputs.
    query_status(info)
    perps_param(info)
    perps_state(info)
    pair_states(info)
    all_perps_pair_stats(info)

    # Pair-keyed queries — pick the first pair and the smallest bucket.
    pairs = pair_params(info)
    sample_pair = next(iter(pairs))
    smallest_bucket = min(pairs[sample_pair]["bucket_sizes"], key=float)
    pair_param_one(info, sample_pair)
    pair_state_one(info, sample_pair)
    liquidity_depth(info, sample_pair, smallest_bucket)
    perps_pair_stats(info, sample_pair)
    perps_candles(info, sample_pair)
    perps_events(info)

    # User-keyed queries against the perps vault — always populated on mainnet.
    user_state(info, _VAULT_ADDR)
    user_state_extended(info, _VAULT_ADDR)
    volume(info, _VAULT_ADDR)
    orders = orders_by_user(info, _VAULT_ADDR)
    if orders:
        order_one(info, OrderId(next(iter(orders))))


if __name__ == "__main__":
    main()

"""Native Dango API: market open, then market close (reduce-only)."""

from __future__ import annotations

import json

import example_utils

from dango.utils.constants import TESTNET_API_URL
from dango.utils.types import Addr, PairId


def main() -> None:
    address, info, exchange = example_utils.setup(
        base_url=TESTNET_API_URL,
        skip_ws=True,
    )

    pair_id = PairId("perp/ethusd")

    # Open a small long via a market order. `max_slippage` is a
    # 6-decimal `Dimensionless` cap (0.01 = 1%); the chain rejects
    # the order if the realized slippage would exceed this.
    open_result = exchange.submit_market_order(
        pair_id,
        size="0.01",
        max_slippage="0.01",
    )
    print("market open result:")
    print(json.dumps(open_result, indent=2))

    # Read back the current position, if any. `user_state` returns the
    # native `UserState` snake_case shape: `positions` is a
    # `dict[PairId, Position]` keyed by pair_id (NOT a list — `pair_id`
    # is the dict key, not a field on Position). See
    # `dango/types/src/perps.rs::UserState`.
    state = info.user_state(Addr(address))
    positions = (state or {}).get("positions") or {}
    pos = positions.get(pair_id)
    if pos is None:
        print(f"no open position on {pair_id}; skipping reduce-only close")
        return
    print("current position:")
    print(json.dumps({"pair_id": pair_id, **pos}, indent=2))

    # Reduce-only close: pass the OPPOSITE-signed size so the order
    # can only shrink the existing position. `reduce_only=True`
    # belt-and-braces this on the contract side too.
    held_size = pos["size"]
    closing_size = f"-{held_size}" if not held_size.startswith("-") else held_size[1:]
    close_result = exchange.submit_market_order(
        pair_id,
        size=closing_size,
        max_slippage="0.01",
        reduce_only=True,
    )
    print("market close (reduce-only) result:")
    print(json.dumps(close_result, indent=2))


if __name__ == "__main__":
    main()

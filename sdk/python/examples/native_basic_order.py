"""Native Dango API: place, query, and cancel a single resting limit order."""

from __future__ import annotations

import json

import example_utils

from dango.utils.constants import TESTNET_API_URL
from dango.utils.types import Addr, OrderId, PairId, TimeInForce


def main() -> None:
    address, info, exchange = example_utils.setup(
        base_url=TESTNET_API_URL,
        skip_ws=True,
    )

    # Read the user's perps state and print existing positions, if any.
    # Native `user_state` returns the contract's snake_case `UserState`
    # shape — see `dango/types/src/perps.rs::UserState`. `positions` is
    # a `dict[PairId, Position]` keyed by pair_id, so iterate `.items()`.
    state = info.user_state(Addr(address))
    positions = (state or {}).get("positions") or {}
    if positions:
        print("positions:")
        for pair_id, position in positions.items():
            print(json.dumps({"pair_id": pair_id, **position}, indent=2))
    else:
        print("no open positions")

    # Place a limit order that should rest by setting the price very low.
    # Sign convention is signed-size: positive = buy, negative = sell.
    # `submit_limit_order` is the convenience helper that wraps
    # `submit_order` with a `LimitKind` payload.
    pair_id = PairId("perp/ethusd")
    order_result = exchange.submit_limit_order(
        pair_id,
        size="0.2",
        limit_price="1100",
        time_in_force=TimeInForce.GTC,
    )
    print("submit_order result:")
    print(json.dumps(order_result, indent=2))

    # The broadcast outcome carries the chain's tx hash. Resting orders
    # are surfaced via subsequent indexer events — we look them up by
    # querying `orders_by_user`.
    open_orders = info.orders_by_user(Addr(address))
    if open_orders:
        first_oid = next(iter(open_orders))
        print(f"resting order {first_oid}:")
        order = info.order(OrderId(first_oid))
        print(json.dumps(order, indent=2))

        # Cancel by chain OrderId.
        cancel_result = exchange.cancel_order(OrderId(first_oid))
        print("cancel_order result:")
        print(json.dumps(cancel_result, indent=2))
    else:
        print("no resting order found to cancel")


if __name__ == "__main__":
    main()

"""Native Dango API: subscribe to trades, candles, user events, and blocks."""

from __future__ import annotations

import time

import example_utils

from dango.utils.constants import TESTNET_API_URL
from dango.utils.types import Addr, CandleInterval, PairId


def main() -> None:
    address, info, _exchange = example_utils.setup_native(base_url=TESTNET_API_URL)

    pair_id = PairId("perp/ethusd")

    # Each `subscribe_*` returns an int subscription id; we keep them so
    # the script can unsubscribe explicitly on shutdown. The callback
    # receives the unwrapped node payload — one Trade per fill, one
    # Block per block, etc. Server-side errors arrive as
    # `{"_error": ...}` (see `dango.info._unwrap_node`).
    sub_ids: list[int] = []
    sub_ids.append(info.subscribe_perps_trades(pair_id, print))
    sub_ids.append(info.subscribe_perps_candles(pair_id, CandleInterval.ONE_MINUTE, print))
    sub_ids.append(info.subscribe_user_events(Addr(address), print))
    sub_ids.append(info.subscribe_block(print))

    print(f"subscribed: {sub_ids}; streaming for 30s...")
    time.sleep(30)

    # Drop the subscriptions and close the WebSocket.
    for sid in sub_ids:
        info.unsubscribe(sid)
    info.disconnect_websocket()


if __name__ == "__main__":
    main()

"""Native Dango API: stream BTC perps events over a shared `WsConnection`.

Subscribes to the ``perpsEvents`` feed filtered to the BTC pair and the order
lifecycle / forced-exit event types — ``order_persisted``, ``order_removed``,
``order_resized``, ``order_filled``, ``liquidated``, ``deleveraged`` — grouped
per block, over the multiplexed native `/ws` connection (`dango.ws.WsConnection`).
Runnable with no ``.env``: it only reads public chain state.

Run with::

    uv run python examples/native_perps_events.py
"""

from __future__ import annotations

import threading
from typing import cast

from dango.utils.error import ServerError
from dango.utils.types import PerpsEvent2Batch
from dango.ws import WsConnection

# Native `/ws` endpoint (testnet). `WsConnection` is WS-only, so it takes a
# `ws://` / `wss://` URL directly.
WS_URL = "wss://api-testnet.dango.zone/ws"

# Order lifecycle plus the two forced-exit events. The filters AND together, so
# pairing these with `pair_ids` keeps only BTC events of these types.
_EVENT_TYPES = [
    "order_persisted",
    "order_removed",
    "order_resized",
    "order_filled",
    "liquidated",
    "deleveraged",
]


def _print_batch(batch: PerpsEvent2Batch) -> None:
    """Print one line per event in a block's batch."""

    for event in batch["events"]:
        print(
            f"block={batch['blockHeight']} idx={event['idx']} "
            f"type={event['eventType']} user={event['user']} pair={event['pairId']} "
            f"order_id={event['orderId']} client_order_id={event['clientOrderId']} "
            f"data={event['data']}"
        )


def main() -> None:
    with WsConnection.connect(WS_URL) as conn:
        events = conn.subscribe_perps_events(
            pair_ids=["perp/btcusd"],
            event_types=_EVENT_TYPES,
        )

        print("subscribed; streaming BTC perps events from testnet for 30s...")

        # Each `next` blocks until the next block's batch arrives; closing the
        # socket after 30s ends the iterator (the loop sees `StopIteration`), so
        # the demo self-terminates even when the market is quiet.
        threading.Timer(30.0, conn.close).start()

        try:
            for batch in events:
                _print_batch(cast("PerpsEvent2Batch", batch))
        except ServerError as exc:
            # A terminal `resync` / `tooManyRequests` frame surfaces here.
            print("stream error:", exc)


if __name__ == "__main__":
    main()

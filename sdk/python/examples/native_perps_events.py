"""Native Dango API: stream BTC perps events from testnet.

Subscribes to the ``perpsEvents`` feed filtered to the BTC pair and the order
lifecycle / forced-exit event types — ``order_persisted``, ``order_removed``,
``order_resized``, ``order_filled``, ``liquidated``, ``deleveraged`` — grouped
per block. Runnable with no ``.env``: it only reads public chain state.

Run with::

    uv run python examples/native_perps_events.py
"""

from __future__ import annotations

import time

import example_utils

from dango.utils.constants import TESTNET_API_URL
from dango.utils.types import PerpsEvent2Batch

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
    """Print one line per event; pass error envelopes through verbatim."""

    # Server-side errors arrive as `{"_error": ...}` (see
    # `dango.info._unwrap_node`); surface them and skip this message.
    if "_error" in batch:
        print("error:", batch)
        return

    for event in batch["events"]:
        print(
            f"block={batch['blockHeight']} idx={event['idx']} "
            f"type={event['eventType']} user={event['user']} pair={event['pairId']} "
            f"order_id={event['orderId']} client_order_id={event['clientOrderId']} "
            f"data={event['data']}"
        )


def main() -> None:
    info = example_utils.setup_read_only(TESTNET_API_URL)

    sub_id = info.subscribe_perps_events(
        _print_batch,
        pair_ids=["perp/btcusd"],
        event_types=_EVENT_TYPES,
    )

    print(f"subscribed: {sub_id}; streaming BTC perps events from testnet for 30s...")

    time.sleep(30)

    # Drop the subscription and close the WebSocket.
    info.unsubscribe(sub_id)
    info.disconnect_websocket()


if __name__ == "__main__":
    main()

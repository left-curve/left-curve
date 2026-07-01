"""Tests for dango.ws.WsConnection — the multiplexed native `/ws` client.

These drive the reader/keepalive machinery against a fake socket (no real I/O):
`_FakeSocket.recv` blocks on an inbound queue the test feeds, and `send`
records every outbound frame. Because the reader runs on a background thread,
tests synchronise through the same primitives production code does — the
subscription's queue and the broadcast reply slot.
"""

from __future__ import annotations

import json
import queue
import threading
from typing import Any, cast

import pytest
import websocket

from dango.utils.error import ServerError
from dango.utils.types import Tx
from dango.ws import WsConnection

_EOF = object()


class _FakeSocket:
    """A stand-in for `websocket.WebSocket`: records sends, replays fed frames."""

    def __init__(self) -> None:
        self.sent: list[dict[str, Any]] = []
        self.outbound: queue.Queue[dict[str, Any]] = queue.Queue()
        self._inbound: queue.Queue[Any] = queue.Queue()
        self.closed: bool = False

    # --- the WebSocket surface WsConnection uses ---
    def settimeout(self, _timeout: float | None) -> None:
        pass

    def send(self, data: str) -> None:
        frame = json.loads(data)
        self.sent.append(frame)
        self.outbound.put(frame)

    def recv(self) -> str:
        item = self._inbound.get()
        if item is _EOF:
            return ""  # falsy => reader loop ends
        return cast("str", item)

    def close(self) -> None:
        self.closed = True
        self._inbound.put(_EOF)

    # --- test helpers ---
    def push(self, frame: dict[str, Any]) -> None:
        self._inbound.put(json.dumps(frame))

    def next_sent(self, timeout: float = 2.0) -> dict[str, Any]:
        return self.outbound.get(timeout=timeout)


def _connect(monkeypatch: pytest.MonkeyPatch) -> tuple[WsConnection, _FakeSocket]:
    fake = _FakeSocket()
    monkeypatch.setattr(websocket, "create_connection", lambda *_a, **_k: fake)
    conn = WsConnection.connect("http://localhost:8080")
    return conn, fake


def _demo_tx() -> Tx:
    return cast(
        "Tx",
        {
            "sender": "0x000000000000000000000000000000000000beef",
            "gas_limit": 1_000_000,
            "msgs": [],
            "data": {},
            "credential": {},
        },
    )


class TestSubscribe:
    def test_sends_subscribe_frame_with_filters(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """subscribe_perps_events sends a `subscribe` frame with camelCase filters."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub = conn.subscribe_perps_events(pair_ids=["perp/btcusd"], client_order_ids=["7"])
            frame = fake.next_sent()
            assert frame["method"] == "subscribe"
            assert frame["id"] == sub.id
            assert frame["subscription"] == {
                "type": "perpsEvents",
                "pairIds": ["perp/btcusd"],
                "clientOrderIds": ["7"],
            }

    def test_data_frame_yields_batch(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """A `perpsEvents` data frame is delivered as the bare batch to the iterator."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub = conn.subscribe_perps_events(pair_ids=["perp/btcusd"])
            sub_id = fake.next_sent()["id"]
            batch = {"blockHeight": 7, "createdAt": "t", "events": [{"idx": 0}]}
            fake.push({"channel": "perpsEvents", "id": sub_id, "data": batch})
            assert next(sub) == batch

    def test_terminal_error_raises(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """An `error` frame on the subscription channel ends the stream by raising."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub = conn.subscribe_perps_events()
            sub_id = fake.next_sent()["id"]
            fake.push(
                {
                    "channel": "perpsEvents",
                    "id": sub_id,
                    "error": {"code": "resync", "message": "x"},
                }
            )
            with pytest.raises(ServerError, match="resync"):
                next(sub)

    def test_id_demux_routes_to_correct_subscription(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Two subscriptions on one socket each receive only their own id's frames."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub_a = conn.subscribe_perps_events(pair_ids=["perp/btcusd"])
            id_a = fake.next_sent()["id"]
            sub_b = conn.subscribe_perps_events(pair_ids=["perp/ethusd"])
            id_b = fake.next_sent()["id"]
            assert id_a != id_b

            fake.push({"channel": "perpsEvents", "id": id_b, "data": {"which": "b"}})
            fake.push({"channel": "perpsEvents", "id": id_a, "data": {"which": "a"}})
            assert next(sub_b) == {"which": "b"}
            assert next(sub_a) == {"which": "a"}

    def test_unsubscribe_sends_frame(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Exiting the subscription context tells the server to stop streaming."""

        conn, fake = _connect(monkeypatch)
        with conn:
            with conn.subscribe_perps_events():
                sub_id = fake.next_sent()["id"]
            frame = fake.next_sent()
            assert frame == {"method": "unsubscribe", "id": sub_id}

    def test_next_batch_times_out_when_silent(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """`next_batch(timeout)` surfaces a stalled feed as TimeoutError."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub = conn.subscribe_perps_events()
            fake.next_sent()  # drain the subscribe frame
            with pytest.raises(TimeoutError):
                sub.next_batch(timeout=0.05)


class TestBroadcast:
    def _broadcast_async(self, conn: WsConnection, box: dict[str, Any]) -> threading.Thread:
        def run() -> None:
            try:
                box["result"] = conn.broadcast(_demo_tx())
            except Exception as exc:  # surfaced to the asserting thread
                box["error"] = exc

        t = threading.Thread(target=run)
        t.start()
        return t

    def test_roundtrip_returns_outcome(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """broadcast blocks until the matching `broadcast` reply, then returns its data."""

        conn, fake = _connect(monkeypatch)
        with conn:
            box: dict[str, Any] = {}
            thread = self._broadcast_async(conn, box)

            frame = fake.next_sent()
            assert frame["method"] == "broadcast"
            outcome = {"tx_hash": "0x1", "check_tx": {"result": {"Ok": None}}}
            fake.push({"channel": "broadcast", "id": frame["id"], "data": outcome})

            thread.join(timeout=2.0)
            assert box.get("result") == outcome

    def test_error_frame_raises(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """A transport `error` frame on the broadcast channel raises ServerError."""

        conn, fake = _connect(monkeypatch)
        with conn:
            box: dict[str, Any] = {}
            thread = self._broadcast_async(conn, box)

            frame = fake.next_sent()
            fake.push(
                {
                    "channel": "broadcast",
                    "id": frame["id"],
                    "error": {"code": "broadcastFailed", "message": "down"},
                }
            )

            thread.join(timeout=2.0)
            assert isinstance(box.get("error"), ServerError)
            assert "broadcastFailed" in str(box["error"])

    def test_concurrent_ops_do_not_collide(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """A broadcast reply and a subscription frame in flight are routed by id."""

        conn, fake = _connect(monkeypatch)
        with conn:
            sub = conn.subscribe_perps_events(pair_ids=["perp/btcusd"])
            sub_id = fake.next_sent()["id"]

            box: dict[str, Any] = {}
            thread = self._broadcast_async(conn, box)
            bcast_id = fake.next_sent()["id"]

            # Reply to the broadcast and push a subscription frame; each lands
            # with its own consumer despite sharing the socket.
            fake.push({"channel": "broadcast", "id": bcast_id, "data": {"tx_hash": "0x9"}})
            fake.push({"channel": "perpsEvents", "id": sub_id, "data": {"e": 1}})

            thread.join(timeout=2.0)
            assert box.get("result") == {"tx_hash": "0x9"}
            assert next(sub) == {"e": 1}


class TestClose:
    def test_close_ends_subscription_iteration(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Closing the connection ends outstanding subscriptions cleanly."""

        conn, fake = _connect(monkeypatch)
        sub = conn.subscribe_perps_events()
        fake.next_sent()
        conn.close()
        with pytest.raises(StopIteration):
            next(sub)

    def test_close_fails_pending_broadcast(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """A broadcast still awaiting a reply fails when the socket closes."""

        conn, fake = _connect(monkeypatch)
        box: dict[str, Any] = {}
        thread = TestBroadcast()._broadcast_async(conn, box)
        fake.next_sent()  # the broadcast frame is on the wire
        conn.close()

        thread.join(timeout=2.0)
        assert isinstance(box.get("error"), ServerError)
        assert "connection closed" in str(box["error"])

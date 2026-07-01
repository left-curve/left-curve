"""Long-lived, multiplexed client for the native `/ws` endpoint.

A single connection carries any number of subscriptions (`perpsEvents`,
`fullBlock`) *and* transaction broadcasts, demultiplexed by the protocol's
`id`/`channel`. This is the shared-connection counterpart to the one-shot
helpers it replaces: where the old per-operation client opened a fresh socket
for every subscription and every broadcast, `WsConnection` keeps one socket
open and routes each reply back to the caller that issued it — the pattern a
high-performance trading bot wants (subscribe, react, broadcast, all on one
connection).

The Rust SDK splits this into a `Clone` handle plus a private `WsManager`
actor joined by a channel; Python has no such ownership boundary, so the
manager is folded into this one class. A daemon reader thread routes inbound
frames into per-operation queues/slots shared under a lock, while
`subscribe`/`broadcast` run on the caller's thread (register the reply channel,
then send). A daemon keepalive thread sends an app-level ping every 20s so an
idle connection is not reaped by the server's 60s idle timeout.

Not to be confused with `WebsocketManager` (graphql-transport-ws over
`/graphql`, slated for deprecation).
"""

from __future__ import annotations

import contextlib
import json
import queue
import threading
from typing import Any, Final, cast

import websocket  # from websocket-client

from dango.utils.error import ServerError
from dango.utils.types import Tx

# App-level keepalive interval. The server closes a connection it has not heard
# from for 60s; a 20s ping keeps an idle connection alive with a comfortable
# margin (any inbound frame resets the server's idle timer).
_PING_INTERVAL_SECONDS: Final[float] = 20.0

# Sentinel enqueued into a subscription's queue when the socket closes, so the
# `Subscription` iterator ends cleanly (StopIteration) instead of blocking
# forever in `queue.get()`.
_CLOSED: Final[object] = object()


def _error_detail(error: Any) -> str:
    """Render a server `error` value (`{code, message}` or a scalar) as a string."""

    if isinstance(error, dict):
        return f"{error.get('code', 'error')}: {error.get('message', '')}"

    return str(error)


class _OneShot:
    """A broadcast reply slot: a `threading.Event` plus the parked result or error."""

    def __init__(self) -> None:
        self._event: threading.Event = threading.Event()
        self._result: dict[str, Any] | None = None
        self._error: str | None = None

    def set_ok(self, data: dict[str, Any]) -> None:
        self._result = data
        self._event.set()

    def set_err(self, message: str) -> None:
        self._error = message
        self._event.set()

    def wait(self, timeout: float) -> dict[str, Any]:
        if not self._event.wait(timeout):
            raise ServerError("timed out waiting for broadcast reply")

        if self._error is not None:
            raise ServerError(f"broadcast failed: {self._error}")

        return self._result or {}


class WsConnection:
    """One long-lived `/ws` socket multiplexing subscriptions and broadcasts."""

    @classmethod
    def connect(cls, base_url: str, *, timeout: float = 10.0) -> WsConnection:
        """Open the socket and start the reader + keepalive threads."""

        self = cls(base_url)

        try:
            ws = websocket.create_connection(self._ws_url, timeout=timeout)
        except Exception as exc:
            raise ServerError(f"WebSocket connection to {self._ws_url} failed: {exc}") from exc

        # `create_connection` leaves the connect timeout on the socket, which
        # would make the reader's `recv()` bail after `timeout` seconds of an
        # idle (but healthy) subscription. Clear it so `recv()` blocks until a
        # frame arrives; liveness is handled by the keepalive ping instead.
        with contextlib.suppress(Exception):
            ws.settimeout(None)

        self._ws = ws
        threading.Thread(target=self._run, name="dango-ws-reader", daemon=True).start()
        threading.Thread(target=self._keepalive, name="dango-ws-keepalive", daemon=True).start()

        return self

    def __init__(self, base_url: str) -> None:
        self._ws_url: str = _make_ws_url(base_url)
        self._ws: websocket.WebSocket | None = None

        # `_lock` guards the id allocator and both registries, touched by the
        # caller threads (subscribe/broadcast/unsubscribe) and the reader thread
        # (routing replies). `_send_lock` serialises writes to the socket across
        # the caller threads and the keepalive thread.
        self._lock: threading.Lock = threading.Lock()
        self._send_lock: threading.Lock = threading.Lock()
        self._next_id: int = 0
        self._subs: dict[int, queue.Queue[Any]] = {}
        self._pending: dict[int, _OneShot] = {}
        self._stop: threading.Event = threading.Event()

    # --- public API (runs on the caller's thread) ---------------------------

    def subscribe(self, subscription: dict[str, Any]) -> Subscription:
        """Subscribe with a raw `subscription` selector; return a `Subscription` iterator."""

        sub_id = self._alloc_id()
        q: queue.Queue[Any] = queue.Queue()
        with self._lock:
            self._subs[sub_id] = q

        self._send({"method": "subscribe", "id": sub_id, "subscription": subscription})

        return Subscription(self, sub_id, q)

    def subscribe_perps_events(
        self,
        *,
        since_block_height: int | None = None,
        event_types: list[str] | None = None,
        pair_ids: list[str] | None = None,
        users: list[str] | None = None,
        order_ids: list[str] | None = None,
        client_order_ids: list[str] | None = None,
    ) -> Subscription:
        """Subscribe to `perpsEvents`; each item is one block's `PerpsEvent2Batch`.

        The five filters AND together; `None` (or an omitted filter) does not
        filter on that field, while an empty list matches nothing. A
        `client_order_id` is unique only per sender, so combine
        `client_order_ids` with `users` to single out one trader's order.
        """

        subscription: dict[str, Any] = {"type": "perpsEvents"}
        if since_block_height is not None:
            subscription["since"] = since_block_height
        for key, values in (
            ("eventTypes", event_types),
            ("pairIds", pair_ids),
            ("users", users),
            ("orderIds", order_ids),
            ("clientOrderIds", client_order_ids),
        ):
            if values is not None:
                subscription[key] = values

        return self.subscribe(subscription)

    def broadcast(self, tx: Tx, *, timeout: float = 10.0) -> dict[str, Any]:
        """Broadcast a signed tx over the same socket; block for the `BroadcastTxOutcome`.

        A mempool-rejected tx returns normally (the rejection rides
        `check_tx.result`); a transport failure, a timeout, or a socket that
        closes before the reply raises `ServerError`.
        """

        req_id = self._alloc_id()
        slot = _OneShot()
        with self._lock:
            self._pending[req_id] = slot

        self._send({"method": "broadcast", "id": req_id, "tx": cast("dict[str, Any]", tx)})

        return slot.wait(timeout)

    def close(self) -> None:
        """Stop the threads, close the socket, and fail every outstanding operation."""

        if self._stop.is_set():
            return

        self._stop.set()
        if self._ws is not None:
            with contextlib.suppress(Exception):
                self._ws.close()

        self._fail_all("connection closed")

    def __enter__(self) -> WsConnection:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    # --- reader + keepalive threads -----------------------------------------

    def _run(self) -> None:
        ws = self._ws
        assert ws is not None  # set before this thread is started

        while not self._stop.is_set():
            try:
                raw = ws.recv()
            except Exception:
                break

            if not raw:
                break

            # A single malformed frame should not tear down the connection.
            try:
                msg = json.loads(raw)
            except Exception:
                continue

            if isinstance(msg, dict):
                self._dispatch(msg)

        self._fail_all("connection closed")

    def _dispatch(self, msg: dict[str, Any]) -> None:
        channel = msg.get("channel")
        # `id` is absent on connection-level error frames; `dict.get`/`pop` with
        # `None` simply miss the (int-keyed) registries, so no guard is needed.
        msg_id: Any = msg.get("id")

        if channel == "broadcast":
            with self._lock:
                slot = self._pending.pop(msg_id, None)
            if slot is not None:
                # A transport failure to the consensus node is an `error` frame;
                # a mempool rejection arrives as a `data` frame instead.
                if "error" in msg:
                    slot.set_err(_error_detail(msg.get("error")))
                else:
                    slot.set_ok(cast("dict[str, Any]", msg.get("data", {}) or {}))
        elif channel in ("perpsEvents", "fullBlock"):
            # An `error` on a subscription channel (e.g. `resync`,
            # `tooManyRequests`) is terminal, so drop the registration; the
            # iterator surfaces it and ends.
            if "error" in msg:
                with self._lock:
                    q = self._subs.pop(msg_id, None)
            else:
                with self._lock:
                    q = self._subs.get(msg_id)
            if q is not None:
                q.put(msg)
        # `subscriptionResponse` / `pong` / a connection-level `error` (no id)
        # route to no single caller, so they are ignored.

    def _keepalive(self) -> None:
        while not self._stop.wait(_PING_INTERVAL_SECONDS):
            try:
                self._send({"method": "ping"})
            except Exception:
                break

    # --- internals ----------------------------------------------------------

    def _alloc_id(self) -> int:
        with self._lock:
            self._next_id += 1
            return self._next_id

    def _send(self, message: dict[str, Any]) -> None:
        ws = self._ws
        if ws is None:
            raise ServerError("WebSocket is not connected")

        with self._send_lock:
            ws.send(json.dumps(message))

    def _unsubscribe(self, sub_id: int) -> None:
        """Drop a subscription locally and tell the server to stop streaming."""

        with self._lock:
            existed = self._subs.pop(sub_id, None) is not None

        if existed and not self._stop.is_set():
            with contextlib.suppress(Exception):
                self._send({"method": "unsubscribe", "id": sub_id})

    def _fail_all(self, reason: str) -> None:
        with self._lock:
            subs = list(self._subs.values())
            self._subs.clear()
            pending = list(self._pending.values())
            self._pending.clear()

        for q in subs:
            q.put(_CLOSED)
        for slot in pending:
            slot.set_err(reason)


class Subscription:
    """A blocking iterator over one subscription's frames (the `SubscriptionStream` analog).

    Each `next()` blocks until the next `PerpsEvent2Batch` arrives and returns
    it. A terminal server `error` frame raises `ServerError`; a closed
    connection ends the iterator (`StopIteration`). Use it as a context manager
    (or call `unsubscribe()`) to tell the server to stop streaming.
    """

    def __init__(self, conn: WsConnection, sub_id: int, q: queue.Queue[Any]) -> None:
        self._conn: WsConnection = conn
        self._id: int = sub_id
        self._q: queue.Queue[Any] = q

    @property
    def id(self) -> int:
        return self._id

    def __iter__(self) -> Subscription:
        return self

    def __next__(self) -> dict[str, Any]:
        item = self._q.get()

        if item is _CLOSED:
            raise StopIteration

        return self._interpret(item)

    def next_batch(self, timeout: float | None = None) -> dict[str, Any]:
        """Return the next batch, blocking up to `timeout` seconds (None waits forever).

        Unlike iterating, this surfaces a stalled feed: it raises `TimeoutError`
        if no frame arrives in time (the iterator protocol has no way to signal
        that). A terminal server error raises `ServerError`; a closed connection
        raises `ServerError` too, since an explicit read cannot yield a clean end.
        """

        try:
            item = self._q.get(timeout=timeout)
        except queue.Empty as exc:
            raise TimeoutError(f"no perps event within {timeout}s") from exc

        if item is _CLOSED:
            raise ServerError("connection closed")

        return self._interpret(item)

    def _interpret(self, item: dict[str, Any]) -> dict[str, Any]:
        if "error" in item:
            raise ServerError(_error_detail(item.get("error")))

        # The data frame's payload is already the bare batch (no GraphQL `data`
        # wrapper); hand it straight to the caller.
        return cast("dict[str, Any]", item.get("data", {}) or {})

    def unsubscribe(self) -> None:
        self._conn._unsubscribe(self._id)

    def __enter__(self) -> Subscription:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.unsubscribe()


def _make_ws_url(base_url: str) -> str:
    """Convert an http(s) base_url to the matching ws(s)://...ws endpoint."""

    if base_url.startswith("https://"):
        endpoint = "wss://" + base_url[len("https://") :]
    elif base_url.startswith("http://"):
        endpoint = "ws://" + base_url[len("http://") :]
    elif base_url.startswith(("wss://", "ws://")):
        endpoint = base_url
    else:
        raise ValueError(f"unsupported base_url scheme: {base_url!r}")

    endpoint = endpoint.rstrip("/")
    if not endpoint.endswith("/ws"):
        endpoint = endpoint + "/ws"

    return endpoint

"""Thread-based WebSocket subscription manager for the native `/ws` endpoint.

The WebSocket counterpart to `WebsocketManager` (which speaks
graphql-transport-ws against `/graphql`). This one speaks the native
`method`/`channel`-tagged protocol of the `/ws` endpoint, and — like
`SseManager` — gives each subscription its own connection on its own daemon
thread. Used by `Info.subscribe_perps_events`.

Each `perpsEvents` data frame's payload is delivered to the subscription's
callback. An `error` frame (e.g. `resync`, `tooManyRequests`) or a
connection/transport error is delivered as `{"_error": ...}`, the same envelope
convention `WebsocketManager` and `SseManager` use, so callers handle errors
uniformly across transports.
"""

from __future__ import annotations

import contextlib
import json
import threading
from collections.abc import Callable
from typing import Any, Final

import websocket  # from websocket-client

# Frame-level keepalive interval. The server closes a connection it has not
# heard from for 60s; a 20s ping (and the automatic pong to the server's own
# pings) keeps an idle subscription alive with a comfortable margin, while
# `ping_timeout` surfaces a silently-dead connection rather than hanging.
_PING_INTERVAL_SECONDS: Final[float] = 20.0
_PING_TIMEOUT_SECONDS: Final[float] = 10.0


class _Subscription:
    """One in-flight WebSocket stream: its worker thread, stop flag, and socket."""

    def __init__(self, thread: threading.Thread, stop_event: threading.Event) -> None:
        self.thread: threading.Thread = thread
        self.stop_event: threading.Event = stop_event
        self.ws: websocket.WebSocketApp | None = None


class WsStreamManager:
    """Owns the per-subscription WebSocket worker threads for one `/ws` endpoint."""

    def __init__(self, base_url: str) -> None:
        self._ws_url: str = _make_ws_url(base_url)

        # Guards `_next_id` and `_subscriptions`, touched from both the SDK
        # user's thread (subscribe/unsubscribe/stop) and each worker thread
        # (recording its socket, cleaning up on exit).
        self._lock: threading.Lock = threading.Lock()
        self._next_id: int = 0
        self._subscriptions: dict[int, _Subscription] = {}

    def subscribe(
        self,
        subscription: dict[str, Any],
        callback: Callable[[dict[str, Any]], None],
    ) -> int:
        """Open a `/ws` connection and subscribe with `subscription`; return an int id."""

        with self._lock:
            self._next_id += 1
            sub_id = self._next_id
            stop_event = threading.Event()
            thread = threading.Thread(
                target=self._run,
                args=(sub_id, subscription, callback, stop_event),
                name=f"dango-ws-{sub_id}",
                daemon=True,
            )
            self._subscriptions[sub_id] = _Subscription(thread, stop_event)

        thread.start()
        return sub_id

    def unsubscribe(self, subscription_id: int) -> bool:
        """Stop one stream. Returns False if the id is unknown."""

        with self._lock:
            sub = self._subscriptions.pop(subscription_id, None)

        if sub is None:
            return False

        self._close(sub)
        return True

    def stop(self) -> None:
        """Stop every stream."""

        with self._lock:
            subs = list(self._subscriptions.values())
            self._subscriptions.clear()

        for sub in subs:
            self._close(sub)

    def join(self, timeout: float | None = None) -> None:
        """Best-effort join of the worker threads (used on disconnect)."""

        with self._lock:
            threads = [sub.thread for sub in self._subscriptions.values()]

        for thread in threads:
            thread.join(timeout=timeout)

    # --- internals -----------------------------------------------------------

    @staticmethod
    def _close(sub: _Subscription) -> None:
        sub.stop_event.set()
        # Closing the socket unblocks the worker parked in `run_forever`.
        if sub.ws is not None:
            with contextlib.suppress(Exception):
                sub.ws.close()

    def _run(
        self,
        sub_id: int,
        subscription: dict[str, Any],
        callback: Callable[[dict[str, Any]], None],
        stop_event: threading.Event,
    ) -> None:
        def on_open(ws: websocket.WebSocketApp) -> None:
            ws.send(json.dumps({"method": "subscribe", "id": sub_id, "subscription": subscription}))

        def on_message(ws: websocket.WebSocketApp, raw: Any) -> None:
            # Broad `except` covers `JSONDecodeError` and the `TypeError`
            # `json.loads` raises on non-str/bytes input — a single bad frame
            # should not tear down the stream.
            try:
                msg = json.loads(raw)
            except Exception:
                return

            if not isinstance(msg, dict):
                return

            channel = msg.get("channel")
            if "error" in msg:
                # An error is co-located on the operation's own channel as an
                # `error`-keyed frame (a connection-level error uses the
                # dedicated `error` channel). Either way it is terminal for this
                # subscription: notify, then close so the worker thread winds
                # down.
                callback({"_error": msg.get("error")})
                ws.close()
            elif channel == "perpsEvents":
                # The data frame's payload is the bare batch (no GraphQL `data`
                # wrapper), delivered straight to the callback.
                callback(msg.get("data", {}) or {})
            # `subscriptionResponse` / `pong` carry no payload for the caller.

        def on_error(ws: websocket.WebSocketApp, error: Any) -> None:
            with contextlib.suppress(Exception):
                callback({"_error": str(error)})

        def on_close(ws: websocket.WebSocketApp, code: Any, reason: Any) -> None:
            stop_event.set()

        ws = websocket.WebSocketApp(
            self._ws_url,
            on_open=on_open,
            on_message=on_message,
            on_error=on_error,
            on_close=on_close,
        )

        # Record the socket so unsubscribe/stop can close it mid-stream.
        with self._lock:
            sub = self._subscriptions.get(sub_id)
            if sub is not None:
                sub.ws = ws

        # Frame-level keepalive: the server resets its idle timer on any inbound
        # frame, so an outbound ping (and the auto-pong to the server's pings)
        # keeps the connection alive; `ping_timeout` reaps a dead one.
        ws.run_forever(
            ping_interval=_PING_INTERVAL_SECONDS,
            ping_timeout=_PING_TIMEOUT_SECONDS,
        )

        stop_event.set()
        with self._lock:
            self._subscriptions.pop(sub_id, None)


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

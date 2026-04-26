"""Thread-based graphql-transport-ws subscription manager."""

from __future__ import annotations

import contextlib
import json
import threading
from collections.abc import Callable
from typing import Any, Final

import websocket  # from websocket-client

# Protocol-level keepalive interval. The server times out a connection that
# goes 30s without traffic; pinging every 15s gives a 2x margin and one free
# retry attempt before any drop. This is the *graphql-transport-ws* ping (a
# JSON text frame `{"type": "ping"}`), NOT the WebSocket frame-level ping —
# those are different layers and the server only resets its keepalive timer
# on protocol-level traffic.
_KEEPALIVE_INTERVAL_SECONDS: Final[float] = 15.0


class WebsocketManager(threading.Thread):
    """Owns one graphql-transport-ws connection plus subscription state."""

    def __init__(self, base_url: str) -> None:
        super().__init__(daemon=True)
        self._ws_url: str = _make_ws_url(base_url)
        self._ws: websocket.WebSocketApp | None = None

        # The single lock covers all the mutable state below. The dispatch
        # callbacks (on_open, on_message, ...) all run on the websocket-client
        # thread, while subscribe / unsubscribe / stop are called from the SDK
        # user's thread; one lock keeps the invariants between
        # `_subscriptions`, `_next_id`, and `_queued_subscribes` intact across
        # both call sites.
        self._lock: threading.Lock = threading.Lock()
        self._subscriptions: dict[int, Callable[[dict[str, Any]], None]] = {}
        self._next_id: int = 0
        self._queued_subscribes: list[tuple[int, str, dict[str, Any]]] = []

        self._ack_event: threading.Event = threading.Event()
        self._stop_event: threading.Event = threading.Event()
        self._keepalive_thread: threading.Thread | None = None

    def run(self) -> None:
        """threading.Thread entry point; blocks in run_forever() until stop()."""
        self._ws = websocket.WebSocketApp(
            self._ws_url,
            subprotocols=["graphql-transport-ws"],
            on_open=self._on_open,
            on_message=self._on_message,
            on_error=self._on_error,
            on_close=self._on_close,
        )
        # ping_interval=0 disables websocket-client's frame-level ping
        # scheduler (per the library, 0 means "do not auto-ping"). We drive
        # keepalive at the protocol layer in `_keepalive_loop` instead — the
        # server only resets its keepalive timer on protocol-level traffic,
        # so frame-level pings would be invisible to it and we'd still get
        # dropped after 30s of idle.
        self._ws.run_forever(ping_interval=0)

    def stop(self) -> None:
        """Signal shutdown and close the underlying WebSocket."""
        self._stop_event.set()
        if self._ws is not None:
            self._ws.close()

    def subscribe(
        self,
        document: str,
        variables: dict[str, Any],
        callback: Callable[[dict[str, Any]], None],
    ) -> int:
        """Register a subscription and return an int id usable with unsubscribe()."""
        with self._lock:
            self._next_id += 1
            sub_id = self._next_id
            self._subscriptions[sub_id] = callback
            if not self._ack_event.is_set():
                # Pre-ack: the server will reject `subscribe` frames before
                # `connection_ack`, so defer the wire send until `_on_ack`
                # flushes the queue.
                self._queued_subscribes.append((sub_id, document, variables))
                return sub_id

        # Post-ack send happens outside the lock: ws.send() is internally
        # queued and thread-safe per the websocket-client docs, and we don't
        # want to hold the lock across a blocking I/O call.
        self._send_subscribe(sub_id, document, variables)
        return sub_id

    def unsubscribe(self, subscription_id: int) -> bool:
        """Drop a subscription locally and tell the server to stop streaming."""
        with self._lock:
            if subscription_id not in self._subscriptions:
                return False
            del self._subscriptions[subscription_id]

        if self._ws is not None:
            # Best effort: if the socket is already gone, the server will drop
            # us anyway, so a missed `complete` is harmless. Suppressing
            # `Exception` rather than the specific websocket exceptions keeps
            # us safe across websocket-client minor version changes.
            with contextlib.suppress(Exception):
                self._ws.send(json.dumps({"type": "complete", "id": str(subscription_id)}))
        return True

    # --- internals -----------------------------------------------------------

    def _on_open(self, ws: websocket.WebSocketApp) -> None:
        # Per the spec, every connection starts with `connection_init` from
        # the client. Subscriptions are not allowed until the server replies
        # with `connection_ack`.
        ws.send(json.dumps({"type": "connection_init"}))

    def _on_message(self, ws: websocket.WebSocketApp, raw: Any) -> None:
        # `raw` is typed `Any` because websocket-client's stub uses Any here;
        # in practice it's a `str` for text frames. We catch broad `Exception`
        # to cover both `JSONDecodeError` (malformed JSON) and the `TypeError`
        # that `json.loads` raises on an unexpected non-str/bytes input —
        # either way, a single bad frame should not tear down healthy
        # subscriptions.
        try:
            msg = json.loads(raw)
        except Exception:
            return

        if not isinstance(msg, dict):
            return

        match msg.get("type"):
            case "connection_ack":
                self._on_ack(ws)
            case "next":
                self._dispatch_next(msg)
            case "error":
                self._dispatch_error(msg)
            case "complete":
                self._dispatch_complete(msg)
            case "ping":
                # Protocol-level ping from the server; mirror with pong. This
                # is in addition to our own outbound 15s pings.
                ws.send(json.dumps({"type": "pong"}))
            case "pong":
                # Server ack of our ping; nothing to do.
                pass
            case _:
                # Unknown message types are forward-compatible — ignore.
                pass

    def _on_ack(self, ws: websocket.WebSocketApp) -> None:
        self._ack_event.set()
        with self._lock:
            queued = list(self._queued_subscribes)
            self._queued_subscribes.clear()
        for sub_id, document, variables in queued:
            self._send_subscribe(sub_id, document, variables)
        # Spawn the keepalive thread now (not at on_open) — the server only
        # starts counting against the keepalive timer once we're ack'd, and
        # pinging during the handshake would race with the server's reply.
        self._keepalive_thread = threading.Thread(
            target=self._keepalive_loop,
            name="dango-ws-keepalive",
            daemon=True,
        )
        self._keepalive_thread.start()

    def _send_subscribe(
        self,
        sub_id: int,
        document: str,
        variables: dict[str, Any],
    ) -> None:
        if self._ws is None:
            return
        self._ws.send(
            json.dumps(
                {
                    "id": str(sub_id),
                    "type": "subscribe",
                    "payload": {"query": document, "variables": variables},
                }
            )
        )

    def _dispatch_next(self, msg: dict[str, Any]) -> None:
        sub_id = _parse_id(msg.get("id"))
        if sub_id is None:
            return
        with self._lock:
            cb = self._subscriptions.get(sub_id)
        if cb is not None:
            cb(msg.get("payload", {}) or {})

    def _dispatch_error(self, msg: dict[str, Any]) -> None:
        sub_id = _parse_id(msg.get("id"))
        if sub_id is None:
            return
        with self._lock:
            # `error` is terminal for this subscription per spec; drop the
            # callback so the user sees exactly one error notification and
            # any late `next` (shouldn't happen, but defensive) is silently
            # ignored rather than delivered after the failure.
            cb = self._subscriptions.pop(sub_id, None)
        if cb is not None:
            cb({"_error": msg.get("payload")})

    def _dispatch_complete(self, msg: dict[str, Any]) -> None:
        sub_id = _parse_id(msg.get("id"))
        if sub_id is None:
            return
        with self._lock:
            self._subscriptions.pop(sub_id, None)

    def _keepalive_loop(self) -> None:
        # Use Event.wait(timeout=...) instead of time.sleep so stop() can
        # interrupt us promptly; the bool return tells us whether the event
        # fired (stop signaled) vs the timeout elapsed normally.
        while not self._stop_event.wait(timeout=_KEEPALIVE_INTERVAL_SECONDS):
            if self._ws is None:
                return
            try:
                self._ws.send(json.dumps({"type": "ping"}))
            except Exception:
                # Connection broken; let on_close / on_error handle teardown.
                return

    def _on_error(self, ws: websocket.WebSocketApp, error: Any) -> None:
        # Notify every active subscription so callers see a final event,
        # then clear the dict so we don't double-notify on close.
        with self._lock:
            callbacks = list(self._subscriptions.values())
            self._subscriptions.clear()
        for cb in callbacks:
            # Swallow callback exceptions — one bad callback shouldn't
            # prevent us from notifying the others.
            with contextlib.suppress(Exception):
                cb({"_error": str(error)})

    def _on_close(
        self,
        ws: websocket.WebSocketApp,
        code: Any,
        reason: Any,
    ) -> None:
        # Wakes up the keepalive loop and any user code blocking on stop().
        self._stop_event.set()


def _make_ws_url(base_url: str) -> str:
    """Convert an http(s) base_url to the matching ws(s)://...graphql endpoint."""
    if base_url.startswith("https://"):
        endpoint = "wss://" + base_url[len("https://") :]
    elif base_url.startswith("http://"):
        endpoint = "ws://" + base_url[len("http://") :]
    elif base_url.startswith(("wss://", "ws://")):
        endpoint = base_url
    else:
        raise ValueError(f"unsupported base_url scheme: {base_url!r}")
    endpoint = endpoint.rstrip("/")
    if not endpoint.endswith("/graphql"):
        endpoint = endpoint + "/graphql"
    return endpoint


def _parse_id(raw: object) -> int | None:
    """Parse a subscription id from a server message; None if missing/malformed."""
    if not isinstance(raw, str):
        return None
    try:
        return int(raw)
    except ValueError:
        return None

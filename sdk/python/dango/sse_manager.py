"""Thread-based Server-Sent Events (SSE) subscription manager.

The REST/SSE counterpart to `WebsocketManager`: where that multiplexes many
graphql-transport-ws subscriptions over one socket, each SSE subscription here
is its own long-lived HTTP `GET` (`text/event-stream`) on its own daemon
thread. Used by `Info.subscribe_perps_events2`.

Each decoded event's JSON is delivered to the subscription's callback. A
non-200 response or a connection/read error is delivered as `{"_error": ...}`,
the same envelope convention `WebsocketManager` uses, so callers handle errors
uniformly across transports.
"""

from __future__ import annotations

import contextlib
import json
import threading
from collections.abc import Callable
from typing import Any, Final

import requests

# Timeout for opening the stream.
_CONNECT_TIMEOUT_SECONDS: Final[float] = 10.0

# Read timeout. The stream is long-lived, but the server emits a keep-alive
# comment about every 15s, so a 30s (2x) read budget never trips during normal
# operation while still surfacing a silently-dead connection as an error rather
# than hanging forever. `unsubscribe`/`stop` close the response to interrupt a
# parked read immediately.
_READ_TIMEOUT_SECONDS: Final[float] = 30.0


class _Subscription:
    """One in-flight SSE stream: its worker thread, stop flag, and response."""

    def __init__(self, thread: threading.Thread, stop_event: threading.Event) -> None:
        self.thread: threading.Thread = thread
        self.stop_event: threading.Event = stop_event
        self.response: requests.Response | None = None


class SseManager:
    """Owns the per-subscription SSE worker threads for one base URL."""

    def __init__(self, base_url: str) -> None:
        self._base_url: str = base_url.rstrip("/")

        # Guards `_next_id` and `_subscriptions`, which are touched from both
        # the SDK user's thread (subscribe/unsubscribe/stop) and each worker
        # thread (recording its response, cleaning up on exit).
        self._lock: threading.Lock = threading.Lock()
        self._next_id: int = 0
        self._subscriptions: dict[int, _Subscription] = {}

    def subscribe(
        self,
        path: str,
        params: dict[str, str],
        callback: Callable[[dict[str, Any]], None],
    ) -> int:
        """Open an SSE stream at `<base_url>/<path>?<params>`; return an int id."""

        with self._lock:
            self._next_id += 1
            sub_id = self._next_id
            stop_event = threading.Event()
            thread = threading.Thread(
                target=self._run,
                args=(sub_id, path, params, callback, stop_event),
                name=f"dango-sse-{sub_id}",
                daemon=True,
            )
            self._subscriptions[sub_id] = _Subscription(thread, stop_event)

        thread.start()
        return sub_id

    def unsubscribe(self, subscription_id: int) -> bool:
        """Stop one SSE stream. Returns False if the id is unknown."""

        with self._lock:
            sub = self._subscriptions.pop(subscription_id, None)

        if sub is None:
            return False

        self._close(sub)
        return True

    def stop(self) -> None:
        """Stop every SSE stream."""

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
        # Closing the response unblocks a worker parked in `iter_lines`.
        if sub.response is not None:
            with contextlib.suppress(Exception):
                sub.response.close()

    def _run(
        self,
        sub_id: int,
        path: str,
        params: dict[str, str],
        callback: Callable[[dict[str, Any]], None],
        stop_event: threading.Event,
    ) -> None:
        url = f"{self._base_url}/{path.lstrip('/')}"

        try:
            response = requests.get(
                url,
                params=params,
                stream=True,
                timeout=(_CONNECT_TIMEOUT_SECONDS, _READ_TIMEOUT_SECONDS),
                headers={"Accept": "text/event-stream"},
            )
        except Exception as exc:
            callback({"_error": str(exc)})
            with self._lock:
                self._subscriptions.pop(sub_id, None)
            return

        # Record the response so unsubscribe/stop can close it mid-read.
        with self._lock:
            sub = self._subscriptions.get(sub_id)
            if sub is not None:
                sub.response = response

        try:
            with response:
                if response.status_code != 200:
                    # Resync (409) / limit (429) / other: deliver the body as an
                    # error envelope, mirroring the WS error contract.
                    callback({"_error": {"status": response.status_code, "message": response.text}})
                    return

                data_lines: list[str] = []
                for raw in response.iter_lines(decode_unicode=True):
                    if stop_event.is_set():
                        return

                    line = raw if isinstance(raw, str) else raw.decode("utf-8")

                    if line == "":
                        # Blank line: dispatch the accumulated event, if any.
                        if data_lines:
                            self._dispatch(data_lines, callback)
                            data_lines = []
                    elif line.startswith(":"):
                        # Comment (keep-alive) — ignore.
                        continue
                    elif line.startswith("data:"):
                        value = line[len("data:") :]
                        if value.startswith(" "):
                            value = value[1:]
                        data_lines.append(value)
                    # `id:` / `event:` / other fields are not needed client-side.

                # The stream closed; flush a trailing event with no final blank.
                if data_lines and not stop_event.is_set():
                    self._dispatch(data_lines, callback)
        except Exception as exc:
            # A close triggered by unsubscribe/stop surfaces here too; only
            # report when we weren't asked to stop.
            if not stop_event.is_set():
                callback({"_error": str(exc)})
        finally:
            with self._lock:
                self._subscriptions.pop(sub_id, None)

    @staticmethod
    def _dispatch(
        data_lines: list[str],
        callback: Callable[[dict[str, Any]], None],
    ) -> None:
        payload = "\n".join(data_lines)
        try:
            batch = json.loads(payload)
        except Exception:
            callback({"_error": f"malformed SSE data: {payload!r}"})
            return

        callback(batch)

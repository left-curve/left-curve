"""Tests for dango.websocket_manager.WebsocketManager."""

from __future__ import annotations

import json
import threading
from typing import Any

import pytest

import dango.websocket_manager as wsm_module
from dango.websocket_manager import WebsocketManager, _make_ws_url


class _FakeWebSocket:
    """A drop-in for websocket.WebSocketApp that captures sent messages."""

    def __init__(self) -> None:
        self.sent: list[dict[str, Any]] = []
        self.closed: bool = False

    def send(self, raw: str) -> None:
        self.sent.append(json.loads(raw))

    def close(self) -> None:
        self.closed = True


def _new_mgr() -> tuple[WebsocketManager, _FakeWebSocket]:
    """Construct a manager + inject a fake WS without starting the thread."""
    mgr = WebsocketManager("http://localhost:1234")
    fake = _FakeWebSocket()
    # White-box injection: tests drive the dispatch callbacks directly so we
    # never spin up a real network thread or websocket-client run loop.
    mgr._ws = fake  # type: ignore[assignment]
    return mgr, fake


class TestUrlTransformation:
    def test_https_becomes_wss(self) -> None:
        """https:// URLs are upgraded to wss:// and given a /graphql suffix."""
        assert _make_ws_url("https://example.com") == "wss://example.com/graphql"

    def test_http_becomes_ws(self) -> None:
        """http:// URLs become ws:// (insecure)."""
        assert _make_ws_url("http://localhost:8080") == "ws://localhost:8080/graphql"

    def test_passes_through_wss(self) -> None:
        """Already-wss URLs are accepted and just get the suffix."""
        assert _make_ws_url("wss://example.com") == "wss://example.com/graphql"

    def test_passes_through_ws(self) -> None:
        """Already-ws URLs are accepted and just get the suffix."""
        assert _make_ws_url("ws://localhost:8080") == "ws://localhost:8080/graphql"

    def test_strips_trailing_slash_before_graphql(self) -> None:
        """A trailing slash on base_url doesn't produce a //graphql endpoint."""
        assert _make_ws_url("https://example.com/") == "wss://example.com/graphql"

    def test_does_not_double_append_graphql(self) -> None:
        """Already-/graphql URLs aren't suffixed twice."""
        assert _make_ws_url("https://example.com/graphql") == "wss://example.com/graphql"

    def test_invalid_scheme_raises(self) -> None:
        """A non-http(s)/ws(s) scheme raises ValueError."""
        with pytest.raises(ValueError, match="scheme"):
            _make_ws_url("ftp://example.com")


class TestConnectionLifecycle:
    def test_on_open_sends_connection_init(self) -> None:
        """on_open sends `{"type": "connection_init"}` immediately."""
        mgr, fake = _new_mgr()
        mgr._on_open(fake)  # type: ignore[arg-type]
        assert fake.sent == [{"type": "connection_init"}]

    def test_subscribe_before_ack_queues(self) -> None:
        """A subscribe before connection_ack is queued, not sent."""
        mgr, fake = _new_mgr()
        mgr._on_open(fake)  # type: ignore[arg-type]
        sub_id = mgr.subscribe("query { foo }", {}, lambda _: None)
        assert sub_id == 1
        # Only the connection_init has been sent so far — the subscribe is
        # parked in `_queued_subscribes` waiting for the ack.
        assert [m["type"] for m in fake.sent] == ["connection_init"]

    def test_ack_flushes_queued_subscribes(self) -> None:
        """connection_ack triggers the queued subscribe to be sent."""
        mgr, fake = _new_mgr()
        mgr._on_open(fake)  # type: ignore[arg-type]
        mgr.subscribe("query { foo }", {"x": 1}, lambda _: None)
        mgr._on_message(fake, json.dumps({"type": "connection_ack"}))  # type: ignore[arg-type]
        # After ack, the queued subscribe is on the wire.
        types = [m["type"] for m in fake.sent]
        assert types == ["connection_init", "subscribe"]
        sub_msg = fake.sent[1]
        assert sub_msg["id"] == "1"
        assert sub_msg["payload"] == {"query": "query { foo }", "variables": {"x": 1}}
        # Stop the keepalive thread that _on_ack spawned so it doesn't leak
        # into other tests.
        mgr.stop()

    def test_subscribe_after_ack_sends_immediately(self) -> None:
        """A subscribe after ack is sent right away, not queued."""
        mgr, fake = _new_mgr()
        # Simulate ack already received without going through _on_ack (which
        # would also start the keepalive thread we don't need here).
        mgr._ack_event.set()
        mgr.subscribe("query { foo }", {}, lambda _: None)
        assert [m["type"] for m in fake.sent] == ["subscribe"]


class TestDispatch:
    def test_next_routes_to_callback(self) -> None:
        """A `next` message is delivered to the matching subscription's callback."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received: list[dict[str, Any]] = []
        sub_id = mgr.subscribe("q", {}, received.append)
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "next", "id": str(sub_id), "payload": {"data": {"x": 1}}}),
        )
        assert received == [{"data": {"x": 1}}]

    def test_error_removes_callback_and_invokes_with_wrapper(self) -> None:
        """An `error` message wraps the payload and is the final invocation."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received: list[dict[str, Any]] = []
        sub_id = mgr.subscribe("q", {}, received.append)
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "error", "id": str(sub_id), "payload": [{"message": "boom"}]}),
        )
        assert received == [{"_error": [{"message": "boom"}]}]
        # The subscription is dead — no further messages can be delivered.
        assert sub_id not in mgr._subscriptions

    def test_complete_removes_callback_silently(self) -> None:
        """A `complete` message removes the callback without any notification."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received: list[dict[str, Any]] = []
        sub_id = mgr.subscribe("q", {}, received.append)
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "complete", "id": str(sub_id)}),
        )
        assert received == []
        assert sub_id not in mgr._subscriptions

    def test_server_ping_replied_with_pong(self) -> None:
        """An inbound `ping` is acked with `pong`."""
        mgr, fake = _new_mgr()
        mgr._on_message(fake, json.dumps({"type": "ping"}))  # type: ignore[arg-type]
        assert {"type": "pong"} in fake.sent

    def test_server_pong_ignored(self) -> None:
        """An inbound `pong` is silently ignored (just an ack of our ping)."""
        mgr, fake = _new_mgr()
        mgr._on_message(fake, json.dumps({"type": "pong"}))  # type: ignore[arg-type]
        assert fake.sent == []

    def test_unknown_type_ignored(self) -> None:
        """Unknown message types are forward-compatible — silently dropped."""
        mgr, fake = _new_mgr()
        mgr._on_message(fake, json.dumps({"type": "future_thing"}))  # type: ignore[arg-type]
        assert fake.sent == []

    def test_malformed_json_ignored(self) -> None:
        """Bad JSON does not raise; healthy subscriptions keep working."""
        mgr, fake = _new_mgr()
        mgr._on_message(fake, "{not json")  # type: ignore[arg-type]
        # No exception, no sends.
        assert fake.sent == []

    def test_non_object_json_ignored(self) -> None:
        """A JSON array or scalar is not a valid message envelope; drop it."""
        mgr, fake = _new_mgr()
        mgr._on_message(fake, json.dumps([1, 2, 3]))  # type: ignore[arg-type]
        assert fake.sent == []

    def test_unknown_subscription_id_ignored(self) -> None:
        """A `next` for a missing id is silently dropped."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        # No subscriptions registered — dispatch is a no-op.
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "next", "id": "9999", "payload": {}}),
        )
        # Dispatch did nothing; no exception, no sends.
        assert fake.sent == []

    def test_malformed_id_ignored(self) -> None:
        """A non-integer id string in a server message is silently dropped."""
        mgr, fake = _new_mgr()
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "next", "id": "not-an-int", "payload": {}}),
        )
        assert fake.sent == []


class TestUnsubscribe:
    def test_returns_false_for_unknown_id(self) -> None:
        """unsubscribe() of an unknown id returns False."""
        mgr, _ = _new_mgr()
        assert mgr.unsubscribe(42) is False

    def test_returns_true_and_sends_complete(self) -> None:
        """unsubscribe() drops the callback locally and tells the server to stop."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        sub_id = mgr.subscribe("q", {}, lambda _: None)
        assert mgr.unsubscribe(sub_id) is True
        # subscribe + complete on the wire.
        types = [m["type"] for m in fake.sent]
        assert "complete" in types
        complete_msg = next(m for m in fake.sent if m["type"] == "complete")
        assert complete_msg["id"] == str(sub_id)

    def test_unsubscribe_removes_callback(self) -> None:
        """After unsubscribe, late `next` messages don't invoke the callback."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received: list[dict[str, Any]] = []
        sub_id = mgr.subscribe("q", {}, received.append)
        mgr.unsubscribe(sub_id)
        # Server may still emit a `next` before its `complete` ack reaches us.
        mgr._on_message(
            fake,  # type: ignore[arg-type]
            json.dumps({"type": "next", "id": str(sub_id), "payload": {"x": 1}}),
        )
        assert received == []


class TestKeepalive:
    def test_keepalive_pings_at_interval(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """The keepalive thread emits `{"type": "ping"}` at the configured interval."""
        # Shorten the interval drastically so the test is fast but real — we
        # want to assert the thread really pings, not just stub it out.
        monkeypatch.setattr(wsm_module, "_KEEPALIVE_INTERVAL_SECONDS", 0.05)
        mgr, fake = _new_mgr()
        mgr._on_ack(fake)  # type: ignore[arg-type]  # starts the keepalive thread
        # Wait long enough for at least 2 pings to fire.
        threading.Event().wait(timeout=0.18)
        mgr.stop()
        ping_count = sum(1 for m in fake.sent if m == {"type": "ping"})
        assert ping_count >= 2

    def test_keepalive_stops_on_stop(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """The keepalive loop terminates promptly when stop() is called."""
        # 1.0s is much longer than the join() timeout below, so the test only
        # passes if Event.wait() actually wakes up on the stop event rather
        # than waiting out the full interval.
        monkeypatch.setattr(wsm_module, "_KEEPALIVE_INTERVAL_SECONDS", 1.0)
        mgr, fake = _new_mgr()
        mgr._on_ack(fake)  # type: ignore[arg-type]
        mgr.stop()
        # Give the thread a moment to notice the stop event.
        if mgr._keepalive_thread is not None:
            mgr._keepalive_thread.join(timeout=0.5)
            assert not mgr._keepalive_thread.is_alive()


class TestErrorAndClose:
    def test_on_error_notifies_all_subscriptions(self) -> None:
        """on_error fans out a final notification to every active callback."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received_a: list[dict[str, Any]] = []
        received_b: list[dict[str, Any]] = []
        mgr.subscribe("q1", {}, received_a.append)
        mgr.subscribe("q2", {}, received_b.append)
        mgr._on_error(fake, RuntimeError("connection lost"))  # type: ignore[arg-type]
        assert received_a == [{"_error": "connection lost"}]
        assert received_b == [{"_error": "connection lost"}]

    def test_on_error_clears_subscriptions(self) -> None:
        """After on_error, the subscriptions dict is empty so close doesn't double-notify."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        mgr.subscribe("q", {}, lambda _: None)
        mgr._on_error(fake, RuntimeError("boom"))  # type: ignore[arg-type]
        assert mgr._subscriptions == {}

    def test_on_error_swallows_callback_exceptions(self) -> None:
        """A throwing callback doesn't prevent siblings from being notified."""
        mgr, fake = _new_mgr()
        mgr._ack_event.set()
        received: list[dict[str, Any]] = []

        def bad_cb(_: dict[str, Any]) -> None:
            raise RuntimeError("callback boom")

        mgr.subscribe("q1", {}, bad_cb)
        mgr.subscribe("q2", {}, received.append)
        # Should not raise even though bad_cb does.
        mgr._on_error(fake, RuntimeError("connection lost"))  # type: ignore[arg-type]
        # The healthy callback still received its notification.
        assert received == [{"_error": "connection lost"}]

    def test_on_close_sets_stop_event(self) -> None:
        """on_close signals shutdown so the keepalive loop wakes up."""
        mgr, fake = _new_mgr()
        mgr._on_close(fake, 1000, "ok")  # type: ignore[arg-type]
        assert mgr._stop_event.is_set()


class TestStop:
    def test_stop_closes_ws_and_sets_event(self) -> None:
        """stop() closes the underlying WebSocket and signals the manager."""
        mgr, fake = _new_mgr()
        mgr.stop()
        assert fake.closed is True
        assert mgr._stop_event.is_set()

    def test_stop_without_ws_is_safe(self) -> None:
        """stop() before run() (no ws yet) only sets the event; no exception."""
        mgr = WebsocketManager("http://localhost:1234")
        # No ws assigned — stop() must not crash.
        mgr.stop()
        assert mgr._stop_event.is_set()


class TestIdAllocation:
    def test_sequential_ids(self) -> None:
        """Subscription ids are allocated sequentially starting at 1."""
        mgr, _ = _new_mgr()
        mgr._ack_event.set()
        ids = [mgr.subscribe("q", {}, lambda _: None) for _ in range(3)]
        assert ids == [1, 2, 3]

    def test_unsubscribe_does_not_recycle_ids(self) -> None:
        """Unsubscribing a subscription does not free up its id for reuse."""
        # Defensive guarantee: if ids were recycled, a stale `next` for the
        # old subscription could land on a fresh callback.
        mgr, _ = _new_mgr()
        mgr._ack_event.set()
        first = mgr.subscribe("q", {}, lambda _: None)
        mgr.unsubscribe(first)
        second = mgr.subscribe("q", {}, lambda _: None)
        assert second == first + 1

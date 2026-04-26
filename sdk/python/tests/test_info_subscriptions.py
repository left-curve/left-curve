"""Tests for dango.info Info subscription methods."""

from __future__ import annotations

from typing import Any

import pytest

from dango.info import Info
from dango.utils.types import Addr, CandleInterval, PairId


class _FakeWebsocketManager:
    """Captures subscribe/unsubscribe/stop calls without doing real I/O."""

    def __init__(self) -> None:
        # Each entry is (document, variables, callback) so tests can both
        # assert on what got sent AND trigger the callback as if a `next`
        # message had arrived from the server.
        self.subscriptions: list[tuple[str, dict[str, Any], Any]] = []
        self.unsubscribed: list[int] = []
        self.started: bool = False
        self.stopped: bool = False
        self._next_id: int = 0

    def start(self) -> None:
        self.started = True

    def stop(self) -> None:
        self.stopped = True

    def join(self, timeout: float | None = None) -> None:
        # No-op: the fake manager has no thread to join. Accepting `timeout`
        # keeps the signature compatible with `threading.Thread.join`.
        pass

    def subscribe(
        self,
        document: str,
        variables: dict[str, Any],
        callback: Any,
    ) -> int:
        self._next_id += 1
        self.subscriptions.append((document, variables, callback))
        return self._next_id

    def unsubscribe(self, subscription_id: int) -> bool:
        self.unsubscribed.append(subscription_id)
        return True


def _make_info_with_fake_ws() -> tuple[Info, _FakeWebsocketManager]:
    """Build an Info and inject a fake WebsocketManager into its slot."""
    # `skip_ws=False` is the default but spelled out here so the test
    # reads as "subscriptions are enabled, but with a fake transport".
    info = Info("http://localhost:8080", skip_ws=False)
    fake = _FakeWebsocketManager()
    info._ws_manager = fake  # type: ignore[assignment]
    return info, fake


class TestLazyWsConstruction:
    def test_skip_ws_raises_on_subscribe(self) -> None:
        """skip_ws=True turns subscribe_* into a hard error, not a silent no-op."""
        info = Info("http://localhost:8080", skip_ws=True)
        with pytest.raises(RuntimeError, match="skip_ws=True"):
            info.subscribe_block(lambda _: None)

    def test_disconnect_without_connect_is_noop(self) -> None:
        """disconnect_websocket() before any subscribe doesn't crash."""
        info = Info("http://localhost:8080", skip_ws=False)
        # No assertion needed — the call must simply not raise.
        info.disconnect_websocket()


class TestSubscribePerpsTrades:
    def test_passes_pair_id_variable(self) -> None:
        """subscribe_perps_trades posts pairId as a string variable."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_perps_trades(PairId("perp/btcusd"), lambda _: None)
        doc, variables, _cb = fake.subscriptions[-1]
        # Document must be the perpsTrades subscription, not e.g. candles.
        assert "perpsTrades" in doc
        assert variables == {"pairId": "perp/btcusd"}

    def test_callback_receives_unwrapped_trade(self) -> None:
        """The callback gets the inner Trade object, not the GraphQL wrapper."""
        info, fake = _make_info_with_fake_ws()
        received: list[Any] = []
        info.subscribe_perps_trades(PairId("perp/btcusd"), received.append)
        _doc, _vars, cb = fake.subscriptions[-1]
        # Simulate the WS dispatching a `next` payload — the user's
        # callback should see the inner Trade dict, not the wrapper.
        cb({"data": {"perpsTrades": {"orderId": "1", "fillPrice": "1.0"}}})
        assert received == [{"orderId": "1", "fillPrice": "1.0"}]

    def test_callback_forwards_error_envelope(self) -> None:
        """Server errors flow through to the user as {"_error": payload}."""
        info, fake = _make_info_with_fake_ws()
        received: list[Any] = []
        info.subscribe_perps_trades(PairId("perp/btcusd"), received.append)
        _doc, _vars, cb = fake.subscriptions[-1]
        cb({"_error": "boom"})
        # Pass-through, not unwrapping — the user handles errors uniformly.
        assert received == [{"_error": "boom"}]


class TestSubscribePerpsCandles:
    def test_passes_pair_id_and_interval(self) -> None:
        """subscribe_perps_candles uses interval.value (string), not the enum."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_perps_candles(
            PairId("perp/ethusd"),
            CandleInterval.ONE_MINUTE,
            lambda _: None,
        )
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables["pairId"] == "perp/ethusd"
        # The StrEnum value, not the StrEnum object — see the comment
        # in `subscribe_perps_candles` about GraphQL enum encoding.
        assert variables["interval"] == "ONE_MINUTE"
        assert variables["laterThan"] is None


class TestSubscribeQueryApp:
    def test_default_block_interval_is_10(self) -> None:
        """block_interval defaults to 10 blocks (~10s at Dango block time)."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_query_app({"config": {}}, lambda _: None)
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables["blockInterval"] == 10

    def test_explicit_block_interval(self) -> None:
        """Caller can override block_interval to any positive int."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_query_app({"config": {}}, lambda _: None, block_interval=5)
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables["blockInterval"] == 5

    def test_passes_request_through(self) -> None:
        """The arbitrary `request` dict is forwarded as-is."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_query_app({"wasm_smart": {"contract": "0x1", "msg": {}}}, lambda _: None)
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables["request"] == {"wasm_smart": {"contract": "0x1", "msg": {}}}


class TestSubscribeUserEvents:
    def test_user_only_filter(self) -> None:
        """Without event_types, the filter has one entry pinning data.user."""
        info, fake = _make_info_with_fake_ws()
        addr = Addr("0x000000000000000000000000000000000000beef")
        info.subscribe_user_events(addr, lambda _: None)
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables["filter"] == [
            {"data": [{"path": ["user"], "checkMode": "EQUAL", "value": [addr]}]}
        ]

    def test_user_plus_event_types_filter(self) -> None:
        """With event_types, each type gets a filter entry sharing the user constraint."""
        info, fake = _make_info_with_fake_ws()
        addr = Addr("0x000000000000000000000000000000000000beef")
        info.subscribe_user_events(addr, lambda _: None, event_types=["order_filled", "deposited"])
        _doc, variables, _cb = fake.subscriptions[-1]
        # Two filter entries — server intersects per-entry conditions but
        # treats per-type entries as the union of types.
        assert len(variables["filter"]) == 2
        assert variables["filter"][0]["type"] == "order_filled"
        assert variables["filter"][1]["type"] == "deposited"
        # Both entries share the same user constraint.
        for f in variables["filter"]:
            assert f["data"] == [{"path": ["user"], "checkMode": "EQUAL", "value": [addr]}]


class TestSubscribeBlock:
    def test_no_variables(self) -> None:
        """subscribe_block has no variables — every block streams."""
        info, fake = _make_info_with_fake_ws()
        info.subscribe_block(lambda _: None)
        _doc, variables, _cb = fake.subscriptions[-1]
        assert variables == {}


class TestUnsubscribeAndDisconnect:
    def test_unsubscribe_returns_false_if_not_connected(self) -> None:
        """unsubscribe before any subscribe is a no-op returning False."""
        info = Info("http://localhost:8080", skip_ws=False)
        # No manager exists yet because no subscribe_* was ever called.
        assert info.unsubscribe(1) is False

    def test_unsubscribe_forwards_to_manager(self) -> None:
        """unsubscribe forwards the id to the WebsocketManager."""
        info, fake = _make_info_with_fake_ws()
        sub_id = info.subscribe_block(lambda _: None)
        assert info.unsubscribe(sub_id) is True
        assert fake.unsubscribed == [sub_id]

    def test_disconnect_stops_manager_and_clears_reference(self) -> None:
        """disconnect_websocket stops the manager and lets a new one be created."""
        info, fake = _make_info_with_fake_ws()
        info.disconnect_websocket()
        assert fake.stopped is True
        # Reference cleared so a future `_ws` access would create a fresh
        # manager rather than reusing the stopped one.
        assert info._ws_manager is None

    def test_disconnect_then_subscribe_creates_fresh_manager(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """After disconnect, a new subscribe spins up a fresh WebsocketManager."""
        # Pin the lazy-property branch: we need to verify _ws builds a NEW
        # manager (not reuses the stopped one) after disconnect_websocket.
        info, first = _make_info_with_fake_ws()
        info.disconnect_websocket()
        assert info._ws_manager is None

        # Patch WebsocketManager so the lazy property gets a fake on next access.
        from dango import info as info_module

        created: list[_FakeWebsocketManager] = []

        def _factory(_url: str) -> _FakeWebsocketManager:
            fresh = _FakeWebsocketManager()
            created.append(fresh)
            return fresh

        monkeypatch.setattr(info_module, "WebsocketManager", _factory)
        info.subscribe_block(lambda _: None)
        assert len(created) == 1
        assert created[0] is not first
        assert created[0].started is True

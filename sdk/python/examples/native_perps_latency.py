"""Native Dango API: measure end-to-end perps trading round-trip latency.

Some algo traders report high latency trading on Dango. This script measures it
directly, with no fills: it repeatedly places a resting limit order 1% away from
the index price (so it never crosses the book) and cancels it, timing each
action two ways.

In every cycle the timer starts the instant before the broadcast call. We then
record two latencies per action, both anchored at that same start:

* ``mempool`` — when the broadcast call returns. ``submit_limit_order`` /
  ``cancel_order`` resolve through ``broadcast_tx_sync``, which returns once the
  node admits the tx to its mempool. We pass an explicit ``gas_limit`` so the
  SDK skips its pre-broadcast ``simulate`` round-trip; this figure is then just
  signing plus the broadcast hop.
* ``confirm`` — when the matching lifecycle event (``order_persisted`` for the
  place, ``order_removed`` for the cancel) arrives back over the
  ``perps_events2`` WebSocket subscription. This is the full client-observed
  round trip: broadcast -> block inclusion -> indexer -> push back to us.

So ``confirm >= mempool`` always, since both share the same start anchor.

Uses the native Dango API (not the Hyperliquid-compat shim) and the same
``example_utils.setup`` procedure as the other native mutation examples: it reads
``examples/.env`` for ``DANGO_SECRET_KEY`` / ``DANGO_ACCOUNT_ADDRESS`` and refuses
to run if the account has no margin. The account must be funded on the target
network (testnet by default).

Run with::

    uv run python examples/native_perps_latency.py
"""

from __future__ import annotations

import math
import statistics
import threading
import time
from collections.abc import Callable
from decimal import Decimal
from typing import TYPE_CHECKING, Any, cast

import example_utils

from dango.utils.constants import PERPS_CONTRACT_TESTNET, TESTNET_API_URL
from dango.utils.types import (
    Addr,
    ClientOrderIdRef,
    PairId,
    PerpsEvent2Batch,
    TimeInForce,
)

if TYPE_CHECKING:
    from dango.info import Info

# --- Configuration -----------------------------------------------------------

# Target network. Testnet by default. To measure mainnet latency instead, import
# `MAINNET_API_URL` / `PERPS_CONTRACT_MAINNET` from `dango.utils.constants` and
# swap them in here (and make sure the account is funded on mainnet).
API_URL = TESTNET_API_URL
PERPS_CONTRACT = PERPS_CONTRACT_TESTNET

# Market and order. A BUY (positive, signed size) resting 1% BELOW the index
# never crosses the book, so it stays unfilled until we cancel it. Keep the size
# small; if the chain rejects placement on a minimum-size / notional rule,
# increase it.
PAIR_ID = PairId("perp/btcusd")
SIZE = "0.001"
INDEX_PRICE_FACTOR = Decimal("0.99")  # limit_price = index_price * 0.99 -> 1% below index

# Explicit gas limit for the order txs. Passing this makes the SDK skip its
# pre-broadcast `simulate` round-trip and use the value directly — removing a
# full query from the timed path, which is the whole point when measuring
# latency. Must be high enough to cover the tx, else the chain rejects it for
# running out of gas.
GAS_LIMIT = 200_000

ITERATIONS = 30  # number of place + cancel cycles to sample
EVENT_TIMEOUT_S = 30.0  # max wait for a lifecycle event before bailing on a cycle
SETTLE_S = 1.0  # let the subscription go live before the first order
PAUSE_BETWEEN_S = 0.25  # brief breather between cycles

# Lifecycle events we correlate against: `order_persisted` confirms the place
# landed in the book; `order_removed` confirms the cancel took effect.
_AWAITED_EVENT_TYPES = ["order_persisted", "order_removed"]

# Metric keys, in display order.
_METRICS = ("place_mempool", "place_confirm", "cancel_mempool", "cancel_confirm")

# Set to False to silence the step-by-step diagnostic logging.
VERBOSE = True


def _diag(message: str) -> None:
    """Print a step-by-step diagnostic line when VERBOSE is on."""
    if VERBOSE:
        print(f"[diag] {message}")


# --- Event correlation -------------------------------------------------------


class _EventAwaiter:
    """Thread-safe rendezvous between the WS reader thread and the main thread.

    The ``perps_events2`` callback runs on the WebSocket reader thread. Before
    each broadcast, the main thread arms the awaiter with the
    ``(event_type, client_order_id)`` it expects, then blocks in ``wait()``. When
    a matching event arrives, the callback timestamps it with ``perf_counter()``
    — capturing arrival as early as possible, before any main-thread scheduling
    jitter — and wakes ``wait()``.
    """

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._signal = threading.Event()
        self._want_type: str | None = None
        self._want_coid: str | None = None
        self._arrived_at: float | None = None
        # How many events we've observed since the last `arm()` — surfaced on a
        # timeout so we can tell "stream silent" from "stream busy, no match".
        self._observed = 0

    def arm(self, event_type: str, client_order_id: int) -> None:
        """Prime the awaiter for the next event to wait on."""

        with self._lock:
            self._want_type = event_type
            self._want_coid = str(client_order_id)
            self._arrived_at = None
            self._observed = 0
            self._signal.clear()

    def on_event(self, event_type: str, client_order_id: object, at: float) -> None:
        """Feed one observed event in from the WS thread; record the first match.

        ``client_order_id`` is taken as ``object`` and stringified before the
        compare: the wire may deliver a Uint64 as either a JSON string or a JSON
        number, and we armed with the string form.
        """

        coid = None if client_order_id is None else str(client_order_id)

        with self._lock:
            self._observed += 1

            matched = (
                self._arrived_at is None
                and event_type == self._want_type
                and coid == self._want_coid
            )

            if matched:
                self._arrived_at = at
                self._signal.set()

        if matched:
            _diag(f"  >>> matched {event_type} client_order_id={coid}")

    def wait(self, timeout: float) -> float:
        """Block until the armed event arrives; return its arrival perf_counter."""

        if not self._signal.wait(timeout):
            with self._lock:
                want_type, want_coid, observed = (
                    self._want_type,
                    self._want_coid,
                    self._observed,
                )

            raise TimeoutError(
                f"timed out after {timeout:.0f}s waiting for {want_type} "
                f"(client_order_id={want_coid}); observed {observed} event(s) on "
                f"the stream while waiting",
            )

        with self._lock:
            # `_signal` is only ever set together with `_arrived_at`, so this is
            # non-None here; assert for the type-checker's benefit.
            assert self._arrived_at is not None
            return self._arrived_at


def _make_callback(awaiter: _EventAwaiter) -> Callable[[PerpsEvent2Batch], None]:
    """Build the perps_events2 callback that feeds events into ``awaiter``."""

    def _callback(batch: PerpsEvent2Batch) -> None:
        # Stamp arrival immediately, before doing any other work.
        at = time.perf_counter()

        # Access the payload as a plain dict: it may be a server error envelope
        # (`{"_error": ...}`, see `dango.info._unwrap_node`) or a keepalive,
        # neither matching the PerpsEvent2Batch shape. Defensive `.get` keeps a
        # malformed message from killing the WS reader thread silently.
        raw = cast("dict[str, Any]", batch)
        if "_error" in raw:
            print(f"[diag] [ws] ERROR envelope: {raw}")
            return

        events = raw.get("events") or []

        _diag(
            f"[ws] batch block={raw.get('blockHeight')} "
            f"createdAt={raw.get('createdAt')} events={len(events)}"
        )

        for event in events:
            # `!r` on clientOrderId so we can see whether the wire sends it as a
            # string ('123') or a JSON number (123) — the likely match culprit.
            _diag(
                f"[ws]   idx={event.get('idx')} type={event.get('eventType')} "
                f"user={event.get('user')} pair={event.get('pairId')} "
                f"order_id={event.get('orderId')!r} "
                f"client_order_id={event.get('clientOrderId')!r}"
            )

            awaiter.on_event(event.get("eventType", ""), event.get("clientOrderId"), at)

    return _callback


# --- Helpers -----------------------------------------------------------------


def _index_price(info: Info, pair_id: PairId) -> Decimal:
    """Return the current oracle index price for ``pair_id``.

    ``index_price`` is the oracle reference price the contract uses for margin,
    PnL, funding, and liquidation. The Python ``PairState`` TypedDict does not
    declare the field, but ``pair_state`` returns the raw contract response,
    which carries it (see ``dango/exchange/types/src/perps.rs::PairState``).
    """

    state = info.pair_state(pair_id)
    if state is None:
        raise RuntimeError(f"no pair_state for {pair_id}; is the market configured on {API_URL}?")

    raw = cast("dict[str, Any]", state).get("index_price")
    if raw is None:
        raise RuntimeError(f"pair_state for {pair_id} carries no index_price: {state}")

    # `raw` is a 6-decimal fixed-point string; parse it exactly as a Decimal so
    # the tick-size rounding below is exact (floats reintroduce non-multiples).
    return Decimal(str(raw))


def _tick_size(info: Info, pair_id: PairId) -> Decimal:
    """Return the price tick size for ``pair_id``; limit prices must be a multiple."""

    param = info.pair_param(pair_id)
    if param is None:
        raise RuntimeError(f"no pair_param for {pair_id}; is the market configured on {API_URL}?")

    return Decimal(param["tick_size"])


def _round_down_to_tick(price: Decimal, tick: Decimal) -> Decimal:
    """Floor ``price`` to the nearest multiple of ``tick``.

    The contract rejects a limit price that isn't an integer multiple of the
    pair's tick size. Flooring (vs. rounding) keeps a buy's price at or below the
    1%-below-index target, so it still rests unfilled.
    """

    if tick <= 0:
        return price

    return (price // tick) * tick


def _tx_error_message(outcome: dict[str, Any]) -> str | None:
    """Extract an error string from a BroadcastTxOutcome, or None if it succeeded.

    A rejected tx can carry its error in several places depending on which stage
    bounced it; mirror the checks in the HL-compat layer
    (``dango.hyperliquid_compatibility.exchange._extract_error_message``).
    """

    for key in ("error", "err"):
        if isinstance(outcome.get(key), str):
            return cast("str", outcome[key])

    check_tx = outcome.get("check_tx")
    if isinstance(check_tx, dict):
        if isinstance(check_tx.get("error"), str):
            return cast("str", check_tx["error"])

        code = check_tx.get("code")
        if isinstance(code, int) and code != 0:
            return f"check_tx failed with code {code}"

    code = outcome.get("code")
    if isinstance(code, int) and code != 0:
        return f"tx failed with code {code}"

    result = outcome.get("result")
    if isinstance(result, dict):
        for key in ("err", "Err"):
            if isinstance(result.get(key), str):
                return cast("str", result[key])

    return None


def _check_tx(outcome: dict[str, Any], action: str) -> None:
    """Raise if ``outcome`` signals the tx was rejected."""

    message = _tx_error_message(outcome)
    if message is not None:
        raise RuntimeError(f"{action} rejected: {message} (outcome={outcome})")


def _p95(values: list[float]) -> float:
    """95th percentile by nearest-rank on the sorted samples."""

    ordered = sorted(values)
    rank = max(1, math.ceil(0.95 * len(ordered)))

    return ordered[rank - 1]


def _print_summary(samples: dict[str, list[float]]) -> None:
    """Print per-metric min / mean / median / p95 / max, in milliseconds."""

    print("\n=== round-trip latency (ms) ===")
    print("  mempool = broadcast call returned (tx admitted to mempool)")
    print("  confirm = lifecycle event received over perps_events2 (on-chain + indexed)\n")

    header = f"{'metric':<16}{'n':>4}{'min':>9}{'mean':>9}{'median':>9}{'p95':>9}{'max':>9}"
    print(header)
    print("-" * len(header))

    for name in _METRICS:
        xs = samples[name]
        if not xs:
            print(f"{name:<16}{0:>4}{'no samples':>45}")
            continue

        print(
            f"{name:<16}{len(xs):>4}{min(xs):>9.1f}{statistics.mean(xs):>9.1f}"
            f"{statistics.median(xs):>9.1f}{_p95(xs):>9.1f}{max(xs):>9.1f}"
        )


# --- Main --------------------------------------------------------------------


def main() -> None:
    address, info, exchange = example_utils.setup(
        base_url=API_URL,
        perps_contract=Addr(PERPS_CONTRACT),
    )
    # NOTE: unlike native_basic_order.py we must NOT pass skip_ws=True — this
    # script needs the WebSocket for the perps_events2 subscription.

    awaiter = _EventAwaiter()

    # Filter the stream to our pair and the two lifecycle events; we match the
    # exact order in the callback by its client_order_id. (`client_order_id` is
    # unique only per sender, but we mint a fresh one per order from a per-run
    # base, so collisions with other testnet traders are negligible.) Annotate
    # as list[str] so the PairId NewType doesn't trip list-invariance.
    pair_filter: list[str] = [PAIR_ID]
    sub_id = info.subscribe_perps_events2(
        _make_callback(awaiter),
        pair_ids=pair_filter,
        event_types=_AWAITED_EVENT_TYPES,
    )

    print(
        f"subscribed: {sub_id}; account {address}\n"
        f"measuring {ITERATIONS} place/cancel cycles on {PAIR_ID} via {API_URL}\n"
    )
    _diag(f"subscription filters: pair_ids={pair_filter} event_types={_AWAITED_EVENT_TYPES}")
    _diag(f"settling {SETTLE_S}s for the subscription to go live before cycle 1...")

    # Give the subscription a moment to go live before the first order.
    time.sleep(SETTLE_S)

    # Price tick size is a static pair parameter; fetch it once. A limit price
    # must be an integer multiple of it or the chain rejects the order (during
    # execution, NOT at broadcast — see the note on confirmation below).
    tick = _tick_size(info, PAIR_ID)
    _diag(f"tick size for {PAIR_ID}: {tick}")

    samples: dict[str, list[float]] = {name: [] for name in _METRICS}

    # Per-run client_order_id base (ms since epoch); each cycle adds its index.
    coid_base = int(time.time() * 1000)

    # The coid of an order we've placed but not yet confirmed-cancelled, so the
    # `finally` can clean it up if a cycle aborts mid-flight.
    pending_coid: int | None = None

    try:
        for i in range(ITERATIONS):
            coid = coid_base + i

            # Re-read the index each cycle (outside the timed region) so the
            # 1%-away price stays valid as the oracle drifts. Format to 6 dp —
            # the wire precision — so `dango_decimal` accepts it verbatim.
            index_price = _index_price(info, PAIR_ID)
            limit_price = _round_down_to_tick(index_price * INDEX_PRICE_FACTOR, tick)
            _diag(
                f"cycle {i + 1}: index={index_price:,.2f} limit_price={limit_price} "
                f"tick={tick} size={SIZE} coid={coid}"
            )

            # --- Place: time broadcast return, then order_persisted arrival ---
            _diag(f"cycle {i + 1}: arm order_persisted; broadcasting submit_limit_order...")

            awaiter.arm("order_persisted", coid)
            place_start = time.perf_counter()
            place_outcome = exchange.submit_limit_order(
                PAIR_ID,
                size=SIZE,
                limit_price=limit_price,
                time_in_force=TimeInForce.GTC,
                client_order_id=coid,
                gas_limit=GAS_LIMIT,
            )
            place_mempool = time.perf_counter()
            _diag(
                f"cycle {i + 1}: submit returned in "
                f"{(place_mempool - place_start) * 1000:.1f}ms; outcome={place_outcome}"
            )
            _check_tx(place_outcome, "submit_limit_order")

            pending_coid = coid  # accepted; may now be resting
            _diag(f"cycle {i + 1}: waiting up to {EVENT_TIMEOUT_S:.0f}s for order_persisted...")

            place_confirm = awaiter.wait(EVENT_TIMEOUT_S)
            _diag(f"cycle {i + 1}: order_persisted received")

            # --- Cancel: time broadcast return, then order_removed arrival ----
            _diag(f"cycle {i + 1}: arm order_removed; broadcasting cancel_order...")

            awaiter.arm("order_removed", coid)
            cancel_start = time.perf_counter()
            cancel_outcome = exchange.cancel_order(
                ClientOrderIdRef(value=coid),
                gas_limit=GAS_LIMIT,
            )
            cancel_mempool = time.perf_counter()
            _diag(
                f"cycle {i + 1}: cancel returned in "
                f"{(cancel_mempool - cancel_start) * 1000:.1f}ms; outcome={cancel_outcome}"
            )

            _check_tx(cancel_outcome, "cancel_order")
            _diag(f"cycle {i + 1}: waiting up to {EVENT_TIMEOUT_S:.0f}s for order_removed...")

            cancel_confirm = awaiter.wait(EVENT_TIMEOUT_S)
            pending_coid = None  # confirmed removed
            _diag(f"cycle {i + 1}: order_removed received")

            pm = (place_mempool - place_start) * 1000
            pc = (place_confirm - place_start) * 1000
            cm = (cancel_mempool - cancel_start) * 1000
            cc = (cancel_confirm - cancel_start) * 1000
            samples["place_mempool"].append(pm)
            samples["place_confirm"].append(pc)
            samples["cancel_mempool"].append(cm)
            samples["cancel_confirm"].append(cc)

            print(
                f"cycle {i + 1:>2}/{ITERATIONS}  index={index_price:>12,.2f}  "
                f"place[mempool={pm:7.1f} confirm={pc:8.1f}]  "
                f"cancel[mempool={cm:7.1f} confirm={cc:8.1f}]  (ms)"
            )

            if PAUSE_BETWEEN_S:
                time.sleep(PAUSE_BETWEEN_S)
    except KeyboardInterrupt:
        print("\ninterrupted; cleaning up...")
    except Exception as exc:
        # Report, clean up below, and still print stats for the cycles we got.
        print(f"\nstopped after error: {exc}")
    finally:
        if pending_coid is not None:
            try:
                exchange.cancel_order(ClientOrderIdRef(value=pending_coid))
                print(f"cleaned up resting order (client_order_id={pending_coid})")
            except Exception as exc:
                print(f"cleanup cancel failed (client_order_id={pending_coid}): {exc}")

        info.unsubscribe(sub_id)
        info.disconnect_websocket()

    _print_summary(samples)


if __name__ == "__main__":
    main()

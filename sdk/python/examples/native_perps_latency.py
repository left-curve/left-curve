"""Native Dango API: measure perps round-trip latency over one shared `/ws` socket.

Some algo traders report high latency trading on Dango. This script measures it
directly, with no fills: it repeatedly places a resting limit order 1% away from
the index price (so it never crosses the book) and cancels it, timing each
action two ways.

It uses a single long-lived `dango.ws.WsConnection` for **everything** — the
`perps_events` subscription *and* both the place and cancel broadcasts ride the
same socket, which is exactly what a latency-sensitive bot wants (no second
connection, no per-broadcast handshake). Orders are signed locally
(`Exchange.prepare_*`) and sent with `conn.broadcast`.

In every cycle the timer starts the instant before the broadcast call. We then
record two latencies per action, both anchored at that same start:

* ``mempool`` — when the ``conn.broadcast`` call returns. The native `/ws`
  ``broadcast`` reply comes back once the node admits the tx to its mempool. We
  pass an explicit ``gas_limit`` so the SDK skips its pre-broadcast ``simulate``
  round-trip; this figure is then just signing plus the broadcast hop.
* ``confirm`` — when the matching lifecycle event (``order_persisted`` for the
  place, ``order_removed`` for the cancel) arrives back over the same
  ``perps_events`` subscription. This is the full client-observed round trip:
  broadcast -> block inclusion -> indexer -> push back to us. (Arrival is stamped
  on the main thread as the batch is read, so it carries a sub-millisecond
  scheduling delay versus the WS reader thread — negligible at block-time scale.)

So ``confirm >= mempool`` always, since both share the same start anchor.

Each cycle also prints the precise **broadcast wall-clock timestamp** (UTC, to
the millisecond) for both the place and the cancel, plus the block height each
tx landed in. Cross-reference the timestamps against CometBFT logs to confirm a
tx that entered the mempool between blocks N and N+1 is included in N+1 (and not
N+2, N+3, ...). The place->cancel block gap printed each cycle is a
clock-skew-immune version of that same check: the cancel is broadcast the
instant the place's ``order_persisted`` arrives — i.e. right after the place's
block committed — so its gap directly answers "does a tx broadcast just after a
commit make the very next block?". All samples are also written to a CSV (see
``SAMPLE_CSV_PATH``) for offline cross-referencing.

Uses the native Dango API (not the Hyperliquid-compat shim) and the same
``example_utils.setup`` procedure as the other native mutation examples: it reads
``examples/.env`` for ``DANGO_SECRET_KEY`` / ``DANGO_ACCOUNT_ADDRESS`` and refuses
to run if the account has no margin. The account must be funded on the target
network (testnet by default).

Run with::

    uv run python examples/native_perps_latency.py
"""

from __future__ import annotations

import csv
import math
import statistics
import time
from datetime import UTC, datetime
from decimal import Decimal
from pathlib import Path
from typing import TYPE_CHECKING, Any, NamedTuple, cast

import example_utils

from dango.utils.constants import PERPS_CONTRACT_TESTNET, TESTNET_API_URL
from dango.utils.types import (
    Addr,
    ClientOrderIdRef,
    PairId,
    TimeInForce,
)
from dango.ws import WsConnection

if TYPE_CHECKING:
    from dango.info import Info
    from dango.ws import Subscription

# --- Configuration -----------------------------------------------------------

# Target network. Testnet by default. To measure mainnet latency instead, import
# `MAINNET_API_URL` / `PERPS_CONTRACT_MAINNET` from `dango.utils.constants` and
# swap them in here (and make sure the account is funded on mainnet).
#
# For this measurement we hit the INTERNAL url that connects directly to the
# server, bypassing the Cloudflare load balancer / proxy, so the figures reflect
# the node rather than the edge. Set `USE_INTERNAL_URL = False` to measure the
# public `TESTNET_API_URL` endpoint instead.
USE_INTERNAL_URL = True
INTERNAL_API_URL = "https://api-testnet-internal-hetzner4.dango.zone/"
API_URL = INTERNAL_API_URL if USE_INTERNAL_URL else TESTNET_API_URL
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
SETTLE_S = 1.0  # let the first subscription go live before the first order
PAUSE_BETWEEN_S = 0.25  # brief breather between cycles

# Lifecycle events we correlate against: `order_persisted` confirms the place
# landed in the book; `order_removed` confirms the cancel took effect.
_AWAITED_EVENT_TYPES = ["order_persisted", "order_removed"]

# Metric keys, in display order.
_METRICS = ("place_mempool", "place_confirm", "cancel_mempool", "cancel_confirm")

# Set to False to silence the step-by-step diagnostic logging.
VERBOSE = True

# Per-action CSV dump for offline cross-referencing against CometBFT logs. Each
# row carries the broadcast wall-clock timestamp, the measured latencies, and
# the block the tx landed in. Set to None to disable. Written relative to the
# CWD (i.e. `sdk/python/` when run as documented).
SAMPLE_CSV_PATH: str | None = "native_perps_latency_samples.csv"


def _diag(message: str) -> None:
    """Print a step-by-step diagnostic line when VERBOSE is on."""
    if VERBOSE:
        print(f"[diag] {message}")


def _utc_ms(epoch_s: float) -> str:
    """Format a wall-clock epoch (``time.time()``) as UTC ISO-8601 to the ms.

    This is the format to grep CometBFT logs with. It is the CLIENT's wall
    clock, so when matching against server-side log lines account for (a) any
    client<->server clock skew (both should be NTP-synced) and (b) the ~one-way
    network travel (~ping/2) between this call and the tx reaching the node.
    """

    dt = datetime.fromtimestamp(epoch_s, tz=UTC)
    return f"{dt:%Y-%m-%dT%H:%M:%S}.{dt.microsecond // 1000:03d}Z"


def _as_int(value: object) -> int | None:
    """Coerce a wire value (str or number) to int, or None if not parseable.

    ``blockHeight`` may arrive as a JSON string or a JSON number; normalise it to
    an int so we can compute the place->cancel block gap. ``bool`` is excluded so
    a stray ``True``/``False`` isn't silently read as 1/0.
    """

    if isinstance(value, int) and not isinstance(value, bool):
        return value

    if isinstance(value, str):
        try:
            return int(value)
        except ValueError:
            return None

    return None


def _gap_note(place_block: int | None, cancel_block: int | None) -> str:
    """Annotate how many blocks after the place's block the cancel landed.

    The cancel is broadcast the instant the place's ``order_persisted`` arrives —
    i.e. right after the place's block committed — so this gap answers exactly
    "does a tx broadcast just after a commit make the very next block?". +1 means
    yes; +2 or more means it missed the immediately-following block and slipped.
    """

    if place_block is None or cancel_block is None:
        return "[block gap unknown]"

    return f"[cancel landed +{cancel_block - place_block} block(s) after place]"


# --- Event correlation -------------------------------------------------------


class _Match(NamedTuple):
    """The result of awaiting a lifecycle event: when it arrived and where.

    ``arrived_at`` is a ``perf_counter`` reading (for latency math);
    ``arrived_wall`` is the wall clock at arrival (for log matching);
    ``block_height`` / ``block_created_at`` identify the block the event was
    emitted in, so the caller can see exactly which block included the tx.
    """

    arrived_at: float
    arrived_wall: float
    block_height: int | None
    block_created_at: str | None


def _await_event(
    sub: Subscription,
    event_type: str,
    client_order_id: int,
    timeout: float,
) -> _Match:
    """Block until an ``event_type`` event for ``client_order_id`` arrives on ``sub``.

    Reads batches off the subscription (each already filtered server-side to this
    order's pair, lifecycle events, and client_order_id), stamping arrival the
    instant a batch is delivered. Raises ``TimeoutError`` if the matching event
    doesn't arrive within ``timeout`` seconds.
    """

    want_coid = str(client_order_id)
    deadline = time.perf_counter() + timeout
    observed = 0

    while True:
        remaining = deadline - time.perf_counter()
        if remaining <= 0:
            break

        try:
            batch = sub.next_batch(timeout=remaining)
        except TimeoutError:
            break

        # Stamp arrival immediately, before any other work: a monotonic reading
        # for latency math, a wall-clock reading for log correlation.
        at = time.perf_counter()
        wall = time.time()
        block_height = _as_int(batch.get("blockHeight"))
        block_created_at = batch.get("createdAt")
        events = batch.get("events") or []

        _diag(f"[ws] batch block={batch.get('blockHeight')} events={len(events)}")

        for event in events:
            observed += 1
            # `clientOrderId` may arrive as a JSON string or number; stringify
            # both sides before comparing (we armed with the string form).
            coid = event.get("clientOrderId")
            if event.get("eventType") == event_type and str(coid) == want_coid:
                _diag(f"  >>> matched {event_type} client_order_id={coid} block={block_height}")
                return _Match(at, wall, block_height, block_created_at)

    raise TimeoutError(
        f"timed out after {timeout:.0f}s waiting for {event_type} "
        f"(client_order_id={want_coid}); observed {observed} event(s) on the "
        f"stream while waiting",
    )


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
    print("  mempool = conn.broadcast returned (tx admitted to mempool over /ws)")
    print("  confirm = lifecycle event received over perps_events (on-chain + indexed)\n")

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


def _write_samples_csv(path: str, rows: list[dict[str, object]]) -> None:
    """Write the per-action samples to ``path`` as CSV for offline analysis."""

    if not rows:
        return

    fieldnames = [
        "cycle",
        "action",
        "broadcast_utc",
        "mempool_ms",
        "confirm_ms",
        "landed_block",
        "block_created_at",
        "event_recv_utc",
    ]

    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"wrote {len(rows)} samples to {Path(path).resolve()}")


# --- Main --------------------------------------------------------------------


def main() -> None:
    address, info, exchange = example_utils.setup(
        base_url=API_URL,
        perps_contract=Addr(PERPS_CONTRACT),
    )

    print(
        f"account {address}\n"
        f"measuring {ITERATIONS} place/cancel cycles on {PAIR_ID} via {API_URL}\n"
        f"one WsConnection carries the perps_events feed AND both broadcasts\n"
    )

    # Price tick size is a static pair parameter; fetch it once. A limit price
    # must be an integer multiple of it or the chain rejects the order (during
    # execution, NOT at broadcast — see the note on confirmation above).
    tick = _tick_size(info, PAIR_ID)
    _diag(f"tick size for {PAIR_ID}: {tick}")

    samples: dict[str, list[float]] = {name: [] for name in _METRICS}

    # Per-action rows for the CSV dump (one place + one cancel per cycle), so the
    # broadcast timestamps and landed-block heights can be cross-referenced
    # against CometBFT logs offline.
    rows: list[dict[str, object]] = []

    # Per-run client_order_id base (ms since epoch); each cycle adds its index.
    coid_base = int(time.time() * 1000)

    # The coid of an order we've placed but not yet confirmed-cancelled, so the
    # `finally` can clean it up if a cycle aborts mid-flight.
    pending_coid: int | None = None

    # One socket for the whole run: subscriptions AND broadcasts.
    with WsConnection.connect(API_URL) as conn:
        try:
            for i in range(ITERATIONS):
                coid = coid_base + i

                # (a) Subscribe to this order's lifecycle, filtered server-side to
                # its client_order_id (plus the pair and the two lifecycle
                # events). The reader thread buffers frames the instant they
                # arrive, so no event can slip between subscribe and our read.
                sub = conn.subscribe_perps_events(
                    pair_ids=[PAIR_ID],
                    event_types=_AWAITED_EVENT_TYPES,
                    client_order_ids=[str(coid)],
                )
                if i == 0:
                    _diag(f"settling {SETTLE_S}s for the first subscription to go live...")
                    time.sleep(SETTLE_S)

                # Re-read the index each cycle (outside the timed region) so the
                # 1%-away price stays valid as the oracle drifts.
                index_price = _index_price(info, PAIR_ID)
                limit_price = _round_down_to_tick(index_price * INDEX_PRICE_FACTOR, tick)
                _diag(
                    f"cycle {i + 1}: index={index_price:,.2f} limit_price={limit_price} "
                    f"tick={tick} size={SIZE} coid={coid}"
                )

                # (b) Place: sign locally (no broadcast), then broadcast over the
                # SAME ws socket. Wall clock first, then the monotonic anchor,
                # captured back-to-back (sub-microsecond apart).
                place_tx = exchange.prepare_submit_limit_order(
                    PAIR_ID,
                    size=SIZE,
                    limit_price=limit_price,
                    time_in_force=TimeInForce.GTC,
                    client_order_id=coid,
                    gas_limit=GAS_LIMIT,
                )
                _diag(f"cycle {i + 1}: broadcasting submit_limit_order over /ws...")
                place_wall = time.time()
                place_start = time.perf_counter()
                place_outcome = conn.broadcast(place_tx)
                place_mempool = time.perf_counter()
                _check_tx(place_outcome, "submit_limit_order")
                pending_coid = coid  # accepted; may now be resting

                # (c) On order_persisted, cancel immediately (no pause/query), so
                # the cancel is phase-locked to just after the place's block.
                place_match = _await_event(sub, "order_persisted", coid, EVENT_TIMEOUT_S)
                place_confirm = place_match.arrived_at
                _diag(f"cycle {i + 1}: order_persisted in block {place_match.block_height}")

                cancel_tx = exchange.prepare_cancel_order(
                    ClientOrderIdRef(value=coid),
                    gas_limit=GAS_LIMIT,
                )
                cancel_wall = time.time()
                cancel_start = time.perf_counter()
                cancel_outcome = conn.broadcast(cancel_tx)
                cancel_mempool = time.perf_counter()
                _check_tx(cancel_outcome, "cancel_order")

                # (d) On order_removed, record.
                cancel_match = _await_event(sub, "order_removed", coid, EVENT_TIMEOUT_S)
                cancel_confirm = cancel_match.arrived_at
                pending_coid = None  # confirmed removed
                _diag(f"cycle {i + 1}: order_removed in block {cancel_match.block_height}")

                sub.unsubscribe()

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

                # Precise broadcast timestamps (client wall clock, UTC, ms) and
                # the block each tx landed in — for cross-referencing against
                # CometBFT logs. The trailing note is the place->cancel block gap.
                place_block = place_match.block_height
                cancel_block = cancel_match.block_height
                print(
                    f"           place  broadcast={_utc_ms(place_wall)}  "
                    f"-> block {place_block} @ {place_match.block_created_at}"
                )
                print(
                    f"           cancel broadcast={_utc_ms(cancel_wall)}  "
                    f"-> block {cancel_block} @ {cancel_match.block_created_at}  "
                    f"{_gap_note(place_block, cancel_block)}"
                )

                rows.append(
                    {
                        "cycle": i + 1,
                        "action": "place",
                        "broadcast_utc": _utc_ms(place_wall),
                        "mempool_ms": f"{pm:.1f}",
                        "confirm_ms": f"{pc:.1f}",
                        "landed_block": place_block,
                        "block_created_at": place_match.block_created_at,
                        "event_recv_utc": _utc_ms(place_match.arrived_wall),
                    }
                )
                rows.append(
                    {
                        "cycle": i + 1,
                        "action": "cancel",
                        "broadcast_utc": _utc_ms(cancel_wall),
                        "mempool_ms": f"{cm:.1f}",
                        "confirm_ms": f"{cc:.1f}",
                        "landed_block": cancel_block,
                        "block_created_at": cancel_match.block_created_at,
                        "event_recv_utc": _utc_ms(cancel_match.arrived_wall),
                    }
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
                # Clean up over the default (GraphQL) broadcast path — robust even
                # if the `/ws` socket is what failed.
                try:
                    exchange.cancel_order(ClientOrderIdRef(value=pending_coid))
                    print(f"cleaned up resting order (client_order_id={pending_coid})")
                except Exception as exc:
                    print(f"cleanup cancel failed (client_order_id={pending_coid}): {exc}")

    _print_summary(samples)

    if SAMPLE_CSV_PATH:
        try:
            _write_samples_csv(SAMPLE_CSV_PATH, rows)
        except OSError as exc:
            print(f"failed to write samples CSV to {SAMPLE_CSV_PATH}: {exc}")


if __name__ == "__main__":
    main()

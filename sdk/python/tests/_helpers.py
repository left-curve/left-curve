"""Shared white-box test scaffolding: FakeInfo + Exchange constructors."""

from __future__ import annotations

from typing import Any, cast

from dango.exchange import Exchange
from dango.utils.signing import Secp256k1Wallet
from dango.utils.types import Addr

# A fixed demo address shared across the helpers; callers that need a
# different address can construct their own FakeInfo / Exchange via the
# class directly, but every existing test happily reuses this single value.
_DEMO_ADDRESS: Addr = Addr("0x000000000000000000000000000000000000beef")


def wallet(address: Addr = _DEMO_ADDRESS) -> Secp256k1Wallet:
    """Return a deterministic Secp256k1 wallet bound to `address`."""

    # Fixed secret keeps signature outputs deterministic across tests so
    # we can compare credential shapes without recomputing them by hand.
    return Secp256k1Wallet.from_bytes(b"\x01" * 32, address)


# `FakeInfo` is a structural Info stand-in: it implements the subset of
# `Info` that `Exchange` calls (`query_status`, `query_app_smart`,
# `simulate`, `broadcast_tx_sync`) and records each call. The Exchange
# constructor signature uses `info: Info | None = None`, which is a
# concrete-class type in production; tests pass FakeInfo and rely on the
# `# type: ignore[arg-type]` escape hatch on the construction call site
# (standard white-box testing pattern).
class FakeInfo:
    """Captures simulate/broadcast calls and returns canned responses."""

    def __init__(self) -> None:
        self.simulated: list[dict[str, Any]] = []
        self.broadcasted: list[dict[str, Any]] = []
        self.queried_status_count: int = 0
        # Each entry is the (contract, msg) pair that flowed through
        # `query_app_smart`. Tests that pin call-site shape (e.g. the
        # account-factory username lookup in Phase 14) read this list
        # rather than threading a custom mock through.
        self.smart_queries: list[tuple[Addr, dict[str, Any]]] = []

    def query_status(self) -> dict[str, Any]:
        self.queried_status_count += 1

        # Mirror the GraphQL `queryStatus` shape (see
        # `dango/_graphql/queries/queryStatus.graphql`): a `chainId` and
        # a `block` sub-object. Only `chainId` is consumed by Exchange,
        # but the shape must stay realistic so future Phase-X consumers
        # of this fake don't trip on missing fields.
        return {
            "chainId": "dango-mock-1",
            "block": {"blockHeight": 1, "timestamp": "x", "hash": "y"},
        }

    def query_app_smart(
        self,
        contract: Addr,  # mocked: we route on `msg`'s top-level key only
        msg: dict[str, Any],
        **_: Any,
    ) -> Any:
        # Dango's query enums are externally-tagged, so the variant name
        # is the first (and only) key in the dict. We branch on that
        # tag rather than the contract address because both the
        # `account_factory` lookup (variant `account`) and the per-
        # account `seen_nonces` lookup are funneled through this single
        # method.
        self.smart_queries.append((contract, msg))

        if "account" in msg:
            # `User` struct shape — `owner` is the user_index that
            # SingleSigner.query_user_index reads.
            return {"index": 0, "owner": 42}

        if "seen_nonces" in msg:
            # Sorted ascending list; SingleSigner takes max+1, so this
            # produces next_nonce=6.
            return [3, 4, 5]

        if "user" in msg:
            # `QueryMsg::User(UserIndexOrName)` on the account-factory
            # contract — see `dango/types/src/account_factory/msg.rs`.
            # Wire form is `{"user": {"index": <u32>}}` or
            # `{"user": {"name": "<username>"}}`. The response is a
            # `User` struct; only `index` is consumed by `set_referral`,
            # so we hand back a small but shape-correct stub.
            return {"index": 7, "name": "alice", "accounts": {}}

        raise AssertionError(f"unexpected query_app_smart: {msg}")

    def simulate(self, tx: dict[str, Any]) -> dict[str, Any]:
        self.simulated.append(tx)

        # 230_000 chosen so simulate + DEFAULT_GAS_OVERHEAD = 1_000_000
        # — a clean round number that the gas-limit assertion in
        # test_exchange.py checks against.
        return {"gas_used": 230_000, "gas_limit": None, "result": {"ok": []}}

    def broadcast_tx_sync(self, tx: dict[str, Any]) -> dict[str, Any]:
        self.broadcasted.append(tx)

        # Realistic BroadcastTxOutcome envelope — `code=0` is success,
        # the rest is metadata. Exchange methods return this dict
        # verbatim, so consumers can read e.g. `result["hash"]`.
        return {"code": 0, "hash": "TXHASH", "gas_used": 230_000, "events": []}


def exchange(info: FakeInfo, *, address: Addr = _DEMO_ADDRESS, **kwargs: Any) -> Exchange:
    """Construct an Exchange wired to a mock Info (no real network calls)."""

    return Exchange(
        wallet(address),
        "http://localhost:8080",
        account_address=address,
        info=info,  # type: ignore[arg-type]
        **kwargs,
    )


def last_inner_msg(info: FakeInfo) -> dict[str, Any]:
    """Pull the inner execute `msg` payload out of the most-recent broadcast."""

    # Every Exchange action emits a single execute message; the inner
    # `msg` field carries the contract-side dispatch enum
    # (`{"trade": ...}`, `{"vault": ...}`, etc.). The repeated dict-walk
    # in every test would be ceremony; this helper centralizes it so a
    # wire-shape change touches one line per assertion category, not N.
    sent = info.broadcasted[-1]

    return cast("dict[str, Any]", sent["msgs"][0]["execute"]["msg"])

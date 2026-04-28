"""Setup helpers for the native Dango Python SDK examples.

Two factories:

* :func:`setup` — full ``(address, info, exchange)`` trio. Loads ``.env``
  for ``DANGO_SECRET_KEY`` / ``DANGO_ACCOUNT_ADDRESS`` and runs an equity
  guard. Used by the mutation examples (orders, market open/close, vault
  deposits, etc.).
* :func:`setup_read_only` — just an :class:`Info` instance. No creds, no
  equity guard. Used by query and public-subscription examples.

The HL-compat flavor lives in :mod:`example_utils_hl`. Both modules
expose ``setup`` / ``setup_read_only`` under the same names so HL example
scripts can ``import example_utils_hl as example_utils`` and read
verbatim against upstream HL's examples (only the import line differs).

Environment variables (used only by :func:`setup`; see
``examples/.env.example``):

* ``DANGO_SECRET_KEY`` — raw hex secret. Required.
* ``DANGO_ACCOUNT_ADDRESS`` — required for HL-compat (Dango decouples key
  from on-chain account); falls back to the wallet's derived address only on
  the native flavor.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING

import eth_account
from dotenv import load_dotenv
from eth_account.signers.local import LocalAccount

from dango.utils.types import Addr

if TYPE_CHECKING:
    from dango.exchange import Exchange
    from dango.info import Info


def setup(
    base_url: str | None = None,
    *,
    skip_ws: bool = False,
    perp_dexs: list[str] | None = None,
    perps_contract: Addr | None = None,
) -> tuple[str, Info, Exchange]:
    """Build a native ``(address, info, exchange)`` trio from env vars.

    Used by mutation examples; reads ``.env`` and refuses to run if the
    configured account has zero margin. ``perps_contract`` must be
    supplied explicitly when targeting any chain other than the SDK's
    default (mainnet) — Dango has no canonical URL → contract mapping
    and we don't try to guess.
    """

    # `perp_dexs` is accepted for HL-signature symmetry but unused on the
    # native side — Dango has no builder-deployed DEX abstraction.
    _ = perp_dexs

    from dango.exchange import Exchange
    from dango.info import Info
    from dango.utils.constants import LOCAL_API_URL

    load_env()

    account: LocalAccount = eth_account.Account.from_key(get_secret_key())
    address = resolve_account_address(account)
    print("Running with account address:", address)

    if address != account.address:
        print("Running with agent address:", account.address)

    # Native `Info` requires a base_url string (HL-compat coalesces None
    # internally; we mirror that here to keep the two setups feel-alike).
    resolved_url = base_url or LOCAL_API_URL
    info = Info(resolved_url, skip_ws=skip_ws, perps_contract=perps_contract)

    # Native `user_state` returns the raw contract response. Note
    # `margin` is a flat `UsdValue` decimal string (NOT a nested object
    # like HL's `marginSummary`); walk a single string field for the
    # equity guard.
    state = info.user_state(Addr(address))
    if state is None or float(state["margin"]) == 0:
        print("Not running the example because the provided account has no equity.")
        url = resolved_url.split(".", 1)[-1] or resolved_url
        raise Exception(
            f"No accountValue:\nIf you think this is a mistake, "
            f"make sure that {address} has a balance on {url}.\n"
            f"If the address shown is your API wallet address, set DANGO_ACCOUNT_ADDRESS "
            f"to the address of your account, not the API wallet."
        )

    # Reuse the same `info` so the Exchange's queries hit the same
    # perps contract, and pass `perps_contract` so build-side
    # messages target the right deployment.
    exchange = Exchange(
        account,
        resolved_url,
        account_address=Addr(address),
        info=info,
        perps_contract=perps_contract,
    )

    return address, info, exchange


def setup_read_only(
    base_url: str | None = None,
    *,
    skip_ws: bool = False,
    perps_contract: Addr | None = None,
) -> Info:
    """Construct a credential-free native Info for read-only examples.

    ``perps_contract`` must be supplied explicitly when targeting any
    chain other than the SDK's default (mainnet). See :func:`setup` for
    the rationale.
    """

    # No `.env` load, no wallet, no equity guard — read-only callers shouldn't
    # be forced to maintain DANGO_* secrets just to query public chain state.
    from dango.info import Info
    from dango.utils.constants import LOCAL_API_URL

    return Info(
        base_url or LOCAL_API_URL,
        skip_ws=skip_ws,
        perps_contract=perps_contract,
    )


def get_secret_key() -> str:
    """Resolve the signer's secret from ``DANGO_SECRET_KEY``."""

    secret_key = os.environ.get("DANGO_SECRET_KEY", "").strip()

    if not secret_key:
        raise ValueError(
            "DANGO_SECRET_KEY is required — set it in examples/.env or your environment.",
        )

    return secret_key


def load_env() -> None:
    """Load variables from ``examples/.env`` into ``os.environ`` if present."""

    # `load_dotenv` is a no-op when the file doesn't exist, which is the
    # right behavior for production deployments that inject env vars via
    # the orchestrator instead of a `.env` file. We pin the path to this
    # module's directory so running examples from any cwd works.
    load_dotenv(Path(__file__).resolve().parent / ".env")


def resolve_account_address(account: LocalAccount) -> str:
    """Return ``DANGO_ACCOUNT_ADDRESS`` or fall back to the wallet's address.

    Dango's HL-compat ``Exchange`` constructor REQUIRES an explicit
    ``account_address`` — we don't silently default to the wallet's derived
    address because the Dango-side address depends on the KeyType
    (Secp256k1 vs Ethereum) and the activation path. This helper keeps the
    HL-style "fall back to the wallet address" convenience for the native
    flavor; the HL-compat ``Exchange`` constructor enforces the explicit-
    address requirement at construction time.
    """

    address = os.environ.get("DANGO_ACCOUNT_ADDRESS", "").strip()
    if not address:
        address = account.address

    return str(address)

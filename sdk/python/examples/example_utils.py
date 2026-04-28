"""Shared setup helper for the Dango Python SDK examples.

Loads config from environment variables (and optionally an ``examples/.env``
file via python-dotenv). The dotenv dep lives in the ``examples`` group —
install with ``uv sync --group examples``. Production users injecting env
vars from their orchestrator (Docker ``--env-file``, k8s secrets, etc.) can
ignore the dotenv loader entirely; ``os.environ`` is the source of truth.

Offers two flavors of the canonical ``setup`` factory used by HL:

* :func:`setup` — returns the HL-compat trio ``(address, hl_info, hl_exchange)``;
  mirrors the upstream ``hyperliquid.example_utils.setup`` signature so HL
  example files only need their import lines changed.
* :func:`setup_native` — returns the native trio ``(address, native_info, native_exchange)``;
  used by the Dango-native examples.

Both flavors share :func:`get_secret_key` for ``DANGO_SECRET_KEY`` /
``DANGO_KEYSTORE_PATH`` resolution.

Environment variables (see ``examples/.env.example``):

* ``DANGO_SECRET_KEY`` — raw hex secret, OR
* ``DANGO_KEYSTORE_PATH`` — path to encrypted keystore JSON.
* ``DANGO_ACCOUNT_ADDRESS`` — required for HL-compat (Dango decouples key
  from on-chain account); falls back to the wallet's derived address only on
  the native flavor.
"""

from __future__ import annotations

import getpass
import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

import eth_account
from dotenv import load_dotenv
from eth_account.signers.local import LocalAccount

from dango.utils.types import Addr

if TYPE_CHECKING:
    from dango.exchange import Exchange as NativeExchange
    from dango.hyperliquid_compatibility.exchange import Exchange as HlExchange
    from dango.hyperliquid_compatibility.info import Info as HlInfo
    from dango.info import Info as NativeInfo


def setup(
    base_url: str | None = None,
    skip_ws: bool = False,
    perp_dexs: list[str] | None = None,
) -> tuple[str, HlInfo, HlExchange]:
    """Build an HL-compat ``(address, info, exchange)`` trio from env vars.

    Mirrors HL's ``example_utils.setup`` so ports of HL examples are
    one-line import changes only.
    """
    # Lazy imports so callers that only need `setup_native` don't pay
    # the cost of pulling in the HL-compat translation layer.
    from dango.hyperliquid_compatibility.exchange import Exchange as HlExchangeCls
    from dango.hyperliquid_compatibility.info import Info as HlInfoCls

    _load_env()
    account: LocalAccount = eth_account.Account.from_key(get_secret_key())
    address = _resolve_account_address(account)
    print("Running with account address:", address)
    if address != account.address:
        print("Running with agent address:", account.address)

    info = HlInfoCls(base_url=base_url, skip_ws=skip_ws, perp_dexs=perp_dexs)
    # `user_state` is HL-shaped (the HL-compat Info reshapes the native
    # contract response to match HL's TypedDict): `marginSummary` carries
    # `accountValue` as a decimal string, same as upstream HL. Dango is
    # perps-only, so no `spot_user_state` companion check.
    user_state = info.user_state(address)
    margin_summary = user_state["marginSummary"]
    if float(margin_summary["accountValue"]) == 0:
        print("Not running the example because the provided account has no equity.")
        url = (info._native.base_url or "").split(".", 1)[-1] or info._native.base_url
        raise Exception(
            f"No accountValue:\nIf you think this is a mistake, "
            f"make sure that {address} has a balance on {url}.\n"
            f"If the address shown is your API wallet address, set DANGO_ACCOUNT_ADDRESS "
            f"to the address of your account, not the API wallet."
        )
    exchange = HlExchangeCls(
        account,
        base_url,
        account_address=address,
        perp_dexs=perp_dexs,
    )
    return address, info, exchange


def setup_native(
    base_url: str | None = None,
    skip_ws: bool = False,
    perp_dexs: list[str] | None = None,
) -> tuple[str, NativeInfo, NativeExchange]:
    """Build a native ``(address, info, exchange)`` trio from env vars.

    Used by the ``native_*.py`` example scripts; they consume the
    Dango-native API directly (snake_case wire shapes, signed sizes,
    typed contract messages).
    """
    # Lazy imports for symmetry with `setup`. `perp_dexs` is accepted
    # but unused on the native side — Dango has no builder-deployed
    # DEX abstraction.
    _ = perp_dexs
    from dango.exchange import Exchange as NativeExchangeCls
    from dango.info import Info as NativeInfoCls
    from dango.utils.constants import LOCAL_API_URL

    _load_env()
    account: LocalAccount = eth_account.Account.from_key(get_secret_key())
    address = _resolve_account_address(account)
    print("Running with account address:", address)
    if address != account.address:
        print("Running with agent address:", account.address)

    # Native `Info` requires a base_url string (HL-compat coalesces None
    # internally; we mirror that here to keep the two setups feel-alike).
    resolved_url = base_url or LOCAL_API_URL
    info = NativeInfoCls(resolved_url, skip_ws=skip_ws)
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
    exchange = NativeExchangeCls(
        account,
        resolved_url,
        account_address=Addr(address),
    )
    return address, info, exchange


def get_secret_key() -> str | bytes:
    """Resolve the signer's secret from ``DANGO_SECRET_KEY`` or a keystore path."""
    # Resolution order: prefer the inline `DANGO_SECRET_KEY` if non-empty
    # (typical for local dev / tests), else decrypt the keystore at
    # `DANGO_KEYSTORE_PATH` after prompting for a password.
    secret_key = os.environ.get("DANGO_SECRET_KEY", "").strip()
    if secret_key:
        return secret_key
    keystore_path = os.environ.get("DANGO_KEYSTORE_PATH", "").strip()
    if not keystore_path:
        raise ValueError(
            "Provide DANGO_SECRET_KEY (hex) or DANGO_KEYSTORE_PATH "
            "(path to encrypted keystore) in examples/.env or your environment.",
        )
    keystore_path = os.path.expanduser(keystore_path)
    if not os.path.isabs(keystore_path):
        keystore_path = os.path.join(os.path.dirname(__file__), keystore_path)
    if not os.path.exists(keystore_path):
        raise FileNotFoundError(f"Keystore file not found: {keystore_path}")
    if not os.path.isfile(keystore_path):
        raise ValueError(f"Keystore path is not a file: {keystore_path}")
    with open(keystore_path) as f:
        keystore = json.load(f)
    password = getpass.getpass("Enter keystore password: ")
    return eth_account.Account.decrypt(keystore, password)


def _load_env() -> None:
    """Load variables from ``examples/.env`` into ``os.environ`` if present."""
    # `load_dotenv` is a no-op when the file doesn't exist, which is the
    # right behavior for production deployments that inject env vars via
    # the orchestrator instead of a `.env` file. We pin the path to this
    # module's directory so running examples from any cwd works.
    load_dotenv(Path(__file__).resolve().parent / ".env")


def _resolve_account_address(account: LocalAccount) -> str:
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

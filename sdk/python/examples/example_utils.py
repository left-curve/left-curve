"""Shared setup helper for the Dango Python SDK examples.

Offers two flavors of the canonical ``setup`` factory used by HL:

* :func:`setup` — returns the HL-compat trio
  ``(address, hl_info, hl_exchange)``; mirrors the upstream
  ``hyperliquid.example_utils.setup`` signature byte-for-byte so the HL
  example files only need their import lines changed.
* :func:`setup_native` — returns the native trio
  ``(address, native_info, native_exchange)``; used by the Dango-native
  examples.

Both flavors share :func:`get_secret_key` for ``secret_key`` /
``keystore_path`` resolution, mirroring HL's pattern.

Differences from HL's ``example_utils.py``:

* Dango is perps-only — there is no ``spot_user_state``. The "no equity"
  guard checks the perps margin only.
* ``account_address`` is REQUIRED in ``config.json``: Dango decouples the
  signing key from the account address. The HL-compat ``Exchange``
  constructor enforces this — silent auto-derivation would route trades to
  a different address than the one the user expects.
"""

from __future__ import annotations

import getpass
import json
import os
from typing import TYPE_CHECKING

import eth_account
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
    """Build an HL-compat ``(address, info, exchange)`` trio from ``config.json``.

    Mirrors HL's ``example_utils.setup`` so ports of HL examples are
    one-line import changes only.
    """
    # Lazy imports so callers that only need `setup_native` don't pay
    # the cost of pulling in the HL-compat translation layer.
    from dango.hyperliquid_compatibility.exchange import Exchange as HlExchangeCls
    from dango.hyperliquid_compatibility.info import Info as HlInfoCls

    config = _load_config()
    account: LocalAccount = eth_account.Account.from_key(get_secret_key(config))
    address = _resolve_account_address(config, account)
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
            f"If address shown is your API wallet address, update the config to specify "
            f"the address of your account, not the address of the API wallet."
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
    """Build a native ``(address, info, exchange)`` trio from ``config.json``.

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

    config = _load_config()
    account: LocalAccount = eth_account.Account.from_key(get_secret_key(config))
    address = _resolve_account_address(config, account)
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
            f"If address shown is your API wallet address, update the config to specify "
            f"the address of your account, not the address of the API wallet."
        )
    exchange = NativeExchangeCls(
        account,
        resolved_url,
        account_address=Addr(address),
    )
    return address, info, exchange


def get_secret_key(config: dict) -> str | bytes:
    """Resolve the signer's secret from a ``secret_key`` hex or a keystore."""
    # Mirror HL's resolution order: prefer the inline `secret_key` if
    # present (typical for tests), else decrypt the keystore at
    # `keystore_path` after prompting for a password.
    if config["secret_key"]:
        return str(config["secret_key"])
    keystore_path = config["keystore_path"]
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


def _load_config() -> dict:
    """Load ``config.json`` from the same directory as this helper."""
    config_path = os.path.join(os.path.dirname(__file__), "config.json")
    with open(config_path) as f:
        return json.load(f)


def _resolve_account_address(config: dict, account: LocalAccount) -> str:
    """Return the configured ``account_address`` or fall back to the wallet's address.

    Dango's HL-compat ``Exchange`` constructor REQUIRES an explicit
    ``account_address`` — we don't silently default to the wallet's
    derived address because the Dango-side address depends on the
    KeyType (Secp256k1 vs Ethereum) and the activation path. This helper
    keeps the HL-style "fall back to the wallet address" convenience for
    the native flavor; the HL-compat `Exchange` constructor enforces the
    explicit-address requirement at construction time.
    """
    address = config["account_address"]
    if address == "":
        address = account.address
    return str(address)

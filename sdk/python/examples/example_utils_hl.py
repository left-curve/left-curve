"""Setup helpers for the HL-compat Dango Python SDK examples.

Symmetric with :mod:`example_utils`: ``setup`` is the mutation flavor
(returning the trio), and ``setup_read_only`` returns just an
:class:`Info`.

HL example scripts can ``import example_utils_hl as example_utils`` and
keep their bodies verbatim against upstream HL's examples — only the
import line differs. ``setup`` matches upstream HL's signature exactly
(``(address, info, exchange) = example_utils.setup(base_url, skip_ws,
perp_dexs)``).

Environment variables (used only by :func:`setup`; see
``examples/.env.example``):

* ``DANGO_SECRET_KEY`` — raw hex secret, OR
* ``DANGO_KEYSTORE_PATH`` — path to encrypted keystore JSON.
* ``DANGO_ACCOUNT_ADDRESS`` — required (Dango decouples key from on-chain
  account; the HL-compat ``Exchange`` constructor enforces this).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import eth_account
from eth_account.signers.local import LocalAccount

# Reuse the env-loading + secret-resolution helpers from the native module.
# They're identical regardless of which API flavor we're targeting; keeping
# them in one place avoids drift if the secret/keystore policy ever changes.
from example_utils import get_secret_key, load_env, resolve_account_address

if TYPE_CHECKING:
    from dango.hyperliquid_compatibility.exchange import Exchange
    from dango.hyperliquid_compatibility.info import Info


def setup(
    base_url: str | None = None,
    skip_ws: bool = False,
    perp_dexs: list[str] | None = None,
) -> tuple[str, Info, Exchange]:
    """Build an HL-compat ``(address, info, exchange)`` trio from env vars.

    Mirrors HL's ``example_utils.setup`` so HL examples ported into this
    repo are byte-identical except for the import lines.
    """
    from dango.hyperliquid_compatibility.exchange import Exchange as ExchangeCls
    from dango.hyperliquid_compatibility.info import Info as InfoCls

    load_env()
    account: LocalAccount = eth_account.Account.from_key(get_secret_key())
    address = resolve_account_address(account)
    print("Running with account address:", address)
    if address != account.address:
        print("Running with agent address:", account.address)

    info = InfoCls(base_url=base_url, skip_ws=skip_ws, perp_dexs=perp_dexs)
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
    exchange = ExchangeCls(
        account,
        base_url,
        account_address=address,
        perp_dexs=perp_dexs,
    )
    return address, info, exchange


def setup_read_only(
    base_url: str | None = None,
    *,
    skip_ws: bool = False,
    perp_dexs: list[str] | None = None,
) -> Info:
    """Construct a credential-free HL-compat Info for read-only examples."""
    from dango.hyperliquid_compatibility.info import Info as InfoCls

    return InfoCls(base_url=base_url, skip_ws=skip_ws, perp_dexs=perp_dexs)

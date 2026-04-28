"""HL-compat re-export of the Dango environment URLs.

Mirrors the module path of upstream ``hyperliquid.utils.constants`` so HL
example code keeps the same ``from <prefix>.utils import constants`` import
shape after the one-line port:

    from hyperliquid.utils import constants
    # becomes
    from dango.hyperliquid_compatibility import constants

The names re-exported (``MAINNET_API_URL``, ``TESTNET_API_URL``,
``LOCAL_API_URL``) are byte-compatible with HL's at the call-site level
(``constants.TESTNET_API_URL``); only the URL values differ since they
point at Dango environments.
"""

from __future__ import annotations

from dango.utils.constants import (
    LOCAL_API_URL,
    MAINNET_API_URL,
    TESTNET_API_URL,
)

__all__ = ["LOCAL_API_URL", "MAINNET_API_URL", "TESTNET_API_URL"]

# ruff: noqa: E501
# Long lines below are kept verbatim from upstream HL's example to honour
# the "byte-verbatim except imports" claim of the HL-compat layer.
"""HL-compat port of upstream ``hyperliquid-python-sdk``'s ``basic_ws.py``.

Body is verbatim with upstream except for two Dango-specific deltas:

* Imports — ``example_utils_hl as example_utils`` and
  ``dango.hyperliquid_compatibility import constants`` replace HL's
  ``example_utils`` / ``hyperliquid.utils import constants``.
* The ``setup()`` call adds a ``perps_contract=...`` kwarg. Dango has no
  canonical URL → contract mapping, so the deployment address has to be
  passed in explicitly. Everything else inside ``main()`` is verbatim.

Dango gaps to be aware of (relative to Hyperliquid):

* ``activeAssetCtx`` for spot (``"@1"``) — Dango is perps-only; the spot
  variant raises ``NotImplementedError``.
* ``userFundings`` — Dango does not expose funding-history-as-a-series
  via the indexer.
* ``userNonFundingLedgerUpdates`` — no ledger-updates feed on Dango.
* ``webData2`` — HL-specific webapp aggregate; not implemented.

The original lines are kept intact so the file mirrors HL's verbatim;
users running on Dango should comment out those subscriptions or expect
the corresponding ``NotImplementedError``.
"""

import example_utils_hl as example_utils

from dango.hyperliquid_compatibility import constants
from dango.utils.constants import PERPS_CONTRACT_TESTNET
from dango.utils.types import Addr


def main():
    address, info, _ = example_utils.setup(
        constants.TESTNET_API_URL, perps_contract=Addr(PERPS_CONTRACT_TESTNET)
    )

    # An example showing how to subscribe to the different subscription types and prints the returned messages
    # Some subscriptions do not return snapshots, so you will not receive a message until something happens
    info.subscribe({"type": "allMids"}, print)
    info.subscribe({"type": "l2Book", "coin": "ETH"}, print)
    info.subscribe({"type": "trades", "coin": "PURR/USDC"}, print)
    info.subscribe({"type": "userEvents", "user": address}, print)
    info.subscribe({"type": "userFills", "user": address}, print)
    info.subscribe({"type": "candle", "coin": "ETH", "interval": "1m"}, print)
    info.subscribe({"type": "orderUpdates", "user": address}, print)
    info.subscribe({"type": "userFundings", "user": address}, print)
    info.subscribe({"type": "userNonFundingLedgerUpdates", "user": address}, print)
    info.subscribe({"type": "webData2", "user": address}, print)
    info.subscribe({"type": "bbo", "coin": "ETH"}, print)
    info.subscribe({"type": "activeAssetCtx", "coin": "BTC"}, print)  # Perp
    info.subscribe({"type": "activeAssetCtx", "coin": "@1"}, print)  # Spot
    info.subscribe({"type": "activeAssetData", "user": address, "coin": "BTC"}, print)  # Perp only


if __name__ == "__main__":
    main()

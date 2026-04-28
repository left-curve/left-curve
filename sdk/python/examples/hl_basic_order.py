"""HL-compat port of upstream ``hyperliquid-python-sdk``'s ``basic_order.py``.

Body is verbatim with upstream except for two Dango-specific deltas:

* Imports — ``example_utils_hl as example_utils`` and
  ``dango.hyperliquid_compatibility import constants`` replace HL's
  ``example_utils`` / ``hyperliquid.utils import constants``.
* The ``setup()`` call adds a ``perps_contract=...`` kwarg. Dango has no
  canonical URL → contract mapping, so the deployment address has to be
  passed in explicitly. Everything else inside ``main()`` is verbatim.
"""

import json

import example_utils_hl as example_utils

from dango.hyperliquid_compatibility import constants
from dango.utils.constants import PERPS_CONTRACT_TESTNET
from dango.utils.types import Addr


def main():
    address, info, exchange = example_utils.setup(
        base_url=constants.TESTNET_API_URL,
        skip_ws=True,
        perps_contract=Addr(PERPS_CONTRACT_TESTNET),
    )

    # Get the user state and print out position information
    user_state = info.user_state(address)
    positions = []
    for position in user_state["assetPositions"]:
        positions.append(position["position"])
    if len(positions) > 0:
        print("positions:")
        for position in positions:
            print(json.dumps(position, indent=2))
    else:
        print("no open positions")

    # Place an order that should rest by setting the price very low
    order_result = exchange.order("ETH", True, 0.2, 1100, {"limit": {"tif": "Gtc"}})
    print(order_result)

    # Query the order status by oid
    if order_result["status"] == "ok":
        status = order_result["response"]["data"]["statuses"][0]
        if "resting" in status:
            order_status = info.query_order_by_oid(address, status["resting"]["oid"])
            print("Order status by oid:", order_status)

    # Cancel the order
    if order_result["status"] == "ok":
        status = order_result["response"]["data"]["statuses"][0]
        if "resting" in status:
            cancel_result = exchange.cancel("ETH", status["resting"]["oid"])
            print(cancel_result)


if __name__ == "__main__":
    main()

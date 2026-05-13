"""URLs, chain IDs, contract addresses, and gas/settlement constants."""

from __future__ import annotations

from typing import Final

MAINNET_API_URL: Final[str] = "https://api-mainnet.dango.zone"
TESTNET_API_URL: Final[str] = "https://api-testnet.dango.zone"
LOCAL_API_URL: Final[str] = "http://localhost:8080"

CHAIN_ID_MAINNET: Final[str] = "dango-1"
CHAIN_ID_TESTNET: Final[str] = "dango-testnet-1"

ACCOUNT_FACTORY_CONTRACT: Final[str] = "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b"
ORACLE_CONTRACT: Final[str] = "0xcedc5f73cbb963a48471b849c3650e6e34cd3b6d"
PERPS_CONTRACT_MAINNET: Final[str] = "0x90bc84df68d1aa59a857e04ed529e9a26edbea4f"
PERPS_CONTRACT_TESTNET: Final[str] = "0xf6344c5e2792e8f9202c58a2d88fbbde4cd3142f"

SETTLEMENT_DENOM: Final[str] = "bridge/usdc"
SETTLEMENT_DECIMALS: Final[int] = 6
GAS_OVERHEAD_SECP256K1: Final[int] = 770_000

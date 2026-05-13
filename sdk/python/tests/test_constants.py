"""Sanity checks for dango.utils.constants."""

from dango.utils import constants


def test_urls_use_https_for_remote() -> None:
    assert constants.MAINNET_API_URL.startswith("https://")
    assert constants.TESTNET_API_URL.startswith("https://")
    assert constants.LOCAL_API_URL.startswith("http://")


def test_contract_addresses_are_hex_addrs() -> None:
    for addr in (
        constants.ACCOUNT_FACTORY_CONTRACT,
        constants.ORACLE_CONTRACT,
        constants.PERPS_CONTRACT_MAINNET,
        constants.PERPS_CONTRACT_TESTNET,
    ):
        assert addr.startswith("0x")
        assert len(addr) == 42


def test_settlement_denom_uses_bridge_namespace() -> None:
    assert constants.SETTLEMENT_DENOM == "bridge/usdc"
    assert constants.SETTLEMENT_DECIMALS == 6


def test_gas_overhead_is_positive_int() -> None:
    assert isinstance(constants.GAS_OVERHEAD_SECP256K1, int)
    assert constants.GAS_OVERHEAD_SECP256K1 > 0

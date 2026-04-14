use {
    grug::{Addr, BlockInfo, Storage, addr},
    grug_app::AppResult,
};

const _MAINNET_CHAIN_ID: &str = "dango-1";
const _MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const _TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const _TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

pub fn do_upgrade<VM>(_storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("Nothing to do for this upgrade");

    Ok(())
}

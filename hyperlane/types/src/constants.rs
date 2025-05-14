//! Hyperlane domains and Warp contract addresses for various chains.
//!
//! ## Note
//!
//! Throughout this file,
//!
//! - "testnet" for Ethereum, L2s, and sidechains refers to the Sepolia testnet.
//! - "WARP" refers to the collateral token between the source chain and Dango chain.
//!   E.g., `ethereum::USDC_WARP` is the [`HypERC20Collateral`](https://github.com/hyperlane-xyz/hyperlane-monorepo/blob/main/solidity/contracts/token/HypERC20Collateral.sol)
//!   contract on Ethereum with Ethereum as the source chain and Dango as the
//!   destination chain.

use crate::{Addr32, addr32, mailbox::Domain};

pub mod arbitrum {
    use super::*;

    pub const DOMAIN: Domain = 42161;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod arbitrum_testnet {
    use super::*;

    pub const DOMAIN: Domain = 421614;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod base {
    use super::*;

    pub const DOMAIN: Domain = 8453;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod base_testnet {
    use super::*;

    pub const DOMAIN: Domain = 84532;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod ethereum {
    use super::*;

    pub const DOMAIN: Domain = 1;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod ethereum_testnet {
    use super::*;

    pub const DOMAIN: Domain = 11155111;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod optimism {
    use super::*;

    pub const DOMAIN: Domain = 10;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod optimism_testnet {
    use super::*;

    pub const DOMAIN: Domain = 11155420;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod polygon {
    use super::*;

    pub const DOMAIN: Domain = 137;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod polygon_testnet {
    use super::*;

    pub const DOMAIN: Domain = 80002;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const WETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod solana {
    use super::*;

    pub const DOMAIN: Domain = 1399811149;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const SOL_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

pub mod solana_testnet {
    use super::*;

    pub const DOMAIN: Domain = 1399811150;

    // TODO: not yet deployed
    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000000");

    // TODO: not yet deployed
    pub const SOL_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");
}

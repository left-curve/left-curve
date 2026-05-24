use hyperlane_types::{Addr32, addr32, mailbox::Domain};

pub mod mock_ethereum {
    use super::*;

    pub const DOMAIN: Domain = 1;

    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000001");

    pub const ETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000002");
}

pub mod mock_solana {
    use super::*;

    pub const DOMAIN: Domain = 2;

    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000003");

    pub const SOL_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000004");
}

pub mod mock_arbitrum {
    use super::*;

    pub const DOMAIN: Domain = 3;

    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000005");

    pub const ETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000006");
}

pub mod mock_base {
    use super::*;

    pub const DOMAIN: Domain = 4;

    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000007");

    pub const ETH_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000008");
}

pub mod mock_optimism {
    use super::*;

    pub const DOMAIN: Domain = 5;

    pub const USDC_WARP: Addr32 =
        addr32!("0000000000000000000000000000000000000000000000000000000000000009");

    pub const ETH_WARP: Addr32 =
        addr32!("000000000000000000000000000000000000000000000000000000000000000a");
}

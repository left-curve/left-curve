//! Reference: <https://docs.pyth.network/price-feeds/pro/price-feed-ids>

use {
    crate::PythLazerSubscriptionDetails,
    pyth_lazer_protocol::{api::Channel, time::FixedRate},
};

pub const LAZER_ENDPOINTS_TEST: [&str; 3] = [
    "wss://pyth-lazer-0.dourolabs.app/v1/stream",
    "wss://pyth-lazer-1.dourolabs.app/v1/stream",
    "wss://pyth-lazer-2.dourolabs.app/v1/stream",
];

pub const LAZER_TRUSTED_SIGNER: &str = "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb";

pub const LAZER_ID_ALL: [PythLazerSubscriptionDetails; 7] = [
    BTC_USD_ID,
    ETH_USD_ID,
    HYPE_USD_ID,
    SOL_USD_ID,
    USDC_USD_ID,
    XAU_USD_ID,
    XAG_USD_ID,
];

pub const BTC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 1,
    channel: Channel::RealTime,
};

pub const ETH_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 2,
    channel: Channel::RealTime,
};

pub const HYPE_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 110,
    channel: Channel::RealTime,
};

pub const SOL_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 6,
    channel: Channel::RealTime,
};

pub const USDC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 7,
    channel: Channel::RealTime,
};

pub const XAU_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 346,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const XAG_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 345,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

// // https://docs.pyth.network/price-feeds/pro/price-feed-ids?search=brent

/// - **Name**: BRENTQ6
/// - **Symbol**: Commodities.BRENTQ6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 30 JUNE 2026
pub const BRENT_2026_06_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3042,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: BRENTU6
/// - **Symbol**: Commodities.BRENTU6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 31 JULY 2026
pub const BRENT_2026_07_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3043,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: BRENTV6
/// - **Symbol**: Commodities.BRENTV6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 28 AUGUST 2026
pub const BRENT_2026_08_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3044,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: BRENTX6
/// - **Symbol**: Commodities.BRENTX6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 30 SEPTEMBER 2026
pub const BRENT_2026_09_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3045,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

// https://docs.pyth.network/price-feeds/pro/price-feed-ids?search=wti

/// - **Name**: WTIN6
/// - **Symbol**: Commodities.WTIN6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 22 JUNE 2026
pub const WTI_2026_06_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3068,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: WTIQ6
/// - **Symbol**: Commodities.WTIQ6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 21 JULY 2026
pub const WTI_2026_07_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3069,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: WTIU6
/// - **Symbol**: Commodities.WTIU6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 20 AUGUST 2026
pub const WTI_2026_08_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3070,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// - **Name**: WTIV6
/// - **Symbol**: Commodities.WTIV6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 22 SEPTEMBER 2026
pub const WTI_2026_09_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3071,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

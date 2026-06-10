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

// https://docs.pyth.network/price-feeds/pro/price-feed-ids?search=brent

/// Pyth ID for the Brent oil futures contract that ends trading in June 2026.
///
/// Delivery is in August 2026; the `Q` in the contract name is the futures
/// month code for August. Do not confuse this with the contract that delivers
/// in June 2026 (`BRENTM6`), which already ended trading on April 30, 2026.
///
/// - **Name**: BRENTQ6
/// - **Symbol**: Commodities.BRENTQ6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 30 JUNE 2026
pub const BRENT_2026_06_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3042,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the Brent oil futures contract that ends trading in July 2026.
///
/// Delivery is in September 2026; the `U` in the contract name is the futures
/// month code for September. Do not confuse this with the contract that
/// delivers in July 2026 (`BRENTN6`), which already ended trading on May 29,
/// 2026.
///
/// - **Name**: BRENTU6
/// - **Symbol**: Commodities.BRENTU6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 31 JULY 2026
pub const BRENT_2026_07_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3043,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the Brent oil futures contract that ends trading in August
/// 2026.
///
/// Delivery is in October 2026; the `V` in the contract name is the futures
/// month code for October. Do not confuse this with the contract that
/// delivers in August 2026 — that one is [`BRENT_2026_06_ID`] (`BRENTQ6`),
/// which ends trading on June 30, 2026.
///
/// - **Name**: BRENTV6
/// - **Symbol**: Commodities.BRENTV6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 28 AUGUST 2026
pub const BRENT_2026_08_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3044,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the Brent oil futures contract that ends trading in September
/// 2026.
///
/// Delivery is in November 2026; the `X` in the contract name is the futures
/// month code for November. Do not confuse this with the contract that
/// delivers in September 2026 — that one is [`BRENT_2026_07_ID`] (`BRENTU6`),
/// which ends trading on July 31, 2026.
///
/// - **Name**: BRENTX6
/// - **Symbol**: Commodities.BRENTX6/USD
/// - **Description**: PYTH PRICE IN USD FOR BRENT 30 SEPTEMBER 2026
pub const BRENT_2026_09_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3045,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

// https://docs.pyth.network/price-feeds/pro/price-feed-ids?search=wti

/// Pyth ID for the WTI oil futures contract that ends trading in June 2026.
///
/// Delivery is in July 2026; the `N` in the contract name is the futures
/// month code for July. Do not confuse this with the contract that delivers
/// in June 2026 (`WTIM6`), which already ended trading on May 19, 2026.
///
/// - **Name**: WTIN6
/// - **Symbol**: Commodities.WTIN6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 22 JUNE 2026
pub const WTI_2026_06_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3068,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the WTI oil futures contract that ends trading in July 2026.
///
/// Delivery is in August 2026; the `Q` in the contract name is the futures
/// month code for August. Do not confuse this with the contract that delivers
/// in July 2026 — that one is [`WTI_2026_06_ID`] (`WTIN6`), which ends
/// trading on June 22, 2026.
///
/// - **Name**: WTIQ6
/// - **Symbol**: Commodities.WTIQ6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 21 JULY 2026
pub const WTI_2026_07_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3069,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the WTI oil futures contract that ends trading in August 2026.
///
/// Delivery is in September 2026; the `U` in the contract name is the futures
/// month code for September. Do not confuse this with the contract that
/// delivers in August 2026 — that one is [`WTI_2026_07_ID`] (`WTIQ6`), which
/// ends trading on July 21, 2026.
///
/// - **Name**: WTIU6
/// - **Symbol**: Commodities.WTIU6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 20 AUGUST 2026
pub const WTI_2026_08_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3070,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

/// Pyth ID for the WTI oil futures contract that ends trading in September
/// 2026.
///
/// Delivery is in October 2026; the `V` in the contract name is the futures
/// month code for October. Do not confuse this with the contract that
/// delivers in September 2026 — that one is [`WTI_2026_08_ID`] (`WTIU6`),
/// which ends trading on August 20, 2026.
///
/// - **Name**: WTIV6
/// - **Symbol**: Commodities.WTIV6/USD
/// - **Description**: PYTH PRICE IN USD FOR WTI 22 SEPTEMBER 2026
pub const WTI_2026_09_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 3071,
    channel: Channel::FixedRate(FixedRate::RATE_50_MS),
};

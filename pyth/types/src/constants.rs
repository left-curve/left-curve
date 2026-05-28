//! Reference: <https://docs.pyth.network/price-feeds/pro/price-feed-ids>

use {crate::PythLazerSubscriptionDetails, pyth_lazer_protocol::api::Channel};

pub const LAZER_ENDPOINTS_TEST: [&str; 3] = [
    "wss://pyth-lazer-0.dourolabs.app/v1/stream",
    "wss://pyth-lazer-1.dourolabs.app/v1/stream",
    "wss://pyth-lazer-2.dourolabs.app/v1/stream",
];

pub const LAZER_TRUSTED_SIGNER: &str = "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb";

pub const LAZER_ID_ALL: [PythLazerSubscriptionDetails; 5] =
    [BTC_USD_ID, ETH_USD_ID, HYPE_USD_ID, SOL_USD_ID, USDC_USD_ID];

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

use {
    crate::PythLazerSubscriptionDetails,
    pyth_lazer_protocol::router::{Channel, FixedRate},
};

pub const LAZER_ENDPOINTS_TEST: [&str; 1] = ["wss://pyth-lazer-0.dourolabs.app/v1/stream"];

pub const LAZER_ACCESS_TOKEN_TEST: &str = "gr6DS1uhFL7dUrcrboueU4ykRk2XhOfT3GO-demo";

pub const LAZER_TRUSTED_SIGNER: &str = "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb";

pub const LAZER_ID_ALL: [PythLazerSubscriptionDetails; 13] = [
    ATOM_USD_ID_LAZER,
    BCH_USD_ID_LAZER,
    BNB_USD_ID_LAZER,
    BTC_USD_ID_LAZER,
    DOGE_USD_ID_LAZER,
    ETH_USD_ID_LAZER,
    LTC_USD_ID_LAZER,
    SHIB_USD_ID_LAZER,
    SOL_USD_ID_LAZER,
    SUI_USD_ID_LAZER,
    USDC_USD_ID_LAZER,
    WBTC_USD_ID_LAZER,
    XRP_USD_ID_LAZER,
];

pub const ATOM_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 44,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BCH_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 24,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BNB_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 15,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BTC_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 1,
    channel: Channel::RealTime,
};

pub const DOGE_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 13,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const ETH_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 2,
    channel: Channel::RealTime,
};

pub const LTC_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 26,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const SHIB_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 20,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const SOL_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 6,
    channel: Channel::RealTime,
};

pub const SUI_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 11,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const USDC_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 7,
    channel: Channel::RealTime,
};

pub const WBTC_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 103,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const XRP_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 14,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

use {
    crate::PythLazerSubscriptionDetails,
    pyth_lazer_protocol::{api::Channel, time::FixedRate},
};

pub const LAZER_ENDPOINTS_TEST: [&str; 1] = ["wss://pyth-lazer-0.dourolabs.app/v1/stream"];

pub const LAZER_TRUSTED_SIGNER: &str = "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb";

pub const LAZER_ID_ALL: [PythLazerSubscriptionDetails; 13] = [
    ATOM_USD_ID,
    BCH_USD_ID,
    BNB_USD_ID,
    BTC_USD_ID,
    DOGE_USD_ID,
    ETH_USD_ID,
    LTC_USD_ID,
    SHIB_USD_ID,
    SOL_USD_ID,
    SUI_USD_ID,
    USDC_USD_ID,
    WBTC_USD_ID,
    XRP_USD_ID,
];

pub const ATOM_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 44,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BCH_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 24,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BNB_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 15,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const BTC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 1,
    channel: Channel::RealTime,
};

pub const DOGE_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 13,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const ETH_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 2,
    channel: Channel::RealTime,
};

pub const LTC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 26,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const SHIB_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 20,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const SOL_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 6,
    channel: Channel::RealTime,
};

pub const SUI_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 11,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const USDC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 7,
    channel: Channel::RealTime,
};

pub const WBTC_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 103,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

pub const XRP_USD_ID: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 14,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

use {
    crate::PythLazerSubscriptionDetails,
    pyth_lazer_protocol::router::{Channel, FixedRate},
};

pub const LAZER_ENDPOINTS_TEST: [&str; 1] = ["wss://pyth-lazer-0.dourolabs.app/v1/stream"];

pub const LAZER_ACCESS_TOKEN_TEST: &str = "gr6DS1uhFL7dUrcrboueU4ykRk2XhOfT3GO-demo";

pub const LAZER_TRUSTED_SIGNER: &str = "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb";

pub const BTC_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 1,
    channel: Channel::RealTime,
};

pub const ETH_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 2,
    channel: Channel::RealTime,
};

pub const DOGE_USD_ID_LAZER: PythLazerSubscriptionDetails = PythLazerSubscriptionDetails {
    id: 13,
    channel: Channel::FixedRate(FixedRate::RATE_200_MS),
};

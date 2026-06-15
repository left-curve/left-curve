use {
    crate::config::AppConfig,
    dango_primitives::{Addr, Coins, Querier, QuerierExt, StdResult},
};

/// An extension trait that adds some useful, Dango-specific methods to
/// [`QuerierWrapper`](dango_primitives::QuerierWrapper).
pub trait DangoQuerier {
    fn query_dango_config(&self) -> StdResult<AppConfig>;

    fn query_account_factory(&self) -> StdResult<Addr> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.addresses.account_factory)
    }

    fn query_gateway(&self) -> StdResult<Addr> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.addresses.gateway)
    }

    fn query_oracle(&self) -> StdResult<Addr> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.addresses.oracle)
    }

    fn query_perps(&self) -> StdResult<Addr> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.addresses.perps)
    }

    fn query_warp(&self) -> StdResult<Addr> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.addresses.warp)
    }

    fn query_minimum_deposit(&self) -> StdResult<Coins> {
        self.query_dango_config()
            .map(|app_cfg| app_cfg.minimum_deposit)
    }
}

impl<Q> DangoQuerier for Q
where
    Q: Querier,
{
    fn query_dango_config(&self) -> StdResult<AppConfig> {
        self.query_app_config()
    }
}

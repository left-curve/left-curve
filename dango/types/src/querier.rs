use {
    crate::config::AppConfig,
    grug::{Addr, QuerierExt, QuerierWrapper, StdResult},
};

/// An extension trait that adds some useful, Dango-specific methods to
/// [`QuerierWrapper`](grug::QuerierWrapper).
pub trait DangoQuerier {
    fn query_dango_config(&self) -> StdResult<AppConfig>;

    fn query_account_factory(&self) -> StdResult<Addr>;

    fn query_gateway(&self) -> StdResult<Addr>;

    fn query_warp(&self) -> StdResult<Addr>;
}

impl DangoQuerier for QuerierWrapper<'_> {
    fn query_dango_config(&self) -> StdResult<AppConfig> {
        self.query_app_config()
    }

    fn query_account_factory(&self) -> StdResult<Addr> {
        self.query_app_config::<AppConfig>()
            .map(|app_cfg| app_cfg.addresses.account_factory)
    }

    fn query_gateway(&self) -> StdResult<Addr> {
        self.query_app_config::<AppConfig>()
            .map(|app_cfg| app_cfg.addresses.gateway)
    }

    fn query_warp(&self) -> StdResult<Addr> {
        self.query_app_config::<AppConfig>()
            .map(|app_cfg| app_cfg.addresses.warp)
    }
}

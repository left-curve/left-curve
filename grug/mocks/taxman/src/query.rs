use {
    crate::{Config, CONFIG},
    grug_types::{StdResult, Storage},
};

pub fn query_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

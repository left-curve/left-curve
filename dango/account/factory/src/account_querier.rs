use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, Cache, QuerierWrapper, StdResult, StorageQuerier},
};

pub struct AccountQuerier<'a> {
    cache: Cache<'a, Addr, Account>,
}

impl<'a> AccountQuerier<'a> {
    pub fn new(factory: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self {
            cache: Cache::new(move |address, _| {
                querier.query_wasm_path(factory, &ACCOUNTS.path(*address))
            }),
        }
    }

    pub fn query_account(&mut self, address: Addr) -> StdResult<&Account> {
        self.cache.get_or_fetch(&address, None)
    }
}

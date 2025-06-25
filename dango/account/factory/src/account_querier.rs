use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, Cache, QuerierWrapper, StdResult, StorageQuerier},
};

pub struct AccountQuerier<'a> {
    cache: Cache<'a, Addr, Option<Account>>,
}

impl<'a> AccountQuerier<'a> {
    pub fn new(factory: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self {
            cache: Cache::new(move |address, _| {
                querier.may_query_wasm_path(factory, &ACCOUNTS.path(*address))
            }),
        }
    }

    /// Query the account for an address.
    ///
    /// ## Inputs
    ///
    /// - `address`: The address to query the account for.
    ///
    /// ## Returns
    ///
    /// - The `Some(account)` if it exists, which will be the case for user
    ///   addresses. `None` if no account exists, which will be the case for
    ///   contract addresses.
    pub fn query_account(&mut self, address: Addr) -> StdResult<Option<&Account>> {
        self.cache.get_or_fetch(&address, None).map(|a| a.as_ref()) // Convert `&Option<T>` to `Option<&T>`
    }
}

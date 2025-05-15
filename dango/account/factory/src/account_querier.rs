use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, QuerierWrapper, StorageQuerier},
    std::collections::HashMap,
};

pub struct AccountQuerier<'a> {
    querier: QuerierWrapper<'a>,
    address: Addr,
    cache: HashMap<Addr, Option<Account>>,
}

impl<'a> AccountQuerier<'a> {
    pub fn new(address: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self {
            address,
            querier,
            cache: HashMap::new(),
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
    pub fn query_account(&mut self, address: Addr) -> anyhow::Result<Option<Account>> {
        if let Some(cached_account) = self.cache.get(&address) {
            match cached_account {
                Some(account) => Ok(Some(account.clone())),
                None => Ok(None),
            }
        } else {
            let account = self
                .querier
                .may_query_wasm_path(self.address, ACCOUNTS.path(address))?;

            self.cache.insert(address, account.clone());

            Ok(account)
        }
    }
}

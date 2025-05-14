use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, Borsh, Codec, QuerierExt, QuerierWrapper},
    std::collections::HashMap,
};

pub struct AccountQuerier<'a> {
    querier: QuerierWrapper<'a>,
    address: Addr,
    cache: HashMap<Addr, Account>,
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
        if let Some(account) = self.cache.get(&address) {
            return Ok(Some(account.clone()));
        }

        match self
            .querier
            .query_wasm_raw(self.address, ACCOUNTS.path(address).storage_key())?
        {
            Some(account_binary) => {
                let account: Account = Borsh::decode(&account_binary)?;
                self.cache.insert(address, account.clone());
                Ok(Some(account))
            },
            None => Ok(None),
        }
    }
}

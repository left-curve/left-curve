use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, QuerierWrapper, StorageQuerier},
    std::collections::HashMap,
};

pub struct AccountQuerier {
    factory: Addr,
    cache: HashMap<Addr, Account>,
}

impl AccountQuerier {
    pub fn new(factory: Addr) -> Self {
        Self {
            factory,
            cache: HashMap::new(),
        }
    }

    pub fn query_account(
        &mut self,
        querier: &QuerierWrapper,
        address: Addr,
    ) -> anyhow::Result<Account> {
        if let Some(account) = self.cache.get(&address) {
            return Ok(account.clone());
        }

        let account = querier.query_wasm_path(self.factory, &ACCOUNTS.path(address))?;
        self.cache.insert(address, account.clone());
        Ok(account)
    }
}

use {
    crate::ACCOUNTS,
    dango_types::account_factory::Account,
    grug::{Addr, QuerierWrapper, StorageQuerier},
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

    pub fn query_account(&mut self, address: Addr) -> anyhow::Result<Account> {
        if let Some(account) = self.cache.get(&address) {
            return Ok(account.clone());
        }

        let account = self
            .querier
            .query_wasm_path(self.address, &ACCOUNTS.path(address))?;

        self.cache.insert(address, account.clone());

        Ok(account)
    }
}

use {
    crate::{ACCOUNTS, MAIN_ACCOUNT},
    dango_types::account_factory::{Account, UserIndex},
    grug::{Addr, Cache, QuerierWrapper, StdResult, StorageQuerier},
};

pub struct AccountQuerier<'a> {
    account_cache: Cache<'a, Addr, Option<Account>>,
    main_address_cache: Cache<'a, UserIndex, Option<Addr>>,
}

impl<'a> AccountQuerier<'a> {
    pub fn new(factory: Addr, querier: QuerierWrapper<'a>) -> Self {
        Self {
            account_cache: Cache::new(move |address, _| {
                querier.may_query_wasm_path(factory, &ACCOUNTS.path(*address))
            }),
            main_address_cache: Cache::new(move |user, _| {
                querier.may_query_wasm_path(factory, &MAIN_ACCOUNT.path(*user))
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
        self.account_cache
            .get_or_fetch(&address, None)
            .map(|a| a.as_ref()) // Convert `&Option<T>` to `Option<&T>`
    }

    /// Query the main account for an user.
    ///
    /// ## Inputs
    ///
    /// - `user`: The user index to query the main address for.
    ///
    /// ## Returns
    ///
    /// - The `Some(address)` if there the user index exists.
    ///   `None` if no user index exists.
    pub fn query_main_account(&mut self, user: UserIndex) -> StdResult<Option<&Addr>> {
        self.main_address_cache
            .get_or_fetch(&user, None)
            .map(|a| a.as_ref()) // Convert `&Option<T>` to `Option<&T>`
    }
}

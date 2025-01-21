use {
    crate::TestSuite,
    grug_app::{AppError, Db, Indexer, ProposalPreparer, Vm},
    grug_math::{Inner, Int256, NextNumber, NumberConst, PrevNumber, Signed, Unsigned},
    grug_types::{Addressable, Denom},
    std::collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceChange {
    Increased(u128),
    Decreased(u128),
    Unchanged,
}

pub struct BalanceTracker<'a, DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
{
    pub(crate) suite: &'a mut TestSuite<DB, VM, PP, ID>,
}

impl<DB, VM, PP, ID> BalanceTracker<'_, DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    /// Record the current balance of a list of accounts.
    pub fn record_balances<I, A>(&mut self, accounts: I)
    where
        I: IntoIterator<Item = A>,
        A: Addressable,
    {
        let new_balances = accounts
            .into_iter()
            .map(|addr| (addr.address(), self.suite.query_balances(&addr).unwrap()))
            // collect is needed to avoid borrowing issues
            .collect::<BTreeMap<_, _>>();

        self.suite.balances.extend(new_balances);
    }

    /// Record the current balance of a single account.
    pub fn record_balance<A>(&mut self, account: A)
    where
        A: Addressable,
    {
        let account = account.address();
        let coins = self.suite.query_balances(&account).unwrap();
        self.suite.balances.insert(account, coins);
    }

    /// Assert a list of balance changes for an account.
    pub fn assert_balance<A>(&self, account: A, changes: BTreeMap<Denom, BalanceChange>)
    where
        A: Addressable,
    {
        let account = account.address();
        let delta = self.balance_changes(account);

        for (denom, change) in changes {
            let diff = delta.get(&denom).unwrap();
            if change != *diff {
                panic!(
                    "incorrect balance! account: {}, denom: {}, expected: {:?}, actual: {:?}",
                    account, denom, change, diff
                );
            }
        }
    }

    /// Clear all recorded balances.
    pub fn clear(&mut self) {
        self.suite.balances.clear();
    }

    /// Refresh all recorded balances.
    pub fn refresh_balances(&mut self) {
        // Need to collect the addresses first to avoid borrowing issues
        let addresses: Vec<_> = self.suite.balances.keys().cloned().collect();
        for addr in addresses {
            let coins = self.suite.query_balances(&addr).unwrap();
            self.suite.balances.insert(addr, coins);
        }
    }

    /// Refresh the balance of a single account.
    pub fn refresh_balance<A>(&mut self, account: A)
    where
        A: Addressable,
    {
        let account = account.address();
        let coins = self.suite.query_balances(&account).unwrap();
        self.suite.balances.insert(account, coins);
    }

    /// Get the changes in balances of an account since the last recorded balances.
    pub fn balance_changes<A>(&self, account: A) -> BTreeMap<Denom, BalanceChange>
    where
        A: Addressable,
    {
        let account = account.address();
        let old_balances = self.suite.balances.get(&account).unwrap();
        let new_balances = self.suite.query_balances(&account).unwrap();

        old_balances
            .into_iter()
            .chain(&new_balances)
            .map(|coin| coin.denom)
            // Take denoms only once
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|denom| {
                let old_balance = old_balances.amount_of(denom);
                let new_balance = new_balances.amount_of(denom);
                let diff: Int256 = new_balance.into_next().checked_into_signed().unwrap()
                    - old_balance.into_next().checked_into_signed().unwrap();
                let change = match diff {
                    Int256::ZERO => BalanceChange::Unchanged,
                    diff if diff > Int256::ZERO => BalanceChange::Increased(
                        diff.checked_into_unsigned()
                            .unwrap()
                            .checked_into_prev()
                            .unwrap()
                            .into_inner(),
                    ),
                    diff => BalanceChange::Decreased(
                        (- diff).checked_into_unsigned()
                            .unwrap()
                            .checked_into_prev()
                            .unwrap()
                            .into_inner(),
                    ),
                };

                (denom.clone(), change)
            })
            .collect()
    }
}

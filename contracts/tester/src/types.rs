use grug::Empty;

pub type InstantiateMsg = Empty;

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Perform an infinite loop. Test if the VM can properly halt execution
    /// when gas is exhausted.
    InfiniteLoop {},
    /// See `QueryMsg::ForceWrite`.
    ///
    /// This tests that the VM can not only reject a forced write when calling
    /// the `query` export, but also when handling the `query_chain` import call
    /// within an `execute` call.
    ForceWriteOnQuery { key: String, value: String },
    /// The contract attempts to execute itself in a loop.
    ///
    /// Without proper check, this will cause a stack overflow panic, which
    /// halts the chain.
    ///
    /// This is one of two ways a malicious contract can force a stack overflow;
    /// the other is via a query message, also implemented in this contract.
    StackOverflow {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Run a loop of the given number of iterations. Within each iteration, a
    /// set of math operations (addition, subtraction, multiplication, division)
    /// are performed.
    ///
    /// This is used for deducing the relation between Wasmer gas metering
    /// points and CPU time (i.e. how many gas points roughly correspond to one
    /// second of run time).
    #[returns(Empty)]
    Loop { iterations: u64 },
    /// Attempt to write a key-value pair to the contract storage.
    ///
    /// If using the Grug library, this is impossible to do, because the
    /// contract is given an `ImmutableCtx` in the `query` function, which
    /// doesn't come with state mutable methods.
    ///
    /// However, a malicious contract can attempt to directly call the `db_write`,
    /// `db_remove`, or `db_remove_range` FFI import methods directly. We need
    /// to test whether the VM can properly reject this behavior.
    #[returns(Empty)]
    ForceWrite { key: String, value: String },
    /// The contract attempts to make queries in a loop.
    ///
    /// Without proper check, this will cause a stack overflow panic, which
    /// halts the chain.
    ///
    /// This is one of two ways a malicious contract can force a stack overflow;
    /// the other is via an execute message, also implemented in this contract.
    #[returns(Empty)]
    StackOverflow {},
}

use {
    crate::{GasTracker, QuerierProvider, StorageProvider, VmProvider},
    grug_types::{Context, Hash256},
};

/// Represents a virtual machine that can execute programs.
pub trait Vm: Sized {
    type Error: ToString;
    type Instance<'a>: Instance<Error = Self::Error> + 'a;

    /// Create an instance of the VM given a storage, a querier, and a guest
    /// program.
    ///
    /// Need a mutable reference (`&mut self`) because the VM might uses some
    /// sort of caching to speed up instance building.
    fn build_instance<'a>(
        &mut self,
        code: &[u8],
        code_hash: Hash256,
        // storage: StorageProvider,
        state_mutable: bool,
        // querier: Box<dyn QuerierProvider>,
        query_depth: usize,
        gas_tracker: GasTracker,
        vm_provider: VmProvider<'a>,
    ) -> Result<Self::Instance<'a>, Self::Error>;
}

pub trait Instance {
    type Error: ToString;

    /// Call a function that takes exactly 0 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_0_out_1(self, name: &'static str, ctx: &Context) -> Result<Vec<u8>, Self::Error>;

    /// Call a function that takes exactly 1 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_1_out_1<P>(
        self,
        name: &'static str,
        ctx: &Context,
        param: &P,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P: AsRef<[u8]>;

    /// Call a function that takes exactly 2 input parameters (other than the
    /// context) and returns exactly 1 output.
    fn call_in_2_out_1<P1, P2>(
        self,
        name: &'static str,
        ctx: &Context,
        param1: &P1,
        param2: &P2,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P1: AsRef<[u8]>,
        P2: AsRef<[u8]>;
}

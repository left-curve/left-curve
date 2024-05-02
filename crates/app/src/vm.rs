use {
    crate::{AppError, AppResult, PrefixStore, Querier, CODES, CONTRACT_NAMESPACE},
    cw_types::{from_borsh_slice, Addr, BlockInfo, Hash, Storage, Vm},
};

pub fn load_program<VM: Vm>(store: &dyn Storage, code_hash: &Hash) -> AppResult<VM::Program> {
    let code = CODES.load(store, code_hash)?;
    Ok(from_borsh_slice(code)?)
}

pub fn create_vm_instance<S, VM>(
    store: S,
    block: BlockInfo,
    address: &Addr,
    program: VM::Program,
) -> AppResult<VM>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let substore = Box::new(PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]));
    let querier = Box::new(Querier::<S, VM>::new(store, block));
    Ok(VM::build_instance(substore, querier, program)?)
}

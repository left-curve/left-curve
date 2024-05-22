use {
    crate::{AppError, AppResult, PrefixStore, QueryProvider, Vm, CODES, CONTRACT_NAMESPACE},
    grug_types::{from_borsh_slice, Addr, BlockInfo, Hash, Storage},
};

pub fn load_program<VM: Vm>(store: &dyn Storage, code_hash: &Hash) -> AppResult<VM::Program> {
    let code = CODES.load(store, code_hash)?;
    Ok(from_borsh_slice(code)?)
}

pub fn create_vm_instance<VM>(
    store: Box<dyn Storage>,
    block: BlockInfo,
    address: &Addr,
    program: VM::Program,
) -> AppResult<VM>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let storage = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, address]);
    let querier = QueryProvider::new(store, block);
    Ok(VM::build_instance(storage, querier, program)?)
}

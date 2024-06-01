use {
    crate::{AppError, AppResult, PrefixStore, QueryProvider, Vm, CODES, CONTRACT_NAMESPACE},
    grug_types::{from_borsh_slice, Addr, BlockInfo, Hash, Storage},
};

pub fn load_program<VM: Vm>(storage: &dyn Storage, code_hash: &Hash) -> AppResult<VM::Program> {
    let code = CODES.load(storage, code_hash)?;
    Ok(from_borsh_slice(code)?)
}

pub fn create_vm_instance<VM>(
    storage: Box<dyn Storage>,
    block: BlockInfo,
    address: &Addr,
    program: VM::Program,
) -> AppResult<VM>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let prefix_store = PrefixStore::new(storage.clone(), &[CONTRACT_NAMESPACE, address]);
    let querier = QueryProvider::new(storage, block);
    Ok(VM::build_instance(prefix_store, querier, program)?)
}

use {
    crate::{AppError, AppResult, PrefixStore, Querier, CODES, CONTRACT_NAMESPACE},
    cw_std::{from_borsh_slice, hash, to_borsh_vec, Addr, BlockInfo, Hash, Storage, Vm},
};

pub fn load_program<VM: Vm>(store: &dyn Storage, code_hash: &Hash) -> AppResult<VM::Program> {
    let code = CODES.load(store, code_hash)?;
    Ok(from_borsh_slice(&code)?)
}

pub fn save_program<VM: Vm>(store: &mut dyn Storage, program: &VM::Program) -> AppResult<Hash> {
    let code = to_borsh_vec(program)?;
    let code_hash = hash(&code);

    // avoid duplicate uploads
    if CODES.has(store, &code_hash) {
        return Err(AppError::code_exists(code_hash));
    }

    CODES.save(store, &code_hash, &code.into())?;

    Ok(code_hash)
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

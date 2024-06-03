#[macro_export]
macro_rules! return_into_generic_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => GenericResult::Ok(val),
            Err(err) => GenericResult::Err(err.to_string()),
        }
    };
}

// TODO: replace with https://doc.rust-lang.org/std/ops/trait.Try.html once stabilized
#[macro_export]
macro_rules! unwrap_into_generic_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return GenericResult::Err(err.to_string());
            },
        }
    };
}

#[macro_export]
macro_rules! make_immutable_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        ImmutableCtx {
            storage: $storage,
            api: $api,
            querier: $querier,
            chain_id: $ctx.chain_id,
            block_height: $ctx.block.height,
            block_timestamp: $ctx.block.timestamp,
            block_hash: $ctx.block.hash,
            contract: $ctx.contract,
        }
    };
}

#[macro_export]
macro_rules! make_mutable_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        MutableCtx {
            storage: $storage,
            api: $api,
            querier: $querier,
            chain_id: $ctx.chain_id,
            block_height: $ctx.block.height,
            block_timestamp: $ctx.block.timestamp,
            block_hash: $ctx.block.hash,
            contract: $ctx.contract,
            sender: $ctx.sender.unwrap(),
            funds: $ctx.funds.unwrap(),
        }
    };
}

#[macro_export]
macro_rules! make_sudo_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        SudoCtx {
            storage: $storage,
            api: $api,
            querier: $querier,
            chain_id: $ctx.chain_id,
            block_height: $ctx.block.height,
            block_timestamp: $ctx.block.timestamp,
            block_hash: $ctx.block.hash,
            contract: $ctx.contract,
        }
    };
}

#[macro_export]
macro_rules! make_auth_ctx {
    ($ctx:ident, $storage:expr, $api:expr, $querier:expr) => {
        AuthCtx {
            storage: $storage,
            api: $api,
            querier: $querier,
            chain_id: $ctx.chain_id,
            block_height: $ctx.block.height,
            block_timestamp: $ctx.block.timestamp,
            block_hash: $ctx.block.hash,
            contract: $ctx.contract,
            simulate: $ctx.simulate.unwrap(),
        }
    };
}

// TODO: replace with https://doc.rust-lang.org/std/ops/trait.Try.html once stabilized
#[macro_export]
macro_rules! try_into_generic_result {
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
macro_rules! try_unwrap_field {
    ($field:expr, $name:literal) => {
        match $field {
            Some(field) => field,
            None => {
                return Err(StdError::missing_context($name)).into();
            },
        }
    }
}

#[macro_export]
macro_rules! make_immutable_ctx {
    ($ctx:ident) => {
        ImmutableCtx {
            store:           &ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
        }
    }
}

#[macro_export]
macro_rules! make_mutable_ctx {
    ($ctx:ident) => {
        MutableCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
            sender:          try_unwrap_field!($ctx.sender, "sender"),
            funds:           try_unwrap_field!($ctx.funds, "funds"),
        }
    }
}

#[macro_export]
macro_rules! make_sudo_ctx {
    ($ctx:ident) => {
        SudoCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
        }
    }
}

#[macro_export]
macro_rules! make_auth_ctx {
    ($ctx:ident) => {
        AuthCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
            simulate:        try_unwrap_field!($ctx.simulate, "simulate"),
        }
    }
}

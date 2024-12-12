use {
    crate::{
        call_in_1_out_1, catch_event, AppError, EventResult, GasTracker, Vm, CHAIN_ID,
        CLIENT_STATES, CONFIG, CONSENSUS_STATES, CONTRACTS, NEXT_CLIENT_ID,
    },
    grug_types::{
        BlockInfo, Context, EvtCreateClient, GenericResult, IbcClientQuery, IbcClientQueryResponse,
        MsgCreateClient, Storage,
    },
};

pub fn do_create_client<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg: MsgCreateClient,
) -> EventResult<EvtCreateClient>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_create_client(vm, storage, gas_tracker, block, msg);

    #[cfg(feature = "tracing")]
    evt.debug(
        |evt| {
            tracing::info!(
                client_type = evt.client_type.as_str(),
                client_id = evt.client_id.unwrap().to_string(),
                "Created IBC client"
            );
        },
        "Failed to create IBC client",
    );

    evt
}

pub fn _do_create_client<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg: MsgCreateClient,
) -> EventResult<EvtCreateClient>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtCreateClient::base(msg.client_type);

    let (chain_id, client_id, client_impl, code_hash) = catch_event! {
        // TODO: use `load_with_gas` for these
        {
            let cfg = CONFIG.load(&storage)?;
            let chain_id = CHAIN_ID.load(&storage)?;
            let (client_id, _) = NEXT_CLIENT_ID.increment(&mut storage)?;
            let client_impl = cfg.client_impls.get(msg.client_type);
            let contract = CONTRACTS.load(&storage, client_impl)?;

            evt.client_id = Some(client_id);

            Ok((chain_id, client_id, client_impl, contract.code_hash))
        },
        evt
    };

    let ctx = Context {
        chain_id,
        block,
        contract: client_impl,
        sender: None,
        funds: None,
        mode: None,
    };

    // Call the client impl contract to verify the client/consensus states,
    // encode them, and return the latest consensus height.
    let (latest_height, raw_client_state, raw_consensus_state) =
        match call_in_1_out_1::<_, _, GenericResult<IbcClientQueryResponse>>(
            vm,
            storage.clone(),
            gas_tracker,
            0,
            false,
            "ibc_client_query",
            code_hash,
            &ctx,
            &IbcClientQuery::VerifyCreation {
                client_state: msg.client_state,
                consensus_state: msg.consensus_state,
            },
        ) {
            // TODO: simplify the syntax here. hide it behind a macro?
            Ok(GenericResult::Ok(res)) => res.as_verify_creation(),
            Ok(GenericResult::Err(msg)) => {
                return EventResult::Err {
                    event: evt,
                    error: AppError::Guest {
                        address: client_impl,
                        name: "ibc_client_query",
                        msg,
                    },
                };
            },
            Err(err) => {
                return EventResult::Err {
                    event: evt,
                    error: err,
                };
            },
        };

    // Commit the client state and consensus state.
    // TODO: use the proper client/consensus state key types.
    // TODO: use `save_with_gas` for these.
    catch_event! {
        {
            CLIENT_STATES.save(&mut storage, client_id, &raw_client_state)?;
            CONSENSUS_STATES.save(&mut storage, (client_id, latest_height), &raw_consensus_state)?;

            Ok(())
        },
        evt
    }

    EventResult::Ok(evt)
}

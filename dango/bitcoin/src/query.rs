use {
    crate::{CONFIG, NEXT_OUTBOUND_ID, OUTBOUND_QUEUE, OUTBOUNDS, SIGNATURES, UTXOS},
    dango_types::bitcoin::{BitcoinAddress, BitcoinSignature, Config, QueryMsg, Transaction, Utxo},
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, HexBinary, ImmutableCtx, Inner, Json, JsonSerExt, Order,
        StdResult, Storage, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Utxos {
            start_after,
            limit,
            order,
        } => {
            let res = query_utxos(ctx.storage, start_after, limit, order)?;
            res.to_json_value()
        },
        QueryMsg::OutboundQueue { start_after, limit } => {
            let res = query_outbound_queue(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::LastOutboundTransactionId {} => {
            let res = query_last_outbound_transaction_id(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::OutboundTransaction { id } => {
            let res = query_outbound_transaction(ctx.storage, id)?;
            res.to_json_value()
        },
        QueryMsg::OutboundTransactions { start_after, limit } => {
            let res = query_outbound_transactions(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::OutboundSignature { id } => {
            let res = query_outbound_singnature(ctx.storage, id)?;
            res.to_json_value()
        },
        QueryMsg::OutboundSignatures { start_after, limit } => {
            let res = query_outbound_singnatures(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

fn query_utxos(
    storage: &dyn Storage,
    start_after: Option<Utxo>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<Vec<Utxo>> {
    let start = if let Some(utxo) = start_after {
        Some((utxo.amount, utxo.transaction_hash, utxo.vout))
    } else {
        None
    };
    let start = start.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    UTXOS
        .keys(storage, start, None, order)
        .take(limit)
        .map(|res| {
            let (amount, transaction_hash, vout) = res?;
            Ok(Utxo {
                transaction_hash,
                vout,
                amount,
            })
        })
        .collect()
}

fn query_outbound_queue(
    storage: &dyn Storage,
    start_after: Option<BitcoinAddress>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<BitcoinAddress, Uint128>> {
    let start = start_after
        .as_ref()
        .map(|address| address.inner().as_slice());

    let start = start.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    OUTBOUND_QUEUE
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(recipient, amount)| (HexBinary::from_inner(recipient), amount)))
        .collect()
}

fn query_last_outbound_transaction_id(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_OUTBOUND_ID.current(storage)
}

fn query_outbound_transaction(storage: &dyn Storage, id: u32) -> StdResult<Transaction> {
    OUTBOUNDS.load(storage, id)
}

fn query_outbound_transactions(
    storage: &dyn Storage,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<u32, Transaction>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    OUTBOUNDS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_outbound_singnature(
    storage: &dyn Storage,
    id: u32,
) -> StdResult<BTreeMap<Addr, Vec<BitcoinSignature>>> {
    SIGNATURES.load(storage, id)
}

fn query_outbound_singnatures(
    storage: &dyn Storage,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<u32, BTreeMap<Addr, Vec<BitcoinSignature>>>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    SIGNATURES
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

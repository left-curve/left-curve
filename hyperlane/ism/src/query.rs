use {
    crate::VALIDATOR_SETS,
    anyhow::ensure,
    grug::{
        Bound, HashExt, HexBinary, HexByteArray, ImmutableCtx, Json, JsonSerExt, Order, StdResult,
    },
    hyperlane_types::{
        domain_hash, eip191_hash,
        ism::{Metadata, QueryMsg, ValidatorSet},
        mailbox::{Domain, Message},
        multisig_hash,
    },
    std::collections::{BTreeMap, BTreeSet},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::ValidatorSet { domain } => {
            let res = query_validaor_set(ctx, domain)?;
            res.to_json_value()
        },
        QueryMsg::ValidatorSets { start_after, limit } => {
            let res = query_validator_sets(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Verify {
            raw_message,
            metadata,
        } => {
            verify(ctx, raw_message, metadata)?;
            ().to_json_value()
        },
    }
    .map_err(Into::into)
}

#[inline]
fn query_validaor_set(ctx: ImmutableCtx, domain: Domain) -> StdResult<ValidatorSet> {
    VALIDATOR_SETS.load(ctx.storage, domain)
}

#[inline]
fn query_validator_sets(
    ctx: ImmutableCtx,
    start_after: Option<Domain>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Domain, ValidatorSet>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    VALIDATOR_SETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

#[inline]
fn verify(ctx: ImmutableCtx, raw_message: HexBinary, metadata: HexBinary) -> anyhow::Result<()> {
    let message = Message::decode(&raw_message)?;
    let metadata = Metadata::decode(&metadata)?;

    let multisig_hash = eip191_hash(multisig_hash(
        domain_hash(message.origin_domain, metadata.origin_merkle_tree),
        metadata.merkle_root,
        metadata.merkle_index,
        raw_message.keccak256(),
    ));

    let validators = metadata
        .signatures
        .into_iter()
        .map(|packed| {
            let signature = &packed[..64];
            let recovery_id = packed[65];

            let recovered_address = ctx
                .api
                .secp256k1_pubkey_recover(&multisig_hash, &signature, recovery_id, true)?
                .keccak256()[12..]
                .try_into()
                .unwrap();

            Ok(HexByteArray::from_inner(recovered_address))
        })
        .collect::<StdResult<BTreeSet<_>>>()?;

    let validator_set = VALIDATOR_SETS.load(ctx.storage, message.origin_domain)?;

    ensure!(
        validators.len() >= validator_set.threshold as usize,
        "not enough signatures! expecting at least {}, got {}",
        validator_set.threshold,
        validators.len()
    );

    ensure!(
        validators.is_subset(&validator_set.validators),
        "recovered addresses is not a strict subset of the validator set"
    );

    Ok(())
}

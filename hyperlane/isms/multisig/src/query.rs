use {
    crate::VALIDATOR_SETS,
    anyhow::ensure,
    grug::{
        Bound, DEFAULT_PAGE_LIMIT, HashExt, HexByteArray, ImmutableCtx, Json, JsonSerExt, Order,
        StdResult,
    },
    hyperlane_types::{
        domain_hash, eip191_hash,
        isms::{
            HYPERLANE_DOMAIN_KEY, IsmQuery, IsmQueryResponse,
            multisig::{Metadata, QueryMsg, ValidatorSet},
        },
        mailbox::{Domain, Message},
        multisig_hash,
    },
    std::collections::{BTreeMap, BTreeSet},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::ValidatorSet { domain } => {
            let res = query_validator_set(ctx, domain)?;
            res.to_json_value()
        },
        QueryMsg::ValidatorSets { start_after, limit } => {
            let res = query_validator_sets(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Ism(IsmQuery::Verify {
            raw_message,
            raw_metadata,
        }) => {
            let res = IsmQueryResponse::Verify(verify(ctx, &raw_message, &raw_metadata)?);
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

#[inline]
fn query_validator_set(ctx: ImmutableCtx, domain: Domain) -> StdResult<ValidatorSet> {
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

fn verify(ctx: ImmutableCtx, raw_message: &[u8], raw_metadata: &[u8]) -> anyhow::Result<()> {
    let message = Message::decode(raw_message)?;
    let metadata = Metadata::decode(raw_metadata)?;

    // This is the hash that validators are supposed to sign.
    let multisig_hash = eip191_hash(multisig_hash(
        domain_hash(
            message.origin_domain,
            metadata.origin_merkle_tree,
            HYPERLANE_DOMAIN_KEY,
        ),
        metadata.merkle_root,
        metadata.merkle_index,
        raw_message.keccak256(),
    ));

    // Loop through the signatures and recover the addresses.
    let validators = metadata
        .signatures
        .into_iter()
        .map(|signature| {
            let pk = ctx.api.secp256k1_pubkey_recover(
                &multisig_hash,
                &signature[..64],
                signature[64] - 27, // Ethereum uses recovery IDs 27, 28 instead of 0, 1.
                false,              // We need the _uncompressed_ public key for deriving address!
            )?;
            let pk_hash = ctx.api.keccak256(&pk[1..]);
            let address = &pk_hash[12..];

            Ok(HexByteArray::from_inner(address.try_into().unwrap()))
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

// ----------------------------------- tests -----------------------------------

// Adapted from:
// https://github.com/many-things/cw-hyperlane/blob/d07e55e17c791a5f6557f114e3fb6cb433d9b800/contracts/isms/multisig/src/query.rs#L111-L190
#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{MockContext, ResultExt, btree_set, hash},
        hex_literal::hex,
        hyperlane_types::{addr32, mailbox::MAILBOX_VERSION},
        test_case::test_case,
    };

    #[test_case(
        hex!("0000000000000068220000000000000000000000000d1255b09d94659bb0888e0aa9fca60245ce402a0000682155208cd518cffaac1b5d8df216a9bd050c9a03f0d4f3ba88e5268ac4cd12ee2d68656c6c6f"),
        hex!("986a1625d44e4b3969b08a5876171b2b4fcdf61b3e5c70a86ad17b304f17740a9f45d99ea6bec61392a47684f4e5d1416ddbcb5fdef0f132c27d7034e9bbff1c00000000ba9911d78ec6d561413e3589f920388cbd7554fbddd8ce50739337250853ec3577a51fa40e727c05b50f15db13f5aad5857c89d432644be48d70325ea83fdb6c1c"),
        btree_set! {
            HexByteArray::from_inner(hex!("122e0663ccc190266427e7fc0ed6589b5d7d36db")),
            HexByteArray::from_inner(hex!("01d7525e91dfc3f594fd366aad70f956b398de9e")),
        };
        "one signature"
    )]
    #[case(
        hex!("03000000240001388100000000000000000000000004980c17e2ce26578c82f81207e706e4505fae3b0000a8690000000000000000000000000b1c1b54f45e02552331d3106e71f5e0b573d5d448656c6c6f21"),
        hex!("0000000000000000000000009af85731edd41e2e50f81ef8a0a69d2fb836edf9a84430f822e0e9b5942faace72bd5b97f0b59a58a9b8281231d9e5c393b5859c00000024539feceace17782697e29e74151006dc7b47227cf48aba02926336cb5f7fa38b3d05e8293045f7b5811eda3ae8aa070116bb5fbf57c79e143a69e909df90cefa1b6e6ead7180e0415c36642ee4bc5454bc4f5ca250ca77a1a83562035544e0e898734d6541a20404e05fd53eb1c75b0bd21851c3bd8122cfa3550d7b6fb94d7cee1b"),
        btree_set!{
            HexByteArray::from_inner(hex!("ebc301013b6cd2548e347c28d2dc43ec20c068f2")),
            HexByteArray::from_inner(hex!("315db9868fc8813b221b1694f8760ece39f45447")),
            HexByteArray::from_inner(hex!("17517c98358c5937c5d9ee47ce1f5b4c2b7fc9f5")),
        };
        "two signatures"
    )]
    fn verify_with_e2e_data<const A: usize, const B: usize>(
        raw_message: [u8; A],
        raw_metadata: [u8; B],
        validators: BTreeSet<HexByteArray<20>>,
    ) {
        let mut ctx = MockContext::new();
        let mut message = Message::decode(&raw_message).unwrap();

        VALIDATOR_SETS
            .save(&mut ctx.storage, message.origin_domain, &ValidatorSet {
                threshold: 1,
                validators,
            })
            .unwrap();

        verify(ctx.as_immutable(), &raw_message, &raw_metadata).should_succeed();

        // Try forging a false message. Verification should fail.
        message.body = b"larry".to_vec().into();

        verify(ctx.as_immutable(), &message.encode(), &raw_metadata).should_fail();
    }

    #[test]
    fn rejecting_reuse_of_signature() {
        let validators = btree_set! {
            HexByteArray::from_inner(hex!("ebc301013b6cd2548e347c28d2dc43ec20c068f2")),
            HexByteArray::from_inner(hex!("315db9868fc8813b221b1694f8760ece39f45447")),
            HexByteArray::from_inner(hex!("17517c98358c5937c5d9ee47ce1f5b4c2b7fc9f5")),
        };

        let message = Message {
            version: MAILBOX_VERSION,
            nonce: 36,
            origin_domain: 80001,
            sender: addr32!("00000000000000000000000004980c17e2ce26578c82f81207e706e4505fae3b"),
            destination_domain: 43113,
            recipient: addr32!("00000000000000000000000004980c17e2ce26578c82f81207e706e4505fae3b"),
            body: hex!("48656c6c6f21").to_vec().into(),
        };

        let metadata = Metadata {
            origin_merkle_tree: addr32!(
                "0000000000000000000000009af85731edd41e2e50f81ef8a0a69d2fb836edf9"
            ),
            merkle_root: hash!("a84430f822e0e9b5942faace72bd5b97f0b59a58a9b8281231d9e5c393b5859c"),
            merkle_index: 36,
            signatures: btree_set! {
                // Valid signature but used twice.
                HexByteArray::from_inner(hex!(
                    "539feceace17782697e29e74151006dc7b47227cf48aba02926336cb5f7fa38b3d05e8293045f7b5811eda3ae8aa070116bb5fbf57c79e143a69e909df90cefa1b"
                )),
                HexByteArray::from_inner(hex!(
                    "539feceace17782697e29e74151006dc7b47227cf48aba02926336cb5f7fa38b3d05e8293045f7b5811eda3ae8aa070116bb5fbf57c79e143a69e909df90cefa1b"
                )),
            },
        };

        let mut ctx = MockContext::new();

        VALIDATOR_SETS
            .save(&mut ctx.storage, message.origin_domain, &ValidatorSet {
                threshold: 2,
                validators,
            })
            .unwrap();

        verify(ctx.as_immutable(), &message.encode(), &metadata.encode())
            .should_fail_with_error("not enough signatures! expecting at least 2, got 1");
    }
}

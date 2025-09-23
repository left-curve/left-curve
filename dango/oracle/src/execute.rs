use {
    crate::{PRICE_SOURCES, PYTH_PRICES, PYTH_TRUSTED_SIGNERS},
    anyhow::{bail, ensure},
    dango_types::oracle::{ExecuteMsg, InstantiateMsg, PrecisionlessPrice, PriceSource},
    grug::{
        Api, AuthCtx, AuthMode, AuthResponse, Binary, Denom, Inner, JsonDeExt, Message, MsgExecute,
        MutableCtx, QuerierExt, Response, Storage, Timestamp, Tx,
    },
    pyth_types::{LeEcdsaMessage, PayloadData, PriceUpdate},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (denom, price_source) in msg.price_sources {
        PRICE_SOURCES.save(ctx.storage, &denom, &price_source)?;
    }

    for (public_key, expires_at) in msg.trusted_signers {
        PYTH_TRUSTED_SIGNERS.save(ctx.storage, &public_key, &expires_at)?;
    }

    Ok(Response::new())
}

/// The oracle can be used as sender when:
///
/// - Auth mode must be `Finalize`. This ensures such transactions are only
///   inserted by the block proposer during ABCI++ `PrepareProposal`, not by
///   regular users.
/// - The transaction contains exactly one message.
/// - This one message is an `Execute`.
/// - The contract being executed must be the oracle itself.
/// - the execute message must be `FeedPrices`.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    // Authenticate can only be called during finalize.
    ensure!(
        ctx.mode == AuthMode::Finalize,
        "you don't have the right, O you don't have the right"
    );

    let mut msgs = tx.msgs.iter();

    // Assert the transaction contains exactly 1 MsgExecute.
    let (Some(Message::Execute(MsgExecute { contract, msg, .. })), None) =
        (msgs.next(), msgs.next())
    else {
        bail!("transaction must contain exactly one message");
    };

    // Assert the contract is the oracle.
    ensure!(contract == ctx.contract, "contract must be the oracle");

    // Assert the message is `ExecuteMsg::FeedPrices`.
    let Ok(ExecuteMsg::FeedPrices(..)) = msg.clone().deserialize_json() else {
        bail!("the execute message must be feed prices");
    };

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::RegisterPriceSources(price_sources) => {
            register_price_sources(ctx, price_sources)
        },
        ExecuteMsg::RegisterTrustedSigner {
            public_key,
            expires_at,
        } => register_trusted_signer(ctx, public_key, expires_at),
        ExecuteMsg::RemoveTrustedSigner { public_key } => remove_trusted_signer(ctx, public_key),
        ExecuteMsg::FeedPrices(price_update) => feed_prices(ctx, price_update),
    }
}

fn register_price_sources(
    ctx: MutableCtx,
    price_sources: BTreeMap<Denom, PriceSource>,
) -> anyhow::Result<Response> {
    // Only chain owner can register a denom.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    for (denom, price_source) in price_sources {
        PRICE_SOURCES.save(ctx.storage, &denom, &price_source)?;
    }

    Ok(Response::new())
}

fn register_trusted_signer(
    ctx: MutableCtx,
    public_key: Binary,
    expires_at: Timestamp,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    PYTH_TRUSTED_SIGNERS.save(ctx.storage, &public_key, &expires_at)?;

    Ok(Response::new())
}

fn remove_trusted_signer(ctx: MutableCtx, public_key: Binary) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    PYTH_TRUSTED_SIGNERS.remove(ctx.storage, &public_key);

    Ok(Response::new())
}

fn feed_prices(ctx: MutableCtx, price_update: PriceUpdate) -> anyhow::Result<Response> {
    for message in price_update.into_inner() {
        verify_pyth_lazer_message(ctx.storage, ctx.block.timestamp, ctx.api, &message)?;

        // Deserialize the payload.
        let payload = PayloadData::deserialize_slice_le(&message.payload)?;
        let timestamp = Timestamp::from_micros(payload.timestamp_us.as_micros().into());

        // Store the prices from each feed.
        for feed in payload.feeds {
            let id = feed.feed_id.0;
            let price = PrecisionlessPrice::try_from((feed, timestamp))?;

            PYTH_PRICES.may_update(ctx.storage, id, |current| -> anyhow::Result<_> {
                match current {
                    Some(current_price) if current_price.timestamp > timestamp => Ok(current_price),
                    _ => Ok(price),
                }
            })?;
        }
    }

    Ok(Response::new())
}

fn verify_pyth_lazer_message(
    storage: &dyn Storage,
    current_time: Timestamp,
    api: &dyn Api,
    message: &LeEcdsaMessage,
) -> anyhow::Result<()> {
    // Recover the signer public key from the message.
    let pk = api.secp256k1_pubkey_recover(
        &api.keccak256(&message.payload),
        &message.signature,
        message.recovery_id,
        true,
    )?;

    // Ensure the signer is trusted.
    match PYTH_TRUSTED_SIGNERS.may_load(storage, &pk)? {
        Some(expiration) => {
            ensure!(
                expiration > current_time,
                "signer is no longer trusted! public key: {}, expired at: {}, current time: {}",
                hex::encode(&pk),
                expiration.to_rfc3339_string(),
                current_time.to_rfc3339_string()
            );
        },
        None => bail!("signer is not trusted! public key: {}", hex::encode(&pk)),
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{Binary, Duration, EncodedBytes, MockApi, MockStorage, ResultExt},
        pyth_types::{LeEcdsaMessage, constants::LAZER_TRUSTED_SIGNER},
        std::str::FromStr,
    };

    #[test]
    fn test_verify_pyth_lazer_message() {
        let mut storage = MockStorage::default();
        let api = MockApi;
        let current_time = Timestamp::from_seconds(1000);

        let trusted_signer = Binary::from_str(LAZER_TRUSTED_SIGNER).unwrap();

        let message = LeEcdsaMessage {
            payload: vec![
                117, 211, 199, 147, 144, 174, 214, 146, 181, 60, 6, 0, 1, 2, 1, 0, 0, 0, 1, 0, 212,
                148, 165, 115, 126, 10, 0, 0, 2, 0, 0, 0, 1, 0, 177, 175, 142, 195, 99, 0, 0, 0,
            ],
            signature: EncodedBytes::from_inner([
                52, 175, 197, 246, 133, 14, 148, 65, 91, 0, 180, 102, 248, 223, 46, 31, 118, 26,
                20, 175, 7, 25, 83, 195, 13, 207, 197, 56, 214, 149, 21, 131, 122, 198, 58, 56, 87,
                57, 92, 85, 12, 226, 100, 89, 148, 98, 146, 187, 168, 111, 67, 248, 246, 131, 53,
                107, 143, 164, 144, 23, 112, 196, 10, 250,
            ]),
            recovery_id: 0,
        };

        verify_pyth_lazer_message(&storage, current_time, &api, &message.clone())
            .should_fail_with_error("signer is not trusted");

        // Store trusted signer to storage with timestamp in the past.
        PYTH_TRUSTED_SIGNERS
            .save(
                &mut storage,
                &trusted_signer,
                &(current_time - Duration::from_seconds(60)), // 1 minute ago
            )
            .unwrap();

        verify_pyth_lazer_message(&storage, current_time, &api, &message.clone())
            .should_fail_with_error("signer is no longer trusted");

        // Store trusted signer to storage with timestamp in the future.
        PYTH_TRUSTED_SIGNERS
            .save(
                &mut storage,
                &trusted_signer,
                &(current_time + Duration::from_seconds(60)), // 1 minute from now
            )
            .unwrap();

        verify_pyth_lazer_message(&storage, current_time, &api, &message).should_succeed();
    }
}

use {
    crate::{OracleTestEntry, TestSuite},
    byteorder::LE,
    dango_order_book::UsdPrice,
    dango_types::oracle::{self, PriceSource},
    grug_app::{AppError, Indexer, ProposalPreparer},
    grug_db_memory::MemDb,
    grug_math::Fraction,
    grug_types::{Addr, Binary, ByteArray, Coins, Denom, NonEmpty, ResultExt, Signer, Timestamp},
    grug_vm_rust::RustVm,
    identity::Identity256,
    k256::ecdsa::SigningKey,
    pyth_lazer_protocol::{
        ChannelId, Price as LazerPrice, PriceFeedId,
        api::MarketSession as LazerMarketSession,
        payload::{PayloadData, PayloadFeedData, PayloadPropertyValue},
        time::TimestampUs,
    },
    pyth_types::{Channel, LeEcdsaMessage, MarketSession, PythId},
    std::collections::BTreeMap,
};

const MOCK_SIGNER_SECRET: [u8; 32] = [0x42; 32];

struct MockPythSigner {
    signing_key: SigningKey,
}

impl MockPythSigner {
    fn new() -> Self {
        let signing_key =
            SigningKey::from_bytes(&MOCK_SIGNER_SECRET.into()).expect("valid secret key");
        Self { signing_key }
    }

    fn sign_prices(
        &self,
        feeds: &[(PythId, UsdPrice, MarketSession)],
        timestamp: Timestamp,
    ) -> LeEcdsaMessage {
        let timestamp_us = timestamp_to_micros(timestamp);

        let payload = PayloadData {
            timestamp_us: TimestampUs::from_micros(timestamp_us),
            channel_id: ChannelId::REAL_TIME,
            feeds: feeds
                .iter()
                .map(|&(id, price, market_session)| {
                    let raw = price.into_inner().numerator().0;
                    let mantissa: i64 = raw.try_into().expect("price mantissa overflows i64");

                    PayloadFeedData {
                        feed_id: PriceFeedId(id),
                        properties: vec![
                            PayloadPropertyValue::Price(Some(
                                LazerPrice::from_mantissa(mantissa).expect("zero price"),
                            )),
                            PayloadPropertyValue::Exponent(-6),
                            PayloadPropertyValue::MarketSession(match market_session {
                                MarketSession::Regular => LazerMarketSession::Regular,
                                MarketSession::Other => LazerMarketSession::Closed,
                            }),
                        ],
                    }
                })
                .collect(),
        };

        let mut payload_bytes = Vec::new();
        payload
            .serialize::<LE>(&mut payload_bytes)
            .expect("payload serialization");

        let hash = grug_crypto::keccak256(&payload_bytes);
        let (sig, recovery_id) = self
            .signing_key
            .sign_digest_recoverable(Identity256::from(hash))
            .expect("signing");

        LeEcdsaMessage {
            payload: Binary::from_inner(payload_bytes),
            signature: ByteArray::from_inner(sig.to_bytes().into()),
            recovery_id: recovery_id.to_byte(),
        }
    }
}

pub fn mock_pyth_trusted_signer() -> Binary {
    let sk = SigningKey::from_bytes(&MOCK_SIGNER_SECRET.into()).expect("valid secret key");
    let pk: ByteArray<33> = sk
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes()
        .as_ref()
        .try_into()
        .unwrap();
    Binary::from_inner(pk.to_vec())
}

fn timestamp_to_micros(t: Timestamp) -> u64 {
    let micros = t.into_nanos() / 1_000;
    micros.min(u64::MAX as u128) as u64
}

// ---- OracleExt extension trait ----

#[allow(async_fn_in_trait)]
pub trait OracleExt {
    async fn seed_oracle_prices(
        &mut self,
        owner: &mut (dyn Signer + Send + Sync),
        oracle: Addr,
        entries: BTreeMap<Denom, OracleTestEntry>,
    );

    async fn feed_oracle_prices(
        &mut self,
        owner: &mut (dyn Signer + Send + Sync),
        oracle: Addr,
        feeds: &[(PythId, UsdPrice, MarketSession)],
        timestamp: Option<Timestamp>,
    );
}

impl<PP, ID> OracleExt for TestSuite<MemDb, RustVm, PP, ID>
where
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<<PP as ProposalPreparer>::Error>,
{
    async fn seed_oracle_prices(
        &mut self,
        owner: &mut (dyn Signer + Send + Sync),
        oracle: Addr,
        entries: BTreeMap<Denom, OracleTestEntry>,
    ) {
        let price_sources: BTreeMap<Denom, PriceSource> = entries
            .iter()
            .map(|(denom, e)| {
                (denom.clone(), PriceSource {
                    id: e.pyth_id,
                    channel: Channel::RealTime,
                })
            })
            .collect();

        self.execute(
            owner,
            oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(price_sources),
            Coins::new(),
        )
        .await
        .should_succeed();

        let feeds: Vec<_> = entries
            .values()
            .map(|e| (e.pyth_id, e.humanized_price, MarketSession::Regular))
            .collect();

        self.feed_oracle_prices(owner, oracle, &feeds, None).await;
    }

    async fn feed_oracle_prices(
        &mut self,
        owner: &mut (dyn Signer + Send + Sync),
        oracle: Addr,
        feeds: &[(PythId, UsdPrice, MarketSession)],
        timestamp: Option<Timestamp>,
    ) {
        let signer = MockPythSigner::new();
        let timestamp = timestamp.unwrap_or(Timestamp::MAX);
        let message = signer.sign_prices(feeds, timestamp);

        self.execute(
            owner,
            oracle,
            &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message])),
            Coins::new(),
        )
        .await
        .should_succeed();
    }
}

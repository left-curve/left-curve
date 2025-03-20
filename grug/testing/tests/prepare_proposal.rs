use {
    grug_app::{AppError, NaiveProposalPreparer, ProposalPreparer},
    grug_testing::TestBuilder,
    grug_types::{
        Addr, Coins, Empty, Json, JsonSerExt, Message, NonEmpty, QuerierExt, QuerierWrapper,
        ResultExt, StdError, Tx, btree_map,
    },
    grug_vm_rust::ContractBuilder,
    prost::bytes::Bytes,
    std::str::FromStr,
    thiserror::Error,
};

mod mock_oracle {
    use {
        grug_storage::Map,
        grug_types::{
            AuthCtx, AuthResponse, Empty, ImmutableCtx, Json, JsonSerExt, MutableCtx, Order,
            QueryRequest, Response, StdResult, Tx,
        },
        serde::{Deserialize, Serialize},
        std::collections::BTreeMap,
    };

    pub const PRICES: Map<&str, f64> = Map::new("price");

    #[derive(Serialize, Deserialize)]
    pub enum ExecuteMsg {
        FeedPrices { prices: BTreeMap<String, f64> },
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryMsg {
        Prices {},
    }

    pub struct QueryPricesRequest {}

    impl QueryRequest for QueryPricesRequest {
        type Message = QueryMsg;
        type Response = BTreeMap<String, f64>;
    }

    impl From<QueryPricesRequest> for QueryMsg {
        fn from(_req: QueryPricesRequest) -> Self {
            QueryMsg::Prices {}
        }
    }

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn authenticate(_ctx: AuthCtx, _tx: Tx) -> StdResult<AuthResponse> {
        // In practice, the contract should make sure the transaction only
        // contains oracle updates.
        // In this test however, for simplicity, we skip this.
        Ok(AuthResponse::new())
    }

    pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
        match msg {
            ExecuteMsg::FeedPrices { prices } => {
                for (coingecko_id, price) in prices {
                    PRICES.save(ctx.storage, &coingecko_id, &price)?;
                }
            },
        }

        Ok(Response::new())
    }

    pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
        match msg {
            QueryMsg::Prices {} => PRICES
                .range(ctx.storage, None, None, Order::Ascending)
                .collect::<StdResult<BTreeMap<_, _>>>()?
                .to_json_value(),
        }
    }
}

#[derive(Debug, Error)]
enum CoingeckoPriceFeederError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl From<CoingeckoPriceFeederError> for AppError {
    fn from(err: CoingeckoPriceFeederError) -> Self {
        AppError::PrepareProposal(err.to_string())
    }
}

struct CoingeckoPriceFeeder;

impl ProposalPreparer for CoingeckoPriceFeeder {
    type Error = CoingeckoPriceFeederError;

    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        mut txs: Vec<Bytes>,
        max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error> {
        // Check whether the oracle address in app config has been set.
        // If not, then we skip.
        if let Some(oracle) = querier.query_app_config::<Json>()?.as_str() {
            let oracle = Addr::from_str(oracle)?;

            // Query the prices of a few coins from Coingecko.
            let prices = {
                // Use the following to query from Coingecko.
                // In practice, this often fails due to rate limiting. As such,
                // we comment this out and use a hardcoded mock data below instead.
                {
                    // reqwest::blocking::Client::builder()
                    //     .timeout(Duration::from_millis(5000))
                    //     .build()?
                    //     .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,harrypotterobamasonic10in&vs_currencies=usd")
                    //     .send()?
                    //     .json::<BTreeMap<String, Json>>()?
                    //     .into_iter()
                    //     .map(|(coingecko_id, vs_currencies)| {
                    //         let price = vs_currencies["usd"].as_f64().unwrap();
                    //         (coingecko_id, price)
                    //     })
                    //     .collect()
                }

                // The mock data.
                btree_map! {
                    "bitcoin".to_string() => 86706.,
                    "ethereum".to_string() => 2152.79,
                    "harrypotterobamasonic10in".to_string() => 0.071202,
                }
            };

            // Compose an oracle update transaction.
            let tx = Tx {
                sender: oracle,
                gas_limit: 1_000_000,
                msgs: NonEmpty::new_unchecked(vec![Message::execute(
                    oracle,
                    &mock_oracle::ExecuteMsg::FeedPrices { prices },
                    Coins::new(),
                )?]),
                data: Json::null(),
                credential: Json::null(),
            }
            .to_json_vec()?
            .into();

            // Insert the transaction to the beginning of the list.
            txs.insert(0, tx);
        }

        // Use the naive preparer to trim the txs to under the max bytes.
        Ok(NaiveProposalPreparer
            .prepare_proposal(querier, txs, max_tx_bytes)
            .unwrap())
    }
}

#[test]
fn prepare_proposal_works() {
    let (mut suite, mut accounts) = TestBuilder::new_with_pp(CoingeckoPriceFeeder)
        .add_account("larry", Coins::new())
        .set_owner("larry")
        .build();

    let oracle_code = ContractBuilder::new(Box::new(mock_oracle::instantiate))
        .with_authenticate(Box::new(mock_oracle::authenticate))
        .with_execute(Box::new(mock_oracle::execute))
        .with_query(Box::new(mock_oracle::query))
        .build();

    // Deploy the oracle contract.
    let oracle = suite
        .upload_and_instantiate(
            &mut accounts["larry"],
            oracle_code,
            &Empty {},
            "oracle",
            Some("oracle"),
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    // Set oracle contract address as app config.
    suite
        .configure(&mut accounts["larry"], None, Some(oracle))
        .should_succeed();

    // At this point, the feeder shouldn't have fed any price yet, because the
    // oracle address wasn't set in app config.
    suite
        .query_wasm_smart(oracle, mock_oracle::QueryPricesRequest {})
        .should_succeed_and(|prices| prices.is_empty());

    // Make an "empty" block.
    // The block should contain 1 transaction, the price feed inserted by the
    // proposal preparer.
    let outcomes = suite.make_empty_block().tx_outcomes;
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].result.is_ok());

    // Query oracle again. There should now be prices for bitcoin, etherum, and
    // harry potter obama sonic 10 inu.
    suite
        .query_wasm_smart(oracle, mock_oracle::QueryPricesRequest {})
        .should_succeed_and(|prices| {
            prices
                .keys()
                .eq(["bitcoin", "ethereum", "harrypotterobamasonic10in"])
        });
}

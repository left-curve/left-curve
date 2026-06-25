pub trait Variables {
    type Query: graphql_client::GraphQLQuery<Variables = Self>;
}

#[allow(clippy::upper_case_acronyms)]
type JSON = serde_json::Value;
type GrugQueryInput = serde_json::Value;
type UnsignedTx = serde_json::Value;
type Tx = serde_json::Value;
type DateTime = String;
type BigDecimal = String;
type NaiveDateTime = String;

/// Page info for cursor-based pagination in GraphQL responses.
///
/// This struct provides a common type for pagination metadata that can be
/// extracted from the various generated GraphQL query response types.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

// ----------------------------------- Types -----------------------------------

macro_rules! generate_types {
    // The query path is captured as `:tt` (a single token tree), not
    // `:literal`, on purpose. Since `graphql_client` 0.16, the
    // `GraphQLQuery` derive parses the `#[graphql(...)]` attribute by
    // manually walking the raw token stream, and requires the value of
    // `query_path` to be a bare literal token. A `:literal` metavariable is
    // interpolated wrapped in an invisible `Delimiter::None` group, which
    // that walker does not see through, so it reports `Attribute query_path
    // not found`. A `:tt` capture is interpolated transparently, so the
    // string literal arrives as the bare token the derive expects.
    ($({name: $name:ident, path: $path:tt $(,)?}), * $(,)? ) => {
        $(
            #[derive(graphql_client::GraphQLQuery)]
            #[graphql(
                schema_path = "src/schemas/schema.graphql",
                query_path = $path,
                response_derives = "Debug, Clone, PartialEq, Eq",
                variables_derives = "Debug, Clone, Default"
            )]
            pub struct $name;

            paste::paste! {
                impl Variables for [<$name:snake>]::Variables {
                    type Query = $name;
                }
            }
        )*
    };
}

generate_types! {
    {
        name: QueryApp,
        path: "src/schemas/queries/queryApp.graphql",
    },
    {
        name: QueryStore,
        path: "src/schemas/queries/queryStore.graphql",
    },
    {
        name: Simulate,
        path: "src/schemas/queries/simulate.graphql",
    },
    {
        name: BroadcastTxSync,
        path: "src/schemas/mutations/broadcastTxSync.graphql",
    },
    {
        name: SearchTx,
        path: "src/schemas/queries/transaction.graphql",
    },
    {
        name: Block,
        path: "src/schemas/queries/block.graphql",
    },
    {
        name: Blocks,
        path: "src/schemas/queries/blocks.graphql",
    },
    {
        name: Transactions,
        path: "src/schemas/queries/transactions.graphql",
    },
    {
        name: Messages,
        path: "src/schemas/queries/messages.graphql",
    },
    {
        name: Events,
        path: "src/schemas/queries/events.graphql",
    },
    {
        name: Transfers,
        path: "src/schemas/queries/transfers.graphql",
    },
    {
        name: Accounts,
        path: "src/schemas/queries/accounts.graphql",
    },
    {
        name: User,
        path: "src/schemas/queries/user.graphql",
    },
    {
        name: Users,
        path: "src/schemas/queries/users.graphql",
    },
    {
        name: PerpsCandles,
        path: "src/schemas/queries/perpsCandles.graphql",
    },
    {
        name: PerpsEvents,
        path: "src/schemas/queries/perpsEvents.graphql",
    },
    {
        name: PerpsPairStats,
        path: "src/schemas/queries/perpsPairStats.graphql",
    },
    {
        name: PerpsPairStatsPartial,
        path: "src/schemas/queries/perpsPairStatsPartial.graphql",
    },
    {
        name: AllPerpsPairStats,
        path: "src/schemas/queries/allPerpsPairStats.graphql",
    },
    {
        name: QueryStatus,
        path: "src/schemas/queries/queryStatus.graphql",
    }
}

// ---------------------------- Subscription types -----------------------------

macro_rules! generate_subscription_types {
    // `path` is captured as `:tt` rather than `:literal` for the same reason
    // as in `generate_types!` above (graphql_client 0.16 attribute parsing).
    ($({name: $name:ident, path: $path:tt $(,)?}), * $(,)? ) => {
        $(
            #[derive(graphql_client::GraphQLQuery)]
            #[graphql(
                schema_path = "src/schemas/schema.graphql",
                query_path = $path,
                response_derives = "Debug, Clone, PartialEq, Eq",
                variables_derives = "Debug, Clone, Default"
            )]
            pub struct $name;

            paste::paste! {
                impl Variables for [<$name:snake>]::Variables {
                    type Query = $name;
                }
            }
        )*
    };
}

generate_subscription_types! {
    {
        name: SubscribeBlock,
        path: "src/schemas/subscriptions/block.graphql",
    },
    {
        name: SubscribeFullBlock,
        path: "src/schemas/subscriptions/fullBlock.graphql",
    },
    {
        name: SubscribeAccounts,
        path: "src/schemas/subscriptions/accounts.graphql",
    },
    {
        name: SubscribeTransfers,
        path: "src/schemas/subscriptions/transfers.graphql",
    },
    {
        name: SubscribeTransactions,
        path: "src/schemas/subscriptions/transactions.graphql",
    },
    {
        name: SubscribeMessages,
        path: "src/schemas/subscriptions/messages.graphql",
    },
    {
        name: SubscribeEvents,
        path: "src/schemas/subscriptions/events.graphql",
    },
    {
        name: SubscribeEventByAddresses,
        path: "src/schemas/subscriptions/eventByAddresses.graphql",
    },
    {
        name: SubscribePerpsCandles,
        path: "src/schemas/subscriptions/perpsCandles.graphql",
    },
    {
        name: SubscribePerpsTrades,
        path: "src/schemas/subscriptions/perpsTrades.graphql",
    },
    {
        name: SubscribePerpsEvents2,
        path: "src/schemas/subscriptions/perpsEvents2.graphql",
    },
    {
        name: SubscribeQueryApp,
        path: "src/schemas/subscriptions/queryApp.graphql",
    },
    {
        name: SubscribeQueryStore,
        path: "src/schemas/subscriptions/queryStore.graphql",
    },
    {
        name: SubscribeQueryStatus,
        path: "src/schemas/subscriptions/queryStatus.graphql",
    },
}

// Re-export subscription modules
pub mod subscriptions {
    pub use super::{
        subscribe_accounts, subscribe_block, subscribe_event_by_addresses, subscribe_events,
        subscribe_full_block, subscribe_messages, subscribe_perps_candles, subscribe_perps_events2,
        subscribe_perps_trades, subscribe_query_app, subscribe_query_status, subscribe_query_store,
        subscribe_transactions, subscribe_transfers,
    };
}

// --------------------- Implement Default for enum types ----------------------

impl Default for perps_candles::CandleInterval {
    fn default() -> Self {
        Self::ONE_MINUTE
    }
}

impl Default for subscribe_perps_candles::CandleInterval {
    fn default() -> Self {
        Self::ONE_MINUTE
    }
}

impl Default for subscribe_events::CheckValue {
    fn default() -> Self {
        Self::EQUAL
    }
}

pub use indexer_graphql_types::{
    Accounts, Blocks, Events, Messages, PageInfo, Transactions, Transfers,
    accounts as accounts_query, blocks as blocks_query, events as events_query,
    messages as messages_query, transactions as transactions_query, transfers as transfers_query,
};

/// Direction for pagination.
#[derive(Clone, Copy)]
pub enum PaginationDirection {
    /// Paginate forward using `first` and `after`
    Forward,
    /// Paginate backward using `last` and `before`
    Backward,
}

/// Generic macro to generate pagination test helpers for GraphQL queries.
///
/// This is the base macro that can be used with any context type by providing
/// an app builder expression. Use `impl_indexer_paginate!` for the common case
/// with `indexer_httpd::context::FullContext`.
///
/// # Arguments
///
/// * `$fn_name` - The name of the generated function
/// * `$context_type` - The context type (e.g., `indexer_httpd::context::FullContext`)
/// * `$query_type` - The GraphQL query type (e.g., `Blocks`)
/// * `$module` - The module containing the query types (e.g., `indexer_graphql_types::blocks`)
/// * `$field` - The response field name (e.g., `blocks`)
/// * `$node_type` - The node type returned by the query
/// * `$app_builder` - Expression to build the app from context
#[macro_export]
macro_rules! impl_paginate {
    ($fn_name:ident, $context_type:ty, $query_type:ty, $module:ident, $field:ident, $node_type:ident, $app_builder:expr) => {
        /// Paginate through all results using the actix test context.
        ///
        /// # Arguments
        ///
        /// * `context` - The httpd context
        /// * `page_size` - Number of items to fetch per page
        /// * `variables` - Query variables (pagination fields will be overwritten)
        /// * `direction` - Pagination direction: `Forward` or `Backward`
        pub async fn $fn_name(
            context: $context_type,
            page_size: i64,
            mut variables: $module::Variables,
            direction: $crate::PaginationDirection,
        ) -> anyhow::Result<Vec<$module::$node_type>> {
            use graphql_client::GraphQLQuery;

            let mut all_items = vec![];
            let mut after: Option<String> = None;
            let mut before: Option<String> = None;

            loop {
                variables.after = after.clone();
                variables.before = before.clone();

                match direction {
                    $crate::PaginationDirection::Forward => {
                        variables.first = Some(page_size);
                        variables.last = None;
                    },
                    $crate::PaginationDirection::Backward => {
                        variables.first = None;
                        variables.last = Some(page_size);
                    },
                }

                let ctx = context.clone();
                let app = $app_builder(ctx);
                let query_body = <$query_type>::build_query(variables.clone());
                let response = $crate::call_graphql_query::<_, $module::ResponseData, _, _, _>(
                    app, query_body,
                )
                .await?;

                let data = response.data.expect("GraphQL response should have data");
                let connection = data.$field;
                let page_info = $crate::PageInfo {
                    start_cursor: connection.page_info.start_cursor,
                    end_cursor: connection.page_info.end_cursor,
                    has_next_page: connection.page_info.has_next_page,
                    has_previous_page: connection.page_info.has_previous_page,
                };

                match direction {
                    $crate::PaginationDirection::Forward => {
                        all_items.extend(connection.nodes);
                        if !page_info.has_next_page {
                            break;
                        }
                        after = page_info.end_cursor;
                    },
                    $crate::PaginationDirection::Backward => {
                        all_items.extend(connection.nodes.into_iter().rev());
                        if !page_info.has_previous_page {
                            break;
                        }
                        before = page_info.start_cursor;
                    },
                }
            }

            Ok(all_items)
        }
    };
}

/// Convenience macro for indexer_httpd context pagination.
///
/// This wraps `impl_paginate!` with the standard indexer app builder.
#[macro_export]
macro_rules! impl_indexer_paginate {
    ($fn_name:ident, $query_type:ty, $module:ident, $field:ident, $node_type:ident) => {
        $crate::impl_paginate!(
            $fn_name,
            indexer_httpd::context::FullContext,
            $query_type,
            $module,
            $field,
            $node_type,
            $crate::build_app_service
        );
    };
}

// Generate pagination helpers for common query types
impl_indexer_paginate!(
    paginate_blocks,
    Blocks,
    blocks_query,
    blocks,
    BlocksBlocksNodes
);
impl_indexer_paginate!(
    paginate_events,
    Events,
    events_query,
    events,
    EventsEventsNodes
);
impl_indexer_paginate!(
    paginate_messages,
    Messages,
    messages_query,
    messages,
    MessagesMessagesNodes
);
impl_indexer_paginate!(
    paginate_transactions,
    Transactions,
    transactions_query,
    transactions,
    TransactionsTransactionsNodes
);
impl_indexer_paginate!(
    paginate_accounts,
    Accounts,
    accounts_query,
    accounts,
    AccountsAccountsNodes
);
impl_indexer_paginate!(
    paginate_transfers,
    Transfers,
    transfers_query,
    transfers,
    TransfersTransfersNodes
);

use {
    crate::Variables,
    anyhow::bail,
    graphql_client::{GraphQLQuery, Response},
    graphql_ws_client::{Client, graphql::StreamingOperation},
    tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue},
    url::Url,
};

/// A WebSocket client for GraphQL subscriptions.
#[derive(Debug, Clone)]
pub struct WsClient {
    url: Url,
}

/// Type alias for the subscription stream returned by the client.
pub type SubscriptionStream<T> = std::pin::Pin<
    Box<dyn futures::Stream<Item = Result<Response<T>, graphql_ws_client::Error>> + Send>,
>;

impl WsClient {
    /// Create a new WebSocket client.
    ///
    /// The URL should be a WebSocket URL (ws:// or wss://) pointing to the GraphQL endpoint.
    pub fn new(url: impl Into<String>) -> Result<Self, anyhow::Error> {
        let url_str = url.into();
        let url = Url::parse(&url_str)?;

        // Validate the URL scheme
        match url.scheme() {
            "ws" | "wss" => {},
            scheme => bail!("Invalid URL scheme: {scheme}. Expected ws:// or wss://"),
        }

        Ok(Self { url })
    }

    /// Create a new WebSocket client from an HTTP URL.
    ///
    /// Converts http:// to ws:// and https:// to wss://.
    pub fn from_http_url(url: impl Into<String>) -> Result<Self, anyhow::Error> {
        let url_str = url.into();
        let mut url = Url::parse(&url_str)?;

        // Convert HTTP scheme to WebSocket scheme
        match url.scheme() {
            "http" => url
                .set_scheme("ws")
                .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?,
            "https" => url
                .set_scheme("wss")
                .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?,
            "ws" | "wss" => {},
            scheme => {
                bail!("Invalid URL scheme: {scheme}. Expected http://, https://, ws://, or wss://")
            },
        }

        Ok(Self { url })
    }

    /// Subscribe to a GraphQL subscription.
    ///
    /// Returns a stream of subscription responses wrapped in `graphql_client::Response`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use futures::StreamExt;
    /// use indexer_client::{WsClient, SubscribeTrades, subscribe_trades};
    ///
    /// let client = WsClient::new("ws://localhost:8080/graphql")?;
    ///
    /// let variables = subscribe_trades::Variables {
    ///     base_denom: "dango".to_string(),
    ///     quote_denom: "bridge/usdc".to_string(),
    /// };
    ///
    /// let mut stream = client.subscribe::<SubscribeTrades>(variables).await?;
    ///
    /// while let Some(response) = stream.next().await {
    ///     match response {
    ///         Ok(resp) => {
    ///             if let Some(data) = resp.data {
    ///                 println!("{:?}", data);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Error: {e}"),
    ///     }
    /// }
    /// ```
    pub async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<SubscriptionStream<Q::ResponseData>, anyhow::Error>
    where
        Q: GraphQLQuery + Unpin + Send + Sync + 'static,
        Q::Variables: Unpin + Send + Sync + 'static,
        Q::ResponseData: Unpin + Send + Sync + 'static,
    {
        let mut request = self.url.as_str().into_client_request()?;

        // Set the required WebSocket protocol header for GraphQL
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_static("graphql-transport-ws"),
        );

        #[cfg(feature = "tracing")]
        tracing::debug!("Connecting to WebSocket: {}", self.url);

        let (connection, _response) =
            tokio_tungstenite::connect_async(request)
                .await
                .map_err(|e| {
                    #[cfg(feature = "tracing")]
                    tracing::error!("WebSocket connection failed: {e}");
                    anyhow::anyhow!("WebSocket connection failed: {e}")
                })?;

        #[cfg(feature = "tracing")]
        tracing::debug!("WebSocket connected, building client");

        let subscription = Client::build(connection)
            .subscribe(StreamingOperation::<Q>::new(variables))
            .await
            .map_err(|e| anyhow::anyhow!("Subscription failed: {e}"))?;

        #[cfg(feature = "tracing")]
        tracing::debug!("Subscription established");

        Ok(Box::pin(subscription))
    }
}

/// Helper trait for subscription variables with an associated subscription type.
pub trait SubscriptionVariables: Variables {
    /// Subscribe using these variables.
    fn subscribe(
        self,
        client: &WsClient,
    ) -> impl std::future::Future<
        Output = Result<
            SubscriptionStream<<<Self as Variables>::Query as GraphQLQuery>::ResponseData>,
            anyhow::Error,
        >,
    > + Send
    where
        Self: Sized + Unpin + Send + Sync + 'static,
        <Self as Variables>::Query: Unpin + Send + Sync + 'static,
        <<Self as Variables>::Query as GraphQLQuery>::ResponseData: Unpin + Send + Sync + 'static,
    {
        client.subscribe::<Self::Query>(self)
    }
}

// Implement SubscriptionVariables for all subscription variable types
impl SubscriptionVariables for crate::subscribe_block::Variables {}
impl SubscriptionVariables for crate::subscribe_accounts::Variables {}
impl SubscriptionVariables for crate::subscribe_transfers::Variables {}
impl SubscriptionVariables for crate::subscribe_transactions::Variables {}
impl SubscriptionVariables for crate::subscribe_messages::Variables {}
impl SubscriptionVariables for crate::subscribe_events::Variables {}
impl SubscriptionVariables for crate::subscribe_event_by_addresses::Variables {}
impl SubscriptionVariables for crate::subscribe_candles::Variables {}
impl SubscriptionVariables for crate::subscribe_trades::Variables {}
impl SubscriptionVariables for crate::subscribe_query_app::Variables {}
impl SubscriptionVariables for crate::subscribe_query_store::Variables {}
impl SubscriptionVariables for crate::subscribe_query_status::Variables {}

// Re-export graphql_ws_client types that users might need
pub use graphql_ws_client::Error as WsError;

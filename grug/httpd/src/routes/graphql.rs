use {
    crate::{request_ip::RequesterIp, subscription_limiter::SubscriptionLimiter},
    actix_web::{HttpRequest, HttpResponse, Resource, web},
    async_graphql::{Data, Schema},
    async_graphql_actix_web::{GraphQLBatchRequest, GraphQLResponse, GraphQLSubscription},
    std::time::Duration,
};

pub fn graphql_route<Q, M, S>() -> Resource
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    web::resource("/graphql")
        .route(web::post().to(graphql_index::<Q, M, S>))
        .route(
            web::get()
                .guard(actix_web::guard::Header("upgrade", "websocket"))
                .to(graphql_ws::<Q, M, S>),
        )
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn graphql_index<Q, M, S>(
    schema: web::Data<Schema<Q, M, S>>,
    req: HttpRequest,
    gql_request: GraphQLBatchRequest,
) -> GraphQLResponse
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    let requester_ip = RequesterIp::from_request(&req);
    let details = requester_ip.clone().into_http_request_details();

    let request = gql_request.into_inner().data(details).data(requester_ip);

    schema.execute_batch(request).await.into()
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn graphql_ws<Q, M, S>(
    schema: web::Data<Schema<Q, M, S>>,
    req: HttpRequest,
    payload: web::Payload,
    global_limiter: web::Data<SubscriptionLimiter>,
) -> actix_web::Result<HttpResponse>
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    let mut subscription = GraphQLSubscription::new(Schema::clone(&*schema))
        .keepalive_timeout(Duration::from_secs(30));

    let mut data = Data::default();
    data.insert(global_limiter.new_connection());
    subscription = subscription.with_data(data);

    subscription.start(&req, payload)
}

#[cfg(feature = "metrics")]
use crate::metrics::GaugeGuard;
use {
    crate::{
        rate_limit::{GraphqlIpRateLimitRejection, GraphqlIpRateLimiter, GraphqlOperationCounts},
        request_ip::RequesterIp,
        subscription_limiter::SubscriptionLimiter,
    },
    actix_web::{
        HttpRequest, HttpResponse, Resource, Responder,
        http::StatusCode,
        web::{self, Data},
    },
    async_graphql::{BatchRequest, Data as GraphqlData, Schema},
    async_graphql_actix_web::{GraphQLBatchRequest, GraphQLResponse, GraphQLSubscription},
    std::{sync::Arc, time::Duration},
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
) -> HttpResponse
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    let requester_ip = RequesterIp::from_request(&req);
    let mut request = gql_request.into_inner();

    if let Some(response) = rate_limit_response(
        &req,
        requester_ip.remote_ip.as_deref(),
        GraphqlOperationCounts::from_batch_request(&mut request),
    ) {
        return response;
    }

    let request = add_requester_ip_data(request, requester_ip);
    GraphQLResponse::from(schema.execute_batch(request).await).respond_to(&req)
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
    let requester_ip = RequesterIp::from_request(&req);

    if let Some(response) = rate_limit_response(
        &req,
        requester_ip.remote_ip.as_deref(),
        GraphqlOperationCounts::subscription(),
    ) {
        return Ok(response);
    }

    let mut data = requester_ip_data(requester_ip);
    data.insert(global_limiter.new_connection());
    #[cfg(feature = "metrics")]
    data.insert(GaugeGuard::new(
        "graphql.websocket.connections.active",
        "graphql",
        "websocket",
    ));

    GraphQLSubscription::new(Schema::clone(&*schema))
        .with_data(data)
        .keepalive_timeout(Duration::from_secs(30))
        .start(&req, payload)
}

fn add_requester_ip_data(request: BatchRequest, requester_ip: RequesterIp) -> BatchRequest {
    request
        .data(requester_ip.clone().into_http_request_details())
        .data(requester_ip)
}

fn requester_ip_data(requester_ip: RequesterIp) -> GraphqlData {
    let mut data = GraphqlData::default();
    data.insert(requester_ip.clone().into_http_request_details());
    data.insert(requester_ip);
    data
}

fn rate_limit_response(
    req: &HttpRequest,
    ip: Option<&str>,
    counts: GraphqlOperationCounts,
) -> Option<HttpResponse> {
    let limiter = req
        .app_data::<Data<Arc<GraphqlIpRateLimiter>>>()
        .map(Data::get_ref)?;

    limiter
        .check(ip, counts)
        .err()
        .map(graphql_rate_limit_response)
}

fn graphql_rate_limit_response(_rejection: GraphqlIpRateLimitRejection) -> HttpResponse {
    let status = StatusCode::from_u16(420).unwrap_or(StatusCode::TOO_MANY_REQUESTS);

    #[cfg(feature = "tracing")]
    tracing::warn!(rejection = ?_rejection, "graphql request rejected by IP rate limiter");

    HttpResponse::build(status).body("graphql request rejected by IP rate limiter")
}

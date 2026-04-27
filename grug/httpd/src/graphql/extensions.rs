#[cfg(feature = "tracing")]
use {
    crate::request_ip::RequesterIp,
    async_graphql::{
        Request, Response, ServerResult,
        extensions::{
            Extension, ExtensionContext, ExtensionFactory, NextExecute, NextPrepareRequest,
        },
        parser::types::{DocumentOperations, OperationType},
    },
    std::{sync::Arc, time::Instant},
};

#[cfg(feature = "tracing")]
pub struct RequestLoggingExtension;

#[cfg(feature = "tracing")]
impl ExtensionFactory for RequestLoggingExtension {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(RequestLoggingExtension)
    }
}

#[cfg(feature = "tracing")]
#[async_trait::async_trait]
impl Extension for RequestLoggingExtension {
    async fn prepare_request(
        &self,
        ctx: &ExtensionContext<'_>,
        mut request: Request,
        next: NextPrepareRequest<'_>,
    ) -> ServerResult<Request> {
        let operation_name = request.operation_name.clone();

        if let Ok(doc) = request.parsed_query()
            && let Some(operation_type) =
                pick_operation_type(&doc.operations, operation_name.as_deref())
        {
            request = request.data(operation_type);
        }

        next.run(ctx, request).await
    }

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let start = Instant::now();
        let response = next.run(ctx, operation_name).await;
        let requester_ip = ctx.data::<RequesterIp>().ok();

        let operation_type = match ctx.data::<OperationType>() {
            Ok(OperationType::Query) => "query",
            Ok(OperationType::Mutation) => "mutation",
            Ok(OperationType::Subscription) => "subscription",
            Err(_) => "unknown",
        };

        tracing::info!(
            operation_name = operation_name.unwrap_or("anonymous"),
            operation_type,
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            error_count = response.errors.len(),
            remote_ip = requester_ip
                .and_then(|ip| ip.remote_ip.as_deref())
                .unwrap_or("-"),
            peer_ip = requester_ip
                .and_then(|ip| ip.peer_ip.as_deref())
                .unwrap_or("-"),
            x_forwarded_for = requester_ip
                .and_then(|ip| ip.x_forwarded_for.as_deref())
                .unwrap_or("-"),
            forwarded = requester_ip
                .and_then(|ip| ip.forwarded.as_deref())
                .unwrap_or("-"),
            cf_connecting_ip = requester_ip
                .and_then(|ip| ip.cf_connecting_ip.as_deref())
                .unwrap_or("-"),
            true_client_ip = requester_ip
                .and_then(|ip| ip.true_client_ip.as_deref())
                .unwrap_or("-"),
            x_real_ip = requester_ip
                .and_then(|ip| ip.x_real_ip.as_deref())
                .unwrap_or("-"),
            "graphql request completed"
        );

        response
    }
}

#[cfg(feature = "tracing")]
fn pick_operation_type(
    operations: &DocumentOperations,
    operation_name: Option<&str>,
) -> Option<OperationType> {
    match operations {
        DocumentOperations::Single(operation) => Some(operation.node.ty),
        DocumentOperations::Multiple(operations) if !operations.is_empty() => {
            if let Some(name) = operation_name
                && let Some(operation) = operations.get(name)
            {
                return Some(operation.node.ty);
            }

            operations
                .values()
                .next()
                .map(|operation| operation.node.ty)
        },
        _ => None,
    }
}

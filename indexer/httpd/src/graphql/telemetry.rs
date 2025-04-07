use async_graphql::{
    Response,
    extensions::{Extension, ExtensionContext, NextExecute},
};

#[derive(Default)]
pub struct SentryExtension;

#[async_trait::async_trait]
impl Extension for SentryExtension {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let res = next.run(ctx, operation_name).await;

        if res.is_err() {
            for err in &res.errors {
                let msg = format!("GraphQL error: {} | path: {:?}", err.message, err.path);
                sentry::capture_message(&msg, sentry::Level::Error);
            }
        }

        res
    }
}

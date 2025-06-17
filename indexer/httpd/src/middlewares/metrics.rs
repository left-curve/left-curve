use {
    actix_web::{
        Error,
        dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    },
    futures_util::future::LocalBoxFuture,
    metrics::{counter, describe_counter, describe_histogram, histogram},
    std::{
        future::{Ready, ready},
        time::Instant,
    },
};

pub struct HttpMetrics;

impl<S, B> Transform<S, ServiceRequest> for HttpMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<B>;
    type Transform = HttpMetricsMiddleware<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(HttpMetricsMiddleware { service }))
    }
}

/// Middleware for collecting HTTP metrics
pub struct HttpMetricsMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for HttpMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = ServiceResponse<B>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().as_str().to_string();
        let path = req.path().to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            counter!(
                "http.requests.total",
                "method" => method.clone(),
                "path" => path.clone(),
                "status" => res.status().as_u16().to_string()
            )
            .increment(1);

            histogram!(
                "http.request.duration.seconds",
                "method" => method,
                "path" => path,
                "status" => res.status().as_u16().to_string()
            )
            .record(start.elapsed().as_secs_f64());

            Ok(res)
        })
    }
}

pub fn init_httpd_metrics() {
    describe_counter!(
        "http.requests.total",
        "Total HTTP requests by method, path, and status"
    );
    describe_histogram!(
        "http.request.duration.seconds",
        "HTTP request duration in seconds by method, path, and status"
    );
}

use {
    actix_web::{
        Error, HttpResponse,
        dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    },
    futures_util::future::LocalBoxFuture,
    std::{
        future::{Ready, ready},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    },
};

/// Middleware that returns 503 Service Unavailable when the server is shutting down
pub struct ShutdownMiddleware {
    shutdown_flag: Arc<AtomicBool>,
}

impl ShutdownMiddleware {
    pub fn new(shutdown_flag: Arc<AtomicBool>) -> Self {
        Self { shutdown_flag }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ShutdownMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Transform = ShutdownMiddlewareService<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ShutdownMiddlewareService {
            service,
            shutdown_flag: self.shutdown_flag.clone(),
        }))
    }
}

pub struct ShutdownMiddlewareService<S> {
    service: S,
    shutdown_flag: Arc<AtomicBool>,
}

impl<S, B> Service<ServiceRequest> for ShutdownMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<ServiceResponse<actix_web::body::BoxBody>, Error>>;
    type Response = ServiceResponse<actix_web::body::BoxBody>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Check if we're shutting down
        if self.shutdown_flag.load(Ordering::Relaxed) {
            let response = HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Service is shutting down",
                "status": 503
            }));

            return Box::pin(async move {
                let (req, _) = req.into_parts();
                Ok(ServiceResponse::new(req, response))
            });
        }

        // Otherwise, proceed with the request
        let fut = self.service.call(req);
        Box::pin(async move { fut.await.map(|res| res.map_into_boxed_body()) })
    }
}

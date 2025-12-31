use {
    actix_web::{App, HttpResponse, Responder, test, web},
    grug_httpd::middlewares::shutdown::ShutdownMiddleware,
    std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

async fn test_handler() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[tokio::test]
async fn test_shutdown_middleware_returns_503_when_shutting_down() {
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    let app = test::init_service(
        App::new()
            .wrap(ShutdownMiddleware::new(shutdown_flag.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // First request should succeed
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Set shutdown flag
    shutdown_flag.store(true, Ordering::Relaxed);

    // Second request should return 503
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 503);

    // Verify the response body contains the expected error message
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Service is shutting down"));
    assert!(body_str.contains("503"));
}

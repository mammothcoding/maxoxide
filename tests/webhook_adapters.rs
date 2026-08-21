#![cfg(any(feature = "webhook-axum", feature = "webhook-actix"))]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use maxoxide::webhook::WebhookService;
use maxoxide::{Bot, Context, Dispatcher};

#[derive(Clone)]
struct Counter(Arc<AtomicUsize>);

fn service(counter: Arc<AtomicUsize>) -> WebhookService {
    let mut dispatcher = Dispatcher::new(Bot::new("token").unwrap()).with_state(Counter(counter));
    dispatcher.on(|context: Context| async move {
        context
            .state::<Counter>()
            .unwrap()
            .0
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    WebhookService::new(dispatcher)
        .secret("shared-secret")
        .unwrap()
}

#[cfg(feature = "webhook-axum")]
#[tokio::test]
async fn axum_adapter_validates_and_dispatches() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use maxoxide::webhook::axum_adapter;
    use tower::ServiceExt;

    let counter = Arc::new(AtomicUsize::new(0));
    let app = axum_adapter::router(service(counter.clone()), "/webhook").unwrap();
    let response = app
        .oneshot(
            Request::post("/webhook")
                .header("x-max-bot-api-secret", "shared-secret")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "webhook-actix")]
#[actix_web::test]
async fn actix_adapter_validates_and_dispatches() {
    use actix_web::{App, http::StatusCode, test};
    use maxoxide::webhook::actix_adapter;

    let counter = Arc::new(AtomicUsize::new(0));
    let scope = actix_adapter::scope(service(counter.clone()), "/webhook").unwrap();
    let app = test::init_service(App::new().service(scope)).await;
    let request = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("x-max-bot-api-secret", "shared-secret"))
        .set_payload("{}")
        .to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#![cfg(feature = "digital-id")]

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use maxoxide::digital_id::DigitalIdClient;

#[derive(Clone, Default)]
struct StateData {
    authorization: Arc<Mutex<Option<String>>>,
    body: Arc<Mutex<Option<String>>>,
}

async fn verify(State(state): State<StateData>, request: Request<Body>) -> Response {
    *state.authorization.lock().unwrap() = request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(String::from);
    let body = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
    *state.body.lock().unwrap() = Some(String::from_utf8(body.to_vec()).unwrap());
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        r#"{"verified":true}"#,
    )
        .into_response()
}

#[tokio::test]
async fn uses_separate_token_and_partner_payload() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = StateData::default();
    let app = Router::new().fallback(verify).with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = DigitalIdClient::with_client("digital-id-token", reqwest::Client::new())
        .unwrap()
        .endpoint(format!("http://{address}/v2/business/pos/age-verification"))
        .unwrap();

    let response = client
        .verify_age_raw(&serde_json::json!({"partner_payload":"opaque"}))
        .await
        .unwrap();
    assert_eq!(response["verified"], true);
    assert_eq!(
        state.authorization.lock().unwrap().as_deref(),
        Some("digital-id-token")
    );
    assert!(
        state
            .body
            .lock()
            .unwrap()
            .as_deref()
            .unwrap()
            .contains("partner_payload")
    );
    server.abort();
}

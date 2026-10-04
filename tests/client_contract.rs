use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use maxoxide::types::{
    AnswerCallbackBody, AnswerCallbackOptions, BotCommand, GetCommentsOptions, NewCommentBody,
    NewMessageBody,
};
use maxoxide::{Bot, MaxError, RateLimitConfig, RetryPolicy};

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    query: Option<String>,
    authorization: Option<String>,
    body: String,
}

#[derive(Clone)]
struct TestState {
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    me_attempts: Arc<AtomicUsize>,
    fail_me_once: bool,
}

async fn api(State(state): State<TestState>, request: Request<Body>) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let query = request.uri().query().map(String::from);
    let authorization = request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(String::from);
    let body = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
    state.requests.lock().unwrap().push(RecordedRequest {
        method: method.clone(),
        path: path.clone(),
        query,
        authorization,
        body: String::from_utf8(body.to_vec()).unwrap(),
    });

    if path == "/me" && state.fail_me_once && state.me_attempts.fetch_add(1, Ordering::SeqCst) == 0
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("content-type", "application/json"), ("retry-after", "0")],
            r#"{"code":"rate.limit","message":"slow down"}"#,
        )
            .into_response();
    }

    let body = match (method.as_str(), path.as_str()) {
        ("GET", "/me") => {
            r#"{
            "user_id": 7,
            "first_name": "Contract Bot",
            "username": "contract_bot",
            "is_bot": true,
            "commands": []
        }"#
        }
        ("PATCH", "/me/commands") => r#"{"commands":[{"name":"start","description":"Start"}]}"#,
        ("GET", "/messages/post/comments") => r#"{"messages":[],"marker":42}"#,
        ("GET", "/messages/post/comments/comment") => comment_json(),
        ("POST", "/messages/post/comments") => {
            return (
                StatusCode::OK,
                [("content-type", "application/json")],
                format!(r#"{{"message":{}}}"#, comment_json()),
            )
                .into_response();
        }
        ("PUT", "/messages/post/comments") | ("DELETE", "/messages/post/comments") => {
            r#"{"success":true}"#
        }
        ("GET", "/error") => {
            return (
                StatusCode::BAD_REQUEST,
                [("content-type", "application/json")],
                r#"{"code":"request.invalid","message":"bad request","detail":"bounded raw"}"#,
            )
                .into_response();
        }
        _ => r#"{"success":true}"#,
    };
    (StatusCode::OK, [("content-type", "application/json")], body).into_response()
}

fn comment_json() -> &'static str {
    r#"{
        "sender":{"user_id":9,"first_name":"Reader"},
        "recipient":{"chat_id":8,"chat_type":"channel","post_id":"post"},
        "timestamp":1000,
        "body":{"mid":"comment","seq":1,"text":"hello"}
    }"#
}

async fn start_server(fail_me_once: bool) -> (String, TestState, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = TestState {
        requests: Arc::new(Mutex::new(Vec::new())),
        me_attempts: Arc::new(AtomicUsize::new(0)),
        fail_me_once,
    };
    let app = Router::new().fallback(api).with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), state, task)
}

fn bot(base_url: &str, retries: u32) -> Bot {
    Bot::builder("secret-token")
        .base_url(base_url)
        .rate_limits(RateLimitConfig::disabled())
        .retry_policy(RetryPolicy {
            max_attempts: retries,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        })
        .build()
        .unwrap()
}

#[tokio::test]
async fn retries_429_and_preserves_structured_errors() {
    let (base_url, state, server) = start_server(true).await;
    let bot = bot(&base_url, 2);

    let me = bot.get_me().await.unwrap();
    assert_eq!(me.first_name, "Contract Bot");
    assert_eq!(state.me_attempts.load(Ordering::SeqCst), 2);

    let error = bot
        .execute::<serde_json::Value>(reqwest::Method::GET, "/error", &[], None)
        .await
        .unwrap_err();
    let MaxError::Api(error) = error else {
        panic!("expected structured API error");
    };
    assert_eq!(error.status, 400);
    assert_eq!(error.code.as_deref(), Some("request.invalid"));
    assert!(error.raw_response.unwrap().contains("bounded raw"));
    assert!(
        state
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|request| request.authorization.as_deref() == Some("secret-token"))
    );
    server.abort();
}

#[tokio::test]
async fn uses_documented_command_and_comment_routes() {
    let (base_url, state, server) = start_server(false).await;
    let bot = bot(&base_url, 1);

    let commands = bot
        .set_my_commands(vec![BotCommand::new("start", "Start")])
        .await
        .unwrap();
    assert_eq!(commands.commands[0].name, "start");

    let page = bot
        .get_comments(
            "post",
            GetCommentsOptions {
                comment_ids: Some(vec!["comment".into()]),
                before: Some(2000),
                after: Some(1000),
                count: Some(50),
            },
        )
        .await
        .unwrap();
    assert_eq!(page.marker, Some(42));
    assert_eq!(
        bot.get_comment("post", "comment").await.unwrap().body.mid,
        "comment"
    );
    bot.create_comment("post", NewCommentBody::text("hello"), Some(true))
        .await
        .unwrap();
    bot.edit_comment("post", "comment", NewCommentBody::text("edited"))
        .await
        .unwrap();
    bot.delete_comment("post", "comment").await.unwrap();

    let requests = state.requests.lock().unwrap();
    assert_eq!(requests[0].method, "PATCH");
    assert_eq!(requests[0].path, "/me/commands");
    assert!(requests[0].body.contains("\"commands\""));
    assert!(requests.iter().any(|request| {
        request.method == "GET"
            && request.path == "/messages/post/comments"
            && request
                .query
                .as_deref()
                .is_some_and(|query| query.contains("comment_ids=comment"))
    }));
    assert!(requests.iter().any(|request| {
        request.method == "PUT" && request.query.as_deref() == Some("comment_id=comment")
    }));
    assert!(requests.iter().any(|request| {
        request.method == "DELETE" && request.query.as_deref() == Some("comment_id=comment")
    }));
    server.abort();
}

#[tokio::test]
async fn sends_callback_options_only_in_the_query() {
    let (base_url, state, server) = start_server(false).await;
    let bot = bot(&base_url, 1);

    bot.answer_callback(AnswerCallbackBody {
        callback_id: "default-callback".into(),
        message: None,
        notification: Some("done".into()),
    })
    .await
    .unwrap();
    bot.answer_callback_with_options(
        AnswerCallbackBody {
            callback_id: "preview-callback".into(),
            message: Some(NewMessageBody::text("https://example.com")),
            notification: None,
        },
        AnswerCallbackOptions::disable_link_preview(true),
    )
    .await
    .unwrap();

    let requests = state.requests.lock().unwrap();
    let default_request = &requests[0];
    assert_eq!(default_request.method, "POST");
    assert_eq!(default_request.path, "/answers");
    assert_eq!(
        default_request.query.as_deref(),
        Some("callback_id=default-callback")
    );
    assert!(!default_request.body.contains("callback_id"));
    assert!(!default_request.body.contains("disable_link_preview"));
    assert!(default_request.body.contains("notification"));

    let options_request = &requests[1];
    let query = options_request.query.as_deref().unwrap();
    assert!(query.contains("callback_id=preview-callback"));
    assert!(query.contains("disable_link_preview=true"));
    assert!(!options_request.body.contains("callback_id"));
    assert!(!options_request.body.contains("disable_link_preview"));
    assert!(options_request.body.contains("https://example.com"));
    server.abort();
}

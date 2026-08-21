use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher};

#[derive(Debug)]
struct AppState(&'static str);

fn message_update(text: &str) -> Update {
    serde_json::from_value(serde_json::json!({
        "update_type": "message_created",
        "timestamp": 1000,
        "message": {
            "sender": {"user_id": 7, "first_name": "User"},
            "recipient": {"chat_id": 8, "chat_type": "dialog"},
            "timestamp": 1000,
            "body": {"mid": "mid", "seq": 1, "text": text}
        }
    }))
    .unwrap()
}

#[tokio::test]
async fn middleware_wraps_handlers_and_context_exposes_state() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = Dispatcher::new(Bot::new("token").unwrap()).with_state(AppState("ready"));
    let middleware_order = order.clone();
    dispatcher.middleware(move |context, next| {
        let order = middleware_order.clone();
        async move {
            order.lock().unwrap().push("before");
            next.run(context).await?;
            order.lock().unwrap().push("after");
            Ok(())
        }
    });
    let handler_order = order.clone();
    dispatcher.on_command("/start", move |context: Context| {
        let order = handler_order.clone();
        async move {
            assert_eq!(context.state::<AppState>().unwrap().0, "ready");
            assert_eq!(context.command_arguments(), Some("one two"));
            order.lock().unwrap().push("handler");
            Ok(())
        }
    });

    dispatcher
        .dispatch(message_update("/start one two"))
        .await
        .unwrap();
    assert_eq!(*order.lock().unwrap(), ["before", "handler", "after"]);
}

#[tokio::test]
async fn concurrent_dispatch_never_exceeds_the_configured_bound() {
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let mut dispatcher = Dispatcher::new(Bot::new("token").unwrap()).max_concurrent_handlers(2);
    let handler_active = active.clone();
    let handler_maximum = maximum.clone();
    dispatcher.on(move |_context: Context| {
        let active = handler_active.clone();
        let maximum = handler_maximum.clone();
        async move {
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        }
    });
    let dispatcher = Arc::new(dispatcher);
    let mut tasks = Vec::new();
    for index in 0..6 {
        let dispatcher = dispatcher.clone();
        tasks.push(tokio::spawn(async move {
            dispatcher
                .dispatch(message_update(&format!("message {index}")))
                .await
                .unwrap();
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }

    assert_eq!(maximum.load(Ordering::SeqCst), 2);
}

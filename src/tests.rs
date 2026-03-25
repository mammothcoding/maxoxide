//! Unit tests for maxoxide types, filters and builder helpers.
//!
//! Run with: `cargo test`

#[cfg(test)]
mod tests {
    use crate::{
        dispatcher::Filter,
        types::{
            AnswerCallbackBody, Attachment, Button, Callback, ChatType, KeyboardPayload, Message,
            MessageBody, MessageFormat, NewAttachment, NewMessageBody, Recipient, SubscribeBody,
            Update, UploadType, User,
        },
    };

    // ────────────────────────────────────────────────────────────
    // Helper: construct minimal Message
    // ────────────────────────────────────────────────────────────

    fn make_message(chat_id: i64, text: &str) -> Message {
        Message {
            sender: Some(User {
                user_id: 1,
                name: "Alice".into(),
                username: Some("alice".into()),
                is_bot: Some(false),
                last_activity_time: None,
                avatar_url: None,
                full_avatar_url: None,
            }),
            recipient: Recipient {
                chat_id,
                chat_type: ChatType::Dialog,
                user_id: Some(1),
            },
            timestamp: 1_700_000_000,
            link: None,
            body: MessageBody {
                mid: "mid_001".into(),
                seq: 1,
                text: Some(text.into()),
                attachments: None,
            },
            stat: None,
            url: None,
            constructor: None,
        }
    }

    fn make_callback(payload: &str) -> Update {
        Update::MessageCallback {
            timestamp: 1_700_000_000,
            callback: Callback {
                callback_id: "cb_001".into(),
                user: User {
                    user_id: 2,
                    name: "Bob".into(),
                    username: None,
                    is_bot: Some(false),
                    last_activity_time: None,
                    avatar_url: None,
                    full_avatar_url: None,
                },
                payload: Some(payload.into()),
                timestamp: 1_700_000_000,
            },
            message: None,
            user_locale: None,
        }
    }

    // ────────────────────────────────────────────────────────────
    // Serde round-trips
    // ────────────────────────────────────────────────────────────

    #[test]
    fn test_update_message_created_roundtrip() {
        let json = r#"{
            "update_type": "message_created",
            "timestamp": 1700000000,
            "message": {
                "sender": {"user_id": 1, "name": "Alice"},
                "recipient": {"chat_id": 42, "chat_type": "dialog"},
                "timestamp": 1700000000,
                "body": {"mid": "mid_1", "seq": 1, "text": "hello"}
            }
        }"#;

        let update: Update = serde_json::from_str(json).expect("deserialize Update");
        if let Update::MessageCreated { message, .. } = update {
            assert_eq!(message.chat_id(), 42);
            assert_eq!(message.text(), Some("hello"));
            assert_eq!(message.message_id(), "mid_1");
        } else {
            panic!("Expected MessageCreated");
        }
    }

    #[test]
    fn test_update_message_callback_roundtrip() {
        let json = r#"{
            "update_type": "message_callback",
            "timestamp": 1700000000,
            "callback": {
                "callback_id": "cb_1",
                "user": {"user_id": 2, "name": "Bob"},
                "payload": "btn:ok",
                "timestamp": 1700000000
            }
        }"#;

        let update: Update = serde_json::from_str(json).unwrap();
        if let Update::MessageCallback { callback, .. } = update {
            assert_eq!(callback.payload.as_deref(), Some("btn:ok"));
        } else {
            panic!("Expected MessageCallback");
        }
    }

    #[test]
    fn test_update_bot_started_roundtrip() {
        let json = r#"{
            "update_type": "bot_started",
            "timestamp": 1700000000,
            "chat_id": 99,
            "user": {"user_id": 3, "name": "Carol"},
            "payload": "/start"
        }"#;

        let update: Update = serde_json::from_str(json).unwrap();
        assert!(matches!(update, Update::BotStarted { chat_id: 99, .. }));
    }

    #[test]
    fn test_recipient_keeps_chat_id_and_user_id_distinct() {
        let message = make_message(223_921_237, "hello");
        assert_eq!(message.chat_id(), 223_921_237);
        assert_eq!(message.recipient.user_id, Some(1));
    }

    #[test]
    fn test_recipient_roundtrip_preserves_both_ids() {
        let json = r#"{
            "sender": {"user_id": 5465382, "name": "Konstantin"},
            "recipient": {"chat_id": 223921237, "chat_type": "dialog", "user_id": 5465382},
            "timestamp": 1700000000,
            "body": {"mid": "mid_1", "seq": 1, "text": "hello"}
        }"#;

        let message: Message = serde_json::from_str(json).unwrap();
        assert_eq!(message.chat_id(), 223_921_237);
        assert_eq!(message.recipient.user_id, Some(5_465_382));
    }

    #[test]
    fn test_chat_type_serde() {
        let dialog: ChatType = serde_json::from_str(r#""dialog""#).unwrap();
        let chat: ChatType = serde_json::from_str(r#""chat""#).unwrap();
        let channel: ChatType = serde_json::from_str(r#""channel""#).unwrap();

        assert_eq!(dialog, ChatType::Dialog);
        assert_eq!(chat, ChatType::Chat);
        assert_eq!(channel, ChatType::Channel);

        assert_eq!(
            serde_json::to_string(&ChatType::Dialog).unwrap(),
            r#""dialog""#
        );
    }

    #[test]
    fn test_upload_type_serialization() {
        // Ensure no "photo" leaks — Max removed it, only "image" is valid.
        assert_eq!(
            serde_json::to_string(&UploadType::Image).unwrap(),
            r#""image""#
        );
        assert_eq!(
            serde_json::to_string(&UploadType::Video).unwrap(),
            r#""video""#
        );
        assert_eq!(
            serde_json::to_string(&UploadType::Audio).unwrap(),
            r#""audio""#
        );
        assert_eq!(
            serde_json::to_string(&UploadType::File).unwrap(),
            r#""file""#
        );
    }

    #[test]
    fn test_message_format_default() {
        let fmt = MessageFormat::default();
        assert_eq!(serde_json::to_string(&fmt).unwrap(), r#""markdown""#);
    }

    #[test]
    fn test_subscribe_body_secret_serialization() {
        let body = SubscribeBody {
            url: "https://bot.example.com/webhook".into(),
            update_types: Some(vec!["message_created".into()]),
            version: None,
            secret: Some("my_secret_abc".into()),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("my_secret_abc"));
        assert!(json.contains("message_created"));
        assert!(!json.contains("version")); // skipped because None
    }

    #[test]
    fn test_subscribe_body_no_secret_skipped() {
        let body = SubscribeBody {
            url: "https://bot.example.com/webhook".into(),
            update_types: None,
            version: None,
            secret: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        // Optional fields with None must be omitted entirely
        assert!(!json.contains("secret"));
        assert!(!json.contains("update_types"));
    }

    // ────────────────────────────────────────────────────────────
    // NewMessageBody builder
    // ────────────────────────────────────────────────────────────

    #[test]
    fn test_new_message_body_text() {
        let body = NewMessageBody::text("Hello, Max!");
        assert_eq!(body.text.as_deref(), Some("Hello, Max!"));
        assert!(body.attachments.is_none());
    }

    #[test]
    fn test_new_message_body_with_keyboard() {
        let keyboard = KeyboardPayload {
            buttons: vec![vec![Button::callback("OK", "btn:ok")]],
        };
        let body = NewMessageBody::text("Choose:").with_keyboard(keyboard);
        let attachments = body.attachments.as_ref().unwrap();
        assert_eq!(attachments.len(), 1);
        assert!(matches!(
            attachments[0],
            NewAttachment::InlineKeyboard { .. }
        ));
    }

    #[test]
    fn test_new_message_body_serialization() {
        let keyboard = KeyboardPayload {
            buttons: vec![vec![
                Button::callback("Yes ✅", "answer:yes"),
                Button::callback("No ❌", "answer:no"),
            ]],
        };
        let body = NewMessageBody::text("Are you sure?")
            .with_keyboard(keyboard)
            .with_format(MessageFormat::Markdown);

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["text"], "Are you sure?");
        assert_eq!(json["format"], "markdown");
        let buttons = &json["attachments"][0]["payload"]["buttons"][0];
        assert_eq!(buttons[0]["type"], "callback");
        assert_eq!(buttons[0]["payload"], "answer:yes");
        assert_eq!(buttons[1]["payload"], "answer:no");
    }

    #[test]
    fn test_button_link_serialization() {
        let btn = Button::link("Docs", "https://dev.max.ru");
        let json = serde_json::to_value(&btn).unwrap();
        assert_eq!(json["type"], "link");
        assert_eq!(json["url"], "https://dev.max.ru");
    }

    #[test]
    fn test_answer_callback_body_defaults() {
        let body = AnswerCallbackBody {
            callback_id: "cb_123".into(),
            notification: Some("done!".into()),
            ..Default::default()
        };
        assert_eq!(body.callback_id, "cb_123");
        assert!(body.message.is_none());
    }

    // ────────────────────────────────────────────────────────────
    // Filter matching
    // ────────────────────────────────────────────────────────────

    #[test]
    fn test_filter_any() {
        let update = Update::MessageCreated {
            timestamp: 0,
            message: make_message(1, "hi"),
        };
        assert!(Filter::Any.matches(&update));
    }

    #[test]
    fn test_filter_message() {
        let msg_update = Update::MessageCreated {
            timestamp: 0,
            message: make_message(1, "hi"),
        };
        let cb_update = make_callback("btn");

        assert!(Filter::Message.matches(&msg_update));
        assert!(!Filter::Message.matches(&cb_update));
    }

    #[test]
    fn test_filter_callback() {
        let update = make_callback("btn:ok");
        assert!(Filter::Callback.matches(&update));
        assert!(!Filter::Message.matches(&update));
    }

    #[test]
    fn test_filter_command_matches_prefix() {
        let update = Update::MessageCreated {
            timestamp: 0,
            message: make_message(1, "/start payload"),
        };
        assert!(Filter::Command("/start".into()).matches(&update));
        assert!(!Filter::Command("/help".into()).matches(&update));
    }

    #[test]
    fn test_filter_command_exact() {
        let update = Update::MessageCreated {
            timestamp: 0,
            message: make_message(1, "/help"),
        };
        assert!(Filter::Command("/help".into()).matches(&update));
        assert!(!Filter::Command("/start".into()).matches(&update));
    }

    #[test]
    fn test_filter_callback_payload_exact() {
        let update = make_callback("color:red");
        assert!(Filter::CallbackPayload("color:red".into()).matches(&update));
        assert!(!Filter::CallbackPayload("color:blue".into()).matches(&update));
    }

    #[test]
    fn test_filter_callback_payload_none() {
        let update = Update::MessageCallback {
            timestamp: 0,
            callback: Callback {
                callback_id: "cb".into(),
                user: User {
                    user_id: 1,
                    name: "X".into(),
                    username: None,
                    is_bot: None,
                    last_activity_time: None,
                    avatar_url: None,
                    full_avatar_url: None,
                },
                payload: None, // no payload
                timestamp: 0,
            },
            message: None,
            user_locale: None,
        };
        assert!(!Filter::CallbackPayload("color:red".into()).matches(&update));
    }

    #[test]
    fn test_filter_custom() {
        let update = Update::MessageCreated {
            timestamp: 999,
            message: make_message(42, "test"),
        };
        // Match on timestamp
        let f = Filter::Custom(std::sync::Arc::new(|u| u.timestamp() == 999));
        assert!(f.matches(&update));

        let f2 = Filter::Custom(std::sync::Arc::new(|u| u.timestamp() == 0));
        assert!(!f2.matches(&update));
    }

    #[test]
    fn test_update_timestamp() {
        let ts = 1_700_000_042;
        let update = Update::MessageCreated {
            timestamp: ts,
            message: make_message(1, ""),
        };
        assert_eq!(update.timestamp(), ts);
    }

    // ────────────────────────────────────────────────────────────
    // Attachment serde
    // ────────────────────────────────────────────────────────────

    #[test]
    fn test_attachment_image_deserialization() {
        let json = r#"{"type":"image","payload":{"url":"https://cdn.example.com/photo.jpg","token":"tok123"}}"#;
        let att: Attachment = serde_json::from_str(json).unwrap();
        assert!(matches!(att, Attachment::Image { .. }));
    }

    #[test]
    fn test_attachment_inline_keyboard_round_trip() {
        let original = NewAttachment::InlineKeyboard {
            payload: KeyboardPayload {
                buttons: vec![vec![Button::callback("Click", "click:1")]],
            },
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("inline_keyboard"));
        assert!(json.contains("click:1"));
    }
}

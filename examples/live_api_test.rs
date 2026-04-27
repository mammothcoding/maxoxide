//! Interactive live API test harness for a real Max bot.
//!
//! Run:
//!   cargo run --example live_api_test

use maxoxide::types::{
    AnswerCallbackBody, Attachment, BotCommand, Button, Chat, ChatAdmin, ChatAdminPermission,
    ChatType, EditChatBody, KeyboardPayload, Message, NewAttachment, NewMessageBody,
    PinMessageBody, SendMessageOptions, SenderAction, SubscribeBody, Update, UploadType,
};
use maxoxide::{Bot, reqwest::Client};
use std::error::Error;
use std::future::Future;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;

type AnyResult<T> = Result<T, Box<dyn Error>>;

const PRIVATE_WAIT_SECS: u64 = 180;
const GROUP_WAIT_SECS: u64 = 240;
const MANUAL_WAIT_SECS: u64 = 120;
const WAIT_PROMPT_CHUNK_SECS: u64 = 15;
const MAX_NON_MATCHING_UPDATE_LOGS: usize = 5;

#[derive(Clone, Copy)]
enum Language {
    English,
    Russian,
}

impl Language {
    fn prompt() -> AnyResult<Self> {
        loop {
            let value = prompt("Select language / Выберите язык [en/ru] [en]")?;
            let normalized = value.trim().to_ascii_lowercase();

            if normalized.is_empty() || matches!(normalized.as_str(), "en" | "eng" | "english") {
                return Ok(Self::English);
            }

            if matches!(
                normalized.as_str(),
                "ru" | "rus" | "russian" | "рус" | "русский"
            ) {
                return Ok(Self::Russian);
            }

            println!("Expected `en` or `ru` / Ожидается `en` или `ru`.");
        }
    }
}

fn tr<'a>(lang: Language, en: &'a str, ru: &'a str) -> &'a str {
    match lang {
        Language::English => en,
        Language::Russian => ru,
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    tracing_subscriber::fmt::init();

    let lang = Language::prompt()?;
    let config = Config::prompt(lang)?;
    let client = Client::builder().timeout(config.http_timeout).build()?;
    let bot = Bot::with_client(config.token.clone(), client);
    let mut harness = Harness::new(bot, config.request_delay, config.poll_timeout, lang);
    let mut report = Report::default();

    print_section(tr(lang, "Live Test", "Живой тест"));
    match lang {
        Language::English => println!(
            "Interactive real-API run with request delay {} ms, HTTP timeout {} s, polling timeout {} s.",
            config.request_delay.as_millis(),
            config.http_timeout.as_secs(),
            config.poll_timeout
        ),
        Language::Russian => println!(
            "Интерактивный прогон по реальному API: задержка между запросами {} мс, HTTP timeout {} c, polling timeout {} c.",
            config.request_delay.as_millis(),
            config.http_timeout.as_secs(),
            config.poll_timeout
        ),
    }

    let me = match harness
        .api_case(&mut report, "bot.get_me", |bot| async move {
            bot.get_me().await
        })
        .await
    {
        Some(me) => me,
        None => {
            report.print_summary(lang);
            return Ok(());
        }
    };

    match lang {
        Language::English => println!(
            "Authenticated as @{}.",
            me.username.as_deref().unwrap_or("unknown")
        ),
        Language::Russian => println!(
            "Аутентификация выполнена как @{}.",
            me.username.as_deref().unwrap_or("неизвестно")
        ),
    }

    let known_chats = harness
        .api_case(&mut report, "bot.get_chats", |bot| async move {
            bot.get_chats(Some(100), None).await
        })
        .await
        .map(|list| {
            print_known_chats(&list.chats, lang);
            list.chats
        })
        .unwrap_or_default();

    match harness.flush_updates().await {
        Ok(drained) => {
            let detail = match lang {
                Language::English => {
                    format!("marker synchronized, drained {drained} backlog update(s)")
                }
                Language::Russian => {
                    format!("marker синхронизирован, очищено {drained} backlog-обновлений")
                }
            };
            report.pass("bot.get_updates", detail);
        }
        Err(err) => {
            report.fail("bot.get_updates", err.to_string());
            report.print_summary(lang);
            return Ok(());
        }
    }

    let raw_marker = harness.marker;
    let raw_response = harness
        .api_case(&mut report, "bot.get_updates_raw", move |bot| async move {
            bot.get_updates_raw(raw_marker, Some(1), Some(1)).await
        })
        .await;
    if let Some(marker) = raw_response.and_then(|response| response.marker) {
        harness.marker = Some(marker);
    }

    let private_phase = run_private_phase(&mut harness, &mut report, &config).await?;
    run_upload_phase(
        &mut harness,
        &mut report,
        private_phase.chat_id,
        private_phase.user_id,
        &config,
    )
    .await?;
    run_webhook_phase(&mut harness, &mut report, &config).await?;
    run_commands_phase(&mut harness, &mut report, lang).await?;
    run_group_phase(
        &mut harness,
        &mut report,
        &config,
        &known_chats,
        private_phase.user_id,
    )
    .await?;

    report.print_summary(lang);
    Ok(())
}

#[derive(Default)]
struct PrivatePhaseState {
    chat_id: Option<i64>,
    user_id: Option<i64>,
}

async fn run_private_phase(
    harness: &mut Harness,
    report: &mut Report,
    config: &Config,
) -> AnyResult<PrivatePhaseState> {
    let lang = config.lang;

    print_section(tr(lang, "Private Chat", "Личный чат"));
    println!(
        "{}",
        tr(lang, "1. Open the bot in Max.", "1. Откройте бота в Max.")
    );
    if let Some(link) = &config.bot_link {
        println!("   {}: {link}", tr(lang, "Bot URL", "URL бота"));
    }
    println!(
        "{}",
        tr(
            lang,
            "2. Send `/live` to the bot from a private dialog.",
            "2. Отправьте `/live` боту в личном диалоге.",
        )
    );

    let activation = harness
        .wait_case(
            report,
            "manual.private_activation",
            tr(
                lang,
                "Waiting for `/live` in a private chat.",
                "Ожидание `/live` в личном чате.",
            ),
            Duration::from_secs(PRIVATE_WAIT_SECS),
            |update| match update {
                Update::MessageCreated { message, .. } => {
                    message.recipient.chat_type == ChatType::Dialog
                        && message.text() == Some("/live")
                }
                _ => false,
            },
        )
        .await;

    let Some(Update::MessageCreated { message, .. }) = activation else {
        skip_cases(
            report,
            &[
                "bot.get_chat(private)",
                "bot.send_text_to_chat",
                "bot.send_text_to_user",
                "bot.send_markdown_to_chat",
                "bot.send_markdown_to_user",
                "bot.send_message_to_chat(text_body)",
                "bot.send_message_to_user(text_body)",
                "bot.send_message_to_chat_with_options(disable_link_preview)",
                "bot.send_action",
                "bot.send_message_to_chat(keyboard)",
                "bot.send_message_to_chat(open_app_button)",
                "manual.observe_open_app_button",
                "bot.send_message_to_chat(clipboard_button)",
                "manual.observe_clipboard_button",
                "bot.answer_callback",
                "bot.edit_message",
                "bot.get_message",
                "bot.get_messages",
                "bot.get_messages_by_ids",
                "bot.delete_message",
            ],
            tr(
                lang,
                "private chat activation was not completed",
                "активация личного чата не была завершена",
            ),
        );
        return Ok(PrivatePhaseState::default());
    };

    let private_chat_id = message.chat_id();
    let mut private_user_id = message.sender.as_ref().map(|user| user.user_id);
    match lang {
        Language::English => println!("Private chat id: {private_chat_id}"),
        Language::Russian => println!("ID личного чата: {private_chat_id}"),
    }

    let _ = harness
        .api_case(report, "bot.get_chat(private)", move |bot| async move {
            bot.get_chat(private_chat_id).await
        })
        .await;

    let plain_message = harness
        .api_case(report, "bot.send_text_to_chat", move |bot| async move {
            bot.send_text_to_chat(private_chat_id, "maxoxide live test: plain text message")
                .await
        })
        .await;

    if let Some(user_id) = private_user_id {
        let _ = harness
            .api_case(report, "bot.send_text_to_user", move |bot| async move {
                bot.send_text_to_user(user_id, "maxoxide live test: send_text_to_user")
                    .await
            })
            .await;
    } else {
        report.skip(
            "bot.send_text_to_user",
            tr(
                lang,
                "sender.user_id is missing",
                "sender.user_id отсутствует",
            ),
        );
    }

    let _ = harness
        .api_case(report, "bot.send_markdown_to_chat", move |bot| async move {
            bot.send_markdown_to_chat(
                private_chat_id,
                "*maxoxide live test*: `send_markdown_to_chat`",
            )
            .await
        })
        .await;

    if let Some(user_id) = private_user_id {
        let _ = harness
            .api_case(report, "bot.send_markdown_to_user", move |bot| async move {
                bot.send_markdown_to_user(user_id, "*maxoxide live test*: `send_markdown_to_user`")
                    .await
            })
            .await;
    } else {
        report.skip(
            "bot.send_markdown_to_user",
            tr(
                lang,
                "sender.user_id is missing",
                "sender.user_id отсутствует",
            ),
        );
    }

    let _ = harness
        .api_case(
            report,
            "bot.send_message_to_chat(text_body)",
            move |bot| async move {
                bot.send_message_to_chat(
                    private_chat_id,
                    NewMessageBody::text("maxoxide live test: send_message_to_chat"),
                )
                .await
            },
        )
        .await;

    if let Some(user_id) = private_user_id {
        let _ = harness
            .api_case(
                report,
                "bot.send_message_to_user(text_body)",
                move |bot| async move {
                    bot.send_message_to_user(
                        user_id,
                        NewMessageBody::text("maxoxide live test: send_message_to_user"),
                    )
                    .await
                },
            )
            .await;
    } else {
        report.skip(
            "bot.send_message_to_user(text_body)",
            tr(
                lang,
                "sender.user_id is missing",
                "sender.user_id отсутствует",
            ),
        );
    }

    let _ = harness
        .api_case(
            report,
            "bot.send_message_to_chat_with_options(disable_link_preview)",
            move |bot| async move {
                bot.send_message_to_chat_with_options(
                    private_chat_id,
                    NewMessageBody::text("https://max.ru"),
                    SendMessageOptions::disable_link_preview(true),
                )
                .await
            },
        )
        .await;

    let callback_button_text = tr(lang, "Confirm callback", "Подтвердить callback");
    let message_button_text = tr(lang, "live:message_button", "live:message_button_ru");
    let contact_button_text = tr(lang, "Share contact", "Поделиться контактом");
    let location_button_text = tr(lang, "Share location", "Поделиться геопозицией");
    let link_button_text = tr(lang, "Open docs", "Открыть документацию");
    let keyboard_text = tr(
        lang,
        "Live test keyboard: callback, message, contact, location, link.",
        "Клавиатура live-теста: callback, сообщение, контакт, геопозиция, ссылка.",
    );

    let keyboard = KeyboardPayload {
        buttons: vec![
            vec![Button::callback(callback_button_text, "live:callback")],
            vec![Button::Message {
                text: message_button_text.into(),
                intent: None,
            }],
            vec![Button::RequestContact {
                text: contact_button_text.into(),
            }],
            vec![Button::RequestGeoLocation {
                text: location_button_text.into(),
                quick: None,
            }],
            vec![Button::link(
                link_button_text,
                "https://dev.max.ru/docs-api",
            )],
        ],
    };
    let keyboard_body = NewMessageBody::text(keyboard_text).with_keyboard(keyboard);

    let keyboard_message = harness
        .api_case(
            report,
            "bot.send_message_to_chat(keyboard)",
            move |bot| async move {
                bot.send_message_to_chat(private_chat_id, keyboard_body)
                    .await
            },
        )
        .await;

    if keyboard_message.is_some() {
        confirm_case(
            lang,
            report,
            "manual.observe_link_button",
            tr(
                lang,
                "Is the link button visible in the sent keyboard?",
                "Видна ли в отправленной клавиатуре кнопка-ссылка?",
            ),
        )?;

        if let Some(web_app) = prompt_optional(
            lang,
            tr(
                lang,
                "Optional platform probe: enter open_app web_app value, or leave blank to skip",
                "Опциональная platform-проверка: введите значение web_app для open_app или оставьте поле пустым",
            ),
        )? {
            let payload = prompt_optional(
                lang,
                tr(
                    lang,
                    "Optional open_app payload string",
                    "Необязательная строка payload для open_app",
                ),
            )?;
            let contact_id = prompt_optional_i64(
                lang,
                tr(
                    lang,
                    "Optional open_app contact_id",
                    "Необязательный contact_id для open_app",
                ),
            )?;
            let open_app_keyboard = KeyboardPayload {
                buttons: vec![vec![Button::open_app_full(
                    tr(lang, "Open app", "Открыть app"),
                    web_app,
                    payload,
                    contact_id,
                )]],
            };
            let open_app_body = NewMessageBody::text(tr(
                lang,
                "MAX platform probe: open_app button.",
                "Проверка платформы MAX: open_app-кнопка.",
            ))
            .with_keyboard(open_app_keyboard);
            let open_app_message = harness
                .api_case(
                    report,
                    "bot.send_message_to_chat(open_app_button)",
                    move |bot| async move {
                        bot.send_message_to_chat(private_chat_id, open_app_body)
                            .await
                    },
                )
                .await;

            if open_app_message.is_some() {
                confirm_case(
                    lang,
                    report,
                    "manual.observe_open_app_button",
                    tr(
                        lang,
                        "Is the open_app button visible, and does tapping it open a MAX app or show a client error?",
                        "Видна ли open_app-кнопка, и открывает ли нажатие MAX app или показывает ошибку клиента?",
                    ),
                )?;
            } else {
                report.skip(
                    "manual.observe_open_app_button",
                    tr(
                        lang,
                        "open_app button message was not sent",
                        "сообщение с open_app-кнопкой не было отправлено",
                    ),
                );
            }
        } else {
            report.skip(
                "bot.send_message_to_chat(open_app_button)",
                tr(
                    lang,
                    "tester did not provide open_app web_app",
                    "тестер не указал web_app для open_app",
                ),
            );
            report.skip(
                "manual.observe_open_app_button",
                tr(
                    lang,
                    "tester did not provide open_app web_app",
                    "тестер не указал web_app для open_app",
                ),
            );
        }

        let clipboard_body = NewMessageBody::text(tr(
            lang,
            "MAX platform probe: clipboard button.",
            "Проверка платформы MAX: clipboard-кнопка.",
        ))
        .with_keyboard(KeyboardPayload {
            buttons: vec![vec![Button::clipboard(
                tr(lang, "Copy text", "Скопировать текст"),
                "maxoxide-live-clipboard-payload",
            )]],
        });
        let clipboard_message = harness
            .api_case(
                report,
                "bot.send_message_to_chat(clipboard_button)",
                move |bot| async move {
                    bot.send_message_to_chat(private_chat_id, clipboard_body)
                        .await
                },
            )
            .await;
        if clipboard_message.is_some() {
            confirm_case(
                lang,
                report,
                "manual.observe_clipboard_button",
                tr(
                    lang,
                    "Is the clipboard button visible, and does tapping it copy the expected text?",
                    "Видна ли clipboard-кнопка, и копирует ли нажатие ожидаемый текст?",
                ),
            )?;
        } else {
            report.skip(
                "manual.observe_clipboard_button",
                tr(
                    lang,
                    "clipboard button message was not sent",
                    "сообщение с clipboard-кнопкой не было отправлено",
                ),
            );
        }

        if confirm(
            lang,
            tr(
                lang,
                "Test callback button now? Type `y` to wait for click, anything else to skip.",
                "Проверить callback-кнопку сейчас? Введите `y`, чтобы ждать нажатие, иначе шаг будет пропущен.",
            ),
            false,
        )? {
            let callback = harness
                .wait_case(
                    report,
                    "manual.callback_click",
                    match lang {
                        Language::English => {
                            format!("Press `{callback_button_text}` in Max.")
                        }
                        Language::Russian => {
                            format!("Нажмите `{callback_button_text}` в Max.")
                        }
                    }
                    .as_str(),
                    Duration::from_secs(MANUAL_WAIT_SECS),
                    |update| match update {
                        Update::MessageCallback { callback, .. } => {
                            callback.payload.as_deref() == Some("live:callback")
                        }
                        _ => false,
                    },
                )
                .await;

            if let Some(Update::MessageCallback { callback, .. }) = callback {
                let callback_id = callback.callback_id.clone();
                let _ = harness
                    .api_case(report, "bot.answer_callback", move |bot| async move {
                        bot.answer_callback(AnswerCallbackBody {
                            callback_id,
                            notification: Some(
                                tr(lang, "Callback acknowledged.", "Callback подтверждён.").into(),
                            ),
                            ..Default::default()
                        })
                        .await
                    })
                    .await;
            }
        } else {
            report.skip(
                "manual.callback_click",
                tr(
                    lang,
                    "tester skipped callback interaction",
                    "тестер пропустил взаимодействие с callback-кнопкой",
                ),
            );
            report.skip(
                "bot.answer_callback",
                tr(
                    lang,
                    "callback interaction was skipped",
                    "взаимодействие с callback-кнопкой было пропущено",
                ),
            );
        }

        if confirm(
            lang,
            tr(
                lang,
                "Test message button now? Type `y` to wait for the generated message, anything else to skip.",
                "Проверить message-кнопку сейчас? Введите `y`, чтобы ждать сгенерированное сообщение, иначе шаг будет пропущен.",
            ),
            false,
        )? {
            let _ = harness
                .wait_case(
                    report,
                    "manual.message_button",
                    match lang {
                        Language::English => format!("Press `{message_button_text}` in Max."),
                        Language::Russian => format!("Нажмите `{message_button_text}` в Max."),
                    }
                    .as_str(),
                    Duration::from_secs(MANUAL_WAIT_SECS),
                    |update| match update {
                        Update::MessageCreated { message, .. } => {
                            message.chat_id() == private_chat_id
                                && message.text() == Some(message_button_text)
                        }
                        _ => false,
                    },
                )
                .await;
        } else {
            report.skip(
                "manual.message_button",
                tr(
                    lang,
                    "tester skipped message button interaction",
                    "тестер пропустил взаимодействие с message-кнопкой",
                ),
            );
        }

        if confirm(
            lang,
            tr(
                lang,
                "Test request-contact button now? Type `y` to wait for shared contact, anything else to skip.",
                "Проверить кнопку запроса контакта сейчас? Введите `y`, чтобы ждать отправку контакта, иначе шаг будет пропущен.",
            ),
            false,
        )? {
            let contact_update = harness
                .wait_case(
                    report,
                    "manual.contact_share",
                    match lang {
                        Language::English => format!("Press `{contact_button_text}` in Max."),
                        Language::Russian => format!("Нажмите `{contact_button_text}` в Max."),
                    }
                    .as_str(),
                    Duration::from_secs(MANUAL_WAIT_SECS),
                    |update| match update {
                        Update::MessageCreated { message, .. } => {
                            message.chat_id() == private_chat_id
                                && message_has_attachment(&message.body.attachments, is_contact)
                        }
                        _ => false,
                    },
                )
                .await;

            if let Some(update) = contact_update {
                if let Some(phone) = extract_contact_phone(&update) {
                    report.pass(
                        "manual.contact_phone_present",
                        match lang {
                            Language::English => format!("phone={phone}"),
                            Language::Russian => format!("телефон={phone}"),
                        },
                    );
                } else {
                    report.skip(
                        "manual.contact_phone_present",
                        tr(
                            lang,
                            "contact attachment was received, but vcf_phone is empty; treating this as a current MAX platform gap",
                            "contact-вложение пришло, но поле vcf_phone пустое; шаг помечен как текущее платформенное ограничение MAX",
                        ),
                    );
                }
            } else {
                report.skip(
                    "manual.contact_phone_present",
                    tr(
                        lang,
                        "contact share step did not complete",
                        "шаг отправки контакта не был завершён",
                    ),
                );
            }
        } else {
            report.skip(
                "manual.contact_share",
                tr(
                    lang,
                    "tester skipped contact share",
                    "тестер пропустил отправку контакта",
                ),
            );
            report.skip(
                "manual.contact_phone_present",
                tr(
                    lang,
                    "tester skipped contact share",
                    "тестер пропустил отправку контакта",
                ),
            );
        }

        if confirm(
            lang,
            tr(
                lang,
                "Test request-location button now? Type `y` to wait for shared location or a client map-card fallback, anything else to skip.",
                "Проверить кнопку запроса геопозиции сейчас? Введите `y`, чтобы ждать геопозицию или fallback-карточку карты, иначе шаг будет пропущен.",
            ),
            false,
        )? {
            let location_update = harness
                .wait_case(
                    report,
                    "manual.location_share",
                    match lang {
                        Language::English => format!("Press `{location_button_text}` in Max."),
                        Language::Russian => format!("Нажмите `{location_button_text}` в Max."),
                    }
                    .as_str(),
                    Duration::from_secs(MANUAL_WAIT_SECS),
                    |update| match update {
                        Update::MessageCreated { message, .. } => {
                            message.chat_id() == private_chat_id
                                && (message_has_attachment(&message.body.attachments, is_location)
                                    || looks_like_client_map_card(message))
                        }
                        _ => false,
                    },
                )
                .await;

            match location_update {
                Some(Update::MessageCreated { message, .. })
                    if message_has_attachment(&message.body.attachments, is_location) =>
                {
                    report.pass(
                        "manual.location_structured_payload",
                        tr(
                            lang,
                            "structured location attachment received",
                            "получено структурированное location-вложение",
                        ),
                    );
                }
                Some(Update::MessageCreated { message, .. })
                    if looks_like_client_map_card(&message) =>
                {
                    report.skip(
                        "manual.location_structured_payload",
                        tr(
                            lang,
                            "MAX client sent a map link/card instead of a structured location attachment",
                            "клиент MAX отправил ссылку/карточку карты вместо структурированного location-вложения",
                        ),
                    );
                }
                Some(_) => {
                    report.skip(
                        "manual.location_structured_payload",
                        tr(
                            lang,
                            "location step completed with an unexpected update shape",
                            "шаг геопозиции завершился неожиданной формой update",
                        ),
                    );
                }
                None => {
                    report.skip(
                        "manual.location_structured_payload",
                        tr(
                            lang,
                            "location share step did not complete",
                            "шаг отправки геопозиции не был завершён",
                        ),
                    );
                }
            }
        } else {
            report.skip(
                "manual.location_share",
                tr(
                    lang,
                    "tester skipped location share",
                    "тестер пропустил отправку геопозиции",
                ),
            );
            report.skip(
                "manual.location_structured_payload",
                tr(
                    lang,
                    "tester skipped location share",
                    "тестер пропустил отправку геопозиции",
                ),
            );
        }
    }

    if confirm(
        lang,
        tr(
            lang,
            "Test manual file/photo attachment from the Max client? Type `y` to wait for an incoming attachment.",
            "Проверить ручную отправку файла/фото из клиента Max? Введите `y`, чтобы ждать входящее вложение.",
        ),
        false,
    )? {
        let _ = harness
            .wait_case(
                report,
                "manual.client_attachment",
                tr(
                    lang,
                    "Attach any file or image to the private chat in Max.",
                    "Прикрепите любой файл или изображение в личный чат в Max.",
                ),
                Duration::from_secs(MANUAL_WAIT_SECS),
                |update| match update {
                    Update::MessageCreated { message, .. } => {
                        message.chat_id() == private_chat_id
                            && message_has_attachment(
                                &message.body.attachments,
                                is_non_keyboard_attachment,
                            )
                    }
                    _ => false,
                },
            )
            .await;
    } else {
        report.skip(
            "manual.client_attachment",
            tr(
                lang,
                "tester skipped client-side attachment check",
                "тестер пропустил проверку вложения со стороны клиента",
            ),
        );
    }

    if confirm(
        lang,
        tr(
            lang,
            "Test `/get_my_id` now? Type `y`, then send `/get_my_id` to the bot.",
            "Проверить `/get_my_id` сейчас? Введите `y`, затем отправьте `/get_my_id` боту.",
        ),
        false,
    )? {
        let get_my_id_update = harness
            .wait_case(
                report,
                "manual.get_my_id_command",
                tr(
                    lang,
                    "Send `/get_my_id` in the private chat.",
                    "Отправьте `/get_my_id` в личный чат.",
                ),
                Duration::from_secs(MANUAL_WAIT_SECS),
                |update| match update {
                    Update::MessageCreated { message, .. } => {
                        message.chat_id() == private_chat_id && message.text() == Some("/get_my_id")
                    }
                    _ => false,
                },
            )
            .await;

        if let Some(update) = get_my_id_update {
            if let Some(user_id) = extract_sender_user_id(&update) {
                private_user_id = Some(user_id);
                report.pass(
                    "manual.get_my_id_user_id",
                    match lang {
                        Language::English => format!("user_id={user_id}"),
                        Language::Russian => format!("user_id={user_id}"),
                    },
                );

                let reply_text = match lang {
                    Language::English => format!("Your Max ID: {user_id}"),
                    Language::Russian => format!("Ваш Max ID: {user_id}"),
                };
                let _ = harness
                    .api_case(
                        report,
                        "bot.send_text_to_chat(get_my_id_response)",
                        move |bot| async move {
                            bot.send_text_to_chat(private_chat_id, reply_text).await
                        },
                    )
                    .await;
            } else {
                report.fail(
                    "manual.get_my_id_user_id",
                    tr(
                        lang,
                        "message was received, but sender.user_id is missing",
                        "сообщение получено, но sender.user_id отсутствует",
                    ),
                );
                report.skip(
                    "bot.send_text_to_chat(get_my_id_response)",
                    tr(
                        lang,
                        "sender.user_id is missing",
                        "sender.user_id отсутствует",
                    ),
                );
            }
        } else {
            report.skip(
                "manual.get_my_id_user_id",
                tr(
                    lang,
                    "`/get_my_id` step did not complete",
                    "шаг `/get_my_id` не был завершён",
                ),
            );
            report.skip(
                "bot.send_text_to_chat(get_my_id_response)",
                tr(
                    lang,
                    "`/get_my_id` step did not complete",
                    "шаг `/get_my_id` не был завершён",
                ),
            );
        }
    } else {
        report.skip(
            "manual.get_my_id_command",
            tr(
                lang,
                "tester skipped `/get_my_id`",
                "тестер пропустил `/get_my_id`",
            ),
        );
        report.skip(
            "manual.get_my_id_user_id",
            tr(
                lang,
                "tester skipped `/get_my_id`",
                "тестер пропустил `/get_my_id`",
            ),
        );
        report.skip(
            "bot.send_text_to_chat(get_my_id_response)",
            tr(
                lang,
                "tester skipped `/get_my_id`",
                "тестер пропустил `/get_my_id`",
            ),
        );
    }

    if confirm(
        lang,
        tr(
            lang,
            "Test edited-message update? Type `y`, then edit your last text message in Max.",
            "Проверить событие редактирования сообщения? Введите `y`, затем отредактируйте последнее текстовое сообщение в Max.",
        ),
        false,
    )? {
        let _ = harness
            .wait_case(
                report,
                "manual.message_edit",
                tr(
                    lang,
                    "Edit a message in the private chat in Max.",
                    "Отредактируйте сообщение в личном чате в Max.",
                ),
                Duration::from_secs(MANUAL_WAIT_SECS),
                |update| matches!(update, Update::MessageEdited { message, .. } if message.chat_id() == private_chat_id),
            )
            .await;
    } else {
        report.skip(
            "manual.message_edit",
            tr(
                lang,
                "tester skipped edited-message check",
                "тестер пропустил проверку редактирования сообщения",
            ),
        );
    }

    if let Some(plain_message) = plain_message {
        let message_id = plain_message.message_id().to_string();
        let _ = harness
            .api_case(report, "bot.edit_message", move |bot| async move {
                bot.edit_message(
                    &message_id,
                    NewMessageBody::text("maxoxide live test: edited text message"),
                )
                .await
            })
            .await;

        let message_id = plain_message.message_id().to_string();
        let _ = harness
            .api_case(report, "bot.get_message", move |bot| async move {
                bot.get_message(&message_id).await
            })
            .await;

        let _ = harness
            .api_case(report, "bot.get_messages", move |bot| async move {
                bot.get_messages(private_chat_id, Some(20), None, None)
                    .await
            })
            .await;

        let message_id = plain_message.message_id().to_string();
        let _ = harness
            .api_case(report, "bot.get_messages_by_ids", move |bot| async move {
                bot.get_messages_by_ids([message_id], Some(1), None, None)
                    .await
            })
            .await;

        let message_id = plain_message.message_id().to_string();
        let _ = harness
            .api_case(report, "bot.delete_message", move |bot| async move {
                bot.delete_message(&message_id).await
            })
            .await;
    } else {
        skip_cases(
            report,
            &[
                "bot.edit_message",
                "bot.get_message",
                "bot.get_messages",
                "bot.get_messages_by_ids",
                "bot.delete_message",
            ],
            tr(
                lang,
                "plain text message was not sent successfully",
                "простое текстовое сообщение не было успешно отправлено",
            ),
        );
    }

    Ok(PrivatePhaseState {
        chat_id: Some(private_chat_id),
        user_id: private_user_id,
    })
}

async fn run_upload_phase(
    harness: &mut Harness,
    report: &mut Report,
    private_chat_id: Option<i64>,
    private_user_id: Option<i64>,
    config: &Config,
) -> AnyResult<()> {
    let lang = config.lang;

    print_section(tr(lang, "Uploads", "Загрузки"));

    for upload_type in [
        UploadType::Image,
        UploadType::Video,
        UploadType::Audio,
        UploadType::File,
    ] {
        let name = format!("bot.get_upload_url({})", upload_type_name(&upload_type));
        let _ = harness
            .api_case(report, &name, move |bot| async move {
                bot.get_upload_url(upload_type).await
            })
            .await;
    }

    let upload_path = prepare_upload_file(config.upload_file_path.as_deref())?;
    match lang {
        Language::English => println!("Upload source file: {}", upload_path.display()),
        Language::Russian => println!("Файл-источник для загрузки: {}", upload_path.display()),
    }

    let upload_path_for_upload_file = upload_path.clone();
    let upload_file_token = harness
        .api_case(report, "bot.upload_file", move |bot| {
            let upload_path = upload_path_for_upload_file.clone();
            async move {
                bot.upload_file(
                    UploadType::File,
                    upload_path,
                    "maxoxide-live-upload.txt",
                    "text/plain",
                )
                .await
            }
        })
        .await;

    let bytes_payload = b"maxoxide live upload_bytes payload\n".to_vec();
    let upload_bytes_token = harness
        .api_case(report, "bot.upload_bytes", move |bot| async move {
            bot.upload_bytes(
                UploadType::File,
                bytes_payload,
                "maxoxide-live-bytes.txt",
                "text/plain",
            )
            .await
        })
        .await;

    if let Some(chat_id) = private_chat_id {
        if let Some(token) = upload_file_token {
            let body = NewMessageBody {
                text: Some("File attachment sent via upload_file.".into()),
                attachments: Some(vec![NewAttachment::file(token)]),
                ..Default::default()
            };
            let _ = harness
                .api_case(
                    report,
                    "bot.send_message_to_chat(upload_file_attachment)",
                    move |bot| async move { bot.send_message_to_chat(chat_id, body).await },
                )
                .await;
        } else {
            report.skip(
                "bot.send_message_to_chat(upload_file_attachment)",
                tr(
                    lang,
                    "upload_file did not return a token",
                    "upload_file не вернул токен",
                ),
            );
        }

        let helper_path = upload_path.clone();
        let helper_filename = filename_from_path(&helper_path, "maxoxide-live-upload.txt");
        let helper_mime = mime_for_path(&helper_path, "application/octet-stream");
        let _ = harness
            .api_case(report, "bot.send_file_to_chat", move |bot| {
                let helper_path = helper_path.clone();
                let helper_filename = helper_filename.clone();
                let helper_mime = helper_mime.clone();
                async move {
                    bot.send_file_to_chat(
                        chat_id,
                        helper_path,
                        helper_filename,
                        helper_mime,
                        Some("File sent via send_file_to_chat.".into()),
                    )
                    .await
                }
            })
            .await;

        let bytes_payload = b"maxoxide live send_file_bytes_to_chat payload\n".to_vec();
        let _ = harness
            .api_case(
                report,
                "bot.send_file_bytes_to_chat",
                move |bot| async move {
                    bot.send_file_bytes_to_chat(
                        chat_id,
                        bytes_payload,
                        "maxoxide-live-bytes-helper.txt",
                        "text/plain",
                        Some("File sent via send_file_bytes_to_chat.".into()),
                    )
                    .await
                },
            )
            .await;
    } else {
        skip_cases(
            report,
            &[
                "bot.send_message_to_chat(upload_file_attachment)",
                "bot.send_file_to_chat",
                "bot.send_file_bytes_to_chat",
            ],
            tr(
                lang,
                "private chat is not available",
                "личный чат недоступен",
            ),
        );
    }

    if let Some(user_id) = private_user_id {
        if let Some(token) = upload_bytes_token {
            let body = NewMessageBody {
                text: Some("File attachment sent via upload_bytes to user_id.".into()),
                attachments: Some(vec![NewAttachment::file(token)]),
                ..Default::default()
            };
            let _ = harness
                .api_case(
                    report,
                    "bot.send_message_to_user(upload_bytes_attachment)",
                    move |bot| async move { bot.send_message_to_user(user_id, body).await },
                )
                .await;
        } else {
            report.skip(
                "bot.send_message_to_user(upload_bytes_attachment)",
                tr(
                    lang,
                    "upload_bytes did not return a token",
                    "upload_bytes не вернул токен",
                ),
            );
        }

        let helper_path = upload_path.clone();
        let helper_filename = filename_from_path(&helper_path, "maxoxide-live-upload.txt");
        let helper_mime = mime_for_path(&helper_path, "application/octet-stream");
        let _ = harness
            .api_case(report, "bot.send_file_to_user", move |bot| {
                let helper_path = helper_path.clone();
                let helper_filename = helper_filename.clone();
                let helper_mime = helper_mime.clone();
                async move {
                    bot.send_file_to_user(
                        user_id,
                        helper_path,
                        helper_filename,
                        helper_mime,
                        Some("File sent via send_file_to_user.".into()),
                    )
                    .await
                }
            })
            .await;

        let bytes_payload = b"maxoxide live send_file_bytes_to_user payload\n".to_vec();
        let _ = harness
            .api_case(
                report,
                "bot.send_file_bytes_to_user",
                move |bot| async move {
                    bot.send_file_bytes_to_user(
                        user_id,
                        bytes_payload,
                        "maxoxide-live-bytes-helper.txt",
                        "text/plain",
                        Some("File sent via send_file_bytes_to_user.".into()),
                    )
                    .await
                },
            )
            .await;
    } else {
        skip_cases(
            report,
            &[
                "bot.send_message_to_user(upload_bytes_attachment)",
                "bot.send_file_to_user",
                "bot.send_file_bytes_to_user",
            ],
            tr(
                lang,
                "private user_id is not available",
                "private user_id недоступен",
            ),
        );
    }

    if let Some(chat_id) = private_chat_id {
        if let Some(image_path) = config.upload_image_path.clone() {
            let filename = filename_from_path(&image_path, "maxoxide-live-image");
            let mime = mime_for_path(&image_path, "image/jpeg");
            let _ = harness
                .api_case(report, "bot.send_image_to_chat", move |bot| {
                    let image_path = image_path.clone();
                    let filename = filename.clone();
                    let mime = mime.clone();
                    async move {
                        bot.send_image_to_chat(
                            chat_id,
                            image_path,
                            filename,
                            mime,
                            Some("Image sent via send_image_to_chat.".into()),
                        )
                        .await
                    }
                })
                .await;
        } else {
            report.skip(
                "bot.send_image_to_chat",
                tr(
                    lang,
                    "image path was not provided",
                    "путь к изображению не был указан",
                ),
            );
        }

        if let Some(video_path) = config.upload_video_path.clone() {
            let filename = filename_from_path(&video_path, "maxoxide-live-video");
            let mime = mime_for_path(&video_path, "video/mp4");
            let video_message = harness
                .api_case(report, "bot.send_video_to_chat", move |bot| {
                    let video_path = video_path.clone();
                    let filename = filename.clone();
                    let mime = mime.clone();
                    async move {
                        bot.send_video_to_chat(
                            chat_id,
                            video_path,
                            filename,
                            mime,
                            Some("Video sent via send_video_to_chat.".into()),
                        )
                        .await
                    }
                })
                .await;

            if let Some(video_message) = video_message {
                if let Some(video_token) = extract_video_token(&video_message) {
                    let _ = harness
                        .api_case(
                            report,
                            "bot.get_video(uploaded_video)",
                            move |bot| async move { bot.get_video(&video_token).await },
                        )
                        .await;
                } else {
                    report.fail(
                        "bot.get_video(uploaded_video)",
                        tr(
                            lang,
                            "sent video message did not contain a video token",
                            "отправленное видео-сообщение не содержит video token",
                        ),
                    );
                }
            } else {
                report.skip(
                    "bot.get_video(uploaded_video)",
                    tr(
                        lang,
                        "send_video_to_chat did not succeed",
                        "send_video_to_chat не завершился успешно",
                    ),
                );
            }
        } else {
            report.skip(
                "bot.send_video_to_chat",
                tr(
                    lang,
                    "video path was not provided",
                    "путь к видео не был указан",
                ),
            );
            report.skip(
                "bot.get_video(uploaded_video)",
                tr(
                    lang,
                    "video path was not provided",
                    "путь к видео не был указан",
                ),
            );
        }

        if let Some(audio_path) = config.upload_audio_path.clone() {
            let filename = filename_from_path(&audio_path, "maxoxide-live-audio");
            let mime = mime_for_path(&audio_path, "audio/mpeg");
            let _ = harness
                .api_case(report, "bot.send_audio_to_chat", move |bot| {
                    let audio_path = audio_path.clone();
                    let filename = filename.clone();
                    let mime = mime.clone();
                    async move {
                        bot.send_audio_to_chat(
                            chat_id,
                            audio_path,
                            filename,
                            mime,
                            Some("Audio sent via send_audio_to_chat.".into()),
                        )
                        .await
                    }
                })
                .await;
        } else {
            report.skip(
                "bot.send_audio_to_chat",
                tr(
                    lang,
                    "audio path was not provided",
                    "путь к аудиофайлу не был указан",
                ),
            );
        }
    } else {
        skip_cases(
            report,
            &[
                "bot.send_image_to_chat",
                "bot.send_video_to_chat",
                "bot.get_video(uploaded_video)",
                "bot.send_audio_to_chat",
            ],
            tr(
                lang,
                "private chat is not available",
                "личный чат недоступен",
            ),
        );
    }

    Ok(())
}

async fn run_webhook_phase(
    harness: &mut Harness,
    report: &mut Report,
    config: &Config,
) -> AnyResult<()> {
    let lang = config.lang;

    print_section(tr(lang, "Webhook", "Webhook"));

    let _ = harness
        .api_case(report, "bot.get_subscriptions", |bot| async move {
            bot.get_subscriptions().await
        })
        .await;

    let Some(url) = config.webhook_url.clone() else {
        skip_cases(
            report,
            &["bot.subscribe", "bot.unsubscribe"],
            tr(
                lang,
                "webhook URL was not provided",
                "webhook URL не был указан",
            ),
        );
        return Ok(());
    };

    let subscribe_url = url.clone();
    let secret = config.webhook_secret.clone();
    let _ = harness
        .api_case(report, "bot.subscribe", move |bot| async move {
            bot.subscribe(SubscribeBody {
                url: subscribe_url,
                update_types: None,
                version: None,
                secret,
            })
            .await
        })
        .await;

    let unsubscribe_url = url.clone();
    let _ = harness
        .api_case(report, "bot.unsubscribe", move |bot| async move {
            bot.unsubscribe(&unsubscribe_url).await
        })
        .await;

    Ok(())
}

async fn run_commands_phase(
    harness: &mut Harness,
    report: &mut Report,
    lang: Language,
) -> AnyResult<()> {
    print_section(tr(lang, "Commands", "Команды"));

    if confirm(
        lang,
        tr(
            lang,
            "Probe experimental bot.set_my_commands? The public MAX REST API does not currently document a write endpoint and may return 404. This also changes the bot command menu and is not restored automatically. Type `y` to proceed.",
            "Проверить экспериментальный bot.set_my_commands? Публичный REST API MAX сейчас не документирует write-эндпоинт и может вернуть 404. Также это изменит меню команд бота и автоматически не откатывается. Введите `y`, чтобы продолжить.",
        ),
        false,
    )? {
        let commands = vec![
            BotCommand {
                name: "live".into(),
                description: "Run the live API test".into(),
            },
            BotCommand {
                name: "group_live".into(),
                description: "Trigger the group phase".into(),
            },
        ];
        harness.pause().await;
        print_case("bot.set_my_commands");
        let bot = harness.bot.clone();
        match bot.set_my_commands(commands).await {
            Ok(_) => {
                report.pass("bot.set_my_commands", tr(lang, "ok", "ok"));
                println!("   PASS");
            }
            Err(err) => {
                let err_text = err.to_string();
                if err_text.contains("/me/commands")
                    && err_text.contains("404")
                    && err_text.contains("not recognized")
                {
                    let detail = tr(
                        lang,
                        "public MAX API does not currently expose POST /me/commands; treating this as a platform gap",
                        "публичный MAX API сейчас не предоставляет POST /me/commands; шаг помечен как платформенное ограничение",
                    );
                    report.skip("bot.set_my_commands", detail);
                    println!("   SKIP: {detail}");
                } else {
                    report.fail("bot.set_my_commands", err_text.clone());
                    println!("   FAIL: {err}");
                }
            }
        }
    } else {
        report.skip(
            "bot.set_my_commands",
            tr(
                lang,
                "tester did not confirm probing the experimental command-menu endpoint",
                "тестер не подтвердил проверку экспериментального эндпоинта меню команд",
            ),
        );
    }

    Ok(())
}

async fn run_group_phase(
    harness: &mut Harness,
    report: &mut Report,
    config: &Config,
    known_chats: &[Chat],
    known_user_id: Option<i64>,
) -> AnyResult<()> {
    let lang = config.lang;

    if !confirm(
        lang,
        tr(
            lang,
            "Run the optional group-chat phase now? Type `y` to continue, anything else to skip.",
            "Запустить необязательный этап с групповым чатом сейчас? Введите `y`, чтобы продолжить, иначе этап будет пропущен.",
        ),
        false,
    )? {
        skip_cases(
            report,
            &[
                "manual.group_activation",
                "bot.get_chat(group)",
                "bot.get_members",
                "bot.get_members_by_ids",
                "bot.get_admins",
                "bot.get_my_membership",
                "bot.send_sender_action(typing_on)",
                "bot.send_sending_image",
                "bot.send_sending_video",
                "bot.send_sending_audio",
                "bot.send_sending_file",
                "bot.mark_seen",
                "manual.observe_typing_indicator",
                "bot.send_message_to_chat(group)",
                "bot.pin_message",
                "bot.get_pinned_message",
                "bot.unpin_message",
                "bot.edit_chat",
                "bot.edit_chat(rollback)",
                "bot.add_admins",
                "bot.remove_admin",
                "bot.add_members",
                "bot.remove_member",
                "bot.delete_chat",
                "bot.leave_chat",
            ],
            tr(
                lang,
                "tester skipped the optional group-chat phase",
                "тестер пропустил необязательный этап с групповым чатом",
            ),
        );
        return Ok(());
    }

    print_section(tr(lang, "Group Chat", "Групповой чат"));
    println!(
        "{}",
        tr(
            lang,
            "1. Add the bot to a disposable group chat where it has admin rights.",
            "1. Добавьте бота во временную группу, где у него есть права администратора.",
        )
    );
    if let Some(link) = &config.bot_link {
        println!("   {}: {link}", tr(lang, "Bot URL", "URL бота"));
    }
    println!(
        "{}",
        tr(
            lang,
            "2. Send `/group_live` in that group.",
            "2. Отправьте `/group_live` в этой группе.",
        )
    );
    if let Some(user_id) = known_user_id {
        println!(
            "{}",
            match lang {
                Language::English => format!("Known user_id from the private phase: {user_id}"),
                Language::Russian => {
                    format!("Известный user_id из личного этапа: {user_id}")
                }
            }
        );
    }

    let activated_chat_id = harness
        .wait_case(
            report,
            "manual.group_activation",
            tr(
                lang,
                "Waiting for `/group_live` in a group or channel.",
                "Ожидание `/group_live` в группе или канале.",
            ),
            Duration::from_secs(GROUP_WAIT_SECS),
            |update| match update {
                Update::MessageCreated { message, .. } => {
                    message.recipient.chat_type != ChatType::Dialog
                        && message.text() == Some("/group_live")
                }
                _ => false,
            },
        )
        .await
        .and_then(|update| match update {
            Update::MessageCreated { message, .. } => Some(message.chat_id()),
            _ => None,
        });

    let group_chat_id = match activated_chat_id {
        Some(chat_id) => Some(chat_id),
        None => {
            if !known_chats.is_empty() {
                println!(
                    "{}",
                    tr(
                        lang,
                        "Known group chats from bot.get_chats:",
                        "Известные групповые чаты из bot.get_chats:",
                    )
                );
                print_known_chats(known_chats, lang);
            }
            prompt_optional_i64(
                lang,
                tr(
                    lang,
                    "Enter a group chat_id manually to continue the group phase, or leave blank to skip",
                    "Введите group chat_id вручную, чтобы продолжить групповой этап, или оставьте поле пустым для пропуска",
                ),
            )?
        }
    };

    let Some(group_chat_id) = group_chat_id else {
        skip_cases(
            report,
            &[
                "bot.get_chat(group)",
                "bot.get_members",
                "bot.get_members_by_ids",
                "bot.get_admins",
                "bot.get_my_membership",
                "bot.send_sender_action(typing_on)",
                "bot.send_sending_image",
                "bot.send_sending_video",
                "bot.send_sending_audio",
                "bot.send_sending_file",
                "bot.mark_seen",
                "manual.observe_typing_indicator",
                "bot.pin_message",
                "bot.get_pinned_message",
                "bot.unpin_message",
                "bot.edit_chat",
                "bot.add_admins",
                "bot.remove_admin",
                "bot.add_members",
                "bot.remove_member",
                "bot.delete_chat",
                "bot.leave_chat",
            ],
            tr(
                lang,
                "group chat was not selected",
                "групповой чат не был выбран",
            ),
        );
        return Ok(());
    };

    match lang {
        Language::English => println!("Selected group chat id: {group_chat_id}"),
        Language::Russian => println!("Выбранный group chat id: {group_chat_id}"),
    }

    let group_chat = harness
        .api_case(report, "bot.get_chat(group)", move |bot| async move {
            bot.get_chat(group_chat_id).await
        })
        .await;

    let members = harness
        .api_case(report, "bot.get_members", move |bot| async move {
            bot.get_members(group_chat_id, Some(100), None).await
        })
        .await;
    if let Some(members) = members.as_ref() {
        print_chat_members(&members.members, lang);
    }

    let selected_member_id = known_user_id.or_else(|| {
        members
            .as_ref()
            .and_then(|members| members.members.first())
            .map(|member| member.user_id)
    });
    if let Some(user_id) = selected_member_id {
        let _ = harness
            .api_case(report, "bot.get_members_by_ids", move |bot| async move {
                bot.get_members_by_ids(group_chat_id, [user_id]).await
            })
            .await;
    } else {
        report.skip(
            "bot.get_members_by_ids",
            tr(
                lang,
                "no member user_id is available",
                "нет доступного user_id участника",
            ),
        );
    }

    let _ = harness
        .api_case(report, "bot.get_admins", move |bot| async move {
            bot.get_admins(group_chat_id).await
        })
        .await;

    let bot_membership = harness
        .api_case(report, "bot.get_my_membership", move |bot| async move {
            bot.get_my_membership(group_chat_id).await
        })
        .await;
    if let Some(member) = bot_membership.as_ref() {
        print_bot_membership(member, lang);
    }

    if harness
        .api_case(
            report,
            "bot.send_sender_action(typing_on)",
            move |bot| async move {
                bot.send_sender_action(group_chat_id, SenderAction::TypingOn)
                    .await
            },
        )
        .await
        .is_some()
    {
        if confirm(
            lang,
            tr(
                lang,
                "Did the typing indicator become visible in the group chat?",
                "Появился ли в групповом чате индикатор набора текста?",
            ),
            true,
        )? {
            report.pass(
                "manual.observe_typing_indicator",
                tr(lang, "tester confirmed", "тестер подтвердил"),
            );
        } else {
            report.skip(
                "manual.observe_typing_indicator",
                tr(
                    lang,
                    "MAX client did not show a visible typing indicator; treating this as a current platform gap",
                    "клиент MAX не показал видимый индикатор набора текста; шаг помечен как текущее платформенное ограничение",
                ),
            );
        }
    }

    let _ = harness
        .api_case(report, "bot.send_sending_image", move |bot| async move {
            bot.send_sending_image(group_chat_id).await
        })
        .await;
    let _ = harness
        .api_case(report, "bot.send_sending_video", move |bot| async move {
            bot.send_sending_video(group_chat_id).await
        })
        .await;
    let _ = harness
        .api_case(report, "bot.send_sending_audio", move |bot| async move {
            bot.send_sending_audio(group_chat_id).await
        })
        .await;
    let _ = harness
        .api_case(report, "bot.send_sending_file", move |bot| async move {
            bot.send_sending_file(group_chat_id).await
        })
        .await;
    let _ = harness
        .api_case(report, "bot.mark_seen", move |bot| async move {
            bot.mark_seen(group_chat_id).await
        })
        .await;

    let group_message = harness
        .api_case(
            report,
            "bot.send_message_to_chat(group)",
            move |bot| async move {
                bot.send_message_to_chat(
                    group_chat_id,
                    NewMessageBody::text("maxoxide live test: group message for pin/edit flow"),
                )
                .await
            },
        )
        .await;

    if let Some(group_message) = group_message {
        let message_id = group_message.message_id().to_string();
        let _ = harness
            .api_case(report, "bot.pin_message", move |bot| async move {
                bot.pin_message(
                    group_chat_id,
                    PinMessageBody {
                        message_id,
                        notify: Some(false),
                    },
                )
                .await
            })
            .await;

        let _ = harness
            .api_case(report, "bot.get_pinned_message", move |bot| async move {
                bot.get_pinned_message(group_chat_id).await
            })
            .await;

        let _ = harness
            .api_case(report, "bot.unpin_message", move |bot| async move {
                bot.unpin_message(group_chat_id).await
            })
            .await;
    } else {
        skip_cases(
            report,
            &[
                "bot.pin_message",
                "bot.get_pinned_message",
                "bot.unpin_message",
            ],
            tr(
                lang,
                "group message setup failed",
                "не удалось подготовить групповое сообщение",
            ),
        );
    }

    if confirm(
        lang,
        tr(
            lang,
            "Test bot.edit_chat with temporary title change and automatic rollback? Type `y` to proceed.",
            "Проверить bot.edit_chat с временной сменой title и автоматическим откатом? Введите `y`, чтобы продолжить.",
        ),
        false,
    )? {
        if let Some(group_chat) = group_chat.as_ref() {
            if let Some(original_title) = group_chat.title.clone() {
                let temp_title = format!("{original_title} [live]");
                let _ = harness
                    .api_case(report, "bot.edit_chat", move |bot| async move {
                        bot.edit_chat(
                            group_chat_id,
                            EditChatBody {
                                title: Some(temp_title),
                                ..Default::default()
                            },
                        )
                        .await
                    })
                    .await;

                let _ = harness
                    .api_case(report, "bot.edit_chat(rollback)", move |bot| async move {
                        bot.edit_chat(
                            group_chat_id,
                            EditChatBody {
                                title: Some(original_title),
                                ..Default::default()
                            },
                        )
                        .await
                    })
                    .await;
            } else {
                report.skip(
                    "bot.edit_chat",
                    tr(
                        lang,
                        "group chat title is empty, rollback would be unsafe",
                        "title группового чата пустой, откат был бы небезопасен",
                    ),
                );
                report.skip(
                    "bot.edit_chat(rollback)",
                    tr(
                        lang,
                        "group chat title is empty, rollback would be unsafe",
                        "title группового чата пустой, откат был бы небезопасен",
                    ),
                );
            }
        } else {
            report.skip(
                "bot.edit_chat",
                tr(
                    lang,
                    "group chat metadata is unavailable",
                    "метаданные группового чата недоступны",
                ),
            );
            report.skip(
                "bot.edit_chat(rollback)",
                tr(
                    lang,
                    "group chat metadata is unavailable",
                    "метаданные группового чата недоступны",
                ),
            );
        }
    } else {
        report.skip(
            "bot.edit_chat",
            tr(
                lang,
                "tester skipped visible group mutation",
                "тестер пропустил видимое изменение группы",
            ),
        );
        report.skip(
            "bot.edit_chat(rollback)",
            tr(
                lang,
                "tester skipped visible group mutation",
                "тестер пропустил видимое изменение группы",
            ),
        );
    }

    let admin_user_id = prompt_optional_i64(
        lang,
        tr(
            lang,
            "Optional platform probe: enter a user_id for bot.add_admins/bot.remove_admin, or leave blank to skip",
            "Опциональная platform-проверка: введите user_id для bot.add_admins/bot.remove_admin или оставьте поле пустым",
        ),
    )?;
    if let Some(user_id) = admin_user_id {
        if typed_confirmation(
            tr(
                lang,
                "Type `ADMIN` to confirm temporary admin rights change for this user_id",
                "Введите `ADMIN`, чтобы подтвердить временное изменение admin-прав для этого user_id",
            ),
            "ADMIN",
        )? {
            let bot_can_add_admins = member_can_add_admins(bot_membership.as_ref());
            if !bot_can_add_admins {
                skip_cases(
                    report,
                    &["bot.add_admins", "bot.remove_admin"],
                    tr(
                        lang,
                        "bot membership does not include the add_admins permission",
                        "в правах бота нет add_admins",
                    ),
                );
            } else {
                let permissions_to_grant = admin_probe_permissions(bot_membership.as_ref());
                harness.pause().await;
                print_case("bot.add_admins");
                let bot = harness.bot.clone();
                let add_admins_result = bot
                    .add_admins(
                        group_chat_id,
                        vec![ChatAdmin {
                            user_id,
                            permissions: permissions_to_grant,
                            alias: None,
                        }],
                    )
                    .await;

                let added = match add_admins_result {
                    Ok(_) => {
                        report.pass("bot.add_admins", tr(lang, "ok", "ok"));
                        println!("   PASS");
                        true
                    }
                    Err(err) => {
                        report.fail("bot.add_admins", err.to_string());
                        println!("   FAIL: {err}");
                        false
                    }
                };

                if added {
                    let _ = harness
                        .api_case(report, "bot.remove_admin", move |bot| async move {
                            bot.remove_admin(group_chat_id, user_id).await
                        })
                        .await;
                } else {
                    report.skip(
                        "bot.remove_admin",
                        tr(
                            lang,
                            "bot.add_admins did not succeed",
                            "bot.add_admins не завершился успешно",
                        ),
                    );
                }
            }
        } else {
            skip_cases(
                report,
                &["bot.add_admins", "bot.remove_admin"],
                tr(
                    lang,
                    "tester did not confirm admin rights probe",
                    "тестер не подтвердил проверку admin-прав",
                ),
            );
        }
    } else {
        skip_cases(
            report,
            &["bot.add_admins", "bot.remove_admin"],
            tr(
                lang,
                "tester did not provide a user_id",
                "тестер не указал user_id",
            ),
        );
    }

    let member_user_id = prompt_optional_i64(
        lang,
        tr(
            lang,
            "Enter a user_id for bot.add_members/bot.remove_member, or leave blank to skip",
            "Введите user_id для bot.add_members/bot.remove_member, или оставьте поле пустым для пропуска",
        ),
    )?;
    if let Some(user_id) = member_user_id {
        let added = harness
            .api_case(report, "bot.add_members", move |bot| async move {
                bot.add_members(group_chat_id, vec![user_id]).await
            })
            .await
            .is_some();

        if added {
            let _ = harness
                .api_case(report, "bot.remove_member", move |bot| async move {
                    bot.remove_member(group_chat_id, user_id).await
                })
                .await;
        } else {
            report.skip(
                "bot.remove_member",
                tr(
                    lang,
                    "bot.add_members did not succeed",
                    "bot.add_members не завершился успешно",
                ),
            );
        }
    } else {
        report.skip(
            "bot.add_members",
            tr(
                lang,
                "tester did not provide a user_id",
                "тестер не указал user_id",
            ),
        );
        report.skip(
            "bot.remove_member",
            tr(
                lang,
                "tester did not provide a user_id",
                "тестер не указал user_id",
            ),
        );
    }

    let delete_chat_id = prompt_optional_i64(
        lang,
        tr(
            lang,
            "Enter a disposable chat_id for bot.delete_chat, or leave blank to skip",
            "Введите disposable chat_id для bot.delete_chat, или оставьте поле пустым для пропуска",
        ),
    )?;
    let mut deleted_selected_group = false;
    if let Some(delete_chat_id) = delete_chat_id {
        if typed_confirmation(
            tr(
                lang,
                "Type `DELETE` to confirm bot.delete_chat on the provided chat_id",
                "Введите `УДАЛИТЬ`, чтобы подтвердить bot.delete_chat для указанного chat_id",
            ),
            tr(lang, "DELETE", "УДАЛИТЬ"),
        )? {
            let deleted = harness
                .api_case(report, "bot.delete_chat", move |bot| async move {
                    bot.delete_chat(delete_chat_id).await
                })
                .await
                .is_some();
            deleted_selected_group = delete_chat_id == group_chat_id && deleted;
        } else {
            report.skip(
                "bot.delete_chat",
                tr(
                    lang,
                    "tester did not confirm delete_chat",
                    "тестер не подтвердил delete_chat",
                ),
            );
        }
    } else {
        report.skip(
            "bot.delete_chat",
            tr(
                lang,
                "tester did not provide a disposable chat_id",
                "тестер не указал disposable chat_id",
            ),
        );
    }

    if deleted_selected_group {
        report.skip(
            "bot.leave_chat",
            tr(
                lang,
                "selected group chat was deleted",
                "выбранный групповой чат был удалён",
            ),
        );
    } else if confirm(
        lang,
        tr(
            lang,
            "Test bot.leave_chat on the selected group now? Type `y` to make the bot leave the group.",
            "Проверить bot.leave_chat для выбранной группы сейчас? Введите `y`, чтобы бот покинул группу.",
        ),
        false,
    )? {
        let _ = harness
            .api_case(report, "bot.leave_chat", move |bot| async move {
                bot.leave_chat(group_chat_id).await
            })
            .await;
    } else {
        report.skip(
            "bot.leave_chat",
            tr(
                lang,
                "tester skipped leave_chat",
                "тестер пропустил leave_chat",
            ),
        );
    }

    Ok(())
}

#[derive(Clone)]
struct Config {
    lang: Language,
    token: String,
    bot_link: Option<String>,
    webhook_url: Option<String>,
    webhook_secret: Option<String>,
    upload_file_path: Option<PathBuf>,
    upload_image_path: Option<PathBuf>,
    upload_video_path: Option<PathBuf>,
    upload_audio_path: Option<PathBuf>,
    request_delay: Duration,
    http_timeout: Duration,
    poll_timeout: u32,
}

impl Config {
    fn prompt(lang: Language) -> AnyResult<Self> {
        print_section(tr(lang, "Configuration", "Конфигурация"));
        println!(
            "{}",
            tr(
                lang,
                "Secrets entered here are echoed in the terminal.",
                "Секреты, введённые здесь, будут отображаться в терминале.",
            )
        );

        let token = prompt_required(lang, tr(lang, "Bot token", "Токен бота"))?;
        let bot_link = prompt_optional(
            lang,
            tr(
                lang,
                "Bot URL for the tester (optional)",
                "URL бота для тестера (необязательно)",
            ),
        )?;
        let webhook_url = prompt_optional(
            lang,
            tr(
                lang,
                "Webhook URL for subscribe/unsubscribe (optional)",
                "Webhook URL для subscribe/unsubscribe (необязательно)",
            ),
        )?;
        let webhook_secret = prompt_optional(
            lang,
            tr(
                lang,
                "Webhook secret (optional)",
                "Webhook secret (необязательно)",
            ),
        )?;
        let upload_file_path = prompt_optional(
            lang,
            tr(
                lang,
                "Path to a local file for bot.upload_file (optional)",
                "Путь к локальному файлу для bot.upload_file (необязательно)",
            ),
        )?
        .map(PathBuf::from);
        let upload_image_path = prompt_optional(
            lang,
            tr(
                lang,
                "Path to an image for send_image_to_chat (optional)",
                "Путь к изображению для send_image_to_chat (необязательно)",
            ),
        )?
        .map(PathBuf::from);
        let upload_video_path = prompt_optional(
            lang,
            tr(
                lang,
                "Path to a video for send_video_to_chat/get_video (optional)",
                "Путь к видео для send_video_to_chat/get_video (необязательно)",
            ),
        )?
        .map(PathBuf::from);
        let upload_audio_path = prompt_optional(
            lang,
            tr(
                lang,
                "Path to an audio file for send_audio_to_chat (optional)",
                "Путь к аудиофайлу для send_audio_to_chat (необязательно)",
            ),
        )?
        .map(PathBuf::from);
        let request_delay_ms = prompt_u64(
            lang,
            tr(
                lang,
                "Delay between API requests in ms",
                "Задержка между API-запросами в мс",
            ),
            400,
        )?;
        let http_timeout_secs = prompt_u64(
            lang,
            tr(lang, "HTTP timeout in seconds", "HTTP timeout в секундах"),
            15,
        )?;
        let poll_timeout = prompt_u32(
            lang,
            tr(
                lang,
                "Long polling timeout in seconds",
                "Long polling timeout в секундах",
            ),
            5,
        )?;

        Ok(Self {
            lang,
            token,
            bot_link,
            webhook_url,
            webhook_secret,
            upload_file_path,
            upload_image_path,
            upload_video_path,
            upload_audio_path,
            request_delay: Duration::from_millis(request_delay_ms),
            http_timeout: Duration::from_secs(http_timeout_secs.max(1)),
            poll_timeout: poll_timeout.max(1),
        })
    }
}

struct Harness {
    bot: Bot,
    marker: Option<i64>,
    request_delay: Duration,
    poll_timeout: u32,
    lang: Language,
}

impl Harness {
    fn new(bot: Bot, request_delay: Duration, poll_timeout: u32, lang: Language) -> Self {
        Self {
            bot,
            marker: None,
            request_delay,
            poll_timeout,
            lang,
        }
    }

    async fn api_case<T, F, Fut>(
        &mut self,
        report: &mut Report,
        name: &str,
        operation: F,
    ) -> Option<T>
    where
        F: FnOnce(Bot) -> Fut,
        Fut: Future<Output = maxoxide::Result<T>>,
    {
        self.pause().await;
        print_case(name);
        let bot = self.bot.clone();
        match operation(bot).await {
            Ok(value) => {
                report.pass(name, tr(self.lang, "ok", "ok"));
                println!("   PASS");
                Some(value)
            }
            Err(err) => {
                report.fail(name, err.to_string());
                println!("   FAIL: {err}");
                None
            }
        }
    }

    async fn flush_updates(&mut self) -> maxoxide::Result<usize> {
        let mut drained = 0usize;
        loop {
            self.pause().await;
            let response = self
                .bot
                .get_updates(self.marker, Some(1), Some(100))
                .await?;
            if let Some(marker) = response.marker {
                self.marker = Some(marker);
            }
            drained += response.updates.len();
            if response.updates.is_empty() {
                return Ok(drained);
            }
        }
    }

    async fn wait_case<F>(
        &mut self,
        report: &mut Report,
        name: &str,
        instructions: &str,
        timeout: Duration,
        predicate: F,
    ) -> Option<Update>
    where
        F: Fn(&Update) -> bool,
    {
        print_case(name);
        println!("   {instructions}");
        let started = Instant::now();

        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                let detail = tr(
                    self.lang,
                    "timeout while waiting for update",
                    "таймаут ожидания обновления",
                );
                report.fail(name, detail);
                println!("   FAIL: {detail}");
                return None;
            }

            let chunk = remaining.min(Duration::from_secs(WAIT_PROMPT_CHUNK_SECS));
            match self.wait_for_update_chunk(chunk, &predicate).await {
                Ok(Some(update)) => {
                    report.pass(name, tr(self.lang, "event received", "событие получено"));
                    println!("   PASS");
                    print_update_details(self.lang, &update);
                    return Some(update);
                }
                Ok(None) => match prompt_wait_decision(self.lang) {
                    Ok(WaitDecision::Continue) => continue,
                    Ok(WaitDecision::Skip) => {
                        let detail = tr(
                            self.lang,
                            "tester skipped this waiting step",
                            "тестер пропустил этот шаг ожидания",
                        );
                        report.skip(name, detail);
                        println!("   SKIP: {detail}");
                        return None;
                    }
                    Ok(WaitDecision::Fail) => {
                        let detail = tr(
                            self.lang,
                            "tester marked this waiting step as failed",
                            "тестер пометил этот шаг ожидания как проваленный",
                        );
                        report.fail(name, detail);
                        println!("   FAIL: {detail}");
                        return None;
                    }
                    Err(err) => {
                        report.fail(name, err.to_string());
                        println!("   FAIL: {err}");
                        return None;
                    }
                },
                Err(err) => {
                    report.fail(name, err.to_string());
                    println!("   FAIL: {err}");
                    return None;
                }
            }
        }
    }

    async fn wait_for_update_chunk<F>(
        &mut self,
        timeout: Duration,
        predicate: &F,
    ) -> AnyResult<Option<Update>>
    where
        F: Fn(&Update) -> bool,
    {
        let started = Instant::now();
        let mut logged_non_matching = 0usize;
        loop {
            if started.elapsed() >= timeout {
                return Ok(None);
            }

            self.pause().await;

            let remaining = timeout.saturating_sub(started.elapsed());
            let poll_secs = remaining.as_secs().min(self.poll_timeout as u64).max(1) as u32;
            let response = self
                .bot
                .get_updates(self.marker, Some(poll_secs), Some(100))
                .await?;

            if let Some(marker) = response.marker {
                self.marker = Some(marker);
            }

            for update in response.updates {
                if predicate(&update) {
                    return Ok(Some(update));
                }

                if logged_non_matching < MAX_NON_MATCHING_UPDATE_LOGS {
                    println!(
                        "   {}",
                        tr(
                            self.lang,
                            "Observed a non-matching update while waiting:",
                            "Получено неподходящее обновление во время ожидания:",
                        )
                    );
                    print_update_details(self.lang, &update);
                    logged_non_matching += 1;
                } else if logged_non_matching == MAX_NON_MATCHING_UPDATE_LOGS {
                    println!(
                        "   {}",
                        tr(
                            self.lang,
                            "Further non-matching updates are hidden for this wait chunk.",
                            "Дальнейшие неподходящие обновления в этом интервале скрыты.",
                        )
                    );
                    logged_non_matching += 1;
                }
            }
        }
    }

    async fn pause(&self) {
        if !self.request_delay.is_zero() {
            sleep(self.request_delay).await;
        }
    }
}

#[derive(Default)]
struct Report {
    records: Vec<Record>,
}

impl Report {
    fn pass(&mut self, name: impl Into<String>, detail: impl Into<String>) {
        self.records.push(Record {
            name: name.into(),
            outcome: Outcome::Passed(detail.into()),
        });
    }

    fn fail(&mut self, name: impl Into<String>, detail: impl Into<String>) {
        self.records.push(Record {
            name: name.into(),
            outcome: Outcome::Failed(detail.into()),
        });
    }

    fn skip(&mut self, name: impl Into<String>, detail: impl Into<String>) {
        self.records.push(Record {
            name: name.into(),
            outcome: Outcome::Skipped(detail.into()),
        });
    }

    fn print_summary(&self, lang: Language) {
        print_section(tr(lang, "Summary", "Сводка"));

        let passed = self
            .records
            .iter()
            .filter(|record| matches!(record.outcome, Outcome::Passed(_)))
            .count();
        let failed = self
            .records
            .iter()
            .filter(|record| matches!(record.outcome, Outcome::Failed(_)))
            .count();
        let skipped = self
            .records
            .iter()
            .filter(|record| matches!(record.outcome, Outcome::Skipped(_)))
            .count();

        match lang {
            Language::English => {
                println!("Passed: {passed}");
                println!("Failed: {failed}");
                println!("Skipped: {skipped}");
            }
            Language::Russian => {
                println!("Успешно: {passed}");
                println!("Провалено: {failed}");
                println!("Пропущено: {skipped}");
            }
        }

        for record in &self.records {
            match &record.outcome {
                Outcome::Passed(detail) => println!("[PASS] {}: {}", record.name, detail),
                Outcome::Failed(detail) => println!("[FAIL] {}: {}", record.name, detail),
                Outcome::Skipped(detail) => println!("[SKIP] {}: {}", record.name, detail),
            }
        }
    }
}

struct Record {
    name: String,
    outcome: Outcome,
}

enum Outcome {
    Passed(String),
    Failed(String),
    Skipped(String),
}

enum WaitDecision {
    Continue,
    Skip,
    Fail,
}

fn prepare_upload_file(path: Option<&Path>) -> AnyResult<PathBuf> {
    if let Some(path) = path {
        return Ok(path.to_path_buf());
    }

    let path = std::env::temp_dir().join("maxoxide-live-upload.txt");
    std::fs::write(&path, b"maxoxide live upload_file payload\n")?;
    Ok(path)
}

fn filename_from_path(path: &Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(fallback)
        .to_string()
}

fn mime_for_path(path: &Path, fallback: &str) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("m4a") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        _ => fallback,
    }
    .to_string()
}

fn message_has_attachment<F>(attachments: &Option<Vec<Attachment>>, predicate: F) -> bool
where
    F: Fn(&Attachment) -> bool,
{
    attachments
        .as_ref()
        .map(|items| items.iter().any(predicate))
        .unwrap_or(false)
}

fn is_contact(attachment: &Attachment) -> bool {
    matches!(attachment, Attachment::Contact { .. })
}

fn is_location(attachment: &Attachment) -> bool {
    matches!(attachment, Attachment::Location { .. })
}

fn looks_like_client_map_card(message: &Message) -> bool {
    let mut haystack = String::new();

    if let Some(text) = message.text() {
        haystack.push_str(text);
        haystack.push('\n');
    }

    if let Some(url) = &message.url {
        haystack.push_str(url);
        haystack.push('\n');
    }

    if let Some(constructor) = &message.constructor {
        haystack.push_str(&constructor.to_string());
    }

    if let Some(attachments) = &message.body.attachments {
        for attachment in attachments {
            if let Ok(json) = serde_json::to_string(attachment) {
                haystack.push('\n');
                haystack.push_str(&json);
            }
        }
    }

    let normalized = haystack.to_ascii_lowercase();
    normalized.contains("yandex")
        || normalized.contains("яндекс")
        || normalized.contains("maps")
        || normalized.contains("yandex.ru/maps")
}

fn is_non_keyboard_attachment(attachment: &Attachment) -> bool {
    !matches!(attachment, Attachment::InlineKeyboard { .. })
}

fn extract_video_token(message: &Message) -> Option<String> {
    message
        .body
        .attachments
        .as_ref()?
        .iter()
        .find_map(|attachment| match attachment {
            Attachment::Video { payload } => payload.token.clone(),
            _ => None,
        })
}

fn upload_type_name(upload_type: &UploadType) -> &'static str {
    match upload_type {
        UploadType::Image => "image",
        UploadType::Video => "video",
        UploadType::Audio => "audio",
        UploadType::File => "file",
        _ => "unknown",
    }
}

fn skip_cases(report: &mut Report, names: &[&str], reason: &str) {
    for name in names {
        report.skip(*name, reason);
    }
}

fn member_can_add_admins(member: Option<&maxoxide::types::ChatMember>) -> bool {
    member
        .map(|member| {
            member.is_owner == Some(true)
                || member
                    .permissions
                    .as_deref()
                    .map(|permissions| permissions.contains(&ChatAdminPermission::AddAdmins))
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn admin_probe_permissions(
    member: Option<&maxoxide::types::ChatMember>,
) -> Vec<ChatAdminPermission> {
    let Some(member) = member else {
        return vec![ChatAdminPermission::ReadAllMessages];
    };

    if member.is_owner == Some(true) {
        return vec![ChatAdminPermission::ReadAllMessages];
    }

    let permissions = member.permissions.as_deref().unwrap_or(&[]);
    for preferred in [
        ChatAdminPermission::ReadAllMessages,
        ChatAdminPermission::Write,
        ChatAdminPermission::AddRemoveMembers,
    ] {
        if permissions.contains(&preferred) {
            return vec![preferred];
        }
    }

    permissions
        .iter()
        .find(|permission| **permission != ChatAdminPermission::AddAdmins)
        .cloned()
        .or_else(|| permissions.first().cloned())
        .map(|permission| vec![permission])
        .unwrap_or_else(|| vec![ChatAdminPermission::ReadAllMessages])
}

fn prompt_wait_decision(lang: Language) -> AnyResult<WaitDecision> {
    loop {
        let answer = prompt(tr(
            lang,
            "No matching update yet. Press Enter to continue waiting, type `skip` to skip, or `fail` to mark this step as failed",
            "Подходящее обновление пока не пришло. Нажмите Enter, чтобы ждать дальше, введите `skip` для пропуска или `fail`, чтобы пометить шаг как проваленный",
        ))?;

        let normalized = answer.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "c" | "continue" | "wait" | "ждать" => return Ok(WaitDecision::Continue),
            "s" | "skip" | "пропуск" | "пропустить" => {
                return Ok(WaitDecision::Skip);
            }
            "f" | "fail" | "ошибка" | "провал" => return Ok(WaitDecision::Fail),
            _ => println!(
                "{}",
                tr(
                    lang,
                    "Expected Enter, `skip`, or `fail`.",
                    "Ожидался Enter, `skip` или `fail`.",
                )
            ),
        }
    }
}

fn print_update_details(lang: Language, update: &Update) {
    if let Some(update_type) = update.update_type() {
        println!("   update_type: {update_type}");
    }

    match update {
        Update::MessageCallback { callback, .. } => {
            println!("   callback_id: {}", callback.callback_id);
            println!(
                "   {}: {}",
                tr(lang, "user_id", "user_id"),
                callback.user.user_id
            );
            if let Some(payload) = &callback.payload {
                println!("   payload: {payload}");
            }
        }
        Update::MessageCreated { message, .. } | Update::MessageEdited { message, .. } => {
            println!("   chat_id: {}", message.chat_id());
            println!("   message_id: {}", message.message_id());
            if let Some(sender) = &message.sender {
                println!("   {}: {}", tr(lang, "user_id", "user_id"), sender.user_id);
                println!(
                    "   {}: {}",
                    tr(lang, "sender", "отправитель"),
                    sender.display_name()
                );
            }
            if let Some(text) = message.text() {
                println!("   {}: {text}", tr(lang, "text", "текст"));
            }
            if let Some(url) = &message.url {
                println!("   url: {url}");
            }
            if let Some(constructor) = &message.constructor {
                println!("   constructor: {constructor}");
            }
            if let Some(attachments) = &message.body.attachments {
                for attachment in attachments {
                    println!(
                        "   {}: {}",
                        tr(lang, "attachment", "вложение"),
                        attachment_debug_name(attachment)
                    );
                    match attachment {
                        Attachment::Image { payload }
                        | Attachment::Video { payload }
                        | Attachment::Audio { payload } => {
                            if let Some(url) = &payload.url {
                                println!("   attachment_url: {url}");
                            }
                            if let Some(token) = &payload.token {
                                println!("   attachment_token: {token}");
                            }
                            if let Some(photo_id) = payload.photo_id {
                                println!("   photo_id: {photo_id}");
                            }
                        }
                        Attachment::File { payload } => {
                            if let Some(url) = &payload.url {
                                println!("   attachment_url: {url}");
                            }
                            if let Some(token) = &payload.token {
                                println!("   attachment_token: {token}");
                            }
                            if let Some(filename) = &payload.filename {
                                println!("   filename: {filename}");
                            }
                            if let Some(size) = payload.size {
                                println!("   size: {size}");
                            }
                        }
                        Attachment::Contact { payload } => {
                            println!(
                                "   {}: {:?}",
                                tr(lang, "contact_name", "имя_контакта"),
                                payload.name
                            );
                            println!(
                                "   {}: {:?}",
                                tr(lang, "contact_id", "contact_id"),
                                payload.contact_id
                            );
                            println!(
                                "   {}: {:?}",
                                tr(lang, "phone", "телефон"),
                                payload.vcf_phone
                            );
                        }
                        Attachment::Location { payload } => {
                            println!(
                                "   {}: {}, {}: {}",
                                tr(lang, "latitude", "широта"),
                                payload.latitude,
                                tr(lang, "longitude", "долгота"),
                                payload.longitude
                            );
                        }
                        Attachment::Unknown { payload, raw, .. } => {
                            if let Some(payload) = payload {
                                println!("   attachment_payload: {payload}");
                            }
                            println!("   attachment_raw: {raw}");
                        }
                        _ => {}
                    }
                }
            }
        }
        Update::Unknown { raw, .. } => {
            println!("   raw_update: {raw}");
        }
        _ => {}
    }
}

fn attachment_debug_name(attachment: &Attachment) -> &str {
    match attachment {
        Attachment::Image { .. } => "image",
        Attachment::Video { .. } => "video",
        Attachment::Audio { .. } => "audio",
        Attachment::File { .. } => "file",
        Attachment::Sticker { .. } => "sticker",
        Attachment::InlineKeyboard { .. } => "inline_keyboard",
        Attachment::Location { .. } => "location",
        Attachment::Contact { .. } => "contact",
        Attachment::Unknown { r#type, .. } => r#type.as_str(),
        _ => "unknown",
    }
}

fn extract_contact_phone(update: &Update) -> Option<&str> {
    let attachments = match update {
        Update::MessageCreated { message, .. } | Update::MessageEdited { message, .. } => {
            message.body.attachments.as_ref()?
        }
        _ => return None,
    };

    attachments.iter().find_map(|attachment| match attachment {
        Attachment::Contact { payload } => payload.vcf_phone.as_deref(),
        _ => None,
    })
}

fn extract_sender_user_id(update: &Update) -> Option<i64> {
    match update {
        Update::MessageCreated { message, .. } | Update::MessageEdited { message, .. } => {
            message.sender.as_ref().map(|user| user.user_id)
        }
        Update::MessageCallback { callback, .. } => Some(callback.user.user_id),
        Update::BotStarted { user, .. }
        | Update::BotAdded { user, .. }
        | Update::BotRemoved { user, .. }
        | Update::UserAdded { user, .. }
        | Update::UserRemoved { user, .. }
        | Update::ChatTitleChanged { user, .. } => Some(user.user_id),
        Update::MessageRemoved { user_id, .. } => Some(*user_id),
        Update::Unknown { .. } => None,
        _ => None,
    }
}

fn confirm_case(lang: Language, report: &mut Report, name: &str, question: &str) -> AnyResult<()> {
    if confirm(lang, question, true)? {
        report.pass(name, tr(lang, "tester confirmed", "тестер подтвердил"));
    } else {
        report.skip(
            name,
            tr(lang, "tester did not confirm", "тестер не подтвердил"),
        );
    }
    Ok(())
}

fn prompt_required(lang: Language, label: &str) -> AnyResult<String> {
    loop {
        let value = prompt(label)?;
        if !value.is_empty() {
            return Ok(value);
        }
        println!(
            "{}",
            tr(lang, "Value is required.", "Значение обязательно.")
        );
    }
}

fn prompt_optional(_lang: Language, label: &str) -> AnyResult<Option<String>> {
    let value = prompt(label)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn prompt_optional_i64(lang: Language, label: &str) -> AnyResult<Option<i64>> {
    loop {
        let value = prompt(label)?;
        if value.is_empty() {
            return Ok(None);
        }
        match value.parse::<i64>() {
            Ok(parsed) => return Ok(Some(parsed)),
            Err(_) => println!(
                "{}",
                tr(
                    lang,
                    "Expected an integer chat_id/user_id.",
                    "Ожидался целочисленный chat_id/user_id.",
                )
            ),
        }
    }
}

fn prompt_u64(lang: Language, label: &str, default: u64) -> AnyResult<u64> {
    loop {
        let prompt_label = format!("{label} [{default}]");
        let value = prompt(&prompt_label)?;
        if value.is_empty() {
            return Ok(default);
        }
        match value.parse::<u64>() {
            Ok(parsed) => return Ok(parsed),
            Err(_) => println!(
                "{}",
                tr(
                    lang,
                    "Expected an unsigned integer.",
                    "Ожидалось беззнаковое целое число.",
                )
            ),
        }
    }
}

fn prompt_u32(lang: Language, label: &str, default: u32) -> AnyResult<u32> {
    loop {
        let prompt_label = format!("{label} [{default}]");
        let value = prompt(&prompt_label)?;
        if value.is_empty() {
            return Ok(default);
        }
        match value.parse::<u32>() {
            Ok(parsed) => return Ok(parsed),
            Err(_) => println!(
                "{}",
                tr(
                    lang,
                    "Expected an unsigned integer.",
                    "Ожидалось беззнаковое целое число.",
                )
            ),
        }
    }
}

fn confirm(lang: Language, question: &str, default_yes: bool) -> AnyResult<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let value = prompt(&format!("{question} {suffix}"))?;
    if value.is_empty() {
        return Ok(default_yes);
    }

    let normalized = value.to_ascii_lowercase();
    Ok(
        matches!(normalized.as_str(), "y" | "yes" | "да" | "д" | "lf")
            || matches!(lang, Language::Russian) && normalized == "ага",
    )
}

fn typed_confirmation(question: &str, expected: &str) -> AnyResult<bool> {
    let value = prompt(question)?;
    Ok(value == expected)
}

fn prompt(label: &str) -> AnyResult<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

fn print_section(title: &str) {
    println!();
    println!("=== {title} ===");
}

fn print_case(name: &str) {
    println!();
    println!("-> {name}");
}

fn print_known_chats(chats: &[Chat], lang: Language) {
    if chats.is_empty() {
        println!(
            "{}",
            tr(
                lang,
                "No group chats returned.",
                "Групповые чаты не были возвращены.",
            )
        );
        return;
    }

    for chat in chats {
        let title = chat
            .title
            .as_deref()
            .unwrap_or(tr(lang, "(no title)", "(без названия)"));
        println!(
            "  - {} [{}] {}",
            chat.chat_id,
            chat_type_name(&chat.r#type),
            title
        );
    }
}

fn print_chat_members(members: &[maxoxide::types::ChatMember], lang: Language) {
    if members.is_empty() {
        println!(
            "{}",
            tr(
                lang,
                "No chat members were returned.",
                "Участники чата не были возвращены.",
            )
        );
        return;
    }

    println!(
        "{}",
        tr(
            lang,
            "Chat members returned by bot.get_members:",
            "Участники, возвращённые bot.get_members:",
        )
    );

    for member in members {
        let last_name = member.last_name.as_deref().unwrap_or("");
        let role = member_role(member, lang);
        let permissions = permission_list(member.permissions.as_deref(), lang);
        println!(
            "  - {} {} {} [{}] permissions={}",
            member.user_id, member.first_name, last_name, role, permissions
        );
    }
}

fn print_bot_membership(member: &maxoxide::types::ChatMember, lang: Language) {
    println!(
        "{}",
        tr(
            lang,
            "Current bot membership returned by bot.get_my_membership:",
            "Текущее членство бота из bot.get_my_membership:",
        )
    );
    println!(
        "  - user_id={} role={} permissions={}",
        member.user_id,
        member_role(member, lang),
        permission_list(member.permissions.as_deref(), lang)
    );
}

fn member_role(member: &maxoxide::types::ChatMember, lang: Language) -> &'static str {
    if member.is_owner == Some(true) {
        tr(lang, "owner", "owner")
    } else if member.is_admin == Some(true) {
        tr(lang, "admin", "admin")
    } else {
        tr(lang, "member", "member")
    }
}

fn permission_list(permissions: Option<&[ChatAdminPermission]>, lang: Language) -> String {
    let Some(permissions) = permissions else {
        return tr(lang, "none", "нет").into();
    };

    if permissions.is_empty() {
        return tr(lang, "empty", "пусто").into();
    }

    permissions
        .iter()
        .map(ChatAdminPermission::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

fn chat_type_name(chat_type: &ChatType) -> &str {
    match chat_type {
        ChatType::Dialog => "dialog",
        ChatType::Chat => "chat",
        ChatType::Channel => "channel",
        ChatType::Unknown(value) => value.as_str(),
        _ => "unknown",
    }
}

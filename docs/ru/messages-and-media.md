# Сообщения и медиа

## Адресация

Каждый send helper явно указывает тип получателя:

```rust
bot.send_text_to_chat(chat_id, "Ответить в этот диалог").await?;
bot.send_text_to_user(user_id, "Отправить по глобальному user ID").await?;
```

Если нужен соответствующий query-флаг MAX, используйте `SendMessageOptions::disable_link_preview`.

Для заменяющего сообщения при ответе на callback используйте `answer_callback_with_options` и `AnswerCallbackOptions::disable_link_preview`. Флаг передаётся как query-параметр `POST /answers`, а не в JSON body.

## Форматирование и ссылки

```rust
use maxoxide::types::{MessageFormat, NewMessageBody};

let sent = bot.send_markdown_to_chat(chat_id, "**Markdown**").await?;
bot.send_html_to_chat(chat_id, "<strong>HTML</strong>").await?;
bot.send_message_to_chat(
    chat_id,
    NewMessageBody::text("Ответ")
        .with_format(MessageFormat::Markdown)
        .with_reply_to(sent.message_id()),
).await?;
```

`with_forward_from` создаёт forward-ссылку. Комментарии поддерживают только reply, поэтому `NewCommentBody::validate` отклоняет forward.

MAX ограничивает текст сообщения/комментария 4000 Unicode-символами. Send/edit/callback/comment methods проверяют это до запроса. `send_long_text_to_chat` и `send_long_formatted_text_to_chat` делят строку по границе Unicode-символов и по порядку возвращают все отправленные `Message`.

## Клавиатуры

```rust
use maxoxide::types::{Button, KeyboardPayload, NewMessageBody};

let keyboard = KeyboardPayload {
    buttons: vec![
        vec![Button::callback("Подтвердить", "confirm")],
        vec![Button::link("Открыть", "https://max.ru")],
        vec![Button::request_contact("Телефон")],
    ],
};
bot.send_message_to_chat(chat_id, NewMessageBody::text("Выберите").with_keyboard(keyboard)).await?;
```

Validation проверяет 30 рядов, 7 кнопок в обычном ряду, 3 кнопки в ряду с широкими link/open-app/contact/location, непустые text/payload и лимит ссылки 2048 символов. Устаревшее свойство `intent` не отправляется: официальные клиенты помечают его как неподдерживаемое.

Доступны builders для callback, link, message, open app, clipboard, contact, geo location и создания чата.

## Исходящие вложения

`NewAttachment` поддерживает:

- image по upload token, URL или карте MAX photo tokens;
- video, audio и file по upload token;
- sticker по code;
- карточку contact;
- location с latitude/longitude;
- share по URL или token;
- inline keyboard.

Проверка payload обнаруживает отсутствующий token/URL, пустой sticker code, некорректные координаты и клавиатуру. Для полученных вложений неизвестный будущий payload сохраняется.

## Streaming upload

`upload_file` не читает файл целиком в память. Метод получает `/uploads`, асинхронно открывает файл и отправляет multipart stream, если endpoint не содержит заранее выданного token:

```rust
let token = bot.upload_file(
    UploadType::File,
    "report.pdf",
    "report.pdf",
    "application/pdf",
).await?;
```

`upload_bytes` намеренно работает в памяти, поскольку вызывающий код уже владеет `Vec<u8>`.

До запроса `/uploads` все методы для файлов и bytes отклоняют пустое содержимое и проверяют десятичные лимиты MAX: изображение 50 MB, видео 250 MB, аудио 256 MB, файл 4 GB. Ограничение изображения 7680x7680 и длительность аудио до 60 минут остаются проверками сервера.

## Resumable upload

Если MAX вернул upload token, файловая загрузка автоматически переходит на raw `Content-Range` chunks с заголовками из официального TypeScript SDK. Чанки отправляются последовательно. После transport, 429 или 5xx повторяется только неуспешный chunk; завершённые части не отправляются заново.

```rust
use maxoxide::uploader::UploadOptions;

let cancellation = tokio_util::sync::CancellationToken::new();
let options = UploadOptions::default()
    .with_chunk_size(2 * 1024 * 1024)
    .with_timeout(std::time::Duration::from_secs(120))
    .with_cancellation_token(cancellation.clone())
    .with_progress(|event| {
        println!("{:.0}%", event.percent);
    });

let token = bot.upload_file_with_options(
    UploadType::Video,
    "video.mp4",
    "video.mp4",
    "video/mp4",
    options,
).await?;
```

Chunk size должен находиться между 1 байтом и 16 МиБ. По умолчанию используется 1 МиБ и три попытки. Filename, позволяющий внедрить upload headers, отклоняется. Cancellation прерывает активный request и возвращает `MaxError::Cancelled`.

## Upload-and-send helpers

Методы `send_image_to_chat`, `send_video_to_user` и byte-варианты выполняют upload, создают attachment и отправляют сообщение. Гонка обработки вложения повторяется центральной retry policy, когда MAX отвечает `attachment.not.ready` или старым сообщением `.not.processed`.

## Комментарии

Доступны пять типизированных методов: `get_comments`, `get_comment`, `create_comment`, `edit_comment`, `delete_comment`. Боту нужны доступ к каналу и требуемые MAX права администратора. Входящие изменения представлены типизированными updates `CommentCreated`, `CommentEdited` и `CommentRemoved`.

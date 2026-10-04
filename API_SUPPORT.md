# MAX API

## English

The table summarizes MAX API support in maxoxide 3.1.

Support statuses are based on the public MAX API documentation and the public
contracts of the official Go and TypeScript SDKs. Experimental and partner-only
areas are identified explicitly.

| Area | Support | Notes |
|---|---|---|
| `GET /me` | Stable typed | Returns `BotInfo` |
| `PATCH /me/commands` | Stable typed | Replaces up to 32 commands |
| `GET /chats` | Removed | MAX no longer supports it; persist IDs from updates |
| Chat by ID/link, edit, delete, actions | Typed | Link variants are safely path-encoded |
| Pin and membership/admin methods | Typed | Includes `view_stats` and unknown permissions; `add_members` is deprecated after endpoint removal |
| Send/edit/delete/get messages | Typed | Chat/user addressing, validation, reply/forward |
| Callback answers | Typed | Structured notification/message body and `disable_link_preview` query option |
| Subscriptions and long polling | Typed | Webhook recommended for production |
| Upload URL | Typed with preflight | Decimal limits: image 50 MB, video 250 MB, audio 256 MB, file 4 GB |
| Multipart upload | Streaming | File is not fully buffered |
| Resumable upload | Streaming typed helper | `Content-Range`, current-chunk retry, progress, cancellation |
| Comments CRUD (5 methods) | Stable typed | Read, create, edit, and delete channel-post comments |
| Comment updates | Typed | Created, edited, and removed events |
| Administrator permission update | Typed webhook event | Includes bot/user IDs, channel/admin flags, and permissions |
| Constructed-message updates | Typed SDK extension | Confirmed by the official TypeScript SDK |
| `bot_stopped.payload` | Typed SDK extension | Confirmed by the official TypeScript SDK |
| Incoming/outgoing attachments | Typed with unknown preservation | Image, video, audio, file, sticker, contact, location, share, keyboard |
| Mini App init data | Stable validator | HMAC, duplicates, freshness, typed user/chat |
| Mini App contact | Stable validator | Signature/phone normalization; caller controls freshness |
| Digital ID | Experimental partner integration | Not verified against the live service; validate endpoint, authorization, and partner-owned models against onboarding docs before production |
| Unknown Bot endpoint | Raw escape hatch | Relative paths only through `Bot::execute` |

### Compatibility Policy

- Wire enums that MAX may extend use `Unknown(String)` where practical.
- Unknown update types retain raw JSON.
- A malformed known update falls back to `Update::Unknown` instead of losing delivery.
- Typed request methods validate documented limits before I/O.
- A removed endpoint may retain a deprecated signature for a compatible release, but calls can still fail at the platform boundary.

### Comments

maxoxide provides typed request and response shapes for the documented routes:

- `GET /messages/{messageId}/comments`
- `POST /messages/{messageId}/comments`
- `PUT /messages/{messageId}/comments?comment_id=...`
- `DELETE /messages/{messageId}/comments?comment_id=...`
- `GET /messages/{messageId}/comments/{commentId}`

Comment events are also represented by `Update::CommentCreated`, `Update::CommentEdited`, and `Update::CommentRemoved`.

## Русский

В таблице приведена поддержка MAX API в maxoxide 3.1.

Статусы основаны на публичной документации MAX API и открытых контрактах
официальных SDK для Go и TypeScript. Экспериментальные и доступные только партнёрам
области обозначены явно.

| Область | Поддержка | Примечания |
|---|---|---|
| `GET /me` | Стабильная типизация | Возвращает `BotInfo` |
| `PATCH /me/commands` | Стабильная типизация | Заменяет до 32 команд |
| `GET /chats` | Удалено | MAX больше не поддерживает метод; сохраняйте идентификаторы из обновлений |
| Получение чата по ID/ссылке, изменение, удаление, действия | Типизировано | Варианты ссылок безопасно кодируются как сегменты пути |
| Закрепление сообщений, участники и администраторы | Типизировано | Включает `view_stats` и сохранение неизвестных разрешений; `add_members` устарел после удаления endpoint |
| Отправка, изменение, удаление и получение сообщений | Типизировано | Адресация по чату/пользователю, валидация, ответ на исходное сообщение и пересылка |
| Ответы на callback-запросы | Типизировано | Структурированное тело уведомления или сообщения и query-параметр `disable_link_preview` |
| Подписки и длительный опрос | Типизировано | Для рабочего окружения рекомендуется webhook |
| Получение URL для загрузки | Типизировано с предварительной проверкой | Десятичные лимиты: image 50 MB, video 250 MB, audio 256 MB, file 4 GB |
| Multipart-загрузка | Потоковая загрузка | Файл не буферизуется целиком |
| Возобновляемая загрузка | Типизированная потоковая утилита | `Content-Range`, повтор текущего чанка, уведомление о ходе выполнения и отмена |
| CRUD комментариев (5 методов) | Стабильная типизация | Чтение, создание, изменение и удаление комментариев к постам канала |
| Updates комментариев | Типизировано | События создания, изменения и удаления |
| Update прав администратора | Типизированное webhook-событие | Содержит ID бота и пользователя, признаки канала и администратора, а также разрешения |
| Обновления `message_construction_request` и `message_constructed` | Типизированное расширение SDK | Подтверждено официальным SDK для TypeScript |
| `bot_stopped.payload` | Типизированное расширение SDK | Подтверждено официальным SDK для TypeScript |
| Входящие и исходящие вложения | Типизировано с сохранением неизвестных значений | `image`, `video`, `audio`, `file`, `sticker`, `contact`, `location`, `share`, `keyboard` |
| Начальные данные Mini App | Стабильный валидатор | HMAC, дубликаты, срок действия, типизированные пользователь и чат |
| Контакт Mini App | Стабильный валидатор | Нормализация подписи и телефона; срок действия контролирует вызывающий код |
| Digital ID | Экспериментальная партнёрская интеграция | Не проверена на реальном сервисе; перед рабочим использованием сверьте endpoint, формат авторизации и модели партнёра с документацией подключения |
| Неизвестный метод Bot API | Низкоуровневый вызов | Только относительные пути через `Bot::execute` |

### Политика совместимости

- Расширяемые перечисления формата протокола по возможности используют `Unknown(String)`.
- Неизвестные типы обновлений сохраняют исходный JSON.
- Некорректное известное обновление преобразуется в `Update::Unknown`, чтобы не потерять доставленное событие.
- Типизированные методы запросов проверяют документированные ограничения до сетевого вызова.
- Для удалённого endpoint совместимая сигнатура может временно сохраняться как устаревшая, но вызов всё равно может завершиться ошибкой платформы.

### Комментарии

maxoxide предоставляет типизированные запросы и ответы для документированных маршрутов:

- `GET /messages/{messageId}/comments`
- `POST /messages/{messageId}/comments`
- `PUT /messages/{messageId}/comments?comment_id=...`
- `DELETE /messages/{messageId}/comments?comment_id=...`
- `GET /messages/{messageId}/comments/{commentId}`

События комментариев также представлены вариантами `Update::CommentCreated`, `Update::CommentEdited` и `Update::CommentRemoved`.

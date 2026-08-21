# MAX API

## English

The table summarizes MAX API support in maxoxide 3.0.

Support statuses are based on the public MAX API documentation and the public
contracts of the official Go and TypeScript SDKs. Experimental and partner-only
areas are identified explicitly.

| Area | Support | Notes |
|---|---|---|
| `GET /me` | Stable typed | Returns `BotInfo` |
| `PATCH /me/commands` | Stable typed | Replaces up to 32 commands |
| `GET /chats` | Removed | MAX no longer supports it; persist IDs from updates |
| Chat by ID/link, edit, delete, actions | Typed | Link variants are safely path-encoded |
| Pin and membership/admin methods | Typed | Includes `view_stats` and unknown permissions |
| Send/edit/delete/get messages | Typed | Chat/user addressing, validation, reply/forward |
| Callback answers | Typed | Structured notification/message body |
| Subscriptions and long polling | Typed | Webhook recommended for production |
| Upload URL | Typed | Image/video/audio/file |
| Multipart upload | Streaming | File is not fully buffered |
| Resumable upload | Streaming typed helper | `Content-Range`, current-chunk retry, progress, cancellation |
| Comments CRUD (5 methods) | Experimental typed | Public docs say temporarily unavailable |
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
- No compatibility wrapper remains for APIs deliberately removed in v3.

### Experimental Comments

MAX documents these routes but marks each as temporarily unavailable:

- `GET /messages/{messageId}/comments`
- `POST /messages/{messageId}/comments`
- `PUT /messages/{messageId}/comments?comment_id=...`
- `DELETE /messages/{messageId}/comments?comment_id=...`
- `GET /messages/{messageId}/comments/{commentId}`

maxoxide provides typed request and response shapes for these routes. Applications must handle availability errors and should not rely on live behavior until MAX removes the notice.

## Русский

В таблице приведена поддержка MAX API в maxoxide 3.0.

Статусы основаны на публичной документации MAX API и открытых контрактах
официальных SDK для Go и TypeScript. Экспериментальные и доступные только партнёрам
области обозначены явно.

| Область | Поддержка | Примечания |
|---|---|---|
| `GET /me` | Стабильная типизация | Возвращает `BotInfo` |
| `PATCH /me/commands` | Стабильная типизация | Заменяет до 32 команд |
| `GET /chats` | Удалено | MAX больше не поддерживает метод; сохраняйте идентификаторы из обновлений |
| Получение чата по ID/ссылке, изменение, удаление, действия | Типизировано | Варианты ссылок безопасно кодируются как сегменты пути |
| Закрепление сообщений, участники и администраторы | Типизировано | Включает `view_stats` и сохранение неизвестных разрешений |
| Отправка, изменение, удаление и получение сообщений | Типизировано | Адресация по чату/пользователю, валидация, ответ на исходное сообщение и пересылка |
| Ответы на callback-запросы | Типизировано | Структурированное тело уведомления или сообщения |
| Подписки и длительный опрос | Типизировано | Для рабочего окружения рекомендуется webhook |
| Получение URL для загрузки | Типизировано | `image`, `video`, `audio`, `file` |
| Multipart-загрузка | Потоковая загрузка | Файл не буферизуется целиком |
| Возобновляемая загрузка | Типизированная потоковая утилита | `Content-Range`, повтор текущего чанка, уведомление о ходе выполнения и отмена |
| CRUD комментариев (5 методов) | Экспериментальная типизация | Публичная документация помечает API временно недоступным |
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
- Для API, намеренно удалённых в v3, не сохраняются обёртки обратной совместимости.

### Экспериментальные комментарии

MAX документирует следующие маршруты, но помечает каждый из них временно недоступным:

- `GET /messages/{messageId}/comments`
- `POST /messages/{messageId}/comments`
- `PUT /messages/{messageId}/comments?comment_id=...`
- `DELETE /messages/{messageId}/comments?comment_id=...`
- `GET /messages/{messageId}/comments/{commentId}`

maxoxide предоставляет типизированные формы запросов и ответов для этих маршрутов. Приложение должно обрабатывать ошибки доступности и не должно полагаться на поведение реального API, пока MAX не снимет предупреждение.

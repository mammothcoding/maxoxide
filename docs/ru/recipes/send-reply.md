# Ответ в правильный диалог

Используйте `chat_id` получателя сообщения, а не глобальный `user_id` отправителя:

```rust
dispatcher.on_message(|context: Context| async move {
    if let Update::MessageCreated { message, .. } = &context.update {
        context.bot.send_message_to_chat(
            message.chat_id(),
            NewMessageBody::text("Ответ").with_reply_to(message.message_id()),
        ).await?;
    }
    Ok(())
});
```

`send_message_to_user` нужен, только если приложение знает глобальный user ID, но не ID конкретного диалога.

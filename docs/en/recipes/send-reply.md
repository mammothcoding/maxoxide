# Reply to the correct dialog

Use the message recipient's `chat_id`, not the sender's global `user_id`:

```rust
dispatcher.on_message(|context: Context| async move {
    if let Update::MessageCreated { message, .. } = &context.update {
        context.bot.send_message_to_chat(
            message.chat_id(),
            NewMessageBody::text("Reply").with_reply_to(message.message_id()),
        ).await?;
    }
    Ok(())
});
```

Use `send_message_to_user` only when your application has a global user ID but no concrete dialog ID.

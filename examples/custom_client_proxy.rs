//! # English
//!
//! Demonstrates all proxy modes supported by `BotBuilder`. By default reqwest uses
//! system proxy variables. Set `MAX_DISABLE_PROXY=1` to connect to MAX directly,
//! for example when the same server needs a global proxy for Telegram. Alternatively,
//! set `MAX_PROXY_URL` and optionally both `MAX_PROXY_USERNAME` and
//! `MAX_PROXY_PASSWORD` to use an explicit authenticated proxy.
//!
//! Requires `MAX_BOT_TOKEN`. HTTP/HTTPS proxies work normally; enable feature
//! `socks-proxy` for a SOCKS URL. Do not put credentials in the proxy URL or log
//! the configured proxy. The example performs the read-only `GET /me` request.
//! Run: `MAX_BOT_TOKEN=... cargo run --example custom_client_proxy`.
//!
//! # Русский
//!
//! Показывает все proxy-режимы `BotBuilder`. По умолчанию reqwest использует системные
//! proxy variables. Установите `MAX_DISABLE_PROXY=1` для прямого подключения к MAX,
//! например когда этому же серверу нужен глобальный proxy для Telegram. Либо задайте
//! `MAX_PROXY_URL` и при необходимости оба параметра `MAX_PROXY_USERNAME` и
//! `MAX_PROXY_PASSWORD` для явного authenticated proxy.
//!
//! Нужен `MAX_BOT_TOKEN`. HTTP/HTTPS proxy работают обычно, для SOCKS URL включите
//! feature `socks-proxy`. Не помещайте credentials в proxy URL и не логируйте
//! настроенный proxy. Пример выполняет read-only запрос `GET /me`.
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example custom_client_proxy`.

use std::env::VarError;

use maxoxide::{Bot, MaxError, Result, reqwest::Proxy};

#[tokio::main]
async fn main() -> Result<()> {
    let optional_env = |name: &str| -> Result<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => Err(MaxError::Configuration(format!(
                "{name} must contain valid Unicode"
            ))),
        }
    };

    let disable_proxy = match optional_env("MAX_DISABLE_PROXY")? {
        None => false,
        Some(value) if value == "1" || value.eq_ignore_ascii_case("true") => true,
        Some(value) if value == "0" || value.eq_ignore_ascii_case("false") => false,
        Some(_) => {
            return Err(MaxError::Configuration(
                "MAX_DISABLE_PROXY must be 1, 0, true, or false".into(),
            ));
        }
    };
    let proxy_url = optional_env("MAX_PROXY_URL")?;
    let proxy_username = optional_env("MAX_PROXY_USERNAME")?;
    let proxy_password = optional_env("MAX_PROXY_PASSWORD")?;

    if disable_proxy && proxy_url.is_some() {
        return Err(MaxError::Configuration(
            "MAX_DISABLE_PROXY cannot be combined with MAX_PROXY_URL".into(),
        ));
    }
    if proxy_username.is_some() != proxy_password.is_some() {
        return Err(MaxError::Configuration(
            "MAX_PROXY_USERNAME and MAX_PROXY_PASSWORD must be set together".into(),
        ));
    }
    if proxy_url.is_none() && proxy_username.is_some() {
        return Err(MaxError::Configuration(
            "proxy credentials require MAX_PROXY_URL".into(),
        ));
    }

    let token = optional_env("MAX_BOT_TOKEN")?
        .ok_or_else(|| MaxError::Configuration("MAX_BOT_TOKEN is required".into()))?;
    let builder = Bot::builder(token);
    let builder = if disable_proxy {
        builder.no_proxy()
    } else if let Some(proxy_url) = proxy_url {
        let mut proxy = Proxy::all(proxy_url)?;
        if let (Some(username), Some(password)) = (proxy_username, proxy_password) {
            proxy = proxy.basic_auth(&username, &password);
        }
        builder.proxy(proxy)
    } else {
        builder
    };

    let me = builder.build()?.get_me().await?;
    println!("Connected to MAX as {} ({})", me.display_name(), me.user_id);
    Ok(())
}

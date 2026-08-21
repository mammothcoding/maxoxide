//! Server-side validation for MAX Mini App launch and contact data.

use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Default maximum age recommended by MAX for Mini App launch data.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(60 * 60);

/// A user embedded in validated Mini App launch data.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MiniAppUser {
    /// Global MAX user identifier.
    pub id: i64,
    /// User's first name.
    pub first_name: String,
    /// User's last name, when available.
    pub last_name: Option<String>,
    /// Public username, when configured.
    pub username: Option<String>,
    /// User interface language code.
    pub language_code: Option<String>,
    /// User avatar URL, when available.
    pub photo_url: Option<String>,
}

/// A chat embedded in validated Mini App launch data.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MiniAppChat {
    /// MAX chat identifier.
    pub id: i64,
    /// MAX chat type as provided by the Mini App runtime.
    pub r#type: String,
}

/// Parsed launch data returned only after its signature and age are valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniAppInitData {
    /// Identifier of the Mini App query, when present.
    pub query_id: Option<String>,
    /// Client IP address supplied by MAX, when present.
    pub ip: Option<String>,
    /// Unix timestamp at which MAX authenticated the data.
    pub auth_date: u64,
    /// Authenticated user payload, when present.
    pub user: Option<MiniAppUser>,
    /// Authenticated chat payload, when present.
    pub chat: Option<MiniAppChat>,
    /// Deep-link start parameter, when present.
    pub start_param: Option<String>,
    /// All authenticated launch parameters except `hash`.
    pub fields: BTreeMap<String, String>,
}

/// Data returned by `window.WebApp.requestContact()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniAppContact {
    /// Contact phone number returned by MAX.
    pub phone: String,
    /// Contact authorization timestamp as returned by MAX.
    pub auth_date: String,
    /// Hex-encoded signature protecting the contact data.
    pub hash: String,
}

/// Why Mini App data was rejected.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MiniAppValidationError {
    /// The bot token used to derive the signing key is empty.
    #[error("bot token must not be empty")]
    EmptyToken,
    /// The encoded initialization data is empty.
    #[error("initialization data is empty")]
    EmptyData,
    /// A parameter occurs more than once.
    #[error("parameter `{0}` occurs more than once")]
    DuplicateParameter(String),
    /// A required parameter is absent.
    #[error("required parameter `{0}` is missing")]
    MissingParameter(&'static str),
    /// A parameter cannot be parsed or validated.
    #[error("parameter `{field}` is invalid: {message}")]
    InvalidParameter {
        /// Name of the invalid parameter.
        field: &'static str,
        /// Human-readable validation failure.
        message: String,
    },
    /// The provided HMAC signature does not match the data.
    #[error("initialization data signature is invalid")]
    InvalidSignature,
    /// The data is older than the configured maximum age.
    #[error("initialization data has expired")]
    Expired,
    /// The authorization timestamp exceeds the tolerated clock skew.
    #[error("initialization data is dated too far in the future")]
    FutureAuthDate,
    /// The host system clock is earlier than the Unix epoch.
    #[error("system time is before the Unix epoch")]
    InvalidSystemTime,
}

/// Reusable Mini App validator tied to one bot token.
#[derive(Clone)]
pub struct MiniAppValidator {
    bot_token: String,
    max_age: Duration,
    future_tolerance: Duration,
}

impl MiniAppValidator {
    /// Creates a validator with a one-hour maximum age.
    pub fn new(bot_token: impl Into<String>) -> Result<Self, MiniAppValidationError> {
        let bot_token = bot_token.into();
        if bot_token.is_empty() {
            return Err(MiniAppValidationError::EmptyToken);
        }
        Ok(Self {
            bot_token,
            max_age: DEFAULT_MAX_AGE,
            future_tolerance: Duration::from_secs(30),
        })
    }

    /// Replaces the maximum accepted age.
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Replaces the tolerated positive clock skew.
    pub fn future_tolerance(mut self, tolerance: Duration) -> Self {
        self.future_tolerance = tolerance;
        self
    }

    /// Validates URL-encoded `window.WebApp.initData` using the current time.
    pub fn validate(&self, init_data: &str) -> Result<MiniAppInitData, MiniAppValidationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MiniAppValidationError::InvalidSystemTime)?
            .as_secs();
        self.validate_at(init_data, now)
    }

    /// Validates launch data at an explicit Unix timestamp for deterministic use.
    pub fn validate_at(
        &self,
        init_data: &str,
        now: u64,
    ) -> Result<MiniAppInitData, MiniAppValidationError> {
        let mut fields = parse_unique_parameters(init_data)?;
        let hash = fields
            .remove("hash")
            .ok_or(MiniAppValidationError::MissingParameter("hash"))?;
        let provided_hash = decode_hash(&hash)?;
        let launch_params = fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut secret =
            Hmac::<Sha256>::new_from_slice(b"WebAppData").expect("HMAC accepts keys of any size");
        secret.update(self.bot_token.as_bytes());
        let secret = secret.finalize().into_bytes();
        let mut signature =
            Hmac::<Sha256>::new_from_slice(&secret).expect("HMAC accepts keys of any size");
        signature.update(launch_params.as_bytes());
        if !bool::from(signature.finalize().into_bytes().ct_eq(&provided_hash)) {
            return Err(MiniAppValidationError::InvalidSignature);
        }

        let auth_date = fields
            .get("auth_date")
            .ok_or(MiniAppValidationError::MissingParameter("auth_date"))?
            .parse::<u64>()
            .map_err(|error| MiniAppValidationError::InvalidParameter {
                field: "auth_date",
                message: error.to_string(),
            })?;
        if auth_date > now.saturating_add(self.future_tolerance.as_secs()) {
            return Err(MiniAppValidationError::FutureAuthDate);
        }
        if now.saturating_sub(auth_date) > self.max_age.as_secs() {
            return Err(MiniAppValidationError::Expired);
        }

        let user = parse_json_field(&fields, "user")?;
        let chat = parse_json_field(&fields, "chat")?;
        Ok(MiniAppInitData {
            query_id: fields.get("query_id").cloned(),
            ip: fields.get("ip").cloned(),
            auth_date,
            user,
            chat,
            start_param: fields.get("start_param").cloned(),
            fields,
        })
    }

    /// Verifies data returned by `window.WebApp.requestContact()`.
    pub fn validate_contact(&self, contact: &MiniAppContact, user_id: i64) -> bool {
        let phone = contact.phone.strip_prefix('+').unwrap_or(&contact.phone);
        let data = format!(
            "auth_date={}\nphone={}\nuser_id={user_id}",
            contact.auth_date, phone
        );
        let Ok(provided_hash) = decode_hash(&contact.hash) else {
            return false;
        };
        let mut signature = Hmac::<Sha256>::new_from_slice(self.bot_token.as_bytes())
            .expect("HMAC accepts keys of any size");
        signature.update(data.as_bytes());
        bool::from(signature.finalize().into_bytes().ct_eq(&provided_hash))
    }
}

fn parse_unique_parameters(
    input: &str,
) -> Result<BTreeMap<String, String>, MiniAppValidationError> {
    if input.is_empty() {
        return Err(MiniAppValidationError::EmptyData);
    }
    let mut fields = BTreeMap::new();
    for (key, value) in url::form_urlencoded::parse(input.as_bytes()) {
        let key = key.into_owned();
        if fields.insert(key.clone(), value.into_owned()).is_some() {
            return Err(MiniAppValidationError::DuplicateParameter(key));
        }
    }
    Ok(fields)
}

fn parse_json_field<T: for<'de> Deserialize<'de>>(
    fields: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<Option<T>, MiniAppValidationError> {
    fields
        .get(field)
        .map(|value| {
            serde_json::from_str(value).map_err(|error| MiniAppValidationError::InvalidParameter {
                field,
                message: error.to_string(),
            })
        })
        .transpose()
}

fn decode_hash(hash: &str) -> Result<[u8; 32], MiniAppValidationError> {
    if hash.len() != 64 {
        return Err(MiniAppValidationError::InvalidParameter {
            field: "hash",
            message: "must contain 64 hexadecimal characters".into(),
        });
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&hash[offset..offset + 2], 16).map_err(|error| {
            MiniAppValidationError::InvalidParameter {
                field: "hash",
                message: error.to_string(),
            }
        })?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{Hmac, Mac, MiniAppContact, MiniAppValidationError, MiniAppValidator, Sha256};

    fn signed_data(token: &str, auth_date: u64) -> String {
        let user = r#"{"id":7,"first_name":"Max","last_name":null,"username":"max","language_code":"ru","photo_url":null}"#;
        let encoded_user: String = url::form_urlencoded::byte_serialize(user.as_bytes()).collect();
        let check = format!("auth_date={auth_date}\nquery_id=q1\nuser={user}");
        let mut secret = Hmac::<Sha256>::new_from_slice(b"WebAppData").unwrap();
        secret.update(token.as_bytes());
        let mut signature =
            Hmac::<Sha256>::new_from_slice(&secret.finalize().into_bytes()).unwrap();
        signature.update(check.as_bytes());
        let hash = signature
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("query_id=q1&user={encoded_user}&auth_date={auth_date}&hash={hash}")
    }

    #[test]
    fn validates_signed_and_fresh_init_data() {
        let validator = MiniAppValidator::new("token").unwrap();
        let data = validator
            .validate_at(&signed_data("token", 1_000), 1_100)
            .unwrap();
        assert_eq!(data.user.unwrap().id, 7);
    }

    #[test]
    fn rejects_duplicates_tampering_and_expiration() {
        let validator = MiniAppValidator::new("token").unwrap();
        let duplicate = format!("{}&auth_date=1000", signed_data("token", 1_000));
        assert!(matches!(
            validator.validate_at(&duplicate, 1_100),
            Err(MiniAppValidationError::DuplicateParameter(_))
        ));
        assert!(matches!(
            validator.validate_at(&signed_data("other", 1_000), 1_100),
            Err(MiniAppValidationError::InvalidSignature)
        ));
        assert!(matches!(
            validator.validate_at(&signed_data("token", 1_000), 5_000),
            Err(MiniAppValidationError::Expired)
        ));
    }

    #[test]
    fn validates_contact_and_normalizes_plus_prefix() {
        let validator = MiniAppValidator::new("token").unwrap();
        let data = "auth_date=1000\nphone=79991234567\nuser_id=7";
        let mut signature = Hmac::<Sha256>::new_from_slice(b"token").unwrap();
        signature.update(data.as_bytes());
        let hash = signature
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert!(validator.validate_contact(
            &MiniAppContact {
                phone: "+79991234567".into(),
                auth_date: "1000".into(),
                hash,
            },
            7
        ));
    }
}

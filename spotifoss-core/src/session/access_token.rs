use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::error::Error;
use crate::system_info::device_id;

use super::SessionService;

// Client ID of the official Web Spotify front-end.
pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

// All scopes we could possibly require.
pub const ACCESS_SCOPES: &str = "streaming,user-read-email,user-read-private,playlist-read-private,playlist-read-collaborative,playlist-modify-public,playlist-modify-private,user-follow-modify,user-follow-read,user-library-read,user-library-modify,user-top-read,user-read-recently-played";

// Consider token expired even before the official expiration time.  Spotify
// seems to be reporting excessive token TTLs so let's cut it down by 30
// minutes.
const EXPIRATION_TIME_THRESHOLD: Duration = Duration::from_secs(60 * 30);

#[derive(Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires: Instant,
}

impl AccessToken {
    fn expired() -> Self {
        Self {
            token: String::new(),
            expires: Instant::now(),
        }
    }

    pub fn request(session: &SessionService) -> Result<Self, Error> {
        let payload = session.connected()?.get_mercury_bytes(format!(
            "hm://keymaster/token/authenticated?client_id={CLIENT_ID}&scope={ACCESS_SCOPES}&device_id={}",
            device_id()
        ))?;
        let value: serde_json::Value = serde_json::from_slice(&payload)?;

        let access_token = value
            .get("accessToken")
            .or_else(|| value.get("access_token"))
            .and_then(|val| val.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                log::warn!("keymaster token response missing access token: {value}");
                Error::UnexpectedResponse
            })?;

        let expires_in = value
            .get("expiresIn")
            .or_else(|| value.get("expires_in"))
            .or_else(|| value.get("expires"))
            .and_then(|val| val.as_u64())
            .unwrap_or_else(|| {
                log::warn!("keymaster token response missing expiresIn; defaulting to 3600s");
                3600
            });

        Ok(Self {
            token: access_token,
            expires: Instant::now() + Duration::from_secs(expires_in),
        })
    }

    fn is_expired(&self) -> bool {
        self.expires.saturating_duration_since(Instant::now()) < EXPIRATION_TIME_THRESHOLD
    }
}

pub struct TokenProvider {
    token: Mutex<AccessToken>,
}

impl TokenProvider {
    pub fn new() -> Self {
        Self {
            token: Mutex::new(AccessToken::expired()),
        }
    }

    pub fn get(&self, session: &SessionService) -> Result<AccessToken, Error> {
        let mut token = self.token.lock();
        if token.is_expired() {
            log::info!("access token expired, requesting");
            *token = AccessToken::request(session)?;
        }
        Ok(token.clone())
    }
}

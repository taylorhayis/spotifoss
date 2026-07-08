use std::{
    env::{self, VarError},
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(target_family = "unix")]
use std::os::unix::fs::OpenOptionsExt;

use druid::{Data, Lens, Size};
use platform_dirs::AppDirs;
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use spotifoss_core::{
    audio::equalizer::EqConfig,
    cache::{CacheHandle, mkdir_if_not_exists},
    connection::Credentials,
    oauth::{self, OAuthToken},
    player::{PlaybackConfig, PlaybackEngine as CorePlaybackEngine},
    session::{SessionConfig, SessionConnection},
};

const OAUTH_EXPIRY_BUFFER: Duration = Duration::from_secs(120);

use super::{Nav, Promise, RepeatMode, SliderScrollScale, playback::LegacyQueueBehavior};
use crate::ui::theme;

#[derive(Clone, Debug, Data, Lens)]
pub struct Preferences {
    pub active: PreferencesTab,
    #[data(ignore)]
    pub cache: Option<CacheHandle>,
    pub cache_usage: Promise<CacheUsage, (), ()>,
    pub auth: Authentication,
    pub lastfm_auth_result: Option<String>,
}

impl Preferences {
    pub fn reset(&mut self) {
        self.cache_usage.clear();
        self.auth.result.clear();
        self.auth.lastfm_api_key_input.clear();
        self.auth.lastfm_api_secret_input.clear();
    }

    pub fn measure_cache_usage() -> Option<CacheUsage> {
        let path = Config::cache_dir()?;
        let mut usage = CacheUsage::default();

        let entries = fs::read_dir(&path).ok()?;
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let size = entry_path
                .metadata()
                .ok()
                .map(|meta| {
                    if meta.is_dir() {
                        get_dir_size(&entry_path).unwrap_or(0)
                    } else {
                        meta.len()
                    }
                })
                .unwrap_or(0);

            if entry_path.is_dir() {
                match entry_path.file_name().and_then(|name| name.to_str()) {
                    Some("audio") | Some("librespot-audio") => usage.audio += size,
                    Some("track") | Some("episode") | Some("key") => usage.metadata += size,
                    _ => usage.webapi += size,
                }
            } else {
                usage.other += size;
            }
        }

        usage.total = usage.audio + usage.metadata + usage.webapi + usage.other;
        Some(usage)
    }
}

#[derive(Clone, Debug, Data, Lens, Default)]
pub struct CacheUsage {
    pub total: u64,
    pub audio: u64,
    pub metadata: u64,
    pub webapi: u64,
    pub other: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Data)]
pub enum PreferencesTab {
    General,
    Playback,
    Account,
    Cache,
    About,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Authentication {
    pub username: String,
    pub password: String,
    pub access_token: String,
    pub result: Promise<(), (), String>,
    #[data(ignore)]
    pub lastfm_api_key_input: String,
    #[data(ignore)]
    pub lastfm_api_secret_input: String,
}

impl Authentication {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            access_token: String::new(),
            result: Promise::Empty,
            lastfm_api_key_input: String::new(),
            lastfm_api_secret_input: String::new(),
        }
    }

    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            login_creds: if !self.access_token.is_empty() {
                Credentials::from_access_token(self.access_token.clone())
            } else {
                Credentials::from_username_and_password(
                    self.username.clone(),
                    self.password.clone(),
                )
            },
            proxy_url: Config::proxy(),
        }
    }

    pub fn authenticate_and_get_credentials(config: SessionConfig) -> Result<Credentials, String> {
        let connection = SessionConnection::open(config).map_err(|err| err.to_string())?;
        Ok(connection.credentials)
    }

    pub fn clear(&mut self) {
        self.username.clear();
        self.password.clear();
    }
}

const APP_NAME: &str = "Spotifoss";
const LEGACY_APP_NAME: &str = "Spotix";
const CONFIG_FILENAME: &str = "config.json";
const PROXY_ENV_VAR: &str = "SOCKS_PROXY";

#[derive(Clone, Debug, Data, Lens, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[data(ignore)]
    credentials: Option<Credentials>,
    #[data(ignore)]
    oauth_token: Option<OAuthToken>,
    /// Client ID that was used when `oauth_token` was issued. Refresh tokens
    /// are bound to this value — if `webapi_client_id` changes, re-auth is
    /// required.
    #[data(ignore)]
    oauth_token_client_id: Option<String>,
    pub device_id: Option<String>,
    pub audio_quality: AudioQuality,
    pub playback_engine: PlaybackEngine,
    pub theme: Theme,
    pub volume: f64,
    pub last_route: Option<Nav>,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default)]
    pub repeat: RepeatMode,
    #[serde(default, rename = "queue_behavior", skip_serializing)]
    #[data(ignore)]
    queue_behavior: Option<LegacyQueueBehavior>,
    pub show_track_cover: bool,
    pub window_size: Size,
    pub slider_scroll_scale: SliderScrollScale,
    pub sort_order: SortOrder,
    pub sort_criteria: SortCriteria,
    pub paginated_limit: usize,
    pub seek_duration: usize,
    /// Audio cache limit in megabytes. 0 = unlimited.
    pub audio_cache_limit_mb: f64,
    pub enable_pagination: bool,
    pub crossfade_duration_secs: f64,
    pub mono_audio: bool,
    pub normalization_enabled: bool,
    pub autoplay_enabled: bool,
    pub lastfm_session_key: Option<String>,
    pub lastfm_api_key: Option<String>,
    pub lastfm_api_secret: Option<String>,
    pub lastfm_enable: bool,
    pub eq: EqSettings,
    /// Optional client ID for Spotify Web API requests.
    /// If unset, falls back to the default Spotify client ID.
    pub webapi_client_id: Option<String>,
    /// Lyrics appearance mode.
    pub lyrics_appearance: LyricsAppearance,
    /// Enable dynamic playing bar with album-art-derived colors and pulse.
    pub dynamic_playing_bar: bool,
    /// Minimize to system tray when the main window is closed.
    pub close_to_tray: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Data, Serialize, Deserialize)]
pub enum LyricsAppearance {
    #[default]
    Default,
    SpotifyStyled,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credentials: Default::default(),
            oauth_token: Default::default(),
            oauth_token_client_id: Default::default(),
            device_id: None,
            audio_quality: Default::default(),
            playback_engine: PlaybackEngine::default(),
            theme: Default::default(),
            volume: 1.0,
            last_route: Default::default(),
            shuffle: Default::default(),
            repeat: Default::default(),
            queue_behavior: Default::default(),
            show_track_cover: Default::default(),
            window_size: Size::new(theme::grid(80.0), theme::grid(100.0)),
            slider_scroll_scale: Default::default(),
            sort_order: Default::default(),
            sort_criteria: Default::default(),
            paginated_limit: 500,
            seek_duration: 10,
            audio_cache_limit_mb: 4096.0,
            enable_pagination: true,
            crossfade_duration_secs: 0.0,
            mono_audio: false,
            normalization_enabled: true,
            autoplay_enabled: true,
            lastfm_session_key: None,
            lastfm_api_key: None,
            lastfm_api_secret: None,
            lastfm_enable: false,
            eq: EqSettings::default(),
            webapi_client_id: None,
            lyrics_appearance: LyricsAppearance::default(),
            dynamic_playing_bar: true,
            close_to_tray: false,
        }
    }
}

impl Config {
    fn app_dirs() -> Option<AppDirs> {
        const USE_XDG_ON_MACOS: bool = false;

        AppDirs::new(Some(APP_NAME), USE_XDG_ON_MACOS)
    }

    pub fn spotify_local_files_file(username: &str) -> Option<PathBuf> {
        AppDirs::new(Some("spotify"), false).map(|dir| {
            let path = format!("Users/{username}-user/local-files.bnk");
            dir.config_dir.join(path)
        })
    }

    pub fn cache_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.cache_dir)
    }

    pub fn config_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.config_dir)
    }

    pub fn themes_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("themes"))
    }

    pub fn last_playback_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("last_playback.json"))
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join(CONFIG_FILENAME))
    }

    fn legacy_config_path() -> Option<PathBuf> {
        AppDirs::new(Some(LEGACY_APP_NAME), false).map(|dirs| dirs.config_dir.join(CONFIG_FILENAME))
    }

    fn migrate_legacy_config() {
        let Some(new_path) = Self::config_path() else {
            return;
        };
        if new_path.exists() {
            return;
        }
        let Some(legacy_path) = Self::legacy_config_path() else {
            return;
        };
        if !legacy_path.is_file() {
            return;
        }

        let Some(new_dir) = Self::config_dir() else {
            return;
        };
        let Some(legacy_dir) = AppDirs::new(Some(LEGACY_APP_NAME), false).map(|dirs| dirs.config_dir)
        else {
            return;
        };

        log::info!(
            "migrating config from Spotix to Spotifoss: {:?} -> {:?}",
            legacy_path,
            new_path
        );
        if mkdir_if_not_exists(&new_dir).is_err() {
            log::warn!("failed to create Spotifoss config directory");
            return;
        }

        if fs::copy(&legacy_path, &new_path).is_err() {
            log::warn!("failed to migrate config.json from Spotix");
            return;
        }

        let legacy_playback = legacy_dir.join("last_playback.json");
        let new_playback = new_dir.join("last_playback.json");
        if legacy_playback.is_file() && fs::copy(&legacy_playback, &new_playback).is_ok() {
            log::info!("migrated last_playback.json from Spotix");
        }

        let legacy_themes = legacy_dir.join("themes");
        let new_themes = new_dir.join("themes");
        if legacy_themes.is_dir()
            && copy_dir_all(&legacy_themes, &new_themes).is_ok()
        {
            log::info!("migrated themes from Spotix");
        }
    }

    pub fn load() -> Option<Config> {
        Self::migrate_legacy_config();
        let path = Self::config_path().expect("Failed to get config path");
        if let Ok(file) = File::open(&path) {
            log::info!("loading config: {:?}", &path);
            let reader = BufReader::new(file);
            let mut config: Config = serde_json::from_reader(reader).expect("Failed to read config");
            config.finalize_queue_settings();
            Some(config)
        } else {
            None
        }
    }

    fn finalize_queue_settings(&mut self) {
        if let Some(legacy) = self.queue_behavior.take() {
            let (shuffle, repeat) =
                super::playback::queue_settings_from_legacy(legacy);
            self.shuffle = shuffle;
            self.repeat = repeat;
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir().expect("Failed to get config dir");
        let path = Self::config_path().expect("Failed to get config path");
        mkdir_if_not_exists(&dir).expect("Failed to create config dir");

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(target_family = "unix")]
        options.mode(0o600);

        let file = options.open(&path).expect("Failed to create config");
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).expect("Failed to write config");
        log::info!("saved config: {:?}", &path);
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    pub fn store_credentials(&mut self, credentials: Credentials) {
        self.credentials = Some(credentials);
    }

    pub fn credentials_clone(&self) -> Option<Credentials> {
        self.credentials.clone()
    }

    pub fn clear_credentials(&mut self) {
        self.credentials = Default::default();
        self.clear_oauth_token();
    }

    pub fn clear_oauth_token(&mut self) {
        self.oauth_token = Default::default();
        self.oauth_token_client_id = Default::default();
    }

    pub fn ensure_device_id(&mut self) -> String {
        if let Some(id) = self.device_id.clone() {
            return id;
        }
        let mut bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut bytes);
        let mut id = String::with_capacity(32);
        for b in bytes {
            id.push_str(&format!("{b:02x}"));
        }
        self.device_id = Some(id.clone());
        id
    }

    pub fn oauth_token_clone(&self) -> Option<OAuthToken> {
        self.oauth_token.clone()
    }

    pub fn store_oauth_token(&mut self, token: OAuthToken) {
        let client_id = self.effective_webapi_client_id().to_string();
        self.store_oauth_token_with_client_id(token, &client_id);
    }

    pub fn store_oauth_token_with_client_id(&mut self, token: OAuthToken, client_id: &str) {
        self.oauth_token = Some(token);
        self.oauth_token_client_id = Some(client_id.to_string());
    }

    /// Returns `true` when the stored OAuth token cannot be used and the user
    /// should sign in again via the browser.
    pub fn oauth_needs_reauth(&self) -> bool {
        let Some(token) = self.oauth_token.as_ref() else {
            return false;
        };
        if !self.oauth_client_id_matches() {
            return true;
        }
        if token.is_expired(OAUTH_EXPIRY_BUFFER) && token.refresh_token.is_none() {
            return true;
        }
        false
    }

    pub fn oauth_client_id_matches(&self) -> bool {
        let Some(stored_client_id) = self.oauth_token_client_id.as_deref() else {
            // Legacy configs without a stored client ID — attempt refresh with
            // the current client ID and see if it works.
            return true;
        };
        stored_client_id == self.effective_webapi_client_id()
    }

    /// Refresh an expired OAuth access token on startup. Returns `true` when
    /// the user must re-authenticate in the browser.
    pub fn refresh_oauth_if_needed(&mut self) -> bool {
        if self.oauth_token.is_none() {
            return false;
        }

        if !self.oauth_client_id_matches() {
            log::warn!(
                "webapi: Spotify client ID changed since last sign-in; clearing stored OAuth token"
            );
            self.clear_oauth_token();
            return true;
        }

        let Some(token) = self.oauth_token.clone() else {
            return false;
        };

        if !token.is_expired(OAUTH_EXPIRY_BUFFER) {
            return false;
        }

        let Some(refresh_token) = token.refresh_token.clone() else {
            log::warn!("webapi: OAuth access token expired with no refresh token");
            return true;
        };

        let client_id = self.effective_webapi_client_id().to_string();
        match oauth::refresh_access_token(&refresh_token, &client_id) {
            Ok(refreshed) => {
                log::info!("webapi: refreshed OAuth access token on startup");
                self.store_oauth_token_with_client_id(refreshed, &client_id);
                self.save();
                false
            }
            Err(err) => {
                let message = err.to_string();
                log::warn!("webapi: startup OAuth refresh failed: {message}");
                if message.contains("invalid_grant") {
                    self.clear_oauth_token();
                    self.save();
                }
                true
            }
        }
    }

    /// Write the current OAuth token to disk (called after in-app refresh).
    pub fn persist_oauth_token_to_disk(token: &OAuthToken, client_id: &str) {
        let Some(mut config) = Config::load() else {
            return;
        };
        config.store_oauth_token_with_client_id(token.clone(), client_id);
        config.save();
    }

    /// Remove a revoked OAuth token from disk.
    pub fn clear_oauth_token_on_disk() {
        let Some(mut config) = Config::load() else {
            return;
        };
        if config.oauth_token.is_some() {
            config.clear_oauth_token();
            config.save();
        }
    }

    pub fn effective_webapi_client_id(&self) -> &str {
        self.webapi_client_id
            .as_deref()
            .map(str::trim)
            .filter(|client_id| !client_id.is_empty())
            .unwrap_or(spotifoss_core::session::access_token::CLIENT_ID)
    }

    pub fn username(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .and_then(|c| c.username.as_deref())
    }

    pub fn session(&self) -> SessionConfig {
        SessionConfig {
            login_creds: self.credentials.clone().expect("Missing credentials"),
            proxy_url: Config::proxy(),
        }
    }

    pub fn playback(&self) -> PlaybackConfig {
        PlaybackConfig {
            bitrate: self.audio_quality.as_bitrate(),
            audio_cache_limit: if self.audio_cache_limit_mb <= 0.0 {
                None
            } else {
                Some((self.audio_cache_limit_mb * 1024.0 * 1024.0) as u64)
            },
            crossfade_duration: Duration::from_secs_f64(self.crossfade_duration_secs.max(0.0)),
            mono_audio: self.mono_audio,
            eq: self.eq.to_core(),
            normalization_enabled: self.normalization_enabled,
            engine: match self.playback_engine {
                PlaybackEngine::Librespot => CorePlaybackEngine::Librespot,
                PlaybackEngine::Native => CorePlaybackEngine::Native,
            },
            ..PlaybackConfig::default()
        }
    }

    pub fn proxy() -> Option<String> {
        env::var(PROXY_ENV_VAR).map_or_else(
            |err| match err {
                VarError::NotPresent => None,
                VarError::NotUnicode(_) => {
                    log::error!("proxy URL is not a valid unicode");
                    None
                }
            },
            Some,
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum AudioQuality {
    Low,
    Normal,
    #[default]
    High,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum PlaybackEngine {
    Native,
    #[default]
    Librespot,
}

impl AudioQuality {
    fn as_bitrate(self) -> usize {
        match self {
            AudioQuality::Low => 96,
            AudioQuality::Normal => 160,
            AudioQuality::High => 320,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum EqPreset {
    Flat,
    Acoustic,
    BassBoost,
    Classical,
    Dance,
    Electronic,
    HipHop,
    Jazz,
    Pop,
    Rock,
    TrebleBoost,
    Vocal,
    SmallSpeakers,
    SpokenWord,
    Loudness,
    Custom,
}

impl EqPreset {
    pub fn label(self) -> &'static str {
        match self {
            EqPreset::Flat => "Flat",
            EqPreset::Acoustic => "Acoustic",
            EqPreset::BassBoost => "Bass Boost",
            EqPreset::Classical => "Classical",
            EqPreset::Dance => "Dance",
            EqPreset::Electronic => "Electronic",
            EqPreset::HipHop => "Hip-Hop",
            EqPreset::Jazz => "Jazz",
            EqPreset::Pop => "Pop",
            EqPreset::Rock => "Rock",
            EqPreset::TrebleBoost => "Treble Boost",
            EqPreset::Vocal => "Vocal",
            EqPreset::SmallSpeakers => "Small Speakers",
            EqPreset::SpokenWord => "Spoken Word",
            EqPreset::Loudness => "Loudness",
            EqPreset::Custom => "Custom",
        }
    }
}

#[derive(Clone, Debug, Data, Lens, Serialize, Deserialize, PartialEq)]
pub struct EqSettings {
    pub enabled: bool,
    pub preset: EqPreset,
    pub bands: EqBands,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: EqPreset::Flat,
            bands: EqBands::default(),
        }
    }
}

impl EqSettings {
    pub fn to_core(&self) -> EqConfig {
        EqConfig {
            enabled: self.enabled,
            gains_db: self.bands.as_array(),
        }
    }

    pub fn apply_preset(&mut self, preset: EqPreset) {
        if preset == EqPreset::Custom {
            return;
        }
        self.bands = EqBands::from_preset(preset);
    }
}

#[derive(Clone, Debug, Data, Lens, Serialize, Deserialize, PartialEq)]
pub struct EqBands {
    pub band_31: f64,
    pub band_62: f64,
    pub band_125: f64,
    pub band_250: f64,
    pub band_500: f64,
    pub band_1k: f64,
    pub band_2k: f64,
    pub band_4k: f64,
    pub band_8k: f64,
    pub band_16k: f64,
}

impl Default for EqBands {
    fn default() -> Self {
        Self {
            band_31: 0.0,
            band_62: 0.0,
            band_125: 0.0,
            band_250: 0.0,
            band_500: 0.0,
            band_1k: 0.0,
            band_2k: 0.0,
            band_4k: 0.0,
            band_8k: 0.0,
            band_16k: 0.0,
        }
    }
}

impl EqBands {
    pub fn from_preset(preset: EqPreset) -> Self {
        match preset {
            EqPreset::Flat | EqPreset::Custom => Self::default(),
            EqPreset::Acoustic => Self::from_db([3.0, 3.0, 2.0, 1.0, 0.0, 1.0, 2.0, 2.0, 1.0, 0.0]),
            EqPreset::BassBoost => {
                Self::from_db([6.0, 5.0, 4.0, 3.0, 1.5, 0.0, -1.0, -1.5, -2.0, -2.0])
            }
            EqPreset::Classical => {
                Self::from_db([3.0, 2.0, 1.0, 0.0, -1.0, 0.0, 2.0, 3.0, 4.0, 5.0])
            }
            EqPreset::Dance => Self::from_db([5.0, 4.0, 2.0, 0.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            EqPreset::Electronic => {
                Self::from_db([4.0, 3.0, 0.0, -2.0, -2.0, 0.0, 2.0, 3.0, 4.0, 4.0])
            }
            EqPreset::HipHop => Self::from_db([5.0, 4.0, 3.0, 1.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            EqPreset::Jazz => Self::from_db([4.0, 3.0, 1.0, 0.0, -2.0, -2.0, 0.0, 1.0, 3.0, 4.0]),
            EqPreset::Pop => Self::from_db([-1.0, 2.0, 4.0, 5.0, 3.0, 0.0, -1.0, -1.0, -1.0, -2.0]),
            EqPreset::Rock => Self::from_db([4.0, 3.0, 1.0, 0.0, -1.0, 1.5, 3.0, 3.5, 3.5, 4.0]),
            EqPreset::TrebleBoost => {
                Self::from_db([-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 6.0])
            }
            EqPreset::Vocal => {
                Self::from_db([-2.0, -2.0, -1.0, 0.0, 3.0, 4.0, 3.0, 2.0, 0.0, -1.0])
            }
            EqPreset::SmallSpeakers => {
                Self::from_db([-4.0, -3.0, 0.0, 3.0, 5.0, 4.0, 2.0, 0.0, -1.5, -3.0])
            }
            EqPreset::SpokenWord => {
                Self::from_db([-4.0, -2.0, 0.0, 2.0, 4.0, 4.0, 2.0, 0.0, -2.0, -4.0])
            }
            EqPreset::Loudness => {
                Self::from_db([5.0, 4.0, 2.0, 0.0, -2.0, -2.0, 0.0, 2.0, 4.0, 5.0])
            }
        }
    }

    pub fn as_array(&self) -> [f32; 10] {
        [
            self.band_31 as f32,
            self.band_62 as f32,
            self.band_125 as f32,
            self.band_250 as f32,
            self.band_500 as f32,
            self.band_1k as f32,
            self.band_2k as f32,
            self.band_4k as f32,
            self.band_8k as f32,
            self.band_16k as f32,
        ]
    }

    fn from_db(values: [f64; 10]) -> Self {
        Self {
            band_31: values[0],
            band_62: values[1],
            band_125: values[2],
            band_250: values[3],
            band_500: values[4],
            band_1k: values[5],
            band_2k: values[6],
            band_4k: values[7],
            band_8k: values[8],
            band_16k: values[9],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Data, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    Custom(String),
}

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Theme::Light => serializer.serialize_str("Light"),
            Theme::Dark => serializer.serialize_str("Dark"),
            Theme::Custom(name) => serializer.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "Light" | "light" => Ok(Theme::Light),
            "Dark" | "dark" => Ok(Theme::Dark),
            other => Ok(Theme::Custom(other.to_string())),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize, Default)]
pub enum SortCriteria {
    Title,
    Artist,
    Album,
    Duration,
    #[default]
    DateAdded,
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    mkdir_if_not_exists(dst).map_err(|err| std::io::Error::other(err.to_string()))?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn get_dir_size(path: &Path) -> Option<u64> {
    fs::read_dir(path).ok()?.try_fold(0, |acc, entry| {
        let entry = entry.ok()?;
        let size = if entry.file_type().ok()?.is_dir() {
            get_dir_size(&entry.path())?
        } else {
            entry.metadata().ok()?.len()
        };
        Some(acc + size)
    })
}

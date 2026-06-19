//! Core logic for the GSB WiFi login tool.
//!
//! This module holds the reusable, UI-agnostic logic: credential storage,
//! INI parsing, the captive-portal login flow, backoff/jitter, URL encoding,
//! TC masking, and Cloudflare WARP management.
//!
//! None of these functions print to stdout. Instead, the ones that produce
//! progress output take a `report` callback (`&dyn Fn(String)`) and hand each
//! status line to it. The GUI wires that callback to an mpsc channel so the
//! messages stream into the on-screen log; a CLI could wire it to `println!`.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::PathBuf;
// Subprocess execution (warp-cli) only exists on desktop; gated to match the
// `#[cfg(desktop)]` WARP routines below so mobile builds do not reference it.
#[cfg(desktop)]
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::secret_store;

/// Captive portal login endpoint (Spring Security form handler).
const LOGIN_URL: &str = "https://wifi.gsb.gov.tr/j_spring_security_check";

/// File name of the credentials file. It lives in a per-user, writable
/// directory (see [`config_path`]) rather than next to the executable, so the
/// app works even when installed read-only under Program Files.
pub const CONFIG_FILE: &str = "config.ini";

/// Per-user application directory name (created under `%APPDATA%` on Windows).
const APP_DIR: &str = "GSBConnect";

/// Previous per-user application directory name. If a config still lives here
/// (from before the rename to "GSB Connect") it is migrated once into [`APP_DIR`]
/// so existing users keep their saved credentials and language.
const LEGACY_APP_DIR: &str = "GSBWiFiLogin";

/// INI key whose presence/value marks the file as holding protected (encrypted)
/// credentials. `1` means the values are produced by [`secret_store::protect`];
/// a missing key (or any other value) means a legacy plaintext file.
const ENCRYPTED_KEY: &str = "ENCRYPTED";

/// INI section holding the credential fields.
const CREDENTIALS_SECTION: &str = "Credentials";

/// INI section holding non-secret user settings (e.g. the language).
const SETTINGS_SECTION: &str = "Settings";

/// INI key under `[Settings]` storing the chosen language code (`tr` / `en`).
const LANGUAGE_KEY: &str = "LANGUAGE";

/// INI key under `[Settings]` storing the "start with Windows" preference
/// (`1` = enabled, anything else = disabled). The registry Run key is the source
/// of truth at startup; this value mirrors it so the file stays self-describing.
const AUTOSTART_KEY: &str = "AUTOSTART";

/// Language code written when none is stored yet (Turkish is the default).
const DEFAULT_LANGUAGE_CODE: &str = "tr";

/// Browser-like User-Agent so the portal treats us like a normal client.
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Marker string searched for in an HTTP 200 body to confirm a successful
/// login. NOTE: You may need to adjust this to match the portal's actual
/// success response (it is matched case-insensitively).
const SUCCESS_MARKER: &str = "login successful";

/// Placeholder value written into the generated config template for the TC ID.
pub const TC_PLACEHOLDER: &str = "PUT_YOUR_TC_HERE";

/// Placeholder value written into the generated config template for the password.
pub const PASSWORD_PLACEHOLDER: &str = "PUT_YOUR_PASSWORD_HERE";

/// Maximum number of login attempts before giving up. We are a polite client
/// and never loop forever hammering the server.
const MAX_ATTEMPTS: u32 = 15;

/// Per-request HTTP timeout, in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 4;

/// Base delay used for exponential backoff between attempts, in milliseconds.
const BASE_DELAY_MS: u64 = 300;

/// Maximum delay between attempts (cap for the exponential backoff), in
/// milliseconds.
const MAX_DELAY_MS: u64 = 3000;

/// Maximum random jitter added on top of the backoff delay, in milliseconds.
const MAX_JITTER_MS: u64 = 250;

/// A progress/status event emitted by the login and WARP routines.
///
/// The core logic is UI-agnostic and knows nothing about languages: it emits
/// these English-named, data-only variants and lets the UI layer translate and
/// format them. This keeps every user-facing string out of `core` (they live in
/// the localization module) while the login/WARP behavior stays identical.
/// Serialized to the frontend as `{ "kind": "VariantName", "data": <payload> }`
/// (unit variants omit `data`). The React i18n layer switches on `kind` and
/// fills `data` into the localized template — the same data-only, translate-in-
/// the-UI contract the egui app used, now across the Tauri IPC boundary.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum LogMsg {
    /// The HTTP client could not be built; carries the error text.
    HttpClientBuildFailed(String),
    /// Login succeeded on the given (1-based) attempt number.
    LoggedInOnAttempt(u32),
    /// The server responded but login was not confirmed on this attempt.
    AttemptNotConfirmed(u32),
    /// The request timed out on this attempt.
    AttemptTimeout(u32),
    /// A non-timeout network error occurred on this attempt.
    AttemptNetworkError(u32),
    /// All attempts were exhausted; carries the attempt count.
    GivingUp(u32),
    /// The login was cancelled by the user (via the Stop button).
    LoginCancelled,
    /// WARP connected successfully.
    WarpConnected,
    /// `warp-cli connect` ran but failed; carries the trimmed stderr.
    WarpConnectFailed(String),
    /// `warp-cli connect` could not be executed; carries the error text.
    WarpRunFailed(String),
    /// `warp-cli` was not found on PATH (header line for the install hint).
    WarpNotFound,
    /// A platform-specific install command or URL to show beneath the header.
    WarpInstallHint(String),
    /// Follow-up instruction shown after the install hint.
    WarpInstallFollowup,
}

/// Credentials loaded from, or to be saved to, the config file.
#[derive(Clone, Default)]
pub struct Credentials {
    pub tc: String,
    pub password: String,
}

// Note: credential completeness (`is_complete`) and TC masking (`mask_tc`) now
// live in the frontend (`src/App.tsx` / `src/i18n.ts`), since those concerns are
// purely about the on-screen form and its log lines.

/// Resolves the full path to the per-user config file (without touching disk).
///
/// On Windows this is `%APPDATA%\GSBWiFiLogin\config.ini`. On other platforms
/// it falls back to `~/.gsbwifi/config.ini` so the project still builds and runs
/// during development. Returns an error if the base directory cannot be located.
pub fn config_path() -> io::Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE))
}

/// Like [`config_path`], but also creates the parent directory if missing.
/// Used on the write path so saving works on a clean machine.
fn config_path_ensured() -> io::Result<PathBuf> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(CONFIG_FILE))
}

/// Returns the per-user directory that holds the config file.
#[cfg(windows)]
fn config_dir() -> io::Result<PathBuf> {
    Ok(appdata_dir()?.join(APP_DIR))
}

/// Returns the previous per-user directory (before the rename), used once by
/// [`migrate_legacy_config`] to bring an existing config forward.
#[cfg(windows)]
fn legacy_config_dir() -> io::Result<PathBuf> {
    Ok(appdata_dir()?.join(LEGACY_APP_DIR))
}

/// Resolves `%APPDATA%`, the base for both the current and legacy directories.
#[cfg(windows)]
fn appdata_dir() -> io::Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "APPDATA environment variable is not set",
        )
    })?;
    Ok(PathBuf::from(appdata))
}

/// Returns the per-user directory that holds the config file (non-Windows
/// desktop development fallback): `~/.gsbconnect`.
#[cfg(all(not(windows), not(target_os = "android"), not(target_os = "ios")))]
fn config_dir() -> io::Result<PathBuf> {
    Ok(home_dir()?.join(".gsbconnect"))
}

/// Returns the previous non-Windows directory (`~/.gsbwifi`) for migration.
#[cfg(all(not(windows), not(target_os = "android"), not(target_os = "ios")))]
fn legacy_config_dir() -> io::Result<PathBuf> {
    Ok(home_dir()?.join(".gsbwifi"))
}

/// Resolves `$HOME`, the base for both the current and legacy directories.
#[cfg(all(not(windows), not(target_os = "android"), not(target_os = "ios")))]
fn home_dir() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HOME environment variable is not set",
        )
    })?;
    Ok(PathBuf::from(home))
}

// ---------------------------------------------------------------------------
// Mobile (Android / iOS) config directory.
//
// On mobile the OS sandboxes each app and environment variables like `HOME` are
// not usable for storage. Tauri resolves the per-app data directory at startup
// (`app.path().app_data_dir()`) and injects it here once via [`init_mobile_dir`]
// before any config read/write happens. There is no legacy location on mobile,
// so the legacy directory equals the current one (migration becomes a no-op).
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile_dir {
    use std::io;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    static DIR: OnceLock<PathBuf> = OnceLock::new();

    pub fn init(dir: PathBuf) {
        // First write wins; later calls (there should be none) are ignored.
        let _ = DIR.set(dir);
    }

    pub fn get() -> io::Result<PathBuf> {
        DIR.get().cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "mobile config directory was not initialized",
            )
        })
    }
}

/// Injects the OS-sandboxed per-app data directory resolved by Tauri. Called
/// once from the Tauri `setup` hook on mobile before any config access.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn init_mobile_dir(dir: PathBuf) {
    mobile_dir::init(dir);
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn config_dir() -> io::Result<PathBuf> {
    mobile_dir::get()
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn legacy_config_dir() -> io::Result<PathBuf> {
    mobile_dir::get()
}

/// Migrates a config from the legacy directory once, if present.
///
/// If the new config file does not yet exist but a legacy one does, the legacy
/// file is copied into the new location so existing users keep their saved
/// credentials and language. Best-effort: any failure leaves both files as they
/// were, and the app simply starts with an empty config. The legacy file is left
/// in place (copied, not moved) so nothing is destroyed.
fn migrate_legacy_config() {
    let (Ok(new_path), Ok(legacy_path)) = (config_path(), legacy_config_path()) else {
        return;
    };
    if new_path.exists() || !legacy_path.exists() {
        return;
    }
    if let Some(parent) = new_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = fs::copy(&legacy_path, &new_path);
}

/// Resolves the full path to the legacy config file (without touching disk).
fn legacy_config_path() -> io::Result<PathBuf> {
    Ok(legacy_config_dir()?.join(CONFIG_FILE))
}

/// The persisted application configuration loaded at startup.
pub struct AppConfig {
    /// Credentials for prefilling the UI (empty if unset or placeholder).
    pub creds: Credentials,
    /// Persisted language code (`tr` / `en`). Defaults to `tr`.
    pub language_code: String,
}

/// Loads the per-user config (credentials + language) for the GUI.
///
/// This never errors out: a missing or unreadable file yields empty credentials
/// and the default language. Two credential forms are handled transparently:
/// * **Encrypted** (`ENCRYPTED = 1`): decrypted via [`secret_store::unprotect`];
///   a decryption failure (for example a file copied from another user or
///   machine) yields an empty field.
/// * **Legacy plaintext** (no marker): read as-is and, when encryption is
///   available, immediately re-saved encrypted (seamless auto-migration).
///
/// On first run (no `LANGUAGE` key) the default language is written back so the
/// choice persists from then on.
pub fn load_config() -> AppConfig {
    // Bring a pre-rename config forward once, before anything reads the file.
    migrate_legacy_config();

    let contents = read_config_contents();

    let encrypted = ini_get(&contents, CREDENTIALS_SECTION, ENCRYPTED_KEY)
        .map(|value| value.trim() == "1")
        .unwrap_or(false);
    let raw_tc = ini_get(&contents, CREDENTIALS_SECTION, "TC_KIMLIK").unwrap_or_default();
    let raw_password = ini_get(&contents, CREDENTIALS_SECTION, "SIFRE").unwrap_or_default();

    let stored_language = ini_get(&contents, SETTINGS_SECTION, LANGUAGE_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let language_code = stored_language
        .clone()
        .unwrap_or_else(|| DEFAULT_LANGUAGE_CODE.to_string());

    // Preserve any existing AUTOSTART value when we rewrite the file below
    // (e.g. during encryption migration). The actual auto-start state shown in
    // the UI comes from the OS (`autostart::is_enabled`), not this value.
    let autostart = read_autostart(&contents);

    let (creds, needs_migration) = if encrypted {
        let creds = Credentials {
            tc: usable_or_empty(decrypt_field(&raw_tc), TC_PLACEHOLDER),
            password: usable_or_empty(decrypt_field(&raw_password), PASSWORD_PLACEHOLDER),
        };
        (creds, false)
    } else {
        let creds = Credentials {
            tc: usable_or_empty(raw_tc, TC_PLACEHOLDER),
            password: usable_or_empty(raw_password, PASSWORD_PLACEHOLDER),
        };
        let migrate =
            secret_store::ENCRYPTION_ACTIVE && (!creds.tc.is_empty() || !creds.password.is_empty());
        (creds, migrate)
    };

    // Persist when legacy credentials need encrypting or when the language key
    // is missing (so the default is recorded on first run). Best-effort: a
    // failure here leaves the in-memory config usable for this run.
    if needs_migration || stored_language.is_none() {
        let _ = save_config(&creds, &language_code, autostart);
    }

    AppConfig {
        creds,
        language_code,
    }
}

/// Reads the `[Settings] AUTOSTART` flag from raw config text (default `false`).
fn read_autostart(contents: &str) -> bool {
    ini_get(contents, SETTINGS_SECTION, AUTOSTART_KEY)
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

/// Decrypts a single stored field, returning an empty string for empty input or
/// on any decryption failure. Never logs the value or the error.
fn decrypt_field(stored: &str) -> String {
    if stored.is_empty() {
        return String::new();
    }
    secret_store::unprotect(stored).unwrap_or_default()
}

/// Returns `value` unless it is empty or still the placeholder, in which case
/// an empty string is returned.
fn usable_or_empty(value: String, placeholder: &str) -> String {
    if value.is_empty() || value == placeholder {
        String::new()
    } else {
        value
    }
}

/// Reads the raw config file contents, or an empty string if it is missing or
/// unreadable. Shared by the load and save paths that need the existing file.
fn read_config_contents() -> String {
    config_path()
        .and_then(fs::read_to_string)
        .unwrap_or_default()
}

/// Renders the full config file text from already-stored field values.
///
/// `tc` and `password` are expected to be in their stored (protected/base64)
/// form, and `marker` is the matching `ENCRYPTED` value. This is the single
/// place the on-disk layout is defined, so the two save paths cannot drift.
fn render_config(
    marker: &str,
    tc: &str,
    password: &str,
    language_code: &str,
    autostart: bool,
) -> String {
    format!(
        "[{CREDENTIALS_SECTION}]\n{ENCRYPTED_KEY} = {marker}\nTC_KIMLIK = {tc}\nSIFRE = {password}\n\n[{SETTINGS_SECTION}]\n{LANGUAGE_KEY} = {code}\n{AUTOSTART_KEY} = {autostart}\n",
        code = language_code.trim(),
        autostart = if autostart { "1" } else { "0" }
    )
}

/// The `ENCRYPTED` marker value for the current platform.
fn encryption_marker() -> &'static str {
    if secret_store::ENCRYPTION_ACTIVE {
        "1"
    } else {
        "0"
    }
}

/// Writes credentials and language to the per-user config file.
///
/// Credentials are protected via [`secret_store::protect`] (DPAPI-encrypted and
/// base64-encoded on Windows); the `ENCRYPTED` marker records how they were
/// produced so [`load_config`] knows whether to decrypt them on the next launch.
pub fn save_config(creds: &Credentials, language_code: &str, autostart: bool) -> io::Result<()> {
    let path = config_path_ensured()?;
    let tc = secret_store::protect(creds.tc.trim())?;
    let password = secret_store::protect(creds.password.trim())?;
    let contents = render_config(
        encryption_marker(),
        &tc,
        &password,
        language_code,
        autostart,
    );
    fs::write(path, contents)
}

/// Updates only the stored language, preserving the credentials already on disk
/// verbatim (without decrypting or re-encrypting them, and without persisting
/// any unsaved values the user may have typed into the GUI).
pub fn save_language(language_code: &str) -> io::Result<()> {
    let contents = read_config_contents();
    let marker = ini_get(&contents, CREDENTIALS_SECTION, ENCRYPTED_KEY)
        .unwrap_or_else(|| encryption_marker().to_string());
    let tc = ini_get(&contents, CREDENTIALS_SECTION, "TC_KIMLIK").unwrap_or_default();
    let password = ini_get(&contents, CREDENTIALS_SECTION, "SIFRE").unwrap_or_default();
    let autostart = read_autostart(&contents);
    let path = config_path_ensured()?;
    let out = render_config(&marker, &tc, &password, language_code, autostart);
    fs::write(path, out)
}

/// Updates only the stored `AUTOSTART` preference, preserving the credentials and
/// language already on disk verbatim (mirrors [`save_language`]).
pub fn save_autostart(autostart: bool) -> io::Result<()> {
    let contents = read_config_contents();
    let marker = ini_get(&contents, CREDENTIALS_SECTION, ENCRYPTED_KEY)
        .unwrap_or_else(|| encryption_marker().to_string());
    let tc = ini_get(&contents, CREDENTIALS_SECTION, "TC_KIMLIK").unwrap_or_default();
    let password = ini_get(&contents, CREDENTIALS_SECTION, "SIFRE").unwrap_or_default();
    let language_code = ini_get(&contents, SETTINGS_SECTION, LANGUAGE_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_LANGUAGE_CODE.to_string());
    let path = config_path_ensured()?;
    let out = render_config(&marker, &tc, &password, &language_code, autostart);
    fs::write(path, out)
}

/// Minimal hand-written INI lookup.
///
/// Scans for the requested `[section]`, then returns the trimmed value of the
/// first `key = value` line found within it. Lines starting with `#` or `;`
/// are treated as comments.
pub fn ini_get(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            in_section = name.eq_ignore_ascii_case(section);
            continue;
        }
        if in_section {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// Attempts to log in to the captive portal.
///
/// Uses a bounded number of attempts with exponential backoff plus small
/// clock-derived jitter (capped at `MAX_DELAY_MS`). Each progress event is
/// handed to `report` instead of being printed. Returns `true` on success.
///
/// `cancel` is a shared flag the caller (the UI thread) can set to abort the
/// run. It is checked between attempts and, crucially, in small slices during
/// the backoff wait (see [`interruptible_sleep`]), so a Stop request takes
/// effect within a fraction of a second instead of after the full delay. On
/// cancellation the loop stops, a [`LogMsg::LoginCancelled`] event is reported,
/// and the function returns `false`.
pub fn aggressive_login(creds: &Credentials, cancel: &AtomicBool, report: &dyn Fn(LogMsg)) -> bool {
    let client = match reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            report(LogMsg::HttpClientBuildFailed(e.to_string()));
            return false;
        }
    };

    // Pre-encode the form body once; values are URL-encoded.
    let body = format!(
        "j_username={}&j_password={}",
        url_encode(&creds.tc),
        url_encode(&creds.password)
    );

    for attempt in 1..=MAX_ATTEMPTS {
        // Abort before starting an attempt if a Stop was requested while we
        // were waiting out the previous backoff.
        if cancel.load(Ordering::Relaxed) {
            report(LogMsg::LoginCancelled);
            return false;
        }

        let result = client
            .post(LOGIN_URL)
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.clone())
            .send();

        match result {
            Ok(response) => {
                let status = response.status();
                if status.as_u16() == 302 && response.headers().contains_key("location") {
                    report(LogMsg::LoggedInOnAttempt(attempt));
                    return true;
                }
                if status.as_u16() == 200 {
                    let text = response.text().unwrap_or_default();
                    if text.to_lowercase().contains(SUCCESS_MARKER) {
                        report(LogMsg::LoggedInOnAttempt(attempt));
                        return true;
                    }
                }
                report(LogMsg::AttemptNotConfirmed(attempt));
            }
            Err(e) if e.is_timeout() => {
                report(LogMsg::AttemptTimeout(attempt));
            }
            Err(_) => {
                report(LogMsg::AttemptNetworkError(attempt));
            }
        }

        // Do not sleep after the final attempt. The wait is interruptible so a
        // Stop during backoff is honored almost immediately.
        if attempt < MAX_ATTEMPTS && interruptible_sleep(backoff_delay(attempt), cancel) {
            report(LogMsg::LoginCancelled);
            return false;
        }
    }

    report(LogMsg::GivingUp(MAX_ATTEMPTS));
    false
}

/// Sleeps for `delay`, but in small slices so a cancellation can be observed
/// promptly. Returns `true` if `cancel` became set during the wait (in which
/// case the remaining time is skipped), `false` if the full delay elapsed.
fn interruptible_sleep(delay: Duration, cancel: &AtomicBool) -> bool {
    /// Granularity of the cancel check; small enough to feel instant.
    const SLICE: Duration = Duration::from_millis(50);

    let mut remaining = delay;
    while remaining > Duration::ZERO {
        if cancel.load(Ordering::Relaxed) {
            return true;
        }
        let step = remaining.min(SLICE);
        sleep(step);
        remaining -= step;
    }
    cancel.load(Ordering::Relaxed)
}

/// Computes the backoff delay for a given (1-based) attempt number.
///
/// Exponential growth `BASE_DELAY_MS * 2^(attempt-1)`, capped at
/// `MAX_DELAY_MS`, with up to `MAX_JITTER_MS` of jitter derived from the
/// system clock (so we avoid pulling in a random-number crate).
fn backoff_delay(attempt: u32) -> Duration {
    let exponential = BASE_DELAY_MS.saturating_mul(1u64 << (attempt - 1).min(20));
    let capped = exponential.min(MAX_DELAY_MS);
    let jitter = clock_jitter() % (MAX_JITTER_MS + 1);
    Duration::from_millis(capped + jitter)
}

/// Derives a pseudo-random value from the system clock's nanosecond field.
fn clock_jitter() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

/// Percent-encodes a string for use in an `application/x-www-form-urlencoded`
/// body. Unreserved characters pass through; everything else is `%XX`.
fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

/// Returns `true` if `warp-cli` is available on the PATH.
///
/// Desktop-only: WARP is controlled through the `warp-cli` subprocess, which
/// does not exist on mobile. The UI hides every WARP control on mobile, so this
/// is never reached there; gating it keeps `std::process::Command` out of the
/// mobile build entirely.
#[cfg(desktop)]
pub fn check_warp_installed() -> bool {
    Command::new("warp-cli")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Connects WARP if installed, otherwise reports platform-specific install
/// hints. Returns `true` if WARP is installed and the connect succeeded.
///
/// Desktop-only (see [`check_warp_installed`]).
#[cfg(desktop)]
pub fn manage_warp(report: &dyn Fn(LogMsg)) -> bool {
    if check_warp_installed() {
        match Command::new("warp-cli").arg("connect").output() {
            Ok(output) if output.status.success() => {
                report(LogMsg::WarpConnected);
                return true;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                report(LogMsg::WarpConnectFailed(stderr.trim().to_string()));
            }
            Err(e) => {
                report(LogMsg::WarpRunFailed(e.to_string()));
            }
        }
        return false;
    }

    report(LogMsg::WarpNotFound);
    let install_hint = if cfg!(target_os = "windows") {
        "winget install -e --id Cloudflare.Warp"
    } else if cfg!(target_os = "macos") {
        "brew install --cask cloudflare-warp"
    } else {
        "https://pkg.cloudflareclient.com"
    };
    report(LogMsg::WarpInstallHint(install_hint.to_string()));
    report(LogMsg::WarpInstallFollowup);
    false
}

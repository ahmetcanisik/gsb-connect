//! Tauri command surface — the bridge between the React frontend and the
//! reusable Rust logic in `core` / `secret_store` / `autostart`.
//!
//! Long-running work (`connect`, `check_warp`) does not block the IPC call: it
//! is spawned on a background thread that streams progress to the frontend as
//! events (`log`, `login-result`, `warp-result`, `done`). This mirrors the
//! original egui worker-thread + channel design, so `core`'s blocking login is
//! reused unchanged.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::autostart;
use crate::core::{self, Credentials, LogMsg};
use crate::AppState;

/// Snapshot of the persisted configuration sent to the frontend on startup.
#[derive(serde::Serialize)]
pub struct ConfigDto {
    pub tc: String,
    pub password: String,
    pub language: String,
    pub autostart: bool,
}

/// The current OS as a lowercase string (`"windows"`, `"android"`, `"macos"`,
/// `"linux"`, `"ios"`). The frontend uses this to hide desktop-only controls
/// (WARP, "start with the OS") on mobile.
#[tauri::command]
pub fn platform() -> String {
    std::env::consts::OS.to_string()
}

/// Loads persisted credentials, language, and the real auto-start state.
///
/// `autostart` reflects the actual OS state ([`autostart::is_enabled`]) rather
/// than the mirrored config value, so the toggle shows reality even if the user
/// changed it outside the app. On mobile `is_enabled` is a no-op returning
/// `false`, and the control is hidden anyway.
#[tauri::command]
pub fn load_config() -> ConfigDto {
    let cfg = core::load_config();
    ConfigDto {
        tc: cfg.creds.tc,
        password: cfg.creds.password,
        language: cfg.language_code,
        autostart: autostart::is_enabled(),
    }
}

/// Persists credentials and language to the per-user config file. Credentials
/// are encrypted at rest where supported (Windows DPAPI); see `secret_store`.
#[tauri::command]
pub fn save_credentials(tc: String, password: String, language: String) -> Result<(), String> {
    let creds = Credentials { tc, password };
    core::save_config(&creds, &language, autostart::is_enabled()).map_err(|e| e.to_string())
}

/// Persists only the language, leaving stored credentials untouched.
#[tauri::command]
pub fn save_language(language: String) -> Result<(), String> {
    core::save_language(&language).map_err(|e| e.to_string())
}

/// Returns whether "start with the OS" is currently enabled.
#[tauri::command]
pub fn get_autostart() -> bool {
    autostart::is_enabled()
}

/// Enables or disables "start with the OS" and mirrors the choice into config.
/// On mobile this is a successful no-op (the control is hidden).
#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    autostart::set_enabled(enabled).map_err(|e| e.to_string())?;
    let _ = core::save_autostart(enabled);
    Ok(())
}

/// Starts a login attempt (optionally followed by WARP) on a background thread.
///
/// Progress streams to the frontend as `log` events; the final outcomes are
/// emitted as `login-result` / `warp-result` (booleans) and the run always ends
/// with a `done` event so the UI can re-enable its buttons. Honors a `stop`
/// request through the shared cancel flag: on cancellation no result events are
/// emitted (the indicators are left as they were), only `done`.
#[tauri::command]
pub fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    tc: String,
    password: String,
    with_warp: bool,
) {
    let creds = Credentials { tc, password };
    let cancel = state.cancel.clone();
    // Clear any leftover cancel request so this run starts fresh.
    cancel.store(false, Ordering::Relaxed);

    std::thread::spawn(move || {
        let report = {
            let app = app.clone();
            move |msg: LogMsg| {
                let _ = app.emit("log", &msg);
            }
        };

        let login_ok = core::aggressive_login(&creds, &cancel, &report);

        // On cancellation, leave the indicators unchanged: emit only `done`.
        if cancel.load(Ordering::Relaxed) {
            let _ = app.emit("done", ());
            return;
        }
        let _ = app.emit("login-result", login_ok);

        // WARP is desktop-only. On mobile the parameter is ignored and the UI
        // never offers the option.
        #[cfg(desktop)]
        if with_warp && login_ok {
            // Brief pause so the portal registers the session before WARP.
            std::thread::sleep(std::time::Duration::from_secs(1));
            let warp_ok = core::manage_warp(&report);
            let _ = app.emit("warp-result", warp_ok);
        }
        #[cfg(not(desktop))]
        let _ = with_warp;

        let _ = app.emit("done", ());
    });
}

/// Requests cancellation of an in-progress login (sets the shared flag; the
/// worker observes it promptly, even mid-backoff).
#[tauri::command]
pub fn stop(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

/// Checks / connects Cloudflare WARP, or reports install hints (desktop only).
///
/// Like `connect`, it runs on a background thread and streams `log` events,
/// ending with `warp-result` and `done`. On mobile it is a no-op.
#[tauri::command]
pub fn check_warp(app: AppHandle) {
    #[cfg(desktop)]
    std::thread::spawn(move || {
        let report = {
            let app = app.clone();
            move |msg: LogMsg| {
                let _ = app.emit("log", &msg);
            }
        };
        let warp_ok = core::manage_warp(&report);
        let _ = app.emit("warp-result", warp_ok);
        let _ = app.emit("done", ());
    });

    #[cfg(not(desktop))]
    let _ = app;
}

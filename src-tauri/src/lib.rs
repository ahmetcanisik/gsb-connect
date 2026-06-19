//! GSB Connect — Tauri application library.
//!
//! This crate holds the whole app so it can be shared between the desktop
//! binary (`main.rs`) and the generated mobile platform projects. The UI is a
//! web frontend (see `../src`); this side exposes the reusable login/WARP/config
//! logic (`core`), credential-at-rest protection (`secret_store`), and the
//! "start with the OS" toggle (`autostart`) through Tauri commands in
//! `commands`.
//!
//! The login flow streams progress to the frontend as Tauri events rather than
//! returning a single value: `connect`/`check_warp` run the blocking work on a
//! background thread (mirroring the original worker-thread design) and emit
//! `log`, `login-result`, `warp-result`, and `done` events the UI listens for.

mod autostart;
mod commands;
mod core;
mod secret_store;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Shared application state, accessible from every command via `State`.
pub struct AppState {
    /// Cooperative cancellation flag for the in-progress login. `connect` clears
    /// it before each run and `stop` sets it; the login loop observes it between
    /// attempts and during the backoff wait (see [`core::aggressive_login`]).
    pub cancel: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .setup(|app| {
            // On mobile, hand the OS-sandboxed per-app data directory to `core`
            // before any config read/write (env vars are not usable there).
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                use tauri::Manager;
                if let Ok(dir) = app.path().app_data_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    crate::core::init_mobile_dir(dir);
                }
            }
            let _ = &app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::platform,
            commands::load_config,
            commands::save_credentials,
            commands::save_language,
            commands::get_autostart,
            commands::set_autostart,
            commands::connect,
            commands::stop,
            commands::check_warp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

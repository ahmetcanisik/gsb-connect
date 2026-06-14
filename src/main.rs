//! GSB WiFi login automation tool (GUI).
//!
//! This is a personal automation utility for the GSB WiFi captive portal.
//! It logs in with the user's OWN credentials and can optionally enable
//! Cloudflare WARP to avoid bandwidth throttling. This is login automation,
//! not a security bypass.
//!
//! The window runs on the main thread via `eframe::run_native`. All blocking
//! network work happens on background `std::thread`s that report progress back
//! to the UI over an `mpsc` channel (see the worker-thread pattern below).
//!
//! Performance note: the UI is event-driven. egui only repaints on user input;
//! while a worker thread runs it requests repaints from the worker itself (once
//! per status message, plus a final one on completion). When idle, nothing
//! repaints, so the app sits at ~0% CPU.
//!
//! The visual language is modeled on the Cloudflare One Client (WARP desktop
//! app): a dark theme, a left sidebar with nav items, and a main content panel
//! that swaps based on the selected nav item. All theming lives in one place,
//! `apply_theme`, so colors and spacing are easy to tweak.
//!
//! All user-facing text is localized (Turkish + English, default Turkish).
//! Widgets never hardcode display strings; they resolve English ASCII keys
//! through the `i18n` module every frame, so changing the language updates the
//! whole UI live.

// On non-debug (release) builds, suppress the extra console window on Windows.
// Debug builds keep the console so logs/panics remain visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alert;
mod autostart;
mod core;
mod i18n;
mod secret_store;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use eframe::egui;
use egui::{Color32, CornerRadius, FontId, Margin, RichText, Stroke};

use crate::alert::{Alert, AlertKind};
use crate::core::Credentials;
use crate::i18n::Language;

/// Short product name (a brand mark, shown in the sidebar and About page).
/// It is a proper noun and is intentionally not translated.
const APP_NAME: &str = "GSB Connect";

// ---------------------------------------------------------------------------
// Color palette (Cloudflare One Client inspired, dark theme).
//
// All colors are defined here so the look can be retuned in one place. They are
// `const` (Color32::from_rgb is a const fn) so the theme function and the small
// drawing helpers can share them without cloning a struct around.
// ---------------------------------------------------------------------------

/// Main window background: near-black neutral.
const BG_MAIN: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1D);
/// Panel/card background: a touch lighter than the main background.
const BG_PANEL: Color32 = Color32::from_rgb(0x24, 0x24, 0x27);
/// Sidebar background: slightly distinct from the main background.
const BG_SIDEBAR: Color32 = Color32::from_rgb(0x20, 0x20, 0x22);
/// Text-field / log background: slightly darker than the panel.
const BG_FIELD: Color32 = Color32::from_rgb(0x16, 0x16, 0x18);
/// Highlight behind the selected sidebar nav item.
const NAV_SELECTED: Color32 = Color32::from_rgb(0x30, 0x30, 0x34);
/// Highlight behind a hovered sidebar nav item.
const NAV_HOVER: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2E);

/// Primary (near-white) text.
const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF3);
/// Secondary / muted text.
const TEXT_MUTED: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9F);

/// Cloudflare-style primary blue.
const BLUE: Color32 = Color32::from_rgb(0x2F, 0x7E, 0xF0);
/// Secondary button fill (neutral, less prominent than the blue primary).
const BTN_NEUTRAL: Color32 = Color32::from_rgb(0x2E, 0x2E, 0x33);
/// Secondary button hover fill.
const BTN_NEUTRAL_HOVER: Color32 = Color32::from_rgb(0x39, 0x39, 0x3F);

/// "Ok" status color (green).
const STATUS_OK: Color32 = Color32::from_rgb(0x37, 0xB6, 0x5A);
/// "Not ok" status color (red).
const STATUS_BAD: Color32 = Color32::from_rgb(0xE5, 0x53, 0x53);

/// Minimum height of the bottom log panel, in points. It also acts as the
/// floor for the "no more than half the window" cap, so the panel can never
/// collapse to nothing while dragging its top edge.
const LOG_PANEL_MIN_HEIGHT: f32 = 120.0;
/// Default (initial) height of the bottom log panel, VS Code-like strip.
const LOG_PANEL_DEFAULT_HEIGHT: f32 = 160.0;

/// Which sidebar page is currently shown in the central panel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Home,
    Settings,
    About,
}

/// Which Home action is currently running, so its button shows a spinner.
/// `None` means idle (or a non-Home action such as the About WARP-check).
#[derive(Clone, Copy, PartialEq, Eq)]
enum HomeAction {
    Connect,
    ConnectWarp,
}

/// Messages sent from a worker thread back to the UI thread.
///
/// The UI drains these once per frame in `update()`. To add a new long-running
/// action later (for example a cancel acknowledgement), add a variant here and
/// handle it in the drain loop.
enum WorkerMsg {
    /// A progress event to translate, format, and append to the log.
    Status(core::LogMsg),
    /// Final login outcome (drives the "Login" status indicator).
    LoginResult(bool),
    /// Final WARP outcome (drives the "WARP" status indicator).
    WarpResult(bool),
    /// The worker has finished; the UI re-enables the action buttons.
    Done,
}

/// Application state held on the UI thread.
struct GsbWifiApp {
    /// Currently selected sidebar page.
    nav: Nav,
    /// Active UI language (re-resolved into strings every frame).
    language: Language,
    tc: String,
    password: String,
    log: String,
    /// Whether the bottom log panel is expanded. The header/toolbar row stays
    /// visible when collapsed, so the toggle is always reachable (VS Code-style).
    log_visible: bool,
    /// Whether "start with Windows" is enabled (mirrors the registry Run key).
    autostart: bool,
    /// Set when launched with `--autostart`; minimizes the window once on the
    /// first frame (ViewportBuilder has no start-minimized option in this egui
    /// version, so we issue a one-shot viewport command instead).
    pending_minimize: bool,
    /// True while a worker thread is running; disables the action buttons.
    is_busy: bool,
    /// Which Home action is in progress, so its button shows a spinner. `None`
    /// when idle or when the running action is not a Home button.
    running: Option<HomeAction>,
    /// Cancellation flag shared with the running login worker. Setting it (via
    /// the Stop button) makes the worker abort between attempts and during the
    /// backoff wait. Reset to `false` before each new worker so a prior Stop
    /// never aborts the next run.
    cancel: Arc<AtomicBool>,
    /// Last known login state, for the colored status indicator.
    login_ok: bool,
    /// Last known WARP state, for the colored status indicator.
    warp_ok: bool,
    /// Currently active alert banner, if any. Set from anywhere; rendered by the
    /// relevant view (and cleared when it dismisses).
    alert: Option<Alert>,
    /// Sender handed to each worker thread.
    tx: Sender<WorkerMsg>,
    /// Receiver drained once per frame.
    rx: Receiver<WorkerMsg>,
}

impl GsbWifiApp {
    /// Builds the app from the loaded config (prefilled credentials + language +
    /// the resolved auto-start state). `start_minimized` reflects the
    /// `--autostart` launch flag.
    fn new(creds: Credentials, language: Language, autostart: bool, start_minimized: bool) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            nav: Nav::Home,
            language,
            tc: creds.tc,
            password: creds.password,
            log: String::new(),
            log_visible: true,
            autostart,
            pending_minimize: start_minimized,
            is_busy: false,
            running: None,
            cancel: Arc::new(AtomicBool::new(false)),
            login_ok: false,
            warp_ok: false,
            alert: None,
            tx,
            rx,
        }
    }

    /// Resolves a translation key to text in the active language.
    fn tr(&self, key: &'static str) -> &'static str {
        i18n::t(self.language, key)
    }

    /// Appends a line to the on-screen log.
    fn append_log(&mut self, line: impl AsRef<str>) {
        self.log.push_str(line.as_ref());
        self.log.push('\n');
    }

    /// Translates and formats a core [`core::LogMsg`] for the active language.
    fn format_log(&self, msg: core::LogMsg) -> String {
        use core::LogMsg as M;
        let language = self.language;
        let t = |key: &'static str| i18n::t(language, key);
        match msg {
            M::HttpClientBuildFailed(e) => t("log_http_client_failed").replace("{e}", &e),
            M::LoggedInOnAttempt(n) => t("log_logged_in").replace("{n}", &n.to_string()),
            M::AttemptNotConfirmed(n) => t("log_not_confirmed").replace("{n}", &n.to_string()),
            M::AttemptTimeout(n) => t("log_timeout").replace("{n}", &n.to_string()),
            M::AttemptNetworkError(n) => t("log_network_error").replace("{n}", &n.to_string()),
            M::GivingUp(n) => t("log_giving_up").replace("{n}", &n.to_string()),
            M::LoginCancelled => t("log_login_cancelled").to_string(),
            M::WarpConnected => t("log_warp_connected").to_string(),
            M::WarpConnectFailed(e) => t("log_warp_connect_failed").replace("{e}", &e),
            M::WarpRunFailed(e) => t("log_warp_run_failed").replace("{e}", &e),
            M::WarpNotFound => t("log_warp_not_found").to_string(),
            // The install hint is a literal command/URL, not translatable prose.
            M::WarpInstallHint(cmd) => format!("  {cmd}"),
            M::WarpInstallFollowup => t("log_warp_followup").to_string(),
        }
    }

    /// Returns the current field values as `Credentials`, or guides the user to
    /// fix empty fields and returns `None`.
    ///
    /// On empty input it logs an actionable suggestion, switches the active nav
    /// to Settings (where the fields live), and raises a short-lived Warning
    /// alert. This runs only on an explicit Connect button press (the sole
    /// caller), so it never fires per frame and never loops.
    fn validated_credentials(&mut self) -> Option<Credentials> {
        let creds = Credentials {
            tc: self.tc.trim().to_string(),
            password: self.password.trim().to_string(),
        };
        if !creds.is_complete() {
            self.append_log(self.tr("log_credentials_empty_hint"));
            self.nav = Nav::Settings;
            self.alert = Some(Alert::new(
                self.tr("alert_fill_credentials"),
                AlertKind::Warning,
                Some(Duration::from_secs(4)),
            ));
            return None;
        }
        Some(creds)
    }

    /// Spawns a worker thread, wiring its `report` callback to the channel and
    /// requesting a repaint after every message so the UI updates live.
    ///
    /// `work` receives the credentials (already cloned into the thread), the
    /// shared cancel flag, and a `report` closure to stream status events. It
    /// returns `(login_ok, warp_ok)` outcomes; pass `None` for an outcome the
    /// action does not touch so the corresponding indicator is left unchanged
    /// (also used on cancellation, to leave the indicators as they were).
    fn spawn_worker<F>(&mut self, ctx: &egui::Context, creds: Credentials, work: F)
    where
        F: FnOnce(&Credentials, &AtomicBool, &dyn Fn(core::LogMsg)) -> (Option<bool>, Option<bool>)
            + Send
            + 'static,
    {
        self.is_busy = true;
        // Clear any leftover cancel request so this run starts fresh.
        self.cancel.store(false, Ordering::Relaxed);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let cancel = Arc::clone(&self.cancel);

        thread::spawn(move || {
            let report = {
                let tx = tx.clone();
                let ctx = ctx.clone();
                move |msg: core::LogMsg| {
                    let _ = tx.send(WorkerMsg::Status(msg));
                    // Wake the UI thread so it drains and redraws the new line.
                    ctx.request_repaint();
                }
            };

            let (login_ok, warp_ok) = work(&creds, &cancel, &report);
            if let Some(ok) = login_ok {
                let _ = tx.send(WorkerMsg::LoginResult(ok));
            }
            if let Some(ok) = warp_ok {
                let _ = tx.send(WorkerMsg::WarpResult(ok));
            }
            let _ = tx.send(WorkerMsg::Done);
            ctx.request_repaint();
        });
    }

    /// "Connect" action: login only.
    fn start_connect(&mut self, ctx: &egui::Context) {
        let Some(creds) = self.validated_credentials() else {
            return;
        };
        let active = self
            .tr("log_active_tc")
            .replace("{tc}", &core::mask_tc(&creds.tc));
        self.append_log(active);
        self.append_log(self.tr("log_starting_login"));
        self.running = Some(HomeAction::Connect);
        self.spawn_worker(ctx, creds, |creds, cancel, report| {
            let ok = core::aggressive_login(creds, cancel, report);
            // On cancellation leave the login indicator unchanged.
            if cancel.load(Ordering::Relaxed) {
                (None, None)
            } else {
                (Some(ok), None)
            }
        });
    }

    /// "Connect + WARP" action: login, and on success run WARP connect.
    fn start_connect_with_warp(&mut self, ctx: &egui::Context) {
        let Some(creds) = self.validated_credentials() else {
            return;
        };
        let active = self
            .tr("log_active_tc")
            .replace("{tc}", &core::mask_tc(&creds.tc));
        self.append_log(active);
        self.append_log(self.tr("log_starting_login_warp"));
        self.running = Some(HomeAction::ConnectWarp);
        self.spawn_worker(ctx, creds, |creds, cancel, report| {
            let login_ok = core::aggressive_login(creds, cancel, report);
            // On cancellation leave both indicators unchanged and skip WARP.
            if cancel.load(Ordering::Relaxed) {
                return (None, None);
            }
            if !login_ok {
                return (Some(false), None);
            }
            // Brief pause so the portal registers the session before WARP.
            thread::sleep(Duration::from_secs(1));
            let warp_ok = core::manage_warp(report);
            (Some(true), Some(warp_ok))
        });
    }

    /// "Check / install WARP" action: WARP check/connect/hint logic only.
    fn start_warp_check(&mut self, ctx: &egui::Context) {
        self.append_log(self.tr("log_checking_warp"));
        // This action needs no credentials and is not cancellable.
        self.spawn_worker(ctx, Credentials::default(), |_creds, _cancel, report| {
            let warp_ok = core::manage_warp(report);
            (None, Some(warp_ok))
        });
    }

    /// "Stop" action: request cancellation of the in-progress login worker.
    ///
    /// Only sets the shared flag; the worker observes it between attempts and
    /// during the (sliced) backoff wait, then logs a localized cancellation
    /// message and clears the busy state via the usual `Done` message.
    fn stop_login(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// "Save credentials" action: write the current fields to the config file.
    fn save_credentials(&mut self) {
        let creds = Credentials {
            tc: self.tc.trim().to_string(),
            password: self.password.trim().to_string(),
        };
        match core::save_config(&creds, self.language.code(), self.autostart) {
            Ok(()) => {
                let line = self
                    .tr("log_credentials_saved")
                    .replace("{path}", &core::config_location());
                self.append_log(line);
            }
            Err(e) => {
                let line = self
                    .tr("log_credentials_save_failed")
                    .replace("{e}", &e.to_string());
                self.append_log(line);
            }
        }
    }

    /// "Start with Windows" toggle handler.
    ///
    /// Writes or deletes the registry Run entry, then persists the resulting
    /// preference in the config. On failure the in-memory flag is snapped back to
    /// the registry's actual state so the checkbox keeps showing reality.
    fn set_autostart(&mut self, enabled: bool) {
        match autostart::set_enabled(enabled) {
            Ok(()) => {
                self.autostart = enabled;
                let _ = core::save_autostart(enabled);
                self.append_log(self.tr(if enabled {
                    "log_autostart_enabled"
                } else {
                    "log_autostart_disabled"
                }));
            }
            Err(e) => {
                self.autostart = autostart::is_enabled();
                let line = self
                    .tr("log_autostart_failed")
                    .replace("{e}", &e.to_string());
                self.append_log(line);
            }
        }
    }

    /// Drains all pending worker messages into the UI state.
    ///
    /// `try_recv` returns immediately when the channel is empty, so this never
    /// busy-loops: it processes exactly the messages that are already queued.
    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMsg::Status(event) => {
                    let line = self.format_log(event);
                    self.append_log(line);
                }
                WorkerMsg::LoginResult(ok) => self.login_ok = ok,
                WorkerMsg::WarpResult(ok) => self.warp_ok = ok,
                WorkerMsg::Done => {
                    self.is_busy = false;
                    self.running = None;
                }
            }
        }
    }

    // -- Page bodies --------------------------------------------------------

    /// Home page: status headline and the primary Connect actions. While an
    /// action runs, a spinner is shown on whichever Connect button was pressed.
    /// The login/WARP pills live in the global log panel header rather than here.
    fn page_home(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        section_title(ui, self.tr("nav_home"));
        ui.add_space(10.0);

        let (headline, subtitle) = if self.login_ok {
            (
                self.tr("home_headline_connected"),
                self.tr("home_subtitle_connected"),
            )
        } else {
            (
                self.tr("home_headline_disconnected"),
                self.tr("home_subtitle_disconnected"),
            )
        };
        ui.label(
            RichText::new(headline)
                .size(28.0)
                .color(TEXT_PRIMARY)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(14.0).color(TEXT_MUTED));

        ui.add_space(20.0);

        // Both buttons are disabled while busy. The running action's button is
        // rendered with an empty label and gets a spinner overlaid on it (drawn
        // on the enabled parent `ui` below, so it stays bright over the dimmed
        // button) to indicate which action is in progress.
        let buttons = ui.add_enabled_ui(!self.is_busy, |ui| {
            let connect_label = if self.running == Some(HomeAction::Connect) {
                ""
            } else {
                self.tr("button_connect")
            };
            let connect = primary_button(ui, connect_label);
            ui.add_space(8.0);
            let warp_label = if self.running == Some(HomeAction::ConnectWarp) {
                ""
            } else {
                self.tr("button_connect_warp")
            };
            let warp = secondary_button(ui, warp_label);
            (connect, warp)
        });
        let (connect, warp) = buttons.inner;

        match self.running {
            Some(HomeAction::Connect) => {
                ui.put(connect.rect, egui::Spinner::new());
            }
            Some(HomeAction::ConnectWarp) => {
                ui.put(warp.rect, egui::Spinner::new());
            }
            None => {}
        }

        if connect.clicked() {
            self.start_connect(ctx);
        }
        if warp.clicked() {
            self.start_connect_with_warp(ctx);
        }
    }

    /// Persistent, bottom-docked log panel (VS Code "panel" style).
    ///
    /// Rendered BEFORE the sidebar and central panel (see `update`) so it claims
    /// the full window width along the bottom, spanning beneath the sidebar; the
    /// sidebar and the active view then divide the area above it. It stays
    /// anchored to the bottom and is visible on every nav view. The header row
    /// carries a localized "Log" label, the status indicators (login/WARP pills),
    /// a "Stop" control (cancels an in-progress login), "Clear logs", and the
    /// show/hide toggle — so all are reachable regardless of the selected page.
    fn log_panel(&mut self, ctx: &egui::Context) {
        // Cap the panel at roughly half the window height so it never crowds
        // out the view above it (with the min height as a floor on tiny windows).
        let max_height = (ctx.screen_rect().height() * 0.5).max(LOG_PANEL_MIN_HEIGHT);

        let panel = egui::TopBottomPanel::bottom("log_panel").frame(
            egui::Frame::new()
                .fill(BG_PANEL)
                .inner_margin(Margin::same(10)),
        );
        // Only the expanded panel is resizable; collapsed it is just the header
        // strip with no drag handle.
        let panel = if self.log_visible {
            panel
                .resizable(true)
                .default_height(LOG_PANEL_DEFAULT_HEIGHT)
                .min_height(LOG_PANEL_MIN_HEIGHT)
                .max_height(max_height)
        } else {
            panel.resizable(false)
        };

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                field_label(ui, self.tr("log_label"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show/hide toggle: chevron points down to hide, up to show.
                    let (glyph, tip) = if self.log_visible {
                        ("\u{25BC}", self.tr("log_panel_hide"))
                    } else {
                        ("\u{25B2}", self.tr("log_panel_show"))
                    };
                    let toggle =
                        egui::Button::new(RichText::new(glyph).size(12.0).color(TEXT_MUTED))
                            .fill(BTN_NEUTRAL)
                            .corner_radius(CornerRadius::same(6));
                    if ui.add(toggle).on_hover_text(tip).clicked() {
                        self.log_visible = !self.log_visible;
                    }
                    ui.add_space(6.0);
                    let clear = egui::Button::new(
                        RichText::new(self.tr("clear_logs_button"))
                            .size(13.0)
                            .color(TEXT_MUTED),
                    )
                    .fill(BTN_NEUTRAL)
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(clear).clicked() {
                        self.log.clear();
                    }
                    ui.add_space(6.0);
                    // Stop control: cancels an in-progress login. Sits to the
                    // left of "Clear logs" (added after it in this right-to-left
                    // layout). Red while a worker runs, disabled when idle.
                    let stop = egui::Button::new(
                        RichText::new(self.tr("button_stop"))
                            .size(13.0)
                            .color(Color32::WHITE),
                    )
                    .fill(if self.is_busy {
                        STATUS_BAD
                    } else {
                        BTN_NEUTRAL
                    })
                    .corner_radius(CornerRadius::same(6));
                    if ui.add_enabled(self.is_busy, stop).clicked() {
                        self.stop_login();
                    }

                    // Status indicators, lined up to the left of "Stop". Added
                    // after Stop in this right-to-left layout, so the visual
                    // left-to-right order is: login pill, WARP pill, then
                    // Stop / Clear / toggle. (The busy spinner lives on the
                    // active Connect button on the Home view, not here.)
                    let connected = self.tr("state_connected");
                    let disconnected = self.tr("state_disconnected");
                    ui.add_space(8.0);
                    let warp_state = if self.warp_ok {
                        connected
                    } else {
                        disconnected
                    };
                    status_pill(ui, self.tr("pill_warp"), warp_state, self.warp_ok);
                    ui.add_space(8.0);
                    let login_state = if self.login_ok {
                        connected
                    } else {
                        disconnected
                    };
                    status_pill(ui, self.tr("pill_login"), login_state, self.login_ok);
                });
            });

            if self.log_visible {
                ui.add_space(6.0);
                self.show_log(ui);
            }
        });
    }

    /// Read-only, auto-scrolling log area that fills the remaining space.
    fn show_log(&self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(BG_FIELD)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        // Read-only view; `&str` is a no-alloc TextBuffer.
                        let mut shown: &str = if self.log.is_empty() {
                            i18n::t(self.language, "log_empty")
                        } else {
                            self.log.as_str()
                        };
                        ui.add(
                            egui::TextEdit::multiline(&mut shown)
                                .frame(false)
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
    }

    /// Settings page: credential fields, the language selector, and save.
    fn page_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        section_title(ui, self.tr("nav_settings"));
        ui.add_space(10.0);

        // Render the active alert (if any) and clear it once it dismisses.
        let mut clear_alert = false;
        if let Some(alert) = &self.alert {
            if alert.show(ui, ctx) {
                clear_alert = true;
            }
            ui.add_space(12.0);
        }
        if clear_alert {
            self.alert = None;
        }

        ui.label(
            RichText::new(self.tr("settings_credentials_note"))
                .size(14.0)
                .color(TEXT_MUTED),
        );
        ui.add_space(18.0);

        field_label(ui, self.tr("label_tc"));
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.tc)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(10, 8)),
        );

        ui.add_space(14.0);

        field_label(ui, self.tr("label_password"));
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.password)
                .password(true)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(10, 8)),
        );

        ui.add_space(14.0);

        field_label(ui, self.tr("label_language"));
        ui.add_space(4.0);
        self.language_selector(ui, ctx);

        ui.add_space(18.0);
        // "Start with Windows" toggle. The checkbox state is the source of truth
        // for the click; on change we apply it to the registry and persist it.
        let mut autostart = self.autostart;
        if ui
            .checkbox(&mut autostart, self.tr("label_autostart"))
            .changed()
        {
            self.set_autostart(autostart);
        }
        ui.add_space(2.0);
        ui.label(
            RichText::new(self.tr("settings_autostart_note"))
                .size(12.0)
                .color(TEXT_MUTED),
        );

        ui.add_space(20.0);
        if secondary_button(ui, self.tr("button_save")).clicked() {
            self.save_credentials();
        }
    }

    /// Language dropdown. On change, persists the choice and updates the window
    /// title immediately; the rest of the UI re-resolves on the next frame.
    fn language_selector(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let previous = self.language;
        egui::ComboBox::from_id_salt("language_selector")
            .selected_text(self.language.native_name())
            .show_ui(ui, |ui| {
                for language in Language::ALL {
                    ui.selectable_value(&mut self.language, language, language.native_name());
                }
            });

        if self.language != previous {
            // Preserve the credentials on disk; only the language changes.
            let _ = core::save_language(self.language.code());
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                i18n::t(self.language, "window_title").to_string(),
            ));
        }
    }

    /// About page: name, version, description, and the WARP check action.
    fn page_about(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        section_title(ui, self.tr("nav_about"));
        ui.add_space(10.0);

        ui.label(
            RichText::new(APP_NAME)
                .size(22.0)
                .color(TEXT_PRIMARY)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(
                self.tr("about_version")
                    .replace("{v}", env!("CARGO_PKG_VERSION")),
            )
            .size(14.0)
            .color(TEXT_MUTED),
        );
        ui.add_space(12.0);
        ui.label(
            RichText::new(self.tr("about_description"))
                .size(14.0)
                .color(TEXT_MUTED),
        );

        ui.add_space(20.0);
        ui.add_enabled_ui(!self.is_busy, |ui| {
            if secondary_button(ui, self.tr("button_warp_check")).clicked() {
                self.start_warp_check(ctx);
            }
        });
    }
}

impl eframe::App for GsbWifiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Paint the window background with our main color (no flash of white).
        let [r, g, b, a] = BG_MAIN.to_array();
        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // A boot-time (`--autostart`) launch starts minimized. Issue the command
        // once on the first frame, then clear the flag so we never repeat it.
        if self.pending_minimize {
            self.pending_minimize = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        // Drain any messages the worker queued since the last frame. This is
        // cheap when the channel is empty and is the only per-frame work.
        self.drain_messages();

        // Bottom log panel FIRST: declaring it before the sidebar lets it claim
        // the full-width bottom strip (spanning beneath the sidebar, VS Code
        // style); the sidebar and central panel then divide the area above it.
        self.log_panel(ctx);

        // Left sidebar: app identity at the top, vertical nav below.
        egui::SidePanel::left("sidebar")
            .exact_width(180.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(BG_SIDEBAR)
                    .inner_margin(Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(APP_NAME)
                        .size(18.0)
                        .color(TEXT_PRIMARY)
                        .strong(),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(self.tr("sidebar_subtitle"))
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
                ui.add_space(18.0);

                for (nav, icon, key) in [
                    (Nav::Home, "\u{1F3E0}", "nav_home"),
                    (Nav::Settings, "\u{2699}", "nav_settings"),
                    (Nav::About, "\u{2139}", "nav_about"),
                ] {
                    if nav_item(ui, self.nav == nav, icon, self.tr(key)).clicked() {
                        self.nav = nav;
                    }
                    ui.add_space(4.0);
                }
            });

        // Central panel LAST: it fills whatever remains above the bottom panel
        // and to the right of the sidebar. Its content is wrapped in a vertical
        // scroll area so that on short windows (or with a tall log panel) the
        // status text, buttons, and indicators stay reachable by scrolling
        // instead of being clipped or colliding with the log panel. The
        // scrollbar appears only when the content overflows.
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG_MAIN)
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.nav {
                        Nav::Home => self.page_home(ui, ctx),
                        Nav::Settings => self.page_settings(ui, ctx),
                        Nav::About => self.page_about(ui, ctx),
                    });
            });
    }
}

// ---------------------------------------------------------------------------
// Fonts.
// ---------------------------------------------------------------------------

/// Installs the bundled UI font so Turkish glyphs always render.
///
/// egui's built-in proportional font does not reliably cover the Turkish Latin
/// letters, so we embed a font with full coverage and make it the default
/// proportional family. It is also appended as a fallback to the monospace
/// family, so Turkish text in the (monospaced) log renders while the default
/// monospaced font keeps column alignment.
fn apply_fonts(ctx: &egui::Context) {
    use egui::FontFamily::{Monospace, Proportional};

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "ui_font".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/ui-font.ttf"
        ))),
    );
    fonts
        .families
        .entry(Proportional)
        .or_default()
        .insert(0, "ui_font".to_owned());
    fonts
        .families
        .entry(Monospace)
        .or_default()
        .push("ui_font".to_owned());

    ctx.set_fonts(fonts);
}

// ---------------------------------------------------------------------------
// Theme and reusable drawing helpers.
// ---------------------------------------------------------------------------

/// Applies the centralized dark theme (colors, spacing, text sizes) to `ctx`.
///
/// This is the single place to retune the look: every color comes from the
/// `BG_*` / `TEXT_*` / `BLUE` / `STATUS_*` constants near the top of the file.
fn apply_theme(ctx: &egui::Context) {
    use egui::FontFamily::Proportional;
    use egui::TextStyle::{Body, Button, Heading, Monospace, Small};

    let mut style = (*ctx.style()).clone();

    // Type hierarchy: large status headings, comfortable body, small captions.
    style.text_styles = [
        (Heading, FontId::new(26.0, Proportional)),
        (Body, FontId::new(15.0, Proportional)),
        (Button, FontId::new(15.0, Proportional)),
        (Small, FontId::new(12.0, Proportional)),
        (Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
    ]
    .into();

    // Generous, Cloudflare-like spacing instead of the cramped default.
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.interact_size.y = 34.0;

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG_MAIN;
    v.window_fill = BG_PANEL;
    v.faint_bg_color = BG_PANEL;
    v.extreme_bg_color = BG_FIELD;
    v.hyperlink_color = BLUE;
    v.selection.bg_fill = BLUE.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, TEXT_PRIMARY);

    // Default (non-interactive) text and surfaces.
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    // Buttons and other inactive interactive widgets.
    v.widgets.inactive.bg_fill = BTN_NEUTRAL;
    v.widgets.inactive.weak_bg_fill = BTN_NEUTRAL;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = CornerRadius::same(8);
    v.widgets.inactive.bg_stroke = Stroke::NONE;

    v.widgets.hovered.bg_fill = BTN_NEUTRAL_HOVER;
    v.widgets.hovered.weak_bg_fill = BTN_NEUTRAL_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.hovered.corner_radius = CornerRadius::same(8);
    v.widgets.hovered.bg_stroke = Stroke::NONE;

    v.widgets.active.bg_fill = BTN_NEUTRAL_HOVER;
    v.widgets.active.weak_bg_fill = BTN_NEUTRAL_HOVER;
    v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.active.corner_radius = CornerRadius::same(8);
    v.widgets.active.bg_stroke = Stroke::NONE;

    style.visuals = v;
    ctx.set_style(style);
}

/// Draws a section title (the page heading inside the central panel).
fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).color(TEXT_MUTED).strong());
}

/// Draws a small caption label above a form field.
fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(TEXT_MUTED));
}

/// A prominent, tall blue primary button (Cloudflare "Connect" style).
fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let fill = if ui.is_enabled() {
        BLUE
    } else {
        BLUE.gamma_multiply(0.5)
    };
    let label = RichText::new(text)
        .color(Color32::WHITE)
        .size(15.0)
        .strong();
    let button = egui::Button::new(label)
        .fill(fill)
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(0.0, 40.0));
    ui.add_sized([220.0_f32.min(ui.available_width()), 40.0], button)
}

/// A neutral, less prominent secondary button.
fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let label = RichText::new(text).color(TEXT_PRIMARY).size(15.0);
    let button = egui::Button::new(label)
        .fill(BTN_NEUTRAL)
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(0.0, 40.0));
    ui.add_sized([220.0_f32.min(ui.available_width()), 40.0], button)
}

/// Draws one sidebar nav row (icon + label) and returns its click response.
///
/// The selected row gets a highlighted background and brighter text; hovered
/// rows get a subtler highlight; otherwise the text is muted gray.
fn nav_item(ui: &mut egui::Ui, selected: bool, icon: &str, label: &str) -> egui::Response {
    let size = egui::vec2(ui.available_width(), 36.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if selected || response.hovered() {
        let fill = if selected { NAV_SELECTED } else { NAV_HOVER };
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    }

    let text_color = if selected { TEXT_PRIMARY } else { TEXT_MUTED };
    let painter = ui.painter();
    let icon_pos = egui::pos2(rect.left() + 12.0, rect.center().y);
    painter.text(
        icon_pos,
        egui::Align2::LEFT_CENTER,
        icon,
        FontId::proportional(15.0),
        text_color,
    );
    let label_pos = egui::pos2(rect.left() + 38.0, rect.center().y);
    painter.text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(15.0),
        text_color,
    );

    response
}

/// Draws a Cloudflare-style status pill: a rounded tinted capsule with a
/// colored dot, a label, and a localized state word (green = ok, red = not).
fn status_pill(ui: &mut egui::Ui, label: &str, state: &str, ok: bool) {
    let color = if ok { STATUS_OK } else { STATUS_BAD };
    let text = format!("{label}: {state}");

    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(text, font, color);

    let pad_x = 12.0;
    let dot_r = 4.0;
    let gap = 8.0;
    let height = 26.0;
    let width = pad_x + dot_r * 2.0 + gap + galley.size().x + pad_x;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(13), color.gamma_multiply(0.16));

    let cy = rect.center().y;
    let dot_center = egui::pos2(rect.left() + pad_x + dot_r, cy);
    painter.circle_filled(dot_center, dot_r, color);

    let text_pos = egui::pos2(dot_center.x + dot_r + gap, cy - galley.size().y / 2.0);
    painter.galley(text_pos, galley, color);
}

fn main() -> eframe::Result<()> {
    // Load persisted config (credentials + language) before building the window
    // so the initial title is already in the chosen language.
    let config = core::load_config();
    let language = Language::from_code(&config.language_code).unwrap_or_default();
    // If the stored code was missing or invalid, persist the resolved default so
    // the file is consistent on the next launch.
    if Language::from_code(&config.language_code).is_none() {
        let _ = core::save_language(language.code());
    }
    let title = i18n::t(language, "window_title");
    let creds = config.creds;

    // Reflect the real registry Run key so the checkbox shows reality even if the
    // user added or removed the entry manually (e.g. via Task Manager). On
    // non-Windows builds `is_enabled` is always false, so the stored value wins.
    #[cfg(windows)]
    let autostart = autostart::is_enabled();
    #[cfg(not(windows))]
    let autostart = config.autostart;
    // Keep the on-disk preference consistent with the actual state.
    if autostart != config.autostart {
        let _ = core::save_autostart(autostart);
    }

    // A boot-time launch passes `--autostart`; start minimized so it is not
    // intrusive. The existing login retry/backoff handles the network not being
    // up yet, so no extra logic is needed here.
    let start_minimized = std::env::args().any(|arg| arg == autostart::AUTOSTART_FLAG);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 640.0])
            .with_min_inner_size([600.0, 520.0])
            .with_title(title),
        ..Default::default()
    };

    eframe::run_native(
        title,
        native_options,
        Box::new(move |cc| {
            apply_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(GsbWifiApp::new(
                creds,
                language,
                autostart,
                start_minimized,
            )))
        }),
    )
}

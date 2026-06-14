//! Reusable alert/toast banner widget (plain egui, no extra crates).
//!
//! An [`Alert`] is a small dismissible banner that any view can draw. The app
//! holds the currently active alert in `Option<Alert>` state, sets it from
//! anywhere, and renders it where it makes sense (for example at the top of a
//! page). Callers pass an already-localized message string (resolved through the
//! `i18n` module), so the widget itself stays language-agnostic.
//!
//! Performance: an auto-dismissing alert does NOT switch the app to continuous
//! per-frame repainting. Instead [`Alert::show`] issues a single
//! `request_repaint_after(remaining)` so egui wakes up exactly once when the
//! banner is due to disappear. While the alert is idle, nothing repaints.

use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, CornerRadius, Margin, RichText, Stroke};

/// Readable near-white text used for the banner message.
const ALERT_TEXT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF3);

/// Severity of an [`Alert`], selecting its accent (background/border) color.
///
/// This is the full public API of the reusable widget; not every variant is
/// constructed by the app yet, so dead-code analysis is silenced here.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AlertKind {
    Info,
    Success,
    Warning,
    Error,
}

impl AlertKind {
    /// Accent color used for the banner border and the tinted background.
    fn accent(self) -> Color32 {
        match self {
            AlertKind::Info => Color32::from_rgb(0x2F, 0x7E, 0xF0),
            AlertKind::Success => Color32::from_rgb(0x37, 0xB6, 0x5A),
            AlertKind::Warning => Color32::from_rgb(0xE0, 0x8A, 0x1E),
            AlertKind::Error => Color32::from_rgb(0xE5, 0x53, 0x53),
        }
    }
}

/// A dismissible banner. Build it with [`Alert::new`], store it in app state as
/// `Option<Alert>`, and draw it with [`Alert::show`].
pub struct Alert {
    /// Localized message text (resolved by the caller via `i18n`).
    message: String,
    /// Severity, selecting the color scheme.
    kind: AlertKind,
    /// When the alert was created (for the auto-dismiss countdown).
    created: Instant,
    /// Optional time after which the alert removes itself.
    auto_dismiss: Option<Duration>,
}

impl Alert {
    /// Creates an alert. Pass `auto_dismiss = Some(duration)` for a short-lived
    /// banner, or `None` for one that stays until the user closes it.
    pub fn new(
        message: impl Into<String>,
        kind: AlertKind,
        auto_dismiss: Option<Duration>,
    ) -> Self {
        Self {
            message: message.into(),
            kind,
            created: Instant::now(),
            auto_dismiss,
        }
    }

    /// True once an auto-dismiss duration has elapsed.
    fn is_expired(&self) -> bool {
        match self.auto_dismiss {
            Some(d) => self.created.elapsed() >= d,
            None => false,
        }
    }

    /// Draws the banner and returns `true` when it should be removed (the user
    /// clicked the close button, or the auto-dismiss time elapsed). The caller
    /// is responsible for clearing its `Option<Alert>` on a `true` return.
    ///
    /// For an auto-dismissing alert this schedules a single
    /// `request_repaint_after(remaining)` so the banner disappears on time
    /// without continuous repainting.
    pub fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        if self.is_expired() {
            return true;
        }

        let accent = self.kind.accent();
        let mut dismissed = false;

        egui::Frame::new()
            .fill(accent.gamma_multiply(0.16))
            .stroke(Stroke::new(1.0, accent))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Close ("x") button on the right; message fills the rest.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close =
                            egui::Button::new(RichText::new("x").size(15.0).color(accent).strong())
                                .frame(false);
                        if ui.add(close).clicked() {
                            dismissed = true;
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new(&self.message).size(14.0).color(ALERT_TEXT));
                        });
                    });
                });
            });

        // Wake the UI exactly once when the alert is due to expire. This keeps
        // the app event-driven: no unconditional per-frame repaint.
        if let Some(d) = self.auto_dismiss {
            let remaining = d.saturating_sub(self.created.elapsed());
            ctx.request_repaint_after(remaining);
        }

        dismissed
    }
}

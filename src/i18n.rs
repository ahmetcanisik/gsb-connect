//! Localization (Turkish + English).
//!
//! This module is self-contained and is the SINGLE place in the codebase that
//! is allowed to contain Turkish text. That text appears only as string VALUES
//! in the Turkish translation table (`tr`) and in the native language names.
//! Every identifier, key, comment, and the English table is plain ASCII.
//!
//! Widgets never hardcode display text: they call [`t`] with an English ASCII
//! key and render whatever the active [`Language`] resolves it to. Strings are
//! re-resolved every frame, so switching the language updates the UI live.
//!
//! Dynamic messages use brace tokens (for example `{n}`, `{e}`, `{path}`) that
//! the caller fills in with `str::replace`, so each language can place the value
//! wherever its grammar needs it.

/// The languages the app can display in. Turkish is the product default
/// (`#[default]`), shown on first run before any choice is persisted.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Language {
    #[default]
    Turkish,
    English,
}

impl Language {
    /// Every selectable language, in the order shown in the selector.
    pub const ALL: [Language; 2] = [Language::Turkish, Language::English];

    /// Stable, persisted code written to the config file (`tr` / `en`).
    pub fn code(self) -> &'static str {
        match self {
            Language::Turkish => "tr",
            Language::English => "en",
        }
    }

    /// Parses a persisted code back into a `Language`, case-insensitively.
    /// Returns `None` for unknown or empty input so the caller can fall back.
    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "tr" => Some(Language::Turkish),
            "en" => Some(Language::English),
            _ => None,
        }
    }

    /// Native name shown in the language selector (its own language's spelling).
    pub fn native_name(self) -> &'static str {
        match self {
            Language::Turkish => "Türkçe",
            Language::English => "English",
        }
    }
}

/// Resolves an English ASCII `key` to display text for the active `language`.
///
/// Falls back to the English value, then to the key itself, if a translation is
/// missing, so the UI never shows an empty string.
pub fn t(language: Language, key: &'static str) -> &'static str {
    let primary = match language {
        Language::Turkish => tr(key),
        Language::English => en(key),
    };
    primary.or_else(|| en(key)).unwrap_or(key)
}

/// English translation table.
fn en(key: &str) -> Option<&'static str> {
    let value = match key {
        // Shell / navigation
        "window_title" => "GSB Connect",
        "sidebar_subtitle" => "Login client",
        "nav_home" => "Home",
        "nav_settings" => "Settings",
        "nav_about" => "About",

        // Home
        "home_headline_connected" => "Connected",
        "home_headline_disconnected" => "Disconnected",
        "home_subtitle_connected" => "You are logged in to the GSB WiFi portal.",
        "home_subtitle_disconnected" => "Not logged in yet. Use Connect to sign in.",
        "button_connect" => "Connect",
        "button_connect_warp" => "Connect + WARP",
        "button_stop" => "Stop",
        "pill_login" => "Login",
        "pill_warp" => "WARP",
        "state_connected" => "Connected",
        "state_disconnected" => "Disconnected",
        "working" => "Working...",
        "log_heading" => "Logs",
        "log_empty" => "No activity yet.",
        "clear_logs_button" => "Clear logs",
        // Bottom-docked log panel (VS Code "panel" style)
        "log_label" => "Log",
        "log_panel_hide" => "Hide log panel",
        "log_panel_show" => "Show log panel",

        // Settings
        "settings_credentials_note" => {
            "Credentials are stored in a per-user config file (encrypted with Windows DPAPI on \
             Windows)."
        }
        "label_tc" => "TC ID",
        "label_password" => "Password",
        "button_save" => "Save credentials",
        "label_language" => "Language",
        "label_autostart" => "Start with Windows",
        "settings_autostart_note" => {
            "Launch GSB Connect automatically when you sign in to Windows (it starts minimized)."
        }

        // About
        "about_version" => "Version {v}",
        "about_description" => {
            "Automates logging in to the GSB WiFi captive portal with your own credentials, with \
             an optional step to enable Cloudflare WARP."
        }
        "button_warp_check" => "Check / install WARP",

        // Streamed log lines
        "log_active_tc" => "Active user TC: {tc}",
        "log_starting_login" => "Starting login...",
        "log_starting_login_warp" => "Starting login, then WARP...",
        "log_checking_warp" => "Checking Cloudflare WARP...",
        "log_credentials_empty" => "Error: TC ID and password must not be empty.",
        "log_credentials_empty_hint" => "Please enter your TC ID and password in the Settings tab.",
        "log_credentials_saved" => "Credentials saved to {path}.",
        "log_credentials_save_failed" => "Failed to save credentials: {e}",
        "log_autostart_enabled" => "Start with Windows: enabled.",
        "log_autostart_disabled" => "Start with Windows: disabled.",
        "log_autostart_failed" => "Could not update start-with-Windows: {e}",
        "log_http_client_failed" => "Failed to build HTTP client: {e}",
        "log_logged_in" => "Logged in on attempt {n}.",
        "log_not_confirmed" => "Attempt {n} failed: server responded but login not confirmed...",
        "log_timeout" => "Attempt {n}: timeout, retrying...",
        "log_network_error" => "Attempt {n}: network error, retrying...",
        "log_giving_up" => "Giving up after {n} attempts.",
        "log_login_cancelled" => "Login cancelled.",
        "log_warp_connected" => "Traffic is now tunneled through Cloudflare.",
        "log_warp_connect_failed" => "WARP connect failed: {e}",
        "log_warp_run_failed" => "Failed to run warp-cli connect: {e}",
        "log_warp_not_found" => "Cloudflare WARP (warp-cli) not found. Install it with:",
        "log_warp_followup" => "After installing, open the WARP app once, then try again.",

        // Alerts
        "alert_fill_credentials" => "Enter your TC ID and password to continue.",

        _ => return None,
    };
    Some(value)
}

/// Turkish translation table. This is the only function permitted to contain
/// Turkish characters, and only inside the string values.
fn tr(key: &str) -> Option<&'static str> {
    let value = match key {
        // Shell / navigation
        "window_title" => "GSB Connect",
        "sidebar_subtitle" => "Giriş istemcisi",
        "nav_home" => "Ana Sayfa",
        "nav_settings" => "Ayarlar",
        "nav_about" => "Hakkında",

        // Home
        "home_headline_connected" => "Bağlandı",
        "home_headline_disconnected" => "Bağlantı yok",
        "home_subtitle_connected" => "GSB WiFi portalına giriş yapıldı.",
        "home_subtitle_disconnected" => "Henüz giriş yapılmadı. Giriş için Bağlan'a basın.",
        "button_connect" => "Bağlan",
        "button_connect_warp" => "Bağlan + WARP",
        "button_stop" => "Durdur",
        "pill_login" => "Giriş",
        "pill_warp" => "WARP",
        "state_connected" => "Bağlı",
        "state_disconnected" => "Bağlı değil",
        "working" => "Çalışılıyor...",
        "log_heading" => "Günlük",
        "log_empty" => "Henüz işlem yok.",
        "clear_logs_button" => "Günlüğü temizle",
        // Bottom-docked log panel (VS Code "panel" style)
        "log_label" => "Günlük",
        "log_panel_hide" => "Günlük panelini gizle",
        "log_panel_show" => "Günlük panelini göster",

        // Settings
        "settings_credentials_note" => {
            "Kimlik bilgileri kullanıcıya özel bir yapılandırma dosyasında saklanır (Windows'ta \
             Windows DPAPI ile şifrelenir)."
        }
        "label_tc" => "TC Kimlik",
        "label_password" => "Şifre",
        "button_save" => "Kimlik bilgilerini kaydet",
        "label_language" => "Dil",
        "label_autostart" => "Windows ile başlat",
        "settings_autostart_note" => {
            "Windows oturumu açtığınızda GSB Connect otomatik olarak (simge durumunda) başlasın."
        }

        // About
        "about_version" => "Sürüm {v}",
        "about_description" => {
            "GSB WiFi oturum açma portalına kendi kimlik bilgilerinizle giriş yapmayı \
             otomatikleştirir; isteğe bağlı olarak Cloudflare WARP'ı etkinleştirebilir."
        }
        "button_warp_check" => "WARP'ı denetle / kur",

        // Streamed log lines
        "log_active_tc" => "Etkin kullanıcı TC: {tc}",
        "log_starting_login" => "Giriş başlatılıyor...",
        "log_starting_login_warp" => "Önce giriş, ardından WARP başlatılıyor...",
        "log_checking_warp" => "Cloudflare WARP denetleniyor...",
        "log_credentials_empty" => "Hata: TC Kimlik ve şifre boş olamaz.",
        "log_credentials_empty_hint" => "Lütfen Ayarlar sekmesinde TC Kimlik ve şifrenizi girin.",
        "log_credentials_saved" => "Kimlik bilgileri şuraya kaydedildi: {path}.",
        "log_credentials_save_failed" => "Kimlik bilgileri kaydedilemedi: {e}",
        "log_autostart_enabled" => "Windows ile başlatma: etkin.",
        "log_autostart_disabled" => "Windows ile başlatma: devre dışı.",
        "log_autostart_failed" => "Windows ile başlatma güncellenemedi: {e}",
        "log_http_client_failed" => "HTTP istemcisi oluşturulamadı: {e}",
        "log_logged_in" => "{n}. denemede giriş yapıldı.",
        "log_not_confirmed" => {
            "{n}. deneme başarısız: sunucu yanıt verdi ama giriş doğrulanmadı..."
        }
        "log_timeout" => "{n}. deneme: zaman aşımı, yeniden deneniyor...",
        "log_network_error" => "{n}. deneme: ağ hatası, yeniden deneniyor...",
        "log_giving_up" => "{n} denemeden sonra vazgeçiliyor.",
        "log_login_cancelled" => "Giriş iptal edildi.",
        "log_warp_connected" => "Trafik artık Cloudflare üzerinden tünelleniyor.",
        "log_warp_connect_failed" => "WARP bağlantısı başarısız: {e}",
        "log_warp_run_failed" => "warp-cli connect çalıştırılamadı: {e}",
        "log_warp_not_found" => "Cloudflare WARP (warp-cli) bulunamadı. Şunu kullanarak kurun:",
        "log_warp_followup" => {
            "Kurduktan sonra WARP uygulamasını bir kez açın, ardından tekrar deneyin."
        }

        // Alerts
        "alert_fill_credentials" => "Devam etmek için TC Kimlik ve şifrenizi girin.",

        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_round_trips() {
        for lang in Language::ALL {
            assert_eq!(Language::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Language::from_code("EN"), Some(Language::English));
        assert_eq!(Language::from_code("xx"), None);
        assert_eq!(Language::from_code(""), None);
    }

    #[test]
    fn default_is_turkish() {
        assert_eq!(Language::default(), Language::Turkish);
    }

    #[test]
    fn every_english_key_has_turkish() {
        // Keys exercised by the UI. If a key is added to `en` it must also be
        // added here and to `tr`, so the two tables stay in lockstep.
        for key in KEYS {
            assert!(en(key).is_some(), "missing English value for {key}");
            assert!(tr(key).is_some(), "missing Turkish value for {key}");
        }
    }

    /// All translation keys used by the application.
    const KEYS: &[&str] = &[
        "window_title",
        "sidebar_subtitle",
        "nav_home",
        "nav_settings",
        "nav_about",
        "home_headline_connected",
        "home_headline_disconnected",
        "home_subtitle_connected",
        "home_subtitle_disconnected",
        "button_connect",
        "button_connect_warp",
        "button_stop",
        "pill_login",
        "pill_warp",
        "state_connected",
        "state_disconnected",
        "working",
        "log_heading",
        "log_empty",
        "clear_logs_button",
        "log_label",
        "log_panel_hide",
        "log_panel_show",
        "settings_credentials_note",
        "label_tc",
        "label_password",
        "button_save",
        "label_language",
        "label_autostart",
        "settings_autostart_note",
        "about_version",
        "about_description",
        "button_warp_check",
        "log_active_tc",
        "log_starting_login",
        "log_starting_login_warp",
        "log_checking_warp",
        "log_credentials_empty",
        "log_credentials_empty_hint",
        "log_credentials_saved",
        "log_credentials_save_failed",
        "log_autostart_enabled",
        "log_autostart_disabled",
        "log_autostart_failed",
        "log_http_client_failed",
        "log_logged_in",
        "log_not_confirmed",
        "log_timeout",
        "log_network_error",
        "log_giving_up",
        "log_login_cancelled",
        "log_warp_connected",
        "log_warp_connect_failed",
        "log_warp_run_failed",
        "log_warp_not_found",
        "log_warp_followup",
        "alert_fill_credentials",
    ];
}

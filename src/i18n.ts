//! Localization (Turkish + English), ported from the original Rust `i18n`
//! module. This is the single place that holds Turkish text. Components never
//! hardcode display strings; they call `t(key)` and render the result, so
//! switching the language updates the whole UI live.
//!
//! Dynamic log lines use brace tokens (`{n}`, `{e}`, `{tc}`, `{path}`, `{v}`)
//! filled in by the caller, matching the Rust `LogMsg` -> string mapping.

import type { LogMsg } from "./api";

export type Lang = "tr" | "en";

export const LANGUAGES: { code: Lang; nativeName: string }[] = [
  { code: "tr", nativeName: "Türkçe" },
  { code: "en", nativeName: "English" },
];

const en: Record<string, string> = {
  window_title: "GSB Connect",
  sidebar_subtitle: "Login client",
  nav_home: "Home",
  nav_settings: "Settings",
  nav_about: "About",

  home_headline_connected: "Connected",
  home_headline_disconnected: "Disconnected",
  home_subtitle_connected: "You are logged in to the GSB WiFi portal.",
  home_subtitle_disconnected: "Not logged in yet. Use Connect to sign in.",
  button_connect: "Connect",
  button_connect_warp: "Connect + WARP",
  button_stop: "Stop",
  pill_login: "Login",
  pill_warp: "WARP",
  state_connected: "Connected",
  state_disconnected: "Disconnected",
  working: "Working...",
  log_heading: "Logs",
  log_empty: "No activity yet.",
  clear_logs_button: "Clear logs",
  log_label: "Log",
  log_panel_hide: "Hide log panel",
  log_panel_show: "Show log panel",
  drawer_open: "Open menu",
  drawer_close: "Close menu",

  settings_credentials_note:
    "Credentials are stored in a per-user config file (encrypted with Windows DPAPI on Windows).",
  label_tc: "TC ID",
  label_password: "Password",
  button_save: "Save credentials",
  label_language: "Language",
  label_autostart: "Start with Windows",
  settings_autostart_note:
    "Launch GSB Connect automatically when you sign in to Windows (it starts minimized).",

  about_version: "Version {v}",
  about_description:
    "Automates logging in to the GSB WiFi captive portal with your own credentials, with an optional step to enable Cloudflare WARP.",
  button_warp_check: "Check / install WARP",

  log_active_tc: "Active user TC: {tc}",
  log_starting_login: "Starting login...",
  log_starting_login_warp: "Starting login, then WARP...",
  log_checking_warp: "Checking Cloudflare WARP...",
  log_credentials_empty: "Error: TC ID and password must not be empty.",
  log_credentials_empty_hint:
    "Please enter your TC ID and password in the Settings tab.",
  log_credentials_saved: "Credentials saved to {path}.",
  log_credentials_save_failed: "Failed to save credentials: {e}",
  log_autostart_enabled: "Start with Windows: enabled.",
  log_autostart_disabled: "Start with Windows: disabled.",
  log_autostart_failed: "Could not update start-with-Windows: {e}",
  log_http_client_failed: "Failed to build HTTP client: {e}",
  log_logged_in: "Logged in on attempt {n}.",
  log_not_confirmed:
    "Attempt {n} failed: server responded but login not confirmed...",
  log_timeout: "Attempt {n}: timeout, retrying...",
  log_network_error: "Attempt {n}: network error, retrying...",
  log_giving_up: "Giving up after {n} attempts.",
  log_login_cancelled: "Login cancelled.",
  log_warp_connected: "Traffic is now tunneled through Cloudflare.",
  log_warp_connect_failed: "WARP connect failed: {e}",
  log_warp_run_failed: "Failed to run warp-cli connect: {e}",
  log_warp_not_found: "Cloudflare WARP (warp-cli) not found. Install it with:",
  log_warp_followup: "After installing, open the WARP app once, then try again.",

  alert_fill_credentials: "Enter your TC ID and password to continue.",
  alert_credentials_saved: "Credentials saved.",
};

const tr: Record<string, string> = {
  window_title: "GSB Connect",
  sidebar_subtitle: "Giriş istemcisi",
  nav_home: "Ana Sayfa",
  nav_settings: "Ayarlar",
  nav_about: "Hakkında",

  home_headline_connected: "Bağlandı",
  home_headline_disconnected: "Bağlantı yok",
  home_subtitle_connected: "GSB WiFi portalına giriş yapıldı.",
  home_subtitle_disconnected:
    "Henüz giriş yapılmadı. Giriş için Bağlan'a basın.",
  button_connect: "Bağlan",
  button_connect_warp: "Bağlan + WARP",
  button_stop: "Durdur",
  pill_login: "Giriş",
  pill_warp: "WARP",
  state_connected: "Bağlı",
  state_disconnected: "Bağlı değil",
  working: "Çalışılıyor...",
  log_heading: "Günlük",
  log_empty: "Henüz işlem yok.",
  clear_logs_button: "Günlüğü temizle",
  log_label: "Günlük",
  log_panel_hide: "Günlük panelini gizle",
  log_panel_show: "Günlük panelini göster",
  drawer_open: "Menüyü aç",
  drawer_close: "Menüyü kapat",

  settings_credentials_note:
    "Kimlik bilgileri kullanıcıya özel bir yapılandırma dosyasında saklanır (Windows'ta Windows DPAPI ile şifrelenir).",
  label_tc: "TC Kimlik",
  label_password: "Şifre",
  button_save: "Kimlik bilgilerini kaydet",
  label_language: "Dil",
  label_autostart: "Windows ile başlat",
  settings_autostart_note:
    "Windows oturumu açtığınızda GSB Connect otomatik olarak (simge durumunda) başlasın.",

  about_version: "Sürüm {v}",
  about_description:
    "GSB WiFi oturum açma portalına kendi kimlik bilgilerinizle giriş yapmayı otomatikleştirir; isteğe bağlı olarak Cloudflare WARP'ı etkinleştirebilir.",
  button_warp_check: "WARP'ı denetle / kur",

  log_active_tc: "Etkin kullanıcı TC: {tc}",
  log_starting_login: "Giriş başlatılıyor...",
  log_starting_login_warp: "Önce giriş, ardından WARP başlatılıyor...",
  log_checking_warp: "Cloudflare WARP denetleniyor...",
  log_credentials_empty: "Hata: TC Kimlik ve şifre boş olamaz.",
  log_credentials_empty_hint:
    "Lütfen Ayarlar sekmesinde TC Kimlik ve şifrenizi girin.",
  log_credentials_saved: "Kimlik bilgileri şuraya kaydedildi: {path}.",
  log_credentials_save_failed: "Kimlik bilgileri kaydedilemedi: {e}",
  log_autostart_enabled: "Windows ile başlatma: etkin.",
  log_autostart_disabled: "Windows ile başlatma: devre dışı.",
  log_autostart_failed: "Windows ile başlatma güncellenemedi: {e}",
  log_http_client_failed: "HTTP istemcisi oluşturulamadı: {e}",
  log_logged_in: "{n}. denemede giriş yapıldı.",
  log_not_confirmed:
    "{n}. deneme başarısız: sunucu yanıt verdi ama giriş doğrulanmadı...",
  log_timeout: "{n}. deneme: zaman aşımı, yeniden deneniyor...",
  log_network_error: "{n}. deneme: ağ hatası, yeniden deneniyor...",
  log_giving_up: "{n} denemeden sonra vazgeçiliyor.",
  log_login_cancelled: "Giriş iptal edildi.",
  log_warp_connected: "Trafik artık Cloudflare üzerinden tünelleniyor.",
  log_warp_connect_failed: "WARP bağlantısı başarısız: {e}",
  log_warp_run_failed: "warp-cli connect çalıştırılamadı: {e}",
  log_warp_not_found: "Cloudflare WARP (warp-cli) bulunamadı. Şunu kullanarak kurun:",
  log_warp_followup:
    "Kurduktan sonra WARP uygulamasını bir kez açın, ardından tekrar deneyin.",

  alert_fill_credentials: "Devam etmek için TC Kimlik ve şifrenizi girin.",
  alert_credentials_saved: "Kimlik bilgileri kaydedildi.",
};

const tables: Record<Lang, Record<string, string>> = { tr, en };

/** Returns a translator bound to `lang`, falling back to English then the key. */
export function makeT(lang: Lang) {
  return (key: string): string => tables[lang][key] ?? en[key] ?? key;
}

/** Masks a TC ID: first three characters, then asterisks (matches Rust). */
export function maskTc(tc: string): string {
  return tc.slice(0, 3) + "********";
}

/** Formats a backend `LogMsg` into a localized line (mirrors `format_log`). */
export function formatLog(msg: LogMsg, t: (k: string) => string): string {
  switch (msg.kind) {
    case "HttpClientBuildFailed":
      return t("log_http_client_failed").replace("{e}", msg.data);
    case "LoggedInOnAttempt":
      return t("log_logged_in").replace("{n}", String(msg.data));
    case "AttemptNotConfirmed":
      return t("log_not_confirmed").replace("{n}", String(msg.data));
    case "AttemptTimeout":
      return t("log_timeout").replace("{n}", String(msg.data));
    case "AttemptNetworkError":
      return t("log_network_error").replace("{n}", String(msg.data));
    case "GivingUp":
      return t("log_giving_up").replace("{n}", String(msg.data));
    case "LoginCancelled":
      return t("log_login_cancelled");
    case "WarpConnected":
      return t("log_warp_connected");
    case "WarpConnectFailed":
      return t("log_warp_connect_failed").replace("{e}", msg.data);
    case "WarpRunFailed":
      return t("log_warp_run_failed").replace("{e}", msg.data);
    case "WarpNotFound":
      return t("log_warp_not_found");
    case "WarpInstallHint":
      return "  " + msg.data;
    case "WarpInstallFollowup":
      return t("log_warp_followup");
  }
}

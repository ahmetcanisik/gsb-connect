import { LANGUAGES, type Lang } from "../i18n";

export function Settings({
  t,
  lang,
  onLanguageChange,
  tc,
  setTc,
  password,
  setPassword,
  autostart,
  mobile,
  onSave,
  onToggleAutostart,
}: {
  t: (k: string) => string;
  lang: Lang;
  onLanguageChange: (lang: Lang) => void;
  tc: string;
  setTc: (v: string) => void;
  password: string;
  setPassword: (v: string) => void;
  autostart: boolean;
  mobile: boolean;
  onSave: () => void;
  onToggleAutostart: (enabled: boolean) => void;
}) {
  return (
    <div>
      <h2 className="section-title">{t("nav_settings")}</h2>
      <p className="note">{t("settings_credentials_note")}</p>

      <div className="spacer-sm" />

      <label className="field-label">{t("label_tc")}</label>
      <input
        type="text"
        value={tc}
        onChange={(e) => setTc(e.target.value)}
        autoComplete="off"
      />

      <div className="spacer-sm" />

      <label className="field-label">{t("label_password")}</label>
      <input
        type="password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        autoComplete="off"
      />

      <div className="spacer-sm" />

      <label className="field-label">{t("label_language")}</label>
      <select
        value={lang}
        onChange={(e) => onLanguageChange(e.target.value as Lang)}
      >
        {LANGUAGES.map((l) => (
          <option key={l.code} value={l.code}>
            {l.nativeName}
          </option>
        ))}
      </select>

      {!mobile && (
        <>
          <div className="spacer-md" />
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={autostart}
              onChange={(e) => onToggleAutostart(e.target.checked)}
            />
            {t("label_autostart")}
          </label>
          <div style={{ height: 2 }} />
          <p className="note small">{t("settings_autostart_note")}</p>
        </>
      )}

      <div className="spacer-md" />
      <button className="btn secondary" onClick={onSave}>
        {t("button_save")}
      </button>
    </div>
  );
}

import { APP_VERSION } from "../App";

export function About({
  t,
  busy,
  mobile,
  onCheckWarp,
}: {
  t: (k: string) => string;
  busy: boolean;
  mobile: boolean;
  onCheckWarp: () => void;
}) {
  return (
    <div>
      <h2 className="section-title">{t("nav_about")}</h2>
      <p className="headline" style={{ fontSize: 22 }}>
        GSB Connect
      </p>
      <p className="subtitle">
        {t("about_version").replace("{v}", APP_VERSION)}
      </p>
      <div className="spacer-sm" />
      <p className="note">{t("about_description")}</p>

      {!mobile && (
        <>
          <div className="spacer-md" />
          <button className="btn secondary" disabled={busy} onClick={onCheckWarp}>
            {t("button_warp_check")}
          </button>
        </>
      )}
    </div>
  );
}

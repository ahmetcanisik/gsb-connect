import type { Running } from "../App";

export function Home({
  t,
  loginOk,
  busy,
  running,
  mobile,
  onConnect,
}: {
  t: (k: string) => string;
  loginOk: boolean;
  busy: boolean;
  running: Running;
  mobile: boolean;
  onConnect: (withWarp: boolean) => void;
}) {
  return (
    <div>
      <h2 className="section-title">{t("nav_home")}</h2>
      <p className="headline">
        {loginOk
          ? t("home_headline_connected")
          : t("home_headline_disconnected")}
      </p>
      <p className="subtitle">
        {loginOk
          ? t("home_subtitle_connected")
          : t("home_subtitle_disconnected")}
      </p>

      <div className="spacer-md" />

      <button
        className="btn primary"
        disabled={busy}
        onClick={() => onConnect(false)}
      >
        {running === "connect" ? <span className="spinner" /> : t("button_connect")}
      </button>

      {!mobile && (
        <>
          <div className="spacer-sm" />
          <button
            className="btn secondary"
            disabled={busy}
            onClick={() => onConnect(true)}
          >
            {running === "connectWarp" ? (
              <span className="spinner" />
            ) : (
              t("button_connect_warp")
            )}
          </button>
        </>
      )}
    </div>
  );
}

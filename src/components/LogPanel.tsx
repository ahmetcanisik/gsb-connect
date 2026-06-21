import { useEffect, useRef } from "react";
import { StatusPill } from "./StatusPill";

/** Stop (filled square) glyph. */
const StopIcon = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden focusable="false">
    <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />
  </svg>
);

/** Trash-can glyph for clearing the log. */
const TrashIcon = () => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden
    focusable="false"
  >
    <path d="M3 6h18" />
    <path d="M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2" />
    <path d="M6 6l1 14a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-14" />
    <path d="M10 11v6M14 11v6" />
  </svg>
);

/** Persistent bottom-docked log panel (VS Code "panel" style). The header row
 *  carries the status pills, Stop, Clear, and the show/hide toggle, so they are
 *  reachable on every page. On mobile the WARP pill is dropped (WARP is desktop
 *  only) and Stop/Clear collapse to icon-only buttons to save width. */
export function LogPanel({
  t,
  lines,
  visible,
  busy,
  loginOk,
  warpOk,
  mobile,
  onToggle,
  onClear,
  onStop,
}: {
  t: (k: string) => string;
  lines: string[];
  visible: boolean;
  busy: boolean;
  loginOk: boolean;
  warpOk: boolean;
  mobile: boolean;
  onToggle: () => void;
  onClear: () => void;
  onStop: () => void;
}) {
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (visible && bodyRef.current) {
      bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
    }
  }, [lines, visible]);

  const connected = t("state_connected");
  const disconnected = t("state_disconnected");

  return (
    <div className={"log-panel" + (visible ? " expanded" : "")}>
      <div className="log-header">
        <span className="title">{t("log_label")}</span>
        <div className="right">
          <StatusPill
            label={t("pill_login")}
            state={loginOk ? connected : disconnected}
            ok={loginOk}
          />
          {!mobile && (
            <StatusPill
              label={t("pill_warp")}
              state={warpOk ? connected : disconnected}
              ok={warpOk}
            />
          )}
          <button
            className={
              "btn-mini" + (busy ? " danger" : "") + (mobile ? " icon" : "")
            }
            disabled={!busy}
            onClick={onStop}
            aria-label={t("button_stop")}
            title={t("button_stop")}
          >
            {mobile ? <StopIcon /> : t("button_stop")}
          </button>
          <button
            className={"btn-mini" + (mobile ? " icon" : "")}
            onClick={onClear}
            aria-label={t("clear_logs_button")}
            title={t("clear_logs_button")}
          >
            {mobile ? <TrashIcon /> : t("clear_logs_button")}
          </button>
          <button
            className="btn-mini"
            onClick={onToggle}
            title={visible ? t("log_panel_hide") : t("log_panel_show")}
          >
            {visible ? "▼" : "▲"}
          </button>
        </div>
      </div>

      {visible && (
        <div
          ref={bodyRef}
          className={"log-body" + (lines.length === 0 ? " empty" : "")}
        >
          {lines.length === 0 ? t("log_empty") : lines.join("\n")}
        </div>
      )}
    </div>
  );
}

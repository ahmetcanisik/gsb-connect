import { useEffect, useRef } from "react";
import { StatusPill } from "./StatusPill";

/** Persistent bottom-docked log panel (VS Code "panel" style). The header row
 *  carries the status pills, Stop, Clear, and the show/hide toggle, so they are
 *  reachable on every page. */
export function LogPanel({
  t,
  lines,
  visible,
  busy,
  loginOk,
  warpOk,
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
          <StatusPill
            label={t("pill_warp")}
            state={warpOk ? connected : disconnected}
            ok={warpOk}
          />
          <button
            className={"btn-mini" + (busy ? " danger" : "")}
            disabled={!busy}
            onClick={onStop}
          >
            {t("button_stop")}
          </button>
          <button className="btn-mini" onClick={onClear}>
            {t("clear_logs_button")}
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

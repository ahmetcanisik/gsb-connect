import { useEffect } from "react";

export type AlertKind = "info" | "success" | "warning" | "error";

const ACCENT: Record<AlertKind, string> = {
  info: "#2f7ef0",
  success: "#37b65a",
  warning: "#e08a1e",
  error: "#e55353",
};

/** Dismissible banner. Optionally auto-dismisses after `autoDismissMs`. */
export function Alert({
  message,
  kind,
  autoDismissMs,
  onClose,
}: {
  message: string;
  kind: AlertKind;
  autoDismissMs?: number;
  onClose: () => void;
}) {
  useEffect(() => {
    if (!autoDismissMs) return;
    const id = setTimeout(onClose, autoDismissMs);
    return () => clearTimeout(id);
  }, [autoDismissMs, onClose]);

  const accent = ACCENT[kind];
  return (
    <div
      className="alert"
      style={{ background: `${accent}29`, border: `1px solid ${accent}` }}
    >
      <span className="msg">{message}</span>
      <button
        className="close"
        style={{ color: accent }}
        onClick={onClose}
        aria-label="close"
      >
        ✕
      </button>
    </div>
  );
}

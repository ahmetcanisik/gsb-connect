/** Cloudflare-style status pill: a tinted capsule with a colored dot, a label,
 *  and a localized state word (green = ok, red = not). */
export function StatusPill({
  label,
  state,
  ok,
}: {
  label: string;
  state: string;
  ok: boolean;
}) {
  const color = ok ? "var(--status-ok)" : "var(--status-bad)";
  return (
    <span
      className="pill"
      style={{ background: `${color}29`, color }}
    >
      <span className="dot" style={{ background: color }} />
      {label}: {state}
    </span>
  );
}

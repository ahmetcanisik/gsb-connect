import type { Nav } from "../App";

const ITEMS: { nav: Nav; icon: string; key: string }[] = [
  { nav: "home", icon: "🏠", key: "nav_home" },
  { nav: "settings", icon: "⚙", key: "nav_settings" },
  { nav: "about", icon: "ℹ", key: "nav_about" },
];

export function Sidebar({
  nav,
  setNav,
  t,
  mobile,
  drawerOpen,
  onToggleDrawer,
}: {
  nav: Nav;
  setNav: (n: Nav) => void;
  t: (k: string) => string;
  mobile: boolean;
  drawerOpen: boolean;
  onToggleDrawer: () => void;
}) {
  return (
    <nav className="sidebar">
      {mobile && (
        <button
          className="rail-menu"
          onClick={onToggleDrawer}
          aria-label={drawerOpen ? t("drawer_close") : t("drawer_open")}
          title={drawerOpen ? t("drawer_close") : t("drawer_open")}
        >
          ☰
        </button>
      )}
      <div className="brand">GSB Connect</div>
      <div className="brand-sub">{t("sidebar_subtitle")}</div>
      {ITEMS.map((item) => (
        <button
          key={item.nav}
          className={"nav-item" + (nav === item.nav ? " selected" : "")}
          onClick={() => setNav(item.nav)}
          aria-label={t(item.key)}
          title={t(item.key)}
        >
          <span className="nav-icon" aria-hidden>
            {item.icon}
          </span>
          <span className="nav-label">{t(item.key)}</span>
        </button>
      ))}
    </nav>
  );
}

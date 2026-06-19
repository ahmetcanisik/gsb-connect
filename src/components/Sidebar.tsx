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
}: {
  nav: Nav;
  setNav: (n: Nav) => void;
  t: (k: string) => string;
}) {
  return (
    <nav className="sidebar">
      <div className="brand">GSB Connect</div>
      <div className="brand-sub">{t("sidebar_subtitle")}</div>
      {ITEMS.map((item) => (
        <button
          key={item.nav}
          className={"nav-item" + (nav === item.nav ? " selected" : "")}
          onClick={() => setNav(item.nav)}
        >
          <span aria-hidden>{item.icon}</span>
          {t(item.key)}
        </button>
      ))}
    </nav>
  );
}

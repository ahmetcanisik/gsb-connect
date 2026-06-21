import { type TouchEvent, useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

import { api, events, isMobile } from "./api";
import { formatLog, makeT, maskTc, type Lang } from "./i18n";
import { Sidebar } from "./components/Sidebar";
import { LogPanel } from "./components/LogPanel";
import { Alert, type AlertKind } from "./components/Alert";
import { Home } from "./pages/Home";
import { Settings } from "./pages/Settings";
import { About } from "./pages/About";

export const APP_VERSION = "2.0.0";

export type Nav = "home" | "settings" | "about";
/** Which Home action is running, so its button shows a spinner (null = idle or
 *  a non-Home action such as the About WARP check). */
export type Running = "connect" | "connectWarp" | null;

interface AlertState {
  message: string;
  kind: AlertKind;
}

export default function App() {
  const [platform, setPlatform] = useState("");
  const mobile = isMobile(platform);

  const [nav, setNav] = useState<Nav>("home");
  const [lang, setLang] = useState<Lang>("tr");
  const [tc, setTc] = useState("");
  const [password, setPassword] = useState("");
  const [autostart, setAutostart] = useState(false);

  const [log, setLog] = useState<string[]>([]);
  const [logVisible, setLogVisible] = useState(true);

  const [busy, setBusy] = useState(false);
  const [running, setRunning] = useState<Running>(null);
  const [loginOk, setLoginOk] = useState(false);
  const [warpOk, setWarpOk] = useState(false);
  const [alert, setAlert] = useState<AlertState | null>(null);

  // Mobile navigation drawer: collapsed to an icon rail by default; a right
  // swipe (or the rail menu button) slides the labeled panel over the content.
  const [drawerOpen, setDrawerOpen] = useState(false);

  const t = makeT(lang);

  // The event listeners are registered once; read the current language through
  // a ref so log lines are formatted in whatever language is active when they
  // arrive (matching the original "format at drain time" behavior).
  const langRef = useRef(lang);
  useEffect(() => {
    langRef.current = lang;
    document.documentElement.lang = lang;
  }, [lang]);

  const appendLog = useCallback((line: string) => {
    setLog((prev) => [...prev, line]);
  }, []);

  // Initial load: platform + persisted config.
  useEffect(() => {
    (async () => {
      try {
        setPlatform(await api.platform());
      } catch {
        /* ignore: leave desktop defaults */
      }
      try {
        const cfg = await api.loadConfig();
        setTc(cfg.tc);
        setPassword(cfg.password);
        if (cfg.language === "tr" || cfg.language === "en") setLang(cfg.language);
        setAutostart(cfg.autostart);
      } catch {
        /* ignore: start with empty config */
      }
    })();
  }, []);

  // Worker events from the Rust background thread.
  useEffect(() => {
    const subs: Promise<UnlistenFn>[] = [
      events.onLog((msg) => appendLog(formatLog(msg, makeT(langRef.current)))),
      events.onLoginResult((ok) => setLoginOk(ok)),
      events.onWarpResult((ok) => setWarpOk(ok)),
      events.onDone(() => {
        setBusy(false);
        setRunning(null);
      }),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()).catch(() => {}));
    };
  }, [appendLog]);

  // ---- Actions ----------------------------------------------------------

  const startConnect = (withWarp: boolean) => {
    const trimmedTc = tc.trim();
    const trimmedPw = password.trim();
    if (!trimmedTc || !trimmedPw) {
      appendLog(t("log_credentials_empty_hint"));
      setNav("settings");
      setAlert({ message: t("alert_fill_credentials"), kind: "warning" });
      return;
    }
    appendLog(t("log_active_tc").replace("{tc}", maskTc(trimmedTc)));
    appendLog(t(withWarp ? "log_starting_login_warp" : "log_starting_login"));
    setRunning(withWarp ? "connectWarp" : "connect");
    setBusy(true);
    api.connect(trimmedTc, trimmedPw, withWarp).catch((e) => {
      appendLog(String(e));
      setBusy(false);
      setRunning(null);
    });
  };

  const stop = () => {
    api.stop().catch(() => {});
  };

  const checkWarp = () => {
    appendLog(t("log_checking_warp"));
    setBusy(true);
    api.checkWarp().catch((e) => {
      appendLog(String(e));
      setBusy(false);
    });
  };

  const saveCredentials = async () => {
    try {
      await api.saveCredentials(tc.trim(), password.trim(), lang);
      setAlert({ message: t("alert_credentials_saved"), kind: "success" });
    } catch (e) {
      appendLog(t("log_credentials_save_failed").replace("{e}", String(e)));
    }
  };

  const changeLanguage = (next: Lang) => {
    setLang(next);
    api.saveLanguage(next).catch(() => {});
  };

  const toggleAutostart = async (enabled: boolean) => {
    try {
      await api.setAutostart(enabled);
      setAutostart(enabled);
      appendLog(t(enabled ? "log_autostart_enabled" : "log_autostart_disabled"));
    } catch (e) {
      // Snap the checkbox back to the real state on failure.
      try {
        setAutostart(await api.getAutostart());
      } catch {
        /* ignore */
      }
      appendLog(t("log_autostart_failed").replace("{e}", String(e)));
    }
  };

  // Navigating (tapping a rail/drawer item) also closes the mobile drawer.
  const navigate = (next: Nav) => {
    setNav(next);
    setDrawerOpen(false);
  };

  // ---- Touch swipe (mobile drawer) --------------------------------------
  // A short right swipe starting near the left edge opens the drawer; a left
  // swipe while it is open closes it. No-op on desktop (mobile === false).
  const touchStart = useRef<{ x: number; y: number } | null>(null);
  const onTouchStart = (e: TouchEvent) => {
    const t0 = e.touches[0];
    touchStart.current = { x: t0.clientX, y: t0.clientY };
  };
  const onTouchEnd = (e: TouchEvent) => {
    const start = touchStart.current;
    touchStart.current = null;
    if (!mobile || !start) return;
    const t1 = e.changedTouches[0];
    const dx = t1.clientX - start.x;
    const dy = t1.clientY - start.y;
    if (Math.abs(dx) < 50 || Math.abs(dx) < Math.abs(dy)) return; // not horizontal
    if (dx > 0 && !drawerOpen && start.x < 48) setDrawerOpen(true);
    else if (dx < 0 && drawerOpen) setDrawerOpen(false);
  };

  // ---- Render -----------------------------------------------------------

  return (
    <div
      className={
        "app" + (mobile ? " mobile" : "") + (drawerOpen ? " drawer-open" : "")
      }
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      <div className="body-row">
        <Sidebar
          nav={nav}
          setNav={navigate}
          t={t}
          mobile={mobile}
          drawerOpen={drawerOpen}
          onToggleDrawer={() => setDrawerOpen((o) => !o)}
        />
        {mobile && drawerOpen && (
          <div className="scrim" onClick={() => setDrawerOpen(false)} />
        )}
        <main className="content">
          {alert && (
            <Alert
              message={alert.message}
              kind={alert.kind}
              autoDismissMs={4000}
              onClose={() => setAlert(null)}
            />
          )}
          {nav === "home" && (
            <Home
              t={t}
              loginOk={loginOk}
              busy={busy}
              running={running}
              mobile={mobile}
              onConnect={startConnect}
            />
          )}
          {nav === "settings" && (
            <Settings
              t={t}
              lang={lang}
              onLanguageChange={changeLanguage}
              tc={tc}
              setTc={setTc}
              password={password}
              setPassword={setPassword}
              autostart={autostart}
              mobile={mobile}
              onSave={saveCredentials}
              onToggleAutostart={toggleAutostart}
            />
          )}
          {nav === "about" && (
            <About t={t} busy={busy} mobile={mobile} onCheckWarp={checkWarp} />
          )}
        </main>
      </div>

      <LogPanel
        t={t}
        lines={log}
        visible={logVisible}
        busy={busy}
        loginOk={loginOk}
        warpOk={warpOk}
        mobile={mobile}
        onToggle={() => setLogVisible((v) => !v)}
        onClear={() => setLog([])}
        onStop={stop}
      />
    </div>
  );
}

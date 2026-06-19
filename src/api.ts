//! Typed wrappers around the Tauri command + event surface (see
//! `src-tauri/src/commands.rs`). Keeping all `invoke`/`listen` calls here means
//! the React components never touch the raw IPC names.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/// Mirror of the Rust `core::LogMsg` enum (serialized as
/// `{ kind, data? }`). The i18n layer formats these into localized lines.
export type LogMsg =
  | { kind: "HttpClientBuildFailed"; data: string }
  | { kind: "LoggedInOnAttempt"; data: number }
  | { kind: "AttemptNotConfirmed"; data: number }
  | { kind: "AttemptTimeout"; data: number }
  | { kind: "AttemptNetworkError"; data: number }
  | { kind: "GivingUp"; data: number }
  | { kind: "LoginCancelled" }
  | { kind: "WarpConnected" }
  | { kind: "WarpConnectFailed"; data: string }
  | { kind: "WarpRunFailed"; data: string }
  | { kind: "WarpNotFound" }
  | { kind: "WarpInstallHint"; data: string }
  | { kind: "WarpInstallFollowup" };

export interface ConfigDto {
  tc: string;
  password: string;
  language: string;
  autostart: boolean;
}

export const api = {
  /** Current OS, e.g. "windows" | "android" | "macos" | "linux" | "ios". */
  platform: () => invoke<string>("platform"),
  loadConfig: () => invoke<ConfigDto>("load_config"),
  saveCredentials: (tc: string, password: string, language: string) =>
    invoke<void>("save_credentials", { tc, password, language }),
  saveLanguage: (language: string) =>
    invoke<void>("save_language", { language }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) =>
    invoke<void>("set_autostart", { enabled }),
  /** Start login (and optionally WARP). Progress arrives via events below. */
  connect: (tc: string, password: string, withWarp: boolean) =>
    invoke<void>("connect", { tc, password, withWarp }),
  stop: () => invoke<void>("stop"),
  checkWarp: () => invoke<void>("check_warp"),
};

/** Event payloads emitted by the background login/WARP worker. */
export const events = {
  onLog: (cb: (msg: LogMsg) => void): Promise<UnlistenFn> =>
    listen<LogMsg>("log", (e) => cb(e.payload)),
  onLoginResult: (cb: (ok: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>("login-result", (e) => cb(e.payload)),
  onWarpResult: (cb: (ok: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>("warp-result", (e) => cb(e.payload)),
  onDone: (cb: () => void): Promise<UnlistenFn> =>
    listen("done", () => cb()),
};

/** True on mobile platforms, where desktop-only controls are hidden. */
export const isMobile = (platform: string): boolean =>
  platform === "android" || platform === "ios";

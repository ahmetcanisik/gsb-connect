# GSB Connect

GSB Connect is a personal helper that automates logging in to the GSB WiFi
captive portal with your **own** credentials, with optional Cloudflare WARP.

It signs in exactly as you would in the browser login form. It is login
automation, not a security bypass. This is an independent, community tool (see
the [disclaimer](#disclaimer) below).

Repository: <https://github.com/ahmetcanisik/gsb-connect>

## Architecture

As of **v2**, the app is built with **[Tauri 2](https://v2.tauri.app/)**:

- **Rust backend** (`src-tauri/`) holds the reusable login/WARP/config logic,
  exposed to the UI as Tauri commands. The captive-portal login runs on a
  background thread and streams progress to the UI as events, so the window
  never freezes.
- **React + TypeScript frontend** (`src/`) renders the interface in the system
  webview (WebView2 on Windows). The visual language is modeled on the
  Cloudflare One Client: a dark, sidebar-driven layout with **Home**,
  **Settings**, and **About** pages plus a bottom activity-log panel.

Tauri lets the same codebase target **desktop (Windows)** and **mobile
(Android)**. The previous `egui`-based implementation is preserved on the `v1`
branch.

> Development happens on the `dev` branch. Releases are cut from `main`: pushing
> to `main` triggers the [build workflow](.github/workflows/build.yml), which
> runs security checks (gitleaks, `cargo audit`, `npm audit`) and then builds
> Windows, Linux and Android. Only when that build succeeds does the
> [release workflow](.github/workflows/release.yml) run (via `workflow_run`),
> publishing all three platforms' artifacts to a GitHub Release. No other branch
> triggers any workflow.

## Features

- **Turkish / English UI** (Turkish by default), switchable at runtime in
  **Settings** and remembered. A font with full Turkish coverage
  (`public/fonts/ui-font.ttf`) is bundled so all characters render correctly.
- **Encrypted credential storage**: on Windows, your TC ID and password are
  encrypted at rest with the Windows Data Protection API (DPAPI), current-user
  scope, then stored as base64 in a per-user config file. On Android they are
  stored in the OS-sandboxed per-app directory.
- **Optional start with Windows** (desktop only): a per-user, opt-in toggle that
  adds the app to your account's startup. No administrator rights; cleanly
  removable. Hidden on mobile.
- **Optional Cloudflare WARP** (desktop only): connect WARP after a successful
  login, or show platform-specific install hints. Hidden on mobile.
- **Polite retry strategy**: a bounded number of attempts with exponential
  backoff, clock-derived jitter, and a delay cap; cancellable with **Stop**.

## Configuration and data location

Credentials are entered in the window and, when you click **Save credentials**,
stored in a per-user `config.ini`:

- **Windows:** `%APPDATA%\GSBConnect\config.ini`
- **Linux / macOS (development):** `~/.gsbconnect/config.ini`
- **Android:** the app's private data directory (resolved at runtime).

On Windows the saved values are DPAPI-encrypted (marked `ENCRYPTED = 1`), so
another account on the same machine cannot read them and copying the file to a
different machine makes it undecryptable. The `[Settings]` section holds
non-secret preferences (`LANGUAGE`, `AUTOSTART`).

## Prerequisites

- **[Rust](https://www.rust-lang.org/tools/install)** (stable toolchain).
- **[Node.js](https://nodejs.org/)** 18+ and npm (for the frontend / Tauri CLI).
- **Windows:** [Microsoft C++ Build Tools] and the
  [WebView2 runtime] (preinstalled on Windows 11).
- See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
  for the full, current list.

[Microsoft C++ Build Tools]: https://visualstudio.microsoft.com/visual-cpp-build-tools/
[WebView2 runtime]: https://developer.microsoft.com/microsoft-edge/webview2/

## Building (desktop)

```sh
npm install            # install frontend dependencies (first time)
npm run tauri dev      # run the app with hot-reload
npm run tauri build    # produce an optimized NSIS + MSI installer
```

The installer is written to
`src-tauri/target/release/bundle/` (NSIS under `nsis/`, MSI under `msi/`).

### App icon

The icons in `src-tauri/icons/` are generated placeholders. To replace them with
your own artwork, run:

```sh
npm run tauri icon path/to/your-icon.png
```

## Building (Android)

Android can be built on Windows. Requirements:

- **JDK 17**, **Android Studio** with the **SDK** + **NDK**, and the env vars
  `ANDROID_HOME` and `NDK_HOME` set.
- Rust Android targets:

  ```sh
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```

Then:

```sh
npm run tauri android init     # generate src-tauri/gen/android (first time)
npm run tauri android dev      # run on an emulator or connected device
npm run tauri android build    # produce an APK / AAB
```

WARP and "start with Windows" are automatically hidden on mobile. A release
APK/AAB must be signed with your own keystore.

> **iOS** is not configured: building it requires macOS + Xcode. It can be added
> later on a Mac with `npm run tauri ios init`.

## Cloudflare WARP (desktop)

If `warp-cli` is installed it is used directly. Otherwise the tool prints the
install command for your platform:

- Windows: `winget install -e --id Cloudflare.Warp`
- macOS: `brew install --cask cloudflare-warp`
- Linux: see <https://pkg.cloudflareclient.com>

After installing, open the WARP app once before retrying.

## Disclaimer

This is an independent, community-built tool. It is **not affiliated with,
sponsored by, or endorsed by** the Ministry of Youth and Sports (Gençlik ve Spor
Bakanlığı, GSB) or KYK. It uses your own credentials to sign in to the public
captive portal, exactly as you would manually. Use it on networks and accounts
you are authorized to use.

## Acknowledgements

The bundled UI font (`public/fonts/ui-font.ttf`) is **Noto Sans Regular**, which
provides full Turkish glyph coverage. It is distributed under the
[SIL Open Font License 1.1](https://openfontlicense.org/).

## License

MIT

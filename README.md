# GSB Connect

GSB Connect is a personal helper that automates logging in to the GSB WiFi
captive portal with your **own** credentials, with optional Cloudflare WARP.

It signs in exactly as you would in the browser login form. It is login
automation, not a security bypass. This is an independent, community tool (see
the [disclaimer](#disclaimer) below).

Repository: <https://github.com/ahmetcanisik/gsb-connect>

The interface is built with [`egui`](https://github.com/emilk/egui) / `eframe`.
The network login runs on a background thread so the window never freezes,
streaming progress into an on-screen log. The visual language is modeled on the
Cloudflare One Client (the WARP desktop app): a dark, sidebar-driven layout with
**Home**, **Settings**, and **About** pages. The UI is strictly event-driven, so
it idles at ~0% CPU and only repaints on user input or while a background task is
running.

## Features

- **Turkish / English UI** (Turkish by default), switchable at runtime in
  **Settings** and remembered. A font with full Turkish coverage is bundled into
  the binary so all characters render correctly.
- **Encrypted credential storage**: on Windows, your TC ID and password are
  encrypted at rest with the Windows Data Protection API (DPAPI), current-user
  scope, then stored as base64 in a per-user config file.
- **Optional start with Windows**: a per-user, opt-in toggle in **Settings** that
  adds the app to your account's startup. It needs no administrator rights and is
  cleanly removable.
- **Optional Cloudflare WARP**: connect WARP after a successful login, or show
  platform-specific install hints, to avoid bandwidth throttling.
- **Cloudflare One-style interface**: dark theme, left navigation rail, status
  pills, and an auto-scrolling activity log.
- **Polite retry strategy**: a bounded number of attempts with exponential
  backoff, clock-derived jitter, and a delay cap.
- **Cross-platform build**: targets Windows; still builds and runs on Linux and
  macOS for development.

## Configuration and data location

Credentials are entered in the window and, when you click **Save credentials**,
stored in a per-user `config.ini`. They prefill the fields on the next launch.

- **Windows:** `%APPDATA%\GSBConnect\config.ini`
- **Linux / macOS (development):** `~/.gsbconnect/config.ini`

The directory is created automatically on first save.

**Credentials are encrypted at rest (Windows).** The TC ID and password are
DPAPI-encrypted in current-user scope, so another user account on the same
machine cannot read them and copying the file to a different machine makes it
undecryptable. An encrypted file is marked `ENCRYPTED = 1`. On non-Windows
development builds there is a plaintext fallback (`ENCRYPTED = 0`) so the project
still builds; DPAPI is Windows-only. Decrypted values are never logged.

The `[Settings]` section holds non-secret preferences: `LANGUAGE` (`tr` / `en`)
and `AUTOSTART` (`0` / `1`). For auto-start, the per-user registry Run key
(`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) is the source of truth at
startup, so the toggle always reflects reality even if you removed the entry
manually.

**Migration:** if you used an older version that stored its config under
`%APPDATA%\GSBWiFiLogin` (or `~/.gsbwifi`), it is copied once into the new
location on first run, so your saved credentials and language carry over.

## Building

Requires a Rust toolchain (2021 edition). Build a small, optimized release
binary:

```sh
cargo build --release
```

The binary is written to `target/release/gsb-connect`
(`target\release\gsb-connect.exe` on Windows). On Windows, release builds set
`#![windows_subsystem = "windows"]`, so the GUI launches without a console
window. The release binary is plain and unpacked; do not run UPX or any other
executable packer on it (packing frequently triggers antivirus false positives).

### Embedding the icon and version info (Windows)

`build.rs` (via the `winresource` crate) embeds Windows version information
(ProductName, FileDescription, CompanyName, version) into the release `.exe`. To
also embed an application icon, drop a real Windows icon at:

```
assets/app.ico
```

Then rebuild. If no icon is present the build still succeeds and prints a
`cargo:warning` reminder (version info is embedded either way). See
[`assets/README.md`](assets/README.md) for sizing guidance.

### Building the Windows installer

The project ships an [Inno Setup](https://jrsoftware.org/isinfo.php) script.

1. Install **Inno Setup 6.3 or newer** (it provides `ISCC.exe`).
2. Build the release binary first (`cargo build --release`).
3. Compile the installer from the project root:

   ```sh
   ISCC.exe installer/setup.iss
   ```

The setup executable is written to
`installer\dist\GSBConnect-Setup-<version>.exe`. It installs `gsb-connect.exe`
to `Program Files\GSB Connect`, creates a Start Menu shortcut (and an optional
desktop shortcut), and includes no config or credentials.

### Optional: code signing

This step is **entirely optional**. A code-signing certificate lets Windows show
a verified publisher name and helps SmartScreen build reputation faster. If you
have a `.pfx` certificate, sign both the application exe and the setup exe with
Microsoft's `signtool` (part of the Windows SDK), using SHA-256 and an RFC 3161
timestamp:

```sh
signtool sign /fd SHA256 /tr http://timestamp.example-ca.com /td SHA256 /f cert.pfx /p <password> target\release\gsb-connect.exe
signtool sign /fd SHA256 /tr http://timestamp.example-ca.com /td SHA256 /f cert.pfx /p <password> installer\dist\GSBConnect-Setup-1.0.0.exe
```

Sign the application exe **before** compiling the installer if you want the
bundled exe signed too, then sign the resulting setup exe.

**Without a certificate** the above does nothing, and that is fine. Unsigned
freeware may trigger a Windows SmartScreen "unknown publisher" warning until it
builds reputation; this is expected. Do **not** work around it by disabling
antivirus or adding exclusions. Click through "More info" -> "Run anyway", or
build the app yourself from source.

## Cloudflare WARP

If `warp-cli` is installed it is used directly. Otherwise the tool prints the
install command for your platform:

- Windows: `winget install -e --id Cloudflare.Warp`
- macOS: `brew install --cask cloudflare-warp`
- Linux: see <https://pkg.cloudflareclient.com>

After installing, open the WARP app once before retrying.

## Disclaimer

This is an independent, community-built tool. It is **not affiliated with,
sponsored by, or endorsed by** the Ministry of Youth and Sports (Genclik ve Spor
Bakanligi, GSB) or KYK. It uses your own credentials to sign in to the public
captive portal, exactly as you would manually. Use it on networks and accounts
you are authorized to use.

## Acknowledgements

The bundled UI font (`assets/ui-font.ttf`) is **Noto Sans Regular**, which
provides full Turkish glyph coverage. It is distributed under the
[SIL Open Font License 1.1](https://openfontlicense.org/). You can swap in
another OFL-licensed font with Turkish coverage (for example Inter) by replacing
the file; see [`assets/README.md`](assets/README.md) for details.

## License

MIT

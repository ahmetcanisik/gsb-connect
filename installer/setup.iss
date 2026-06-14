; Inno Setup script for GSB Connect.
;
; Build the release exe first:   cargo build --release
; Then compile this installer:   ISCC.exe installer\setup.iss
; The setup executable is written to installer\dist.
;
; This installer does NOT bundle any config.ini or credentials. The application
; creates its own per-user config in %APPDATA%\GSBConnect on first run.
;
; Bump the version in ONE place below.

#define MyAppVersion   "1.0.0"
#define MyAppName       "GSB Connect"
; Edit this to your own name or organization before distributing.
#define MyAppPublisher  "GSB Connect Project"
; Project source so users can verify what they are running.
#define MyAppURL        "https://github.com/ahmetcanisik/gsb-connect"
#define MyAppExeName    "gsb-connect.exe"

; Paths are relative to this script's directory (the installer\ folder).
#define SourceExe       "..\target\release\" + MyAppExeName
#define IconFile        "..\assets\app.ico"

[Setup]
; Keep this AppId STABLE across versions so upgrades replace cleanly. You may
; generate your own GUID, but do not change it once you have published.
AppId={{8F3C2A14-6B7D-4E59-9C0A-7D1F2B3E4A56}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; Installing under Program Files requires elevation.
PrivilegesRequired=admin
; 64-bit application (requires Inno Setup 6.3+ for "x64compatible").
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=dist
OutputBaseFilename=GSBConnect-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; English-only installer UI and an info page shown before installation.
InfoBeforeFile=info-before.txt
; SourcePath is this script's directory, so the check works from any CWD.
#if FileExists(SourcePath + IconFile)
SetupIconFile={#IconFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; Desktop shortcut is OPTIONAL (unchecked by default, matching Windows norms).
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; The plain release binary only. No UPX/packing, no config, no credentials.
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu shortcut (always created).
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
; Desktop shortcut (only if the user ticked the optional task).
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Offer to launch the app at the end of setup.
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

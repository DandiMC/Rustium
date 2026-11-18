; Rustium installer -- Inno Setup 6 script.
;
; Build it with:  installer\build.ps1
; (or:  "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\rustium.iss)
;
; The resulting setup.exe goes into dist\ and is the file to attach to a GitHub
; release. Every build must end up in the SAME folder, because the launcher looks
; for the exe of the selected version next to itself.
;
; Installs per user (no administrator prompt) into %LOCALAPPDATA%\Rustium, which is
; also where the launcher keeps its config (lang=...) and downloaded updates.

#define MyAppName "Rustium"
#define MyAppVersion "0.1.5"
#define MyAppPublisher "DandiMC"
#define MyAppExeName "Rustium.exe"

[Setup]
AppId={{6C1B9E2A-4F3D-4B7A-9C55-1E8D2A7B3F40}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Rustium
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=Rustium-{#MyAppVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\rustium.ico
; Apache 2.0: the licence text travels with every copy, and the page makes the
; attribution visible before the first install.
LicenseFile=..\LICENSE
; Icon of setup.exe itself (the pixel R).
SetupIconFile=..\assets\rustium.ico
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Name it exactly like this so the in-app updater can find and run it.
SetupMutex=RustiumSetupMutex
; Pick the Windows language so a double-click never stops at a language picker:
; the detection alone only preselects the combo, ShowLanguageDialog=no is what
; suppresses the dialog itself.
LanguageDetectionMethod=uilanguage
ShowLanguageDialog=no
UsePreviousLanguage=no

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Keep the release build in target\x86_64-pc-windows-gnu\release\ before compiling.
Source: "..\target\x86_64-pc-windows-gnu\release\rustium.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\rubydung-rs.exe"; DestDir: "{app}"; Flags: ignoreversion
; One exe per pre-classic build, so the picker can start any of them.
Source: "..\target\x86_64-pc-windows-gnu\release\rd132328.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\rd20090515.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\rd160052.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnu\release\rd161348.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\rustium.ico"; DestDir: "{app}"; Flags: ignoreversion
; Apache 2.0 asks for the licence and the attribution notice to be handed to
; every recipient, so they are installed next to the executables.
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rustium.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rustium.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{localappdata}\Rustium\update"

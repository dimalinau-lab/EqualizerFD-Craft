; Script generated for Inno Setup 6.x
; Equalizer FD CRAFT - Professional DSP & Configuration Studio

#define MyAppName "Equalizer FD CRAFT"
#define MyAppVersion "0.1.3-BETA"
#define MyAppPublisher "Equalizer FD CRAFT Team"
#define MyAppURL "https://github.com/dimalinau-lab/EqualizerFD-Craft"
#define MyAppExeName "EqualizerFD-Craft.exe"

[Setup]
AppId={{E6F74D4F-8D75-43B8-8E7D-E4B283FD9C10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\EqualizerFD-Craft
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=LICENSE
OutputDir=dist
OutputBaseFilename=EqualizerFD-Craft-Setup-v0.1.3-BETA
SetupIconFile=EqualizerFD-Craft.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "Запускать Equalizer FD CRAFT при старте Windows (в системном трее)"; GroupDescription: "Параметры запуска:"
Name: "vstplugins"; Description: "Встроенный набор студийных VST плагинов (98 плагинов: ReaPlugs, Blue Cat, Kuassa, компрессоры, гейты, эквалайзеры)"; GroupDescription: "Компоненты:"

[Files]
Source: "EqualizerFD-Craft.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "EqualizerTray.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "ResonaAPO.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "EqualizerFD-Craft.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "config.example.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

; Web studio UI assets
Source: "web\*"; DestDir: "{app}\web"; Flags: ignoreversion recursesubdirs createallsubdirs

; VB-Audio Virtual Cable bundled installer
Source: "tools\vbcable\*"; DestDir: "{app}\tools\vbcable"; Flags: ignoreversion recursesubdirs createallsubdirs

; Bundled Studio VST Plugins (98 plugins)
Source: "vst_plugins\*"; DestDir: "{app}\vst_plugins"; Tasks: vstplugins; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\EqualizerFD-Craft.ico"
Name: "{group}\Мастер быстрой настройки микрофона"; Filename: "{app}\{#MyAppExeName}"; Parameters: "editor"; IconFilename: "{app}\EqualizerFD-Craft.ico"
Name: "{group}\Папка студийных VST плагинов"; Filename: "{app}\vst_plugins"; Tasks: vstplugins
Name: "{group}\Установить VB-Audio Virtual Cable"; Filename: "{app}\tools\vbcable\VBCABLE_Setup_x64.exe"; IconFilename: "{app}\tools\vbcable\pin_in.ico"
Name: "{group}\Удалить {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\EqualizerFD-Craft.ico"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "EqualizerFDCraft"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "icacls.exe"; Parameters: """{app}\vst_plugins"" /grant ""*S-1-5-19:(OI)(CI)RX"" ""*S-1-15-2-1:(OI)(CI)RX"" /T /C /Q"; Flags: runhidden; Tasks: vstplugins
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM EqualizerFD-Craft.exe"; Flags: runhidden
Filename: "powershell"; Parameters: "-NoProfile -Command ""Stop-Process -Name 'EqualizerFD-Craft' -Force -ErrorAction SilentlyContinue"""; Flags: runhidden

[UninstallDelete]
Type: filesandordirs; Name: "{localappdata}\EqualizerFDCraft"

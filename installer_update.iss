; Inno Setup Script for Equalizer FD CRAFT - Lightweight Update Installer (NO VST PLUGINS)
#define MyAppName "Equalizer FD CRAFT"
#define MyAppVersion "0.1.3-BETA"
#define MyAppPublisher "Equalizer FD CRAFT Team"
#define MyAppURL "https://github.com/dimalinau-lab/EqualizerFD-Craft"
#define MyAppExeName "EqualizerFD-Craft.exe"

[Setup]
AppId={{E6F74D4F-8D75-43B8-8E7D-E4B283FD9C10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion} (Обновление)
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\EqualizerFD-Craft
UsePreviousAppDir=yes
DirExistsWarning=no
EnableDirDoesntExistWarning=no
DisableDirPage=yes
DisableProgramGroupPage=yes
AllowNoIcons=yes
OutputDir=dist
OutputBaseFilename=EqualizerFD-Craft-Update-Installer
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

[Files]
Source: "EqualizerFD-Craft.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "EqualizerTray.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "ResonaAPO.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "EqualizerFD-Craft.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion

; Web studio UI assets
Source: "web\*"; DestDir: "{app}\web"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\EqualizerFD-Craft.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\EqualizerFD-Craft.ico"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM EqualizerFD-Craft.exe"; Flags: runhidden

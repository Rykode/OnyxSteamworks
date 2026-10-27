#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\onyx.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif
#ifndef OutputName
  #define OutputName "onyx-setup"
#endif

[Setup]
AppId={{6F1D2B7E-3C54-4E8A-9A0B-5D7A41C9E2F3}
AppName=Onyx
AppVersion={#AppVersion}
AppPublisher=Onyx
DefaultDirName={autopf}\Onyx
DefaultGroupName=Onyx
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\onyx.exe
LicenseFile=..\assets\fonts\LICENSES.md

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\fonts\LICENSES.md"; DestDir: "{app}"; DestName: "FONT-LICENSES.md"; Flags: ignoreversion

[Icons]
Name: "{group}\Onyx"; Filename: "{app}\onyx.exe"
Name: "{autodesktop}\Onyx"; Filename: "{app}\onyx.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\onyx.exe"; Description: "Launch Onyx"; Flags: nowait postinstall skipifsilent

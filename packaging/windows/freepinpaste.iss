#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef PackageDir
  #define PackageDir "..\..\dist\FreePinPaste-windows-x86_64"
#endif

[Setup]
AppId=io.github.freepinpaste.FreePinPaste
AppName=FreePinPaste
AppVersion={#AppVersion}
AppPublisher=FreePinPaste
DefaultDirName={localappdata}\Programs\FreePinPaste
DefaultGroupName=FreePinPaste
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19045
OutputDir=..\..\dist
OutputBaseFilename=FreePinPaste-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\assets\freepinpaste.ico
UninstallDisplayIcon={app}\freepinpaste.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
LicenseFile={#PackageDir}\ocr\models\ocria-nano\LICENSE

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#PackageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\FreePinPaste"; Filename: "{app}\freepinpaste.exe"; Parameters: "--settings"
Name: "{autodesktop}\FreePinPaste"; Filename: "{app}\freepinpaste.exe"; Parameters: "--settings"; Tasks: desktopicon

[Run]
Filename: "{app}\freepinpaste.exe"; Parameters: "--settings"; Description: "Launch FreePinPaste"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\freepinpaste.exe"; Parameters: "--quit"; Flags: runhidden waituntilterminated; RunOnceId: "QuitFreePinPaste"

[Code]
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Value: String;
  Expected: String;
begin
  if CurUninstallStep = usUninstall then begin
    Expected := '"' + ExpandConstant('{app}\freepinpaste.exe') + '" --background';
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'FreePinPaste', Value) then
      if CompareText(Value, Expected) = 0 then
        RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'FreePinPaste');
  end;
end;

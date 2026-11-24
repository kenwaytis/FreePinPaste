# Install, run both distribution forms in the desktop session, and uninstall.
# The test refuses to overwrite an existing installation outside its own cache.
param([string]$Python = '', [switch]$MultiMonitor)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Set-Location $root
$version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
$install = "$root\.cache\install-smoke"
$portable = "$root\.cache\portable with spaces"
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\io.github.freepinpaste.FreePinPaste_is1'
$shortcut = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\FreePinPaste\FreePinPaste.lnk"
if (Test-Path $uninstallKey) {
    $existing = (Get-ItemProperty $uninstallKey).InstallLocation.TrimEnd('\')
    if ($existing -ne $install) { throw "Existing user installation will not be changed: $existing" }
}
function Run-Setup([string]$File, [string[]]$Arguments) {
    $process = Start-Process $File -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer/uninstaller failed: $($process.ExitCode)" }
}
try {
    Run-Setup "$root\dist\FreePinPaste-$version-windows-x86_64-setup.exe" @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$install`"", "/LOG=`"$root\.cache\install-smoke.log`"")
    if (!(Test-Path $shortcut) -or !(Test-Path "$install\ocr\bin\lit.exe")) { throw 'Installed shortcut or OCR runtime missing' }
    & "$PSScriptRoot\run-windows-desktop.ps1" -Python $Python -Arguments "--exe `"$install\freepinpaste.exe`"" -TimeoutSeconds 180
    Copy-Item .cache\windows-validation\result.json .cache\windows-validation\installed-result.json -Force
    if ($MultiMonitor) {
        & "$PSScriptRoot\run-windows-desktop.ps1" -Python $Python -Arguments "--multimon --exe `"$install\freepinpaste.exe`"" -TimeoutSeconds 180
        Copy-Item .cache\windows-validation\result.json .cache\windows-validation\installed-multimon-result.json -Force
    }

    if (Test-Path $portable) { Remove-Item -Recurse -Force $portable }
    Expand-Archive -Path dist\FreePinPaste-windows-x86_64.zip -DestinationPath $portable
    $exe = "$portable\FreePinPaste-windows-x86_64\freepinpaste.exe"
    & "$PSScriptRoot\run-windows-desktop.ps1" -Python $Python -Arguments "--exe `"$exe`"" -TimeoutSeconds 180
    Copy-Item .cache\windows-validation\result.json .cache\windows-validation\portable-result.json -Force
    if ($MultiMonitor) {
        & "$PSScriptRoot\run-windows-desktop.ps1" -Python $Python -Arguments "--multimon --exe `"$exe`"" -TimeoutSeconds 180
        Copy-Item .cache\windows-validation\result.json .cache\windows-validation\portable-multimon-result.json -Force
    }
} finally {
    if (Test-Path "$install\unins000.exe") {
        Run-Setup "$install\unins000.exe" @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$root\.cache\uninstall-smoke.log`"")
    }
}
if ((Test-Path "$install\freepinpaste.exe") -or (Test-Path $uninstallKey) -or (Test-Path $shortcut)) { throw 'Uninstall left application/registration/shortcut behind' }
@{ install = 'passed'; installed_gui = 'passed'; portable_gui = 'passed'; uninstall = 'passed'; multimon = [bool]$MultiMonitor } |
    ConvertTo-Json | Set-Content .cache\windows-validation\package-result.json -Encoding UTF8
Write-Host 'Package install, both GUI runs, and uninstall passed.'

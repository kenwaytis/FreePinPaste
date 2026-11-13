# Optional user-scope Inno Setup compiler. No elevation or system PATH changes.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$tools = "$root\.cache\tools"
$compiler = "$tools\InnoSetup\ISCC.exe"
if (Test-Path $compiler) { Write-Output $compiler; exit 0 }
New-Item -ItemType Directory -Force $tools | Out-Null
$installer = "$tools\innosetup-6.4.3.exe"
if (!(Test-Path $installer)) {
    & curl.exe --fail --location --retry 3 --retry-all-errors 'https://github.com/jrsoftware/issrc/releases/download/is-6_4_3/innosetup-6.4.3.exe' -o "$installer.tmp"
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup download failed' }
    Move-Item -Force "$installer.tmp" $installer
}
if ((Get-FileHash $installer -Algorithm SHA256).Hash.ToLower() -ne 'f3c42116542c4cc57263c5ba6c4feabfc49fe771f2f98a79d2f7628b8762723b') { throw 'Inno Setup hash mismatch' }
$signature = Get-AuthenticodeSignature $installer
if ($signature.Status -ne 'Valid') { throw "Inno Setup signature verification failed: $($signature.Status)" }
Write-Host "Verified publisher: $($signature.SignerCertificate.Subject)"
$process = Start-Process $installer -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', "/DIR=`"$tools\InnoSetup`"", '/NOICONS') -Wait -PassThru
if ($process.ExitCode -ne 0 -or !(Test-Path $compiler)) { throw "Inno Setup installation failed: $($process.ExitCode)" }
Write-Output $compiler

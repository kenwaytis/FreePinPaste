param([switch]$SkipOcr, [string]$Iscc)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Set-Location $root
if (!$SkipOcr) { & "$PSScriptRoot\build-ocr.ps1" }
$version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
if (!$Iscc) { $Iscc = "$root\.cache\tools\InnoSetup\ISCC.exe" }
if (!(Test-Path $Iscc)) { throw 'Inno Setup missing: run scripts/prepare-windows-tools.ps1 or pass -Iscc <path>' }
& cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed' }
$out = "$root\dist\FreePinPaste-windows-x86_64"
# Only remove the generated package, never the installed application or settings.
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force "$out\licenses" | Out-Null
Copy-Item target\release\freepinpaste.exe $out
Copy-Item runtime\ocria "$out\ocr" -Recurse
foreach ($file in 'bin\lit.exe', 'bin\pdfium.dll', 'bin\msvcp140.dll', 'bin\msvcp140_1.dll', 'bin\vcruntime140.dll', 'bin\vcruntime140_1.dll', 'models\ocria-nano\runtime\onnxruntime.dll') {
    if (!(Test-Path "$out\ocr\$file" -PathType Leaf)) { throw "Missing OCR runtime file: $file" }
}
Copy-Item README.md, LICENSE, THIRD_PARTY.md $out
Copy-Item docs "$out\docs" -Recurse
& python scripts/collect-licenses.py "$out\licenses" x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'License collection failed' }
foreach ($line in Get-Content packaging\models.sha256) {
    if ($line -match '^([0-9a-f]{64})\s+(.+)$') {
        if ((Get-FileHash (Join-Path "$out\ocr" $Matches[2]) -Algorithm SHA256).Hash.ToLower() -ne $Matches[1]) { throw "Model hash mismatch: $line" }
    }
}
$zip = "$root\dist\FreePinPaste-windows-x86_64.zip"
if (Test-Path $zip) { Remove-Item $zip }
# Some crates publish license files with a 1970 timestamp; ZIP starts in 1980.
# Clamp only the generated copy, not Cargo's cache or source files.
Get-ChildItem $out -Recurse | Where-Object { $_.LastWriteTime.Year -lt 1980 } | ForEach-Object {
    $_.LastWriteTimeUtc = [datetime]::SpecifyKind([datetime]'1980-01-01', [DateTimeKind]::Utc)
}
# Windows PowerShell 5 Compress-Archive uses backslashes in ZIP entry names.
& python scripts/package-zip.py $out $zip
if ($LASTEXITCODE -ne 0) { throw 'ZIP creation failed' }
& $Iscc "/DAppVersion=$version" "/DPackageDir=$out" packaging\windows\freepinpaste.iss
if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
$installer = "$root\dist\FreePinPaste-$version-windows-x86_64-setup.exe"
foreach ($file in @($zip, $installer)) {
    $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLower()
    [IO.File]::WriteAllText("$file.sha256", "$hash  $([IO.Path]::GetFileName($file))`n", [Text.UTF8Encoding]::new($false))
    Write-Host "$file ($((Get-Item $file).Length) bytes)"
}

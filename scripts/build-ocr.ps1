$ErrorActionPreference = 'Stop'
& python "$PSScriptRoot\prepare-ocr.py"
if ($LASTEXITCODE -ne 0) { throw 'OCRia runtime preparation failed' }

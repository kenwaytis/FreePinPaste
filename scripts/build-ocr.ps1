param([int]$Jobs = 8)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Set-Location $root
$cache = "$root\.cache\ocr-windows"
$prefix = "$cache/prefix" -replace '\\', '/'
$out = "$root\runtime\ocr"
New-Item -ItemType Directory -Force "$cache\downloads", "$cache\src", "$out\bin", "$out\tessdata", "$out\licenses" | Out-Null

function Run([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed ($LASTEXITCODE)" }
}
function Fetch([string]$Url, [string]$Path, [string]$Hash) {
    if (!(Test-Path $Path)) {
        Run 'curl.exe' @('--fail', '--location', '--retry', '3', '--retry-all-errors', $Url, '-o', "$Path.tmp")
        Move-Item -Force "$Path.tmp" $Path
    }
    if ((Get-FileHash $Path -Algorithm SHA256).Hash.ToLower() -ne $Hash) { throw "SHA256 mismatch: $Path" }
}
function Source([string]$Name, [string]$Url, [string]$Hash) {
    $archive = "$cache\downloads\$Name.tar.gz"
    Fetch $Url $archive $Hash
    Run 'tar.exe' @('-xzf', $archive, '-C', "$cache\src")
}
function Configure([string]$Name, [string]$Source, [string[]]$Options) {
    Run 'cmake.exe' (@('-S', $Source, '-B', "$cache\$Name", '-G', 'Visual Studio 17 2022', '-A', 'x64',
        '-DCMAKE_POLICY_VERSION_MINIMUM=3.5', '-DCMAKE_POLICY_DEFAULT_CMP0091=NEW', '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded',
        "-DCMAKE_INSTALL_PREFIX=$prefix", "-DCMAKE_PREFIX_PATH=$prefix", '-DBUILD_SHARED_LIBS=OFF') + $Options)
    Run 'cmake.exe' @('--build', "$cache\$Name", '--config', 'Release', '--parallel', "$Jobs")
    Run 'cmake.exe' @('--install', "$cache\$Name", '--config', 'Release')
}
$lept = '63aef18d98432b8582a1565e241f7bd2ee9cc8d9'
$tess = 'ad3ba9c9c665d879553c27980d868225943ce9f5'
$data = '87416418657359cb625c412a48b6e1d6d41c29bd'
Source 'zlib' 'https://codeload.github.com/madler/zlib/tar.gz/refs/tags/v1.3.1' '17e88863f3600672ab49182f217281b6fc4d3c762bde361935e436a95214d05c'
Source 'libpng' 'https://codeload.github.com/pnggroup/libpng/tar.gz/refs/tags/v1.6.50' '71158e53cfdf2877bc99bcab33641d78df3f48e6e0daad030afe9cb8c031aa46'
Source 'jpeg' 'https://codeload.github.com/libjpeg-turbo/libjpeg-turbo/tar.gz/refs/tags/3.1.0' '35fec2e1ddfb05ecf6d93e50bc57c1e54bc81c16d611ddf6eff73fff266d8285'
Source 'tiff' 'https://download.osgeo.org/libtiff/tiff-4.7.2.tar.gz' '672bd7d10aee4606171afb864f3570b83340f6a33e2c186dc0512f7145ffdf6a'
Source 'leptonica' "https://codeload.github.com/DanBloomberg/leptonica/tar.gz/$lept" 'beb4804d5cc9d274986e22b01a9c7e1ad4858d8487bfe9215728eb5e906f07dc'
Source 'tesseract' "https://codeload.github.com/tesseract-ocr/tesseract/tar.gz/$tess" 'ac044e77d4a8f4a0412d12ddbfc412f33443e5398798bc33076aa89e018d088a'
Configure 'zlib-build' "$cache\src\zlib-1.3.1" @('-DZLIB_BUILD_EXAMPLES=OFF')
$zlib = @("-DZLIB_LIBRARY=$prefix/lib/zlibstatic.lib", "-DZLIB_INCLUDE_DIR=$prefix/include")
Configure 'png-build' "$cache\src\libpng-1.6.50" ($zlib + @('-DPNG_SHARED=OFF', '-DPNG_STATIC=ON', '-DPNG_TESTS=OFF', '-DPNG_TOOLS=OFF'))
$png = $zlib + @("-DPNG_LIBRARY=$prefix/lib/libpng16_static.lib", "-DPNG_PNG_INCLUDE_DIR=$prefix/include")
Configure 'jpeg-build' "$cache\src\libjpeg-turbo-3.1.0" @('-DENABLE_SHARED=OFF', '-DENABLE_STATIC=ON', '-DWITH_TURBOJPEG=OFF', '-DWITH_SIMD=OFF', '-DWITH_CRT_DLL=OFF')
$jpeg = @("-DJPEG_LIBRARY=$prefix/lib/jpeg-static.lib", "-DJPEG_INCLUDE_DIR=$prefix/include")
Configure 'tiff-build' "$cache\src\tiff-4.7.2" ($zlib + $jpeg + @('-Dtiff-tools=OFF', '-Dtiff-tests=OFF', '-Dtiff-contrib=OFF', '-Dtiff-docs=OFF',
    '-Djbig=OFF', '-Dlzma=OFF', '-Dzstd=OFF', '-Dwebp=OFF', '-Dlibdeflate=OFF', '-Djpeg=ON', '-Djpeg-prefer-standard=ON', '-Dzlib=ON'))
$codecs = $png + $jpeg + @('-DCMAKE_DISABLE_FIND_PACKAGE_Tiff=ON', "-DTIFF_INCLUDE_DIR=$prefix/include", "-DTIFF_LIBRARY_RELEASE=$prefix/lib/tiff.lib", "-DTIFF_LIBRARY_DEBUG=$prefix/lib/tiff.lib")
# TIFF/JPEG are required by Leptonica's embedded bitmap fonts, even with PNG
# OCR input. All codecs and the CRT are linked statically; no OpenMP DLL.
Configure 'lept-build' "$cache\src\leptonica-$lept" ($codecs + @('-DBUILD_PROG=OFF', '-DSW_BUILD=OFF',
    '-DENABLE_PNG=ON', '-DENABLE_GIF=OFF', '-DENABLE_JPEG=ON', '-DENABLE_TIFF=ON', '-DENABLE_WEBP=OFF', '-DENABLE_OPENJPEG=OFF'))
Configure 'tess-build' "$cache\src\tesseract-$tess" ($codecs + @('-U', 'LEPT_TIFF_*', '-DBUILD_TRAINING_TOOLS=OFF', '-DBUILD_TESTS=OFF', '-DSW_BUILD=OFF',
    '-DGRAPHICS_DISABLED=ON', '-DDISABLED_LEGACY_ENGINE=ON', '-DOPENMP_BUILD=OFF', '-DENABLE_NATIVE=OFF',
    '-DDISABLE_CURL=ON', '-DDISABLE_ARCHIVE=ON', '-DDISABLE_TIFF=OFF'))
Copy-Item "$prefix\bin\tesseract.exe" "$out\bin\tesseract.exe" -Force
Copy-Item "$cache\src\tesseract-$tess\LICENSE" "$out\licenses\Tesseract-APACHE-2.0.txt" -Force
Copy-Item "$cache\src\leptonica-$lept\leptonica-license.txt" "$out\licenses\Leptonica-BSD.txt" -Force
Copy-Item "$cache\src\zlib-1.3.1\LICENSE" "$out\licenses\zlib.txt" -Force
Copy-Item "$cache\src\libpng-1.6.50\LICENSE" "$out\licenses\libpng.txt" -Force
Copy-Item "$cache\src\tiff-4.7.2\LICENSE.md" "$out\licenses\TIFF.txt" -Force
Copy-Item "$cache\src\libjpeg-turbo-3.1.0\LICENSE.md" "$out\licenses\libjpeg-turbo.txt" -Force
Copy-Item "$cache\src\libjpeg-turbo-3.1.0\README.ijg" "$out\licenses\JPEG-IJG.txt" -Force
foreach ($line in Get-Content "$root\packaging\models.sha256") {
    if ($line -match '^([0-9a-f]{64})\s+tessdata/(.+)$') {
        Fetch "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$data/$($Matches[2])" "$out\tessdata\$($Matches[2])" $Matches[1]
    }
}
Run 'curl.exe' @('--fail', '--location', '--retry', '3', '--retry-all-errors', "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$data/LICENSE", '-o', "$out\licenses\Tessdata-APACHE-2.0.txt")
Copy-Item "$root\packaging\models.sha256" "$out\models.sha256" -Force
Run "$out\bin\tesseract.exe" @('--version')
Run "$out\bin\tesseract.exe" @('--tessdata-dir', "$out\tessdata", '--list-langs')
Write-Host "OCR ready: $out"

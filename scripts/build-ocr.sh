#!/usr/bin/env bash
# Builds the bundled CPU OCR engine without sudo or modifying system packages.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
CACHE="$ROOT/.cache"
PREFIX="$CACHE/ocr-prefix"
OUT="$ROOT/runtime/ocr"
JOBS="${JOBS:-8}"
LEPT=63aef18d98432b8582a1565e241f7bd2ee9cc8d9
TESS=ad3ba9c9c665d879553c27980d868225943ce9f5
DATA=87416418657359cb625c412a48b6e1d6d41c29bd
# Leptonica 1.85 expects classic FindTIFF variables, not modern TIFF exported
# targets whose private JPEG/ZLIB target dependencies it does not re-export.
TIFF_ARGS=(-DCMAKE_DISABLE_FIND_PACKAGE_Tiff=ON -DTIFF_INCLUDE_DIR="$PREFIX/include"
  -DTIFF_LIBRARY_RELEASE="$PREFIX/lib/libtiff.a" -DTIFF_LIBRARY_DEBUG="$PREFIX/lib/libtiff.a")
mkdir -p "$CACHE/downloads" "$CACHE/ocr-src" "$OUT/bin" "$OUT/tessdata" "$OUT/licenses"
fetch() {
  local url="$1" dest="$2"
  if [[ ! -s "$dest" ]]; then
    curl --fail --location --retry 3 "$url" -o "$dest.tmp"
    mv "$dest.tmp" "$dest"
  fi
}
fetch "https://codeload.github.com/DanBloomberg/leptonica/tar.gz/$LEPT" "$CACHE/downloads/leptonica.tar.gz"
fetch "https://codeload.github.com/tesseract-ocr/tesseract/tar.gz/$TESS" "$CACHE/downloads/tesseract.tar.gz"
fetch "https://download.osgeo.org/libtiff/tiff-4.7.2.tar.gz" "$CACHE/downloads/tiff.tar.gz"
printf '%s  %s\n' \
  beb4804d5cc9d274986e22b01a9c7e1ad4858d8487bfe9215728eb5e906f07dc "$CACHE/downloads/leptonica.tar.gz" \
  ac044e77d4a8f4a0412d12ddbfc412f33443e5398798bc33076aa89e018d088a "$CACHE/downloads/tesseract.tar.gz" \
  672bd7d10aee4606171afb864f3570b83340f6a33e2c186dc0512f7145ffdf6a "$CACHE/downloads/tiff.tar.gz" | sha256sum -c -
tar -xzf "$CACHE/downloads/leptonica.tar.gz" -C "$CACHE/ocr-src"
tar -xzf "$CACHE/downloads/tesseract.tar.gz" -C "$CACHE/ocr-src"
tar -xzf "$CACHE/downloads/tiff.tar.gz" -C "$CACHE/ocr-src"
# Leptonica's embedded bitmap fonts need TIFF + JPEG. Build a small static TIFF
# rather than pulling every optional codec (including JBIG) from the host build.
cmake -S "$CACHE/ocr-src/tiff-4.7.2" -B "$CACHE/tiff-build" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" -DCMAKE_INSTALL_LIBDIR=lib \
  -DBUILD_SHARED_LIBS=OFF -Dtiff-tools=OFF -Dtiff-tests=OFF -Dtiff-contrib=OFF -Dtiff-docs=OFF \
  -Djbig=OFF -Dlzma=OFF -Dzstd=OFF -Dwebp=OFF -Dlibdeflate=OFF -Djpeg=ON -Djpeg-prefer-standard=ON -Dzlib=ON
cmake --build "$CACHE/tiff-build" --parallel "$JOBS"
cmake --install "$CACHE/tiff-build"
cmake -S "$CACHE/ocr-src/leptonica-$LEPT" -B "$CACHE/lept-build" \
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" \
  -DCMAKE_PREFIX_PATH="$PREFIX" "${TIFF_ARGS[@]}" \
  -DBUILD_SHARED_LIBS=OFF -DBUILD_PROG=OFF -DSW_BUILD=OFF \
  -DENABLE_GIF=OFF -DENABLE_JPEG=ON -DENABLE_TIFF=ON -DENABLE_WEBP=OFF -DENABLE_OPENJPEG=OFF
cmake --build "$CACHE/lept-build" --parallel "$JOBS"
cmake --install "$CACHE/lept-build"
cmake -S "$CACHE/ocr-src/tesseract-$TESS" -B "$CACHE/tess-build" \
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$PREFIX" \
  "${TIFF_ARGS[@]}" -U 'LEPT_TIFF_*' \
  -DBUILD_SHARED_LIBS=OFF -DBUILD_TRAINING_TOOLS=OFF -DBUILD_TESTS=OFF -DSW_BUILD=OFF \
  -DGRAPHICS_DISABLED=ON -DDISABLED_LEGACY_ENGINE=ON -DOPENMP_BUILD=ON -DENABLE_NATIVE=OFF \
  -DDISABLE_CURL=ON -DDISABLE_ARCHIVE=ON -DDISABLE_TIFF=OFF
cmake --build "$CACHE/tess-build" --target tesseract --parallel "$JOBS"
install -m755 "$CACHE/tess-build/bin/tesseract" "$OUT/bin/tesseract"
strip "$OUT/bin/tesseract"
cp "$CACHE/ocr-src/tesseract-$TESS/LICENSE" "$OUT/licenses/Tesseract-APACHE-2.0.txt"
cp "$CACHE/ocr-src/leptonica-$LEPT/leptonica-license.txt" "$OUT/licenses/Leptonica-BSD.txt"
cp "$CACHE/ocr-src/tiff-4.7.2/LICENSE.md" "$OUT/licenses/TIFF.txt"
for lang in eng chi_sim chi_tra jpn kor fra deu spa rus ara; do
  fetch "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$DATA/$lang.traineddata" "$OUT/tessdata/$lang.traineddata"
done
fetch "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$DATA/LICENSE" "$OUT/licenses/Tessdata-APACHE-2.0.txt"
(cd "$OUT"; sha256sum tessdata/*.traineddata > models.sha256)
if [[ -f "$ROOT/packaging/models.sha256" ]]; then
  (cd "$OUT"; sha256sum -c "$ROOT/packaging/models.sha256")
fi
"$OUT/bin/tesseract" --version
"$OUT/bin/tesseract" --tessdata-dir "$OUT/tessdata" --list-langs
printf '\nOCR ready: %s\n' "$OUT"

#!/bin/bash
# Developer build entry point. Customers receive the resulting signed PKG.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Darwin && "$(uname -m)" == arm64 ]] || { echo 'Compila este paquete en una Mac Apple Silicon.'; exit 1; }
[[ "$(sw_vers -productVersion | cut -d. -f1)" -ge 15 ]] || { echo 'Se requiere macOS 15 o posterior.'; exit 1; }
for tool in node npm cargo rustup xcodebuild pkgbuild productbuild; do command -v "$tool" >/dev/null || { echo "Falta la herramienta de compilación $tool (solo en la Mac de desarrollo)."; exit 1; }; done
xcode-select -p >/dev/null
MODE="${1:---verify}"
if [[ "$MODE" == --release ]]; then
  : "${APPLE_SIGNING_IDENTITY:?Define Developer ID Application}"
  : "${APPLE_INSTALLER_IDENTITY:?Define Developer ID Installer}"
  : "${APPLE_NOTARY_PROFILE:?Define el perfil guardado de notarytool}"
elif [[ "$MODE" != --verify ]]; then echo 'Uso: Build-MacOS.command [--verify|--release]'; exit 1; fi
mkdir -p .tools/mysql desktop/.runtime output/macos
fetch() {
  local key="$1" file url expected
  file=$(node -p "require('./packaging/dependencies.json')['$key'].file")
  url=$(node -p "require('./packaging/dependencies.json')['$key'].url")
  expected=$(node -p "require('./packaging/dependencies.json')['$key'].sha256")
  [[ -f ".tools/mysql/$file" ]] || curl --fail --location --retry 3 "$url" --output ".tools/mysql/$file"
  [[ "$(shasum -a 256 ".tools/mysql/$file" | cut -d' ' -f1)" == "$expected" ]] || { echo "SHA-256 no válido: $file"; exit 1; }
}
fetch mysqlMacArm64
fetch workbenchMacArm64
if [[ ! -x .tools/mysql/mysql-8.4.13-macos15-arm64/bin/mysqld ]]; then tar -xzf .tools/mysql/mysql-8.4.13-macos15-arm64.tar.gz -C .tools/mysql; fi
ditto .tools/mysql/mysql-8.4.13-macos15-arm64 desktop/.runtime/mysql
mkdir -p .tools/mysql/workbench-mac
ditto -x -k .tools/mysql/mysql-workbench-26.10.0-macos-arm64.zip .tools/mysql/workbench-mac
WB=".tools/mysql/workbench-mac/MySQL Workbench.app"
codesign --verify --deep --strict "$WB"
desktop/.runtime/mysql/bin/mysqld --no-defaults --version
rustup target add aarch64-apple-darwin
npm --prefix desktop ci
npm --prefix desktop test
cargo test --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --tests
export DONOVAN_TEST_MYSQL_RUNTIME="$PWD/desktop/.runtime/mysql"
cargo test --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --test mysql_portability -- --ignored --test-threads=1
cargo test --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --test mysql_failures -- --ignored --test-threads=1
cargo test --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --test commerce_equivalence --test excel_import --test reports_and_labels --test business_documents --test security_and_preferences -- --test-threads=1
cargo test --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --test register_and_tickets -- --skip schema_four_upgrades --test-threads=1
unset DONOVAN_TEST_MYSQL_RUNTIME
(cd desktop && npm run tauri -- build --target aarch64-apple-darwin --bundles app --config src-tauri/tauri.macos.conf.json)
APP="desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Donovan POS Pro.app"
VERSION=$(node -p "require('./desktop/package.json').version")
WORK=$(mktemp -d "$PWD/output/macos/package.XXXXXX")
pkgbuild --component "$APP" --install-location /Applications --identifier com.donovansystems.pos --version "$VERSION" "$WORK/DonovanPOS.pkg"
pkgbuild --component "$WB" --install-location '/Applications/Donovan POS Tools' --identifier com.donovansystems.pos.workbench --version 26.10.0 "$WORK/Workbench.pkg"
productbuild --synthesize --package "$WORK/DonovanPOS.pkg" --package "$WORK/Workbench.pkg" "$WORK/Distribution.xml"
OUTPUT="$PWD/output/macos/DonovanPOS-${VERSION}-AppleSilicon"
if [[ "$MODE" == --release ]]; then
  codesign --verify --deep --strict "$APP"
  productbuild --distribution "$WORK/Distribution.xml" --package-path "$WORK" --sign "$APPLE_INSTALLER_IDENTITY" "$OUTPUT.pkg"
  xcrun notarytool submit "$OUTPUT.pkg" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait
  xcrun stapler staple "$OUTPUT.pkg"
  spctl --assess --type install --verbose "$OUTPUT.pkg"
else
  productbuild --distribution "$WORK/Distribution.xml" --package-path "$WORK" "$OUTPUT-validacion-sin-firma.pkg"
  echo 'Paquete para validación local. No distribuir como versión final: falta firma/notarización y la prueba del instalador.'
fi
echo 'Instala en un usuario aislado de macOS y completa docs/v2/MYSQL-Y-MACOS.md antes de entregar.'

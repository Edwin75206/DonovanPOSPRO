# Donovan POS Pro 2.3 — desarrollo de escritorio

La aplicación activa usa Tauri 2, React/TypeScript y servicios Rust. MySQL es el motor local de las instalaciones nuevas; SQLite se conserva para migración controlada y respaldos portables. El cliente instala la aplicación y sus componentes, sin herramientas de desarrollo.

## Windows x64

El equipo de desarrollo necesita PowerShell 7, Node 24, Rust MSVC y Visual Studio Build Tools con C++, Windows SDK y los redistribuibles de Visual C++. La revisión 2.3 se probó con Node 24.22.2 y Rust 1.99.1. `package-lock.json` y `Cargo.lock` deben conservarse.

Desde la raíz del repositorio:

```powershell
. ./scripts/Enter-NativeEnvironment.ps1
npm --prefix desktop ci
npm --prefix desktop test
cargo test --locked --manifest-path desktop/src-tauri/Cargo.toml --no-default-features --tests
./scripts/Build-MySqlWindows.ps1
```

`Enter-NativeEnvironment.ps1` usa el SDK oficial instalado, o la caché local opcional `.tools` si está disponible. El script conserva `RUSTFLAGS=-C target-feature=+crt-static`. `Prepare-MySqlWindows.ps1` descarga paquetes oficiales verificados, prepara una copia privada de MySQL y Workbench y copia las bibliotecas de ejecución de Microsoft; no crea un servidor global.

La salida es `desktop/src-tauri/target/release/bundle/nsis/Donovan POS Pro_2.3.0_x64-setup.exe`. La construcción necesita internet para dependencias y WebView2 offline; el instalador incluye lo necesario para ejecutar sin internet. No publicar `target/` ni `.runtime/` en Git.

## Mac M4 / Apple Silicon

**Pendiente de compilación e instalación reales en macOS.** Leer [el prompt](../PROMPT-PARA-MAC.md) y [el contexto](../CONTINUAR-EN-MAC.md) antes de continuar.

En una Mac de desarrollo con macOS 15 o posterior, Xcode Command Line Tools, Node y Rust:

```sh
bash scripts/Build-MacOS.command --verify
```

El script se ejecuta desde la raíz, prepara dependencias arm64, ejecuta pruebas y genera un PKG para validación local. El modo `--release` exige Developer ID Application, Developer ID Installer y un perfil de notarización. La compilación no sustituye las pruebas de instalación, ventas, Keychain, Workbench, actualización, restauración e impresión.

## Organización y pruebas

`src/pages/` contiene las pantallas; `src/lib/api.ts` define la frontera de acceso. `src-tauri/src/commands.rs` valida los comandos IPC y los servicios Rust aplican permisos y transacciones. `src-tauri/src/database.rs` abstrae SQLite/MySQL; `mysql_runtime.rs` controla la instancia privada. `documents/printing_windows.rs` y `printing_cups.rs` son los adaptadores nativos.

Las pruebas ordinarias usan datos aislados. Las de MySQL requieren `DONOVAN_TEST_MYSQL_RUNTIME` con la ruta absoluta del runtime oficial y ejecución explícita de `mysql_portability` y `mysql_failures` con `--ignored`; consultar [la guía MySQL](../docs/v2/MYSQL-Y-MACOS.md). No apuntar a datos comerciales.

`DonovanPOS --data-dir "ruta-absoluta"` permite validar una instalación aislada. La instalación comercial no necesita este argumento. Los ejemplos de aceptación no forman parte del instalador. No copiar archivos de MySQL como método de traslado: usar los respaldos de la aplicación.

En Windows, `scripts/Test-NativeInstaller.ps1` comprueba instalación, recursos, ejecutable y desinstalación en destinos propios. Sus hashes contemplan el marcador `UNK` → `NSS` que aplica Tauri al empaquetar. Las evidencias locales se guardan fuera de Git.

La política de publicación y exclusiones está en [VERSIONES-Y-GITHUB.md](../docs/VERSIONES-Y-GITHUB.md). El proceso CI no publica instaladores automáticamente ni modifica bases existentes.

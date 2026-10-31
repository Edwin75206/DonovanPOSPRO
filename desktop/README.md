# Donovan POS Pro 2 — escritorio Windows

Aplicación local para Construrama Chicontla. Tauri 2, React, TypeScript, Tailwind CSS y componentes basados en shadcn/ui; Motion, Apache ECharts y TanStack Table. Rust controla permisos, precios, existencias, caja, documentos, archivos y SQLite. React no ejecuta SQL. El programa instalado no necesita Node.js, un SDK, un servidor ni conexión a internet.

## Desarrollo y compilación

El equipo de desarrollo necesita Windows x64, Rust MSVC, Visual Studio Build Tools con C++, Windows SDK y Node.js. Las versiones efectivas se fijan en `Cargo.lock` y `package-lock.json`. Estos componentes se usan para construir el producto y no se piden al cliente.

En este checkout, `../scripts/Enter-NativeEnvironment.ps1` configura las herramientas locales de `.tools`, el SDK y el enlazador. La carpeta `.tools` se excluye del control de versiones. En otro equipo pueden utilizarse las instalaciones oficiales equivalentes. La compilación final debe conservar `RUSTFLAGS=-C target-feature=+crt-static` para incluir el runtime C/C++.

Desde `desktop`, en PowerShell:

```powershell
. ../scripts/Enter-NativeEnvironment.ps1
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features
npm run tauri build -- --bundles nsis
```

Salidas: `src-tauri/target/release/DonovanPOS.exe` y `src-tauri/target/release/bundle/nsis/Donovan POS Pro_2.2.0_x64-setup.exe`. El instalador incluye WebView2 offline; su compilación descarga ese redistribuible si todavía no existe en la caché. La instalación del cliente utiliza el archivo incluido. La configuración sigue la [documentación oficial de Tauri](https://v2.tauri.app/distribute/windows-installer/).

`npm run tauri bundle -- --bundles nsis` vuelve a empaquetar un ejecutable ya compilado. Debe usarse únicamente si ese ejecutable y `dist` corresponden al código que se pretende distribuir. El nombre de binario de Cargo, `default-run` y `mainBinaryName` deben permanecer alineados en `DonovanPOS`.

## Organización

| Ubicación | Responsabilidad |
| --- | --- |
| `src/pages` | Flujos de venta, administración y configuración |
| `src/components` | Tablas, diálogos, documentos y estados compartidos |
| `src/lib/api.ts` | Contratos tipados y aislamiento de respuestas entre sesiones |
| `src-tauri/src/commands.rs` | Única frontera IPC; comandos permitidos y validación de contexto |
| `src-tauri/src/engine.rs` | Instancia local, conexión, reloj y sesiones |
| `src-tauri/src/commerce.rs` | Precios, cobro y transacción de venta |
| `src-tauri/src/contacts.rs`, `purchasing.rs`, `quotes.rs`, `returns.rs` | Servicios de operación |
| `src-tauri/src/storage.rs`, `recovery.rs` | Copias consistentes y activación controlada de bases |
| `src-tauri/src/importing.rs`, `exporting.rs`, `reports.rs` | Importación revisada, Excel e información administrativa |
| `src-tauri/src/documents` | Composición compartida de PDF, vista previa e impresión GDI |
| `src-tauri/src/tickets.rs` | Perfil del ticket y autorización administrativa; controlador Windows desacoplado en `documents/printing.rs` |
| `src/pages/Register.tsx`, `RegisterTools.tsx`, `PaymentForm.tsx` | Mostrador, acceso integrado a caja/ventas recientes y cobro |
| `src/lib/register.ts`, `src/components/QuantityEditor.tsx` | Lecturas HID y validación inmediata de cantidades; la decisión final sigue en Rust |
| `src-tauri/migrations` | Esquemas SQL versionados |
| `src-tauri/tests` | Pruebas de integración con bases aisladas y cuentas sintéticas |

## Revisiones locales

`DonovanPOS.exe --demo` crea un negocio sintético independiente en la carpeta temporal de Windows. No sirve para registrar operaciones reales ni para guardar información del cliente. El rótulo DEMOSTRACIÓN permanece visible.

`DonovanPOS.exe --data-dir "ruta-absoluta"` permite a pruebas y soporte abrir una raíz de datos aislada. No debe apuntar a la carpeta del programa, de un instalador ni a una unidad compartida por varios equipos. La instalación normal no necesita este argumento.

No sustituir una copia de la aplicación que siga abierta. Al revisar una compilación, copiar el EXE a una carpeta nueva y conservar separadas las instancias que contengan trabajo del usuario. El motor impide abrir simultáneamente la misma raíz de datos.

La versión WPF se conserva en `../src` y `../tests`; el hito original está en la etiqueta `v1.0.0`. La migración de sus datos usa una copia validada y conserva el origen.

El estado de entrega y sus comprobaciones están en `../docs/v2/PROGRESO.md`. Compilar un instalador no equivale a haberlo validado en un equipo limpio.

El gancho NSIS usa el redistribuible offline incluido si encuentra un WebView2 anterior a 120. `minimumWebview2Version` permanece sin configurar porque la rama estándar de actualización de Tauri llama al actualizador online. El gancho también impide sustituir o desinstalar el EXE mientras esa copia siga abierta, sin forzar su cierre.

La prueba del paquete está en `scripts/Test-NativeInstaller.ps1` y el soporte de datos sintéticos en `src-tauri/examples/installation_qa.rs`. Tauri cambia exclusivamente el marcador de empaquetado `UNK` a `NSS` al crear NSIS; la prueba deriva ese cambio en memoria para verificar el SHA-256 del EXE instalado. El ejemplo de pruebas no se distribuye con el producto.

# Continuar Donovan POS Pro en la Mac M4

Actualizado: 9 de noviembre de 2026, zona America/Mexico_City.

Este archivo conserva el contexto técnico necesario para retomar el trabajo. Es un resumen del proyecto y sus decisiones, no una exportación íntegra del chat ni de las sesiones internas de Codex.

## Cómo retomar

1. Método vigente: clona `https://github.com/Edwin75206/DonovanPOSPRO.git` en tu Mac. También existe el ZIP de traslado preparado anteriormente, pero GitHub contiene la versión actual del trabajo.
2. Abre esa carpeta como proyecto local en Codex. La raíz correcta contiene este documento, `AGENTS.md`, `desktop/`, `assets/` y `scripts/`.
3. Usa `PROMPT-PARA-MAC.md`, o pega este mensaje cuando quieras reanudar la compilación:

   > Estoy en mi Mac M4. Lee AGENTS.md y CONTINUAR-EN-MAC.md. Continúa desde el código existente para compilar y validar Donovan POS Pro para macOS. Conserva las reglas comerciales, el punto de venta, el Dashboard y la compatibilidad de datos. No lo declares listo hasta probar el instalador y una venta completa.

4. El agente debe comprobar el entorno real de esa Mac antes de ejecutar la compilación. No asumir que tiene instaladas las herramientas ni que un ejecutable Windows puede funcionar allí.

El clon de GitHub incluye código, recursos, migraciones, pruebas, documentación, estos archivos de contexto y el historial del proyecto. Los instaladores se descargan desde GitHub Releases; no se incluyen como archivos del árbol Git. No se suben dependencias compiladas, archivos privados de Codex, contraseñas ni bases del negocio. Los ZIP de traslado anteriores son copias históricas; usa Git para continuar con la versión vigente.

## Decisiones del usuario que deben conservarse

- Producto: **Donovan POS Pro**, cliente **Construrama Chicontla**, desarrollado por **Donovan Systems / Edwin**. La entrega comercial no debe presentarse como prueba o demo. Los datos sintéticos solo se usan en instalaciones aisladas de validación.
- Desktop real: Tauri 2, React, TypeScript, Tailwind, componentes Radix/shadcn, Motion, Apache ECharts, TanStack Table y Rust. No convertirlo en una página web ni regresar a la interfaz WPF.
- La petición más reciente cambia el almacenamiento local a **MySQL Server + MySQL Workbench**, tanto en Windows como en Mac. Sustituye la preferencia inicial de SQLite como motor principal. Workbench es el administrador; el servidor MySQL guarda los datos.
- Se conserva compatibilidad con SQLite y una migración controlada, explícita y con respaldo. Nunca sobrescribir ni borrar el negocio existente para adaptar una instalación.
- Funcionamiento local sin internet para vender. El cliente final no debe instalar SDK, Node, Rust, Python ni herramientas de desarrollo. Esas herramientas solo son necesarias para construir el instalador en la Mac de desarrollo.
- Mac de destino: **Apple M4**, arquitectura `aarch64-apple-darwin`. La configuración preparada requiere macOS 15 o posterior; verificar la versión real.
- Actualizaciones por **enlace a un nuevo instalador**, sin servidor de actualizaciones. Conservar los datos persistentes durante la actualización.
- Cada computadora tiene su base independiente. No se pidió ni implementó sincronización automática entre cajas; traslado mediante respaldo verificado.
- El vendedor **Negociante Uno** entra directamente a caja. Una pantalla con lista compacta de artículos, cantidades editables, total grande, escaneo directo, buscador F2 y cobro F8. Evitar pasos y paneles innecesarios.
- El administrador conserva el Dashboard ejecutivo, KPIs, gráficas, reportes y administración de datos. No simplificarlo al trabajar en caja.
- Mantener identidad visual, modos claro/oscuro, personalización y microanimaciones discretas. El logo original y su variante preparada están en `assets/branding/`; no reconstruir ni deformar el diseño.
- Tickets con logo, folio, vendedor, artículos, cantidades, pagos y cambio. Vista previa/PDF y reimpresión sin duplicar cobros. No implementar CFDI/SAT en esta fase.

## Estado al salir de Windows

Versión del código: **2.3.0**. El instalador Windows final fue construido, instalado, abierto y desinstalado en rutas aisladas. La reinstalación y conservación de datos se verificaron; el último ejecutable mostró la misma venta, inventario y respaldo.

- Instalador: `DonovanPOS-2.3.0-Windows-x64-Instalador.exe`, 681300892 bytes.
- SHA-256: `b9c52eeb41ebd111eafe0cc7e339cdca8c2f28bab0773e29c0eed69b2df689b5`.
- Se distribuye como adjunto de GitHub Releases; la copia local está en `output/DonovanPOS-2.3.0-MySQL/` en el equipo Windows original y se excluye de Git.
- Incluye MySQL 8.4.13 y Workbench 26.10.0, runtimes necesarios y WebView2 offline. Aún no tiene firma Authenticode de Donovan Systems.
- Evidencia original: `artifacts/qa/7e0ec94f-5084-4d0e-b110-71d8185279c3/` y `artifacts/qa/df7a840e-cd65-4b4c-85b9-3446a8ae53be/`. Estas carpetas de aceptación no se incluyen en el ZIP de traslado.
- Venta real desde la interfaz instalada, con datos aislados: `V-000001`, cemento y 2.5 m de cable, total $276.25, efectivo recibido $500, cambio $223.75. Comprobación posterior: una sola venta, 42.5 m de cable y 23 unidades de cemento.
- Pruebas aprobadas: SQLite 31, React 20; además MySQL real con migración/restauración entre dos instalaciones, fallos transaccionales, equivalencia comercial, Excel, reportes, seguridad, sesiones, caja y tickets. Ver detalles en `docs/v2/VALIDACION-2.3.md`.
- Última corrección: `storage.rs::database_bytes` comparte la medición del almacenamiento entre Administración de datos y Dashboard. MySQL mostró 1.3 MB en ambos, verificado por prueba y aplicación instalada.
- Workbench y su MySQL Shell se abrieron desde la instalación. La cuenta destinada a Workbench consultó la misma venta. La primera conexión se registra manualmente con los datos de Administración de datos; no se creó una conexión GUI automáticamente.
- No se afirmó impresión física sin una ticketera conectada. Esa prueba sigue pendiente en el hardware del negocio.

## Lo preparado para Mac y lo pendiente

**No existe un instalador Mac validado.** Edwin pidió dejarlo para después y posteriormente decidió llevar la carpeta a su Mac. Preparar este traslado no constituye una compilación ni autoriza a marcar macOS como terminado.

Preparado en el código:

- Guardado de credenciales en Keychain con `security-framework`.
- Adaptador CUPS (`lpstat`, `lpoptions`, `lp`) y generador PDF compartido; Windows conserva su adaptador.
- Icono `assets/branding/app.icns`, configuración `tauri.macos.conf.json`, runtime arm64 y paquetes oficiales fijados por SHA-256.
- Script `scripts/Build-MacOS.command`: pruebas ordinarias y MySQL, aplicación Tauri, paquete PKG y Workbench. Modos `--verify` para validación local y `--release` con firma y notarización.

Cuando Edwin reanude el trabajo en la Mac:

1. Leer el script y verificar herramientas, macOS y arquitectura. Utilizar dependencias oficiales fijadas en `packaging/dependencies.json`.
2. Ejecutar desde la raíz `bash scripts/Build-MacOS.command --verify`. El script necesita internet para descargar dependencias de desarrollo y paquetes oficiales; el producto resultante debe operar offline.
3. Resolver errores nativos reales que aparezcan. El código Mac todavía no se ha compilado ni ejecutado; no asumir equivalencia de Windows como prueba de macOS.
4. Instalar en un usuario aislado de macOS y comprobar primera configuración, Keychain, inicio/cierre de MySQL, reapertura y sesión persistente.
5. Probar escaneo repetido, F2, categorías, agotados, cantidades fraccionarias, pagos y cambio; confirmar venta transaccional y actualización de inventario. Verificar que el escáner no captura contraseñas ni importes.
6. Abrir Workbench, registrar la conexión del negocio y consultar la venta. No modificar tablas mientras se atienden ventas.
7. Probar respaldo/restauración entre dos instalaciones aisladas y traslado Windows → Mac mediante `.donovanbackup`. Probar actualización sin perder información.
8. Probar vista previa, PDF, impresión y fallo de impresora sin repetir cobro; verificar controlador y formatos disponibles en hardware real.
9. Para distribuir comercialmente, completar Developer ID Application, Developer ID Installer, notarización y pruebas del PKG. No presentar un paquete sin firma de validación como entrega comercial terminada.
10. Guardar evidencia, actualizar el informe de validación y explicar límites reales. No cambiar Windows innecesariamente al corregir aspectos de macOS.

## Mapa del código

| Ruta | Responsabilidad |
|---|---|
| `desktop/src/` | Interfaz React, caja, Dashboard y configuración |
| `desktop/src/pages/DataManager.tsx` | Migración, respaldos, integridad y conexión de Workbench |
| `desktop/src-tauri/src/database.rs` | Acceso común SQLite/MySQL y transacciones |
| `desktop/src-tauri/src/mysql_dialect.rs` | Conversión controlada del dialecto SQL |
| `desktop/src-tauri/src/mysql_runtime.rs` | Instancia local privada, arranque y cierre del servidor |
| `desktop/src-tauri/src/mysql_store.rs` | Contrato, copia y comparación de datos |
| `desktop/src-tauri/src/secure.rs` | DPAPI en Windows / Keychain en Mac |
| `desktop/src-tauri/src/documents/printing*.rs` | Impresión desacoplada por plataforma |
| `desktop/src-tauri/migrations/mysql/` | DDL InnoDB, contrato y hashes de las 35 tablas |
| `desktop/src-tauri/tests/` | Regresión comercial, seguridad, portabilidad y aceptación aislada |
| `scripts/Build-MacOS.command` | Compilación, validación y empaquetado Apple Silicon |
| `packaging/dependencies.json` | URLs oficiales, versiones y SHA-256 |
| `docs/v2/MYSQL-Y-MACOS.md` | Arquitectura, archivos persistentes, tablas, traslado y compilación |
| `docs/v2/VALIDACION-2.3.md` | Qué se probó y qué continúa pendiente |

## Integridad y datos

React no debe ejecutar SQL. Las operaciones críticas pasan por Rust. Los importes son centavos enteros y las cantidades unidades de 1/10,000. Mantener permisos, transacciones, folios e idempotencia ante reintentos y errores de impresión.

MySQL utiliza una instancia privada por usuario en `127.0.0.1`, sin alterar servidores existentes. La aplicación administra su ciclo de vida. Los secretos no viajan en archivos de texto ni deben exportarse desde DPAPI a la Mac. El archivo `.donovanbackup` incluye una copia SQLite consistente y validada como formato portable, aunque la base operativa sea MySQL.

Para mover **datos del negocio**, usar Crear respaldo y Restaurar respaldo. Copiar el **proyecto de desarrollo** no migra la información comercial. No copiar carpetas de datos MySQL, archivos `.secret`, ni la carpeta personal `.codex` para trasladar el proyecto.

## Alcance del contexto

Este resumen conserva las decisiones de desarrollo vigentes. El chat también incluyó cotizaciones de computadoras y documentos comerciales; esos materiales no son necesarios para compilar la app y no se incluyen en el ZIP técnico. La documentación antigua puede describir SQLite o WPF: para la versión actual prevalecen este estado, `MYSQL-Y-MACOS.md`, el informe 2.3 y las instrucciones nuevas de Edwin.

Los chats y el acceso a archivos del proyecto son conceptos separados. La documentación oficial recomienda conservar instrucciones duraderas en `AGENTS.md` o documentación del proyecto: https://learn.chatgpt.com/docs/projects . No se depende de que este chat aparezca automáticamente al copiar una carpeta a otra computadora.

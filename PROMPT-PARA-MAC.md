# Mensaje para continuar en la Mac M4

Primero clona `https://github.com/Edwin75206/DonovanPOSPRO.git` y abre la carpeta clonada como proyecto local en Codex. Pega lo siguiente:

---

Estoy en mi Mac con chip M4. Este es el repositorio oficial de Donovan POS Pro para Construrama Chicontla.

Lee primero AGENTS.md, CONTINUAR-EN-MAC.md, docs/v2/MYSQL-Y-MACOS.md y docs/v2/VALIDACION-2.3.md. Consulta también docs/VERSIONES-Y-GITHUB.md. Usa el código actual; no reconstruyas el producto desde cero.

Quiero terminar la versión nativa para macOS: compilar, generar el instalador y validar que funciona. Windows 2.3.0 ya tiene un instalador probado; Mac todavía no. No confundas las pruebas Windows con validación Mac.

Comprueba arquitectura, macOS y herramientas, y trabaja en una rama nueva sin perder cambios existentes. Usa scripts/Build-MacOS.command y corrige los errores reales que aparezcan. Conserva Tauri 2, React/TypeScript, Rust, MySQL local y Workbench. El usuario final debe instalar sin herramientas de desarrollo y vender sin internet.

Mantén el punto de venta de una sola pantalla para Negociante Uno, el Dashboard ejecutivo del administrador, la identidad visual, tickets y reglas comerciales. No permitas SQL directo desde React, pérdida de datos, cobros duplicados ni migraciones silenciosas que sobrescriban el negocio.

Prueba una instalación nueva, venta con varios productos y fracciones, efectivo y cambio, inventario, reinicio, sesiones/Keychain, conexión de Workbench, vista previa y PDF, respaldo/restauración entre dos instalaciones y actualización conservando datos. Usa datos aislados para las pruebas. Comprueba impresión con el hardware disponible y declara cualquier prueba física que falte.

Para distribución final completa firma y notarización cuando estén disponibles los certificados. Si no están, distingue claramente un paquete de validación de una entrega comercial. No declares terminada la versión hasta validar operaciones e instalador.

Registra resultados y capturas reales, revisa el diff y conserva limpio el repositorio. Sube código, migraciones, pruebas y documentación; los instaladores deben ser adjuntos de GitHub Releases con SHA-256. No subas bases, secretos, certificados, respaldos, runtimes ni carpetas de compilación. Informa el enlace a la versión y los pendientes reales.

---

# Validación de Donovan POS Pro 2.0.0

Paquete Windows x64 para Construrama Chicontla, comprobado el 31 de octubre de 2026. La aplicación de escritorio, el instalador y los datos se validaron por separado.

## Operaciones

| Área | Comprobación |
| --- | --- |
| Ventas y persistencia | Venta completa desde Tauri, cantidades fraccionarias, cambio, pagos mixtos y crédito; reintentos sin duplicar cobros; reinicio conservando información |
| Caja, inventario y compras | Apertura y corte, movimientos, afectación transaccional de existencias, recepción por empaque, costo exacto de la recepción y costo promedio |
| Clientes y devoluciones | Abonos, límites de crédito, devoluciones parciales, asignación del reembolso y rechazo de cantidades o importes excedidos |
| Cotizaciones | Crear, editar conservando folio, vigencia, conversión única, precios acordados y rechazo de modificaciones después de vender |
| Excel | Mapeo, vista previa, transacción, duplicados, fórmulas, códigos con ceros iniciales, límites de archivos y exigencia de cero explícito al reemplazar existencias |
| Seguridad | Permisos en Rust, cuentas locales, contraseñas derivadas, sesión de 24 horas protegida con DPAPI, bloqueo y separación de solicitudes entre sesiones y bases |
| Datos | Compatibilidad estricta, migración controlada del esquema WPF, respaldo SQLite consistente con WAL, rechazo de copias corruptas y conservación de la base previa |
| Administración | Catorce reportes con filtros, KPIs, gráficas, tabla, PDF y Excel; ventas, devoluciones, cobros y crédito diferenciados |
| Documentos | Tickets de 58/80 mm, cotizaciones y compras A4, reportes horizontales y etiquetas Code 128 con logo |

La suite completa de 22 casos del motor pasó antes del último ajuste de importación. Después se reejecutaron los cinco casos de Excel, incluido el nuevo caso de existencia vacía: 23 casos distintos comprobados. Las cinco pruebas de interfaz, TypeScript, compilación de producción y comprobación Rust con escritorio pasaron. Los detalles reproducibles están en `desktop/src-tauri/tests`, `desktop/src/**/*.test.*` y `desktop/README.md`.

Se revisaron visualmente las páginas PDF generadas y la salida de Microsoft Print to PDF. Un decodificador independiente, zxing-cpp 3.1.1, leyó correctamente 51 etiquetas en seis páginas. Esta comprobación no sustituye una prueba con la impresora y el lector físicos del cliente.

## Dos instalaciones locales reales

La prueba usa datos sintéticos y directorios independientes, sin abrir bases del usuario:

1. El instalador NSIS instala el programa en A y B. Se comprueban hashes y documentación distribuida.
2. Los servicios Rust preparan el negocio y las cuentas de prueba. La ventana instalada A registra V-000001 por $235.
3. Los servicios Rust crean y validan un respaldo completo, lo restauran en B, verifican el logo, cierran las sesiones restauradas y autentican a Negociante Uno.
4. La ventana instalada B registra V-000002 por $235. Al reabrir, A conserva una venta y 19 piezas; B conserva dos ventas y 18 piezas, con integridad correcta y sesión persistente.
5. Se reinstala y desinstala el paquete. Con las aplicaciones cerradas y sin consultas autenticadas entre ambas operaciones, los 11 archivos persistentes conservan exactamente sus hashes. Abrir una sesión de validación actualiza `LastSeenAt`; esa actualización no se mezcla con la comprobación de archivos del instalador.
6. Con una venta pendiente, la reinstalación de esa copia se detiene con código 1602 y mantiene el ticket en pantalla. La otra copia puede instalarse sin cerrarla.

Scripts: `scripts/Test-NativeInstaller.ps1` y `desktop/src-tauri/examples/installation_qa.rs`. Evidencia local: `artifacts/v2/installation/47b4ee6b-396a-4cd8-b720-2be27ce42f7c`, incluidos manifiesto, hashes, capturas, comprobaciones de integridad y resultados NSIS. El ejemplo de pruebas no se incluye en el producto.

El EXE autónomo tiene el marcador Tauri `UNK`; NSIS incorpora el mismo binario con marcador `NSS`. La prueba deriva esos tres bytes en memoria y comprueba el SHA-256 esperado del archivo instalado.

## Interfaz y funcionamiento local

Se comprobaron F2/F8, F5/Ctrl+R sin pérdida del ticket, edición de cotizaciones, reportes, vista previa de documentos, tutorial y personalización. El formulario de cobro conserva controles accesibles en una ventana de 1366 × 768 con letra del 150 %. Negociante Uno abre directamente el mostrador con navegación compacta.

Las dos ventanas instaladas usaron perfiles nuevos de WebView2 con HTTP/HTTPS dirigidos a `127.0.0.1:9`, sin un proxy escuchando. La interfaz, las consultas y las ventas funcionaron con recursos locales. Se verificaron los argumentos efectivos de los procesos. No se desconectó la red del equipo ni se alteraron sus políticas de seguridad.

## Alcance y límites

- Host comprobado: Windows 11 x64 con WebView2 156.0.4314.50 previamente instalado. No se ensayó una máquina Windows limpia ni Windows 10.
- El paquete incorpora el instalador offline x64 de Microsoft, cuya firma se verificó. El gancho usa ese mismo archivo si encuentra WebView2 anterior a 120, evitando la rama de actualización online de Tauri. Se probó la lectura real de la versión del runtime; no se simuló una desinstalación de WebView2 del equipo del usuario.
- El EXE no depende de Node.js, .NET SDK ni del runtime redistribuible de Visual C++. Sus dependencias nativas se revisaron con `dumpbin`; WebView2 se distribuye mediante el instalador.
- Los documentos se validaron con PDF y Microsoft Print to PDF. No había impresora térmica ni lector físico disponibles para una comprobación de hardware.
- El paquete de Donovan POS Pro no tiene firma Authenticode. Se necesita un certificado de publicación para firmar una distribución posterior.
- Las bases son locales; no existe sincronización entre cajas. El traslado del negocio se realiza mediante respaldo y restauración.

El manual de operación y la documentación de datos acompañan la entrega. La referencia WPF 1.0 permanece en el repositorio y en su etiqueta original.

# Validación del punto de venta 2.1

Comprobaciones del 31 de octubre de 2026, en Windows x64. Se utilizaron bases sintéticas aisladas; las bases y ventanas de trabajo del usuario se conservaron.

## Recorrido real en la aplicación

Desde la ventana nativa Tauri, Negociante Uno abrió directamente el punto de venta. Se introdujo dos veces el código exacto del cemento, se buscó el cable con el teclado, se editó su cantidad a 2.5 y se agregó una brocha. Las categorías y el cambio de tarjetas a lista conservaron el carrito. Se revisaron las etiquetas de existencia, stock bajo y producto agotado.

| Partida | Cantidad | Precio | Importe |
| --- | ---: | ---: | ---: |
| Cemento gris CPC 30R | 2 pza | $235.00 | $470.00 |
| Cable THW calibre 12 | 2.5 m | $18.50 | $46.25 |
| Brocha profesional 2 pulgadas | 1 pza | $48.00 | $48.00 |
| **Total** | | | **$564.25** |

F8 abrió el cobro integrado. Con **$1,000.00 recibidos**, la pantalla y el comprobante mostraron **$435.75 de cambio**. La operación se guardó como **V-000019** y el carrito quedó preparado para otro cliente. La consulta posterior verificó 19 ventas: 18 de demostración y exactamente una nueva. Las existencias de cemento, cable y brocha disminuyeron respectivamente en 2, 2.5 y 1; la integridad y compatibilidad de SQLite fueron correctas.

El mismo folio se abrió desde Ventas recientes. Se exportó el PDF desde el selector de carpeta de Windows, sin otra venta ni cobro. Las capturas iniciales están en `artifacts/v2.1/capturas`. El recorrido completo se repitió sobre la instalación final y se documentó en `artifacts/v2.1/capturas-final`, incluyendo el PDF y el modo oscuro. Las capturas son de la aplicación, no maquetas.

Se revisó la caja a **1366 × 768**: búsqueda, categorías, total y Cobrar permanecen accesibles, con desplazamiento independiente de catálogo y carrito. También se revisó maximizada en un monitor **1920 × 1080** (área de ventana de 1920 × 1032 por la barra de tareas). La captura con las tres partidas completas usa una ventana de 1366 × 1032. El dashboard administrativo mantiene sus KPIs y gráficas.

## Pruebas automatizadas

**14 casos de interfaz aprobados**, además de TypeScript y compilación de producción. Cubren el intérprete HID con Enter/Tab, lectura repetida, selección manual deliberada, categorías, conservación del carrito, existencias restantes, agotados, cantidades fraccionarias, exceso de stock, F8, cambio, pagos combinados, protección de campos de pago frente al escaneo y doble clic de confirmación. Un aviso de artículo agregado se limpia cuando el carrito queda vacío.

**27 casos distintos del motor Rust aprobados.** La suite completa de 26 pasó; después del ajuste del diagnóstico de impresión se repitieron sus tres casos específicos. Se añadió y aprobó el caso de migración 4 → 5, venta en espera tras reinicio y restauración del nuevo perfil de ticket. Se conservan las comprobaciones anteriores de venta transaccional, idempotencia, crédito, descuentos, devoluciones, inventario, compras, cotizaciones, importación Excel, sesiones y permisos.

Pruebas específicas nuevas: `desktop/src-tauri/tests/register_and_tickets.rs`, `desktop/src/lib/register.test.ts`, `desktop/src/pages/Register.test.tsx` y `desktop/src/pages/PaymentForm.test.tsx`. Los comandos de ejecución se documentan en `desktop/README.md`. Los ejemplos de QA están separados y no se distribuyen como comandos del producto.

## Impresión y documentos

- Enumeración real de impresoras instaladas, consulta de controlador, cola y tamaños anunciados por Windows. La interfaz advierte que ese estado no confirma la salida física del papel.
- Perfil persistente exclusivo del administrador: 58/80 mm, márgenes de 2 a 8 mm, de 1 a 3 copias, logo, monocromía, agradecimiento, leyenda e impresión automática con impresora física configurada.
- Vista previa con cambios antes de guardar y comprobante de prueba que no registra ventas ni mueve inventario.
- PDF nativo de 58 y 80 mm; revisión visual de todas las páginas y del ticket real. Se conserva la geometría del logo original; la variante monocromática se calcula al renderizar.
- Salida GDI a Microsoft Print to PDF, incluyendo dos copias. Se revisaron ambas páginas. La impresora virtual usa su papel A4; el contenido conserva el ancho solicitado del ticket.
- Rechazo de impresora inexistente sin repetir la transacción y detección del estado desconectado simulado. La venta real se completó sin una impresora de tickets configurada.

El controlador puede ignorar un tamaño personalizado. Debe probarse el papel antes de activar la impresión automática. Tarjeta y transferencia son registros manuales; estos comprobantes no son CFDI.

## Instalador y datos

El paquete NSIS incluye WebView2 offline, licencias, manual, documentación de datos y las cinco migraciones. Se comprobó la instalación y apertura de la aplicación instalada, la identidad del ejecutable y sus recursos. El instalador bloqueó la reinstalación de su copia abierta con código 1602. Con las aplicaciones cerradas, reinstalación y desinstalación conservaron los hashes de todos los archivos persistentes de las dos bases de prueba.

Evidencia del primer candidato 2.1: `artifacts/v2/installation/7f94cfb8-5631-4962-a26c-349b68364b4f` y `artifacts/v2.1/preview/d8243c8e-f190-4279-9825-9b4de0f44170`.

La instalación final se comprobó en `artifacts/v2/installation/77d7f6e3-1c63-4d08-b057-a5ce459b6aee`, con venta, sesión, existencias e integridad verificadas en `artifacts/v2.1/preview/8a7a96fc-f55b-47b3-aa7c-2497e3ee04b7`. Su instalador tiene SHA-256 `18f0952a8b0127135733302449e990668e6cc8e283bedab37f6c90bc63c0650f`; el EXE autónomo tiene SHA-256 `333c9e89139a789c2fbcbc91861c496d0df3817a93077c47dc7fcc8fff5b4382`. Los resultados NSIS y los hashes de los datos se conservan en esa carpeta. El manifiesto de entrega incluye copias de las evidencias relevantes.

La migración 5 conserva ventas, existencias y ventas en espera; antes de actualizar una base 4 genera una copia de seguridad verificada. El respaldo y la restauración conservan también las opciones del ticket. La prueba anterior de traslado entre dos instalaciones reales permanece documentada en [VALIDACION.md](VALIDACION.md).

## Límites de la validación

- No se dispone del lector físico ni de la impresora térmica del negocio. Se simularon las pulsaciones HID y los estados de error; no se afirma una prueba física de desconexión, corte de papel o apertura de cajón.
- El equipo de prueba ya dispone de WebView2. El redistribuible offline está empaquetado, pero falta una prueba en una máquina Windows limpia sin ese componente.
- El paquete no tiene firma Authenticode. La identificación comercial no sustituye un certificado de editor.
- Los recursos, fuentes, lógica y datos se ejecutan localmente. La prueba de bloqueo HTTP/HTTPS de la versión 2.0 está documentada en su informe; la sesión gráfica 2.1 se ejecutó con la conexión normal del equipo y no se presenta como una nueva prueba de red desconectada.
- El carrito se conserva al buscar, filtrar y navegar de manera segura. **En espera** guarda explícitamente en SQLite y sobrevive al reinicio. Un carrito aún no guardado no se presenta como recuperable tras un corte de corriente.

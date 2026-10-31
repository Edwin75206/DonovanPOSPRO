# Reconstrucción 2.0 — registro de validación

La versión 1.0 WPF y sus pruebas permanecen en el repositorio como referencia funcional. La nueva aplicación vive en `desktop` y usa Tauri 2, React, TypeScript y servicios Rust con SQLite. Este archivo registra hitos comprobados; no representa una declaración de entrega final.

## Primer flujo completo (30 de octubre de 2026)

- El motor Rust conserva el esquema de negocio 1.0, importes en centavos y cantidades escaladas por 10,000.
- La migración 2 añade sesiones, preferencias del tutorial y recibos de operación; no sustituye los historiales.
- Prueba automatizada: venta de 2.5 metros a $18.50, total $46.25, cambio $3.75, reinicio conservando venta y sesión. Reintentar con la misma referencia no duplica cobro ni inventario.
- Prueba desde la ventana Tauri con recursos incluidos (sin servidor de desarrollo): brocha $48.00 y cable $18.50; F8, efectivo $100.00, venta V-000019 por $66.50 y cambio $33.50.
- Verificación posterior de SQLite: 19 ventas (18 de muestra más la nueva), brochas 31 piezas, cable 446 metros, `integrity_check=ok`.
- Demostración aislada en `%TEMP%/DonovanPOS2-Demo`; nunca se utilizaron datos productivos.
- El logotipo original se copia sin modificaciones. El nombre y colores iniciales se leen desde `assets/branding/customer-profile.json` y se convierten en configuración editable del negocio.

![Confirmación real de la primera venta](images/primera-venta.png)

## Motor comercial y portabilidad local (31 de octubre de 2026)

- Compra por presentaciones con costo promedio, cotización con precio acordado, conversión única, crédito, abonos, devolución parcial, cancelación y venta suspendida comprobados en pruebas de integración. Las operaciones fallidas revierten inventario, caja y documentos en conjunto.
- Caso de redondeo comprobado: devolución de tres unidades con reembolsos de 67, 66 y 67 centavos; reducción de crédito y devolución de lo pagado sin exceder el importe original.
- Sesión bloqueada conservada después de reiniciar; límite fijo de 24 horas, revocación de tokens y permisos de cajero comprobados desde Rust. Los costos no se entregan al vendedor.
- Apariencia independiente por usuario: seis temas, modo de Windows, color, fondo local, densidad, animaciones y escala de letra. La migración 3 conserva las preferencias en SQLite y no modifica la identidad de los documentos.
- Respaldo SQLite consistente que incluye operaciones confirmadas en WAL, logotipo, usuarios y preferencias. El archivo contiene manifiesto y base con comprobación SHA-256; no transporta la sesión de Windows.
- Restauración probada entre dos carpetas de datos aisladas: autenticación con las cuentas conservadas, reinicio y una nueva venta en el destino, sin modificar la instalación de origen.
- Rechazo de copia corrupta y SQLite ajeno; fallo simulado al cambiar el archivo de activación mantiene utilizable la base anterior y conserva el respaldo preventivo. Una base configurada desaparecida no se reemplaza por otra vacía.
- Migración de una base con el esquema WPF original: mantiene el archivo de origen sin cambios, las contraseñas y las preferencias de tema y movimiento.
- Excel: asignación de columnas, revisión sin escritura, políticas explícitas de precio y existencia, transacción única e idempotencia. Se probaron duplicados, fórmulas, códigos numéricos, hoja dispersa fuera de límites y cambio de catálogo entre revisión y confirmación.
- Exportación `.xlsx` conservando ceros iniciales y lectura posterior sin diferencias de precios o cantidades. La plantilla incluye instrucciones y el logotipo del negocio.

Las pruebas de portabilidad anteriores corresponden al motor sobre carpetas aisladas. **No sustituyen la prueba pendiente de dos instalaciones reales con el instalador Windows.**

## Operación de mostrador y documentos (31 de octubre de 2026)

- Interfaz de clientes, proveedores, compras, crédito y abonos, cotizaciones, ventas suspendidas, historial y devoluciones conectada a servicios Rust. Los precios acordados de una cotización se muestran y cobran sin sustituirlos por el precio actual del catálogo.
- La migración 4 conserva el desglose exacto de nuevas compras por empaque y agrega subtítulo comercial e impresión automática. Los registros históricos que no contienen ese desglose se identifican como tales.
- Configuración del negocio, usuarios y permisos, tutorial opcional por perfil y avisos al salir con cambios pendientes integrados. La personalización individual no modifica el papel.
- PDF nativo con tipografía incluida, texto seleccionable en español, logotipo transparente, tickets de 58/80 mm, cotizaciones y comprobantes de compra A4. Se revisaron visualmente los documentos y las cinco páginas de un ticket de 35 partidas largas.
- Impresión mediante GDI y la cola de Windows verificada con Microsoft Print to PDF. El controlador imprime fuera del bloqueo de la base y no vuelve a ejecutar una venta. Esta prueba no representa una prueba en una impresora térmica física.
- Exportación con nombres únicos y publicación del archivo completo sin sobrescribir documentos. Las impresoras virtuales usan la opción explícita Guardar PDF.
- Aperturas, ingresos/egresos y cierres tienen referencias idempotentes. Se comprobó que un reintento no duplica dinero ni cierra una jornada abierta posteriormente.
- El historial ordena en SQLite antes de paginar. Prueba con 62 ventas y permisos de otro usuario; se rechazan columnas de ordenamiento ajenas a la lista permitida.
- Las respuestas y solicitudes en espera se vinculan a la sesión y a la base activa. Una respuesta de la cuenta anterior no entrega sus datos ni cierra la sesión nueva.
- Suite completa del motor: 18 pruebas aprobadas antes del último ajuste de aislamiento de sesión. Después del ajuste se aprobaron las cuatro pruebas de seguridad, incluida la nueva prueba de solicitudes entre sesiones: 19 casos distintos comprobados. La interfaz tiene cinco pruebas aprobadas.

Continúan pendientes los reportes y etiquetas, la revisión nativa completa de las pantallas recientes y la validación del instalador con dos instalaciones aisladas. La versión 2.0 todavía no se declara terminada.

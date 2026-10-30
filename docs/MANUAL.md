# Manual de Donovan POS Pro 1.0

## Primera configuración

El asistente tiene cuatro pasos: negocio, administrador, preferencias e impresión/respaldos. Define tu usuario y una contraseña de al menos diez caracteres. Guarda estas credenciales en un lugar seguro; el programa no crea una contraseña maestra ni una cuenta secreta. Tras cinco intentos incorrectos, la cuenta se bloquea temporalmente durante cinco minutos.

En Configuración puedes cambiar nombre, dirección, teléfono, correo, RFC, pie de ticket, prefijo de venta, IVA predeterminado, ancho de ticket, impresora y carpeta de copias. El logotipo PNG/JPEG admite hasta 2 MB y 4096 × 4096 píxeles; queda incorporado en la base y en los respaldos. El monograma D que identifica el programa es un recurso propio del proyecto, no el logotipo definitivo de una ferretería.

La tasa predeterminada sirve para productos nuevos. Cada producto conserva su propia tasa. Los precios de venta y mayoreo se capturan **con IVA incluido**; el costo se captura **sin IVA**. Cambiar la tasa general no reescribe ventas ni productos existentes. Verifica los datos que correspondan a tu negocio.

## Productos e inventario

Cada variante debe tener una clave independiente: por ejemplo, tornillo de 1/4 y tornillo de 3/8 son dos productos. Captura nombre, categoría, marca, medidas, unidad base, código de barras opcional, costo, precios, mínimo y presentación. No se permiten claves ni códigos duplicados. Los alias ayudan a buscar sin modificar el nombre del ticket: pija, tornillo, perno, etc.

La unidad base representa el inventario. Las piezas requieren cantidades enteras; las unidades medibles admiten hasta cuatro decimales. Una presentación indica cuántas unidades base contiene: por ejemplo, una caja de cien piezas descuenta cien piezas. No mezcles metros y rollos sin definir su equivalencia. El mínimo y la sugerencia de compra también se expresan en la unidad base.

**Inventario** muestra disponibles, existencia baja y agotados. “Reponer” indica la diferencia para alcanzar el mínimo configurado; no es una predicción de demanda. El detalle de movimientos conserva entradas, ventas, ajustes y devoluciones. Los ajustes requieren motivo. Archivar oculta un producto de la venta y conserva su historial.

En **Compras**, selecciona o crea un proveedor, referencia del documento, fecha y partidas. Una recepción actualiza existencias y costo promedio ponderado en una sola operación. Si se paga desde caja, también registra el egreso de efectivo y requiere caja abierta. No dupliques ese egreso en movimientos manuales. La combinación proveedor/referencia se controla para evitar registrar la misma compra dos veces.

## Excel y etiquetas

En Productos, descarga **Plantilla**. Conserva exactamente los encabezados y las unidades de la plantilla. Se admiten hasta 10,000 filas y un archivo de 15 MB; las fórmulas deben convertirse a valores. Conserva códigos de barras como texto para no perder ceros iniciales. La importación muestra errores por fila y duplicados antes de confirmar. Si alguna fila es inválida, no se importa parcialmente el archivo. La importación crea productos; no reemplaza un catálogo existente.

Exportar Excel genera un archivo con filtros, encabezados y columnas adaptadas. Los importes y cantidades reconocidos se escriben como números; claves y códigos permanecen como texto.

Selecciona un producto y **Etiquetas**, define el número de copias y guarda el PDF. Se generan hojas A4 con dieciséis etiquetas por página, máximo quinientas por archivo. El código usa Code 128 y admite caracteres ASCII; los códigos excesivamente largos se rechazan para mantener barras legibles. Imprime al 100 % y comprueba una etiqueta con tu lector antes de imprimir el lote. No está ligado a una marca comercial de hoja adhesiva.

## Vender

1. Abre **Caja** y captura el fondo inicial contado.
2. En **Punto de venta**, busca por descripción, marca, medida o varias palabras: `tornillo 1/4`, `cable calibre 12`. La búsqueda ignora diferencias entre mayúsculas y acentos. Puedes filtrar por categoría.
3. Selecciona un producto o lee su código. El lector debe enviar el código como teclado y terminar con Enter. Revisa la cantidad antes de agregar.
4. Elige cliente o deja Público en general. Ajusta las cantidades con los controles de la partida. Para mayoreo utiliza el precio configurado; los permisos se verifican antes de operaciones restringidas.
5. Si corresponde, aplica el descuento. El total incluye IVA y se calcula con redondeo a centavos.
6. Pulsa **Cobrar venta**. Distribuye el total entre efectivo, tarjeta, transferencia o crédito. En efectivo, captura también cuánto entrega el cliente para calcular cambio. No entregues cambio de tarjeta, transferencia ni crédito.
7. Confirma. El programa vuelve a comprobar existencias, permisos, caja y crédito antes de guardar. Una venta confirmada no se vuelve a cobrar al reintentar la misma solicitud.
8. Abre el PDF o la vista previa del ticket. Imprimir es una acción separada de confirmar la venta: una falla de impresora no debe hacerte registrar la venta otra vez. Reimprime desde Historial.

Las tarjetas y transferencias se anotan en el sistema; verifica por separado que el pago haya sido recibido. En crédito se requiere cliente, límite disponible y vencimiento. Una venta no permite inventario negativo.

## Suspender y recuperar

Usa **Suspender** para conservar una venta sin cobrar. Se guarda en SQLite con sus partidas, cliente y descuento; no reserva ni descuenta existencias. Puedes cerrar y volver a abrir el programa y recuperarla. Cada cajero ve sus ventas pendientes; el administrador puede revisar todas.

Al recuperar, revisa los avisos: los precios o existencias pueden haber cambiado. La venta final valida nuevamente el inventario; recuperar una venta no garantiza que el producto siga disponible. Una suspendida cobrada queda vinculada a la venta y no se puede cobrar dos veces. Puedes descartarla explícitamente cuando ya no sea necesaria.

El carrito que todavía no se ha suspendido o guardado como cotización permanece en memoria. El cierre y la salida de sesión avisan si lo vas a descartar. Cambiar de módulo conserva ese carrito durante la sesión.

## Cotizaciones

Guarda el carrito como cotización, con cliente opcional, vigencia y notas. Una cotización no mueve caja ni inventario. Puedes consultar su PDF y editarla mientras esté disponible. Al convertirla se conservan los precios acordados y se valida vigencia, disponibilidad y que no se haya vendido antes. El cobro produce una venta con referencia a la cotización.

## Clientes, crédito y devoluciones

En Clientes registra los datos y el límite de crédito. Consulta ventas pendientes, saldo y vencimientos. Los abonos pueden ser en efectivo, tarjeta o transferencia, asociados a la venta correspondiente. Un abono en efectivo requiere caja abierta y mueve caja automáticamente. No registres un segundo ingreso manual por ese mismo abono.

Desde Historial selecciona una venta para ver sus partidas, pagos, devoluciones y documentos. La devolución parcial requiere cantidad y motivo; la cancelación devuelve lo pendiente de todas las partidas. Solo se reintegran las cantidades efectivamente devueltas. Los importes acumulados se limitan al cobro original, incluyendo el redondeo de devoluciones parciales.

Si existe crédito pendiente, la devolución reduce primero la deuda. La parte ya pagada se reembolsa según la operación registrada. Un reembolso en efectivo afecta el corte de caja. Cancelar no borra la venta: conserva su folio y su trazabilidad.

## Caja y reportes

Solo puede existir una caja abierta por base. Los ingresos/egresos manuales requieren importe y motivo: positivo para ingreso, negativo para egreso. Ventas, compras pagadas desde caja, abonos y reembolsos generan sus movimientos automáticamente.

Para cerrar, cuenta únicamente el efectivo físico e introdúcelo en **Cerrar caja**. El sistema muestra lo esperado y la diferencia. El historial conserva ambos valores y las notas del corte. Una diferencia se documenta; no se altera una venta antigua para ocultarla.

Los filtros del dashboard y Reportes consultan la base real. Las ventas netas descuentan devoluciones según las fechas de los movimientos. Las gráficas por categoría y método muestran la composición del período; las métricas de caja y cartera representan su estado correspondiente. El resumen de utilidad depende del costo registrado y no equivale a un estado contable fiscal. Revisa los avisos de costos faltantes antes de interpretarlo.

Exporta los reportes visibles a Excel o PDF desde el módulo correspondiente. Los reportes extensos pueden abarcar varias páginas. Los comprobantes impresos y PDF no son CFDI.

## Impresora y ticket

Instala primero el controlador del fabricante en Windows. Elige el ancho correcto, 58 u 80 mm, en Configuración. Abre la vista previa y selecciona la impresora en el diálogo de Windows. Comprueba tamaño de papel, márgenes y área imprimible: dependen del controlador. El programa limita el contenido al área imprimible comunicada por la impresora.

Antes del uso diario, imprime una venta corta y otra con nombres largos, cantidades fraccionarias y descuento. Confirma que no se corten total, cambio ni pie del ticket. No se envían comandos ESC/POS de corte automático ni de apertura de cajón en esta versión.

## Respaldar y restaurar

La carpeta de datos real es `%LOCALAPPDATA%\DonovanSystems\DonovanPOS\data`. La base está separada del ejecutable. No copies únicamente el archivo SQLite mientras se está usando: usa el respaldo del programa, que incorpora las transacciones del registro WAL.

En Configuración elige una carpeta de copias. **Crear respaldo** produce una copia íntegra, comprueba relaciones y registra fecha, tamaño, resultado y SHA-256. El historial muestra las últimas cien operaciones. Las copias automáticas comprueban si corresponde respaldar al iniciar sesión y cada cinco minutos mientras el programa permanece abierto, respetando el intervalo configurado. No funcionan con el programa cerrado. No se eliminan copias antiguas automáticamente.

Para restaurar, selecciona una copia de Donovan POS. El programa valida integridad, relaciones y compatibilidad, prepara la base restaurada, conserva una copia `antes-de-restaurar-...db` y reemplaza la base. La sesión se cierra: entra con un usuario que exista en ese respaldo. Los usuarios y contraseñas también se restauran. Los respaldos inválidos se rechazan sin reemplazar los datos existentes. Se crea igualmente una copia previa antes de aplicar migraciones a una base existente.

Mantén una copia fuera del disco del equipo y comprueba periódicamente que puedes restaurarla en una carpeta de prueba. Los respaldos contienen información de clientes y del negocio; su acceso depende de los permisos de la carpeta. No están cifrados por la aplicación.

## Usuarios, apariencia y teclado

El administrador administra catálogo, compras, clientes, usuarios, reportes, configuración y devoluciones. El cajero tiene acceso a venta, cotizaciones y operaciones de caja; las restricciones se aplican también en los servicios, no solo ocultando botones. Desactiva usuarios que ya no deban operar. No se puede desactivar al último administrador activo.

La luna del encabezado cambia el tema. En Configuración se guarda la preferencia y se puede reducir movimiento. Los estados de inventario utilizan texto además de color. En pantallas estrechas la barra lateral se compacta y conserva las descripciones al colocar el cursor.

| Tecla | Acción |
|---|---|
| F2 | Punto de venta |
| F3 | Buscar producto en el punto de venta |
| F4 | Productos, si el usuario tiene permiso |
| F6 | Cantidad de la partida seleccionada |
| F7 | Descuento de la venta |
| F8 | Cobrar venta |
| F9 | Consulta rápida de precio |
| F10 | Suspender venta |

## Soporte y actualización

La bitácora registra acciones de negocio y las incidencias técnicas se escriben en la carpeta `logs` de los datos. Los errores técnicos no muestran trazas completas en la pantalla del cajero. Si una operación falla, revisa su estado en Historial antes de volver a registrarla.

Antes de actualizar, termina operaciones pendientes y crea un respaldo. La actualización aplica migraciones incrementales y conserva los datos. No reemplaces la base con una vacía para actualizar. La desinstalación quita el programa y sus accesos directos; no borra la carpeta de datos ni los respaldos. El paquete ZIP puede ejecutarse en una carpeta con permiso de escritura, pero los datos siguen guardándose en LocalAppData salvo que se use `--data-dir`.

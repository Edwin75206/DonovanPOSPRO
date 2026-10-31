# Donovan POS Pro — guía de uso

Construrama Chicontla · Donovan Systems · Versión 2.0

## Instalar y empezar

Ejecuta el instalador x64 en Windows 10 u 11. Incluye los componentes necesarios; no debes instalar herramientas de programación. Elige la carpeta del programa y finaliza la instalación. La información del negocio se conserva en una carpeta independiente de los archivos instalados.

Al abrir por primera vez, completa el asistente inicial para comenzar desde cero o selecciona **Ya tengo un respaldo** para recuperar un negocio existente. Revisa el nombre, domicilio, logo y ancho del ticket. Crea las cuentas de administrador y de operación, con contraseñas propias. Conserva las credenciales de administración en un lugar seguro.

El tutorial de bienvenida es opcional. Puedes pausarlo o volver a abrirlo desde **Ayuda y tutoriales**. Si aparece la franja DEMOSTRACIÓN, estás trabajando con datos de ejemplo y debes abrir la instalación normal para registrar ventas reales.

## Administrador y Negociante Uno

El administrador inicia en el tablero: ventas netas, efectivo esperado en caja, deuda de clientes, ganancias estimadas y alertas de inventario. Cambia el periodo o pulsa una gráfica o un enlace de detalle para revisar las operaciones que explican el indicador.

Negociante Uno inicia en el punto de venta. Su perfil operativo permite atender el mostrador y consultar sus operaciones; las opciones administrativas y los costos requieren los permisos correspondientes. Cambiar la apariencia de una cuenta no modifica la de los demás usuarios.

La sesión persiste al cerrar y abrir el programa, hasta un máximo fijo de 24 horas desde el inicio de sesión. Bloquea la pantalla cuando te alejes del mostrador. Recuperar una base en otro equipo requiere iniciar sesión nuevamente.

## Hacer una venta

1. Abre **Caja** e indica el efectivo inicial. La aplicación te avisa si intentas cobrar sin una jornada abierta.
2. En **Punto de venta**, escanea un código o busca por nombre, clave, marca o medida. Pulsa un producto para agregarlo.
3. Ajusta cantidades, selecciona al cliente si hace falta y revisa el total. Los precios muestran IVA incluido. Los descuentos y el mayoreo requieren permiso.
4. Pulsa **Cobrar venta** o F8. Selecciona el método de pago, o combina métodos. Para efectivo, indica lo recibido y revisa el cambio. El crédito requiere cliente, límite disponible y vencimiento.
5. Confirma el cobro una sola vez. La pantalla muestra el folio guardado y permite revisar el ticket. Si la impresión falla, reimprime ese folio desde **Ventas**.

**Atajos:** F2 abre el punto de venta; F3 enfoca la búsqueda; F8 abre el cobro; F1 abre la ayuda. Durante la edición de una cotización, F8 abre su guardado. F5 y Ctrl+R no recargan la aplicación para proteger el ticket en curso.

**Guardar** deja una venta en espera sin cobrar ni apartar existencias. Recupera el carrito desde **En espera**; la aplicación revisa precios y disponibilidad. Guarda el trabajo pendiente antes de cerrar o cambiar de base.

## Cotizaciones

Prepara las partidas desde el punto de venta y pulsa **Cotizar**. Revisa cliente, vigencia y observaciones antes de guardar. La cotización conserva su precio acordado y no modifica el inventario.

Desde **Cotizaciones**, abre el detalle para generar el PDF o continuar al cobro con los importes acordados. Una propuesta vencida debe renovarse antes de vender. **Editar partidas y vigencia** abre una revisión con precios actuales de menudeo y descuento inicial en cero; la propuesta anterior se conserva hasta pulsar **Guardar cambios**. Una cotización convertida en venta conserva su historial y no puede volver a convertirse.

## Inventario, compras y clientes

En **Inventario** busca y filtra productos, revisa existencias y abre el historial de movimientos. Un ajuste requiere motivo. Archivar un producto lo retira de nuevas operaciones sin borrar ventas anteriores. Los códigos de barras se imprimen desde la opción de etiquetas: hoja A4, escala 100%, etiquetas de 91 × 33 mm.

En **Compras** elige proveedor, referencia, fecha y productos. Puedes recibir unidades o empaques configurados. Revisa cantidades, costos y si el pago debe salir de la caja. La recepción confirma inventario y costo en conjunto.

En **Clientes** consulta el saldo y las ventas pendientes de cada cuenta. Un abono se aplica a la venta seleccionada y conserva su referencia. El límite de crédito controla nuevas ventas a crédito; archivar una cuenta no elimina la deuda existente.

## Devoluciones y caja

Desde **Ventas**, abre el folio original. Selecciona cantidades a devolver, escribe el motivo y revisa el método de reembolso. La aplicación limita la devolución a lo pendiente, reintegra las existencias y distingue entre reducir una deuda y devolver dinero ya cobrado. La cancelación conserva un historial del documento original.

En **Caja** registra ingresos y egresos con motivo. Al terminar la jornada, cuenta el efectivo e indica el importe real. El cierre compara lo contado con el efectivo esperado y conserva la diferencia. Una compra pagada desde caja y un reembolso en efectivo afectan ese saldo; un pago por tarjeta no agrega billetes a la caja.

## Reportes para decidir

Los reportes muestran su alcance junto al título. Ventas y devoluciones afectan la fecha en que se registraron. **Cobros por método** separa dinero recibido de crédito pendiente. **Inventario actual** y **Cuentas por cobrar** muestran el saldo actual, no una reconstrucción histórica. **Costos y ganancias** es una estimación sin IVA y antes de gastos; un costo desconocido se muestra como pendiente.

Filtra el reporte, revisa sus indicadores y exporta Excel o PDF. Los documentos usan el logo del negocio. Las etiquetas, reportes, cotizaciones y tickets tienen controles propios para guardar e imprimir; el tema de la interfaz no cambia el papel.

## Respaldar y cambiar de equipo

Entra como administrador en **Configuración → Administración de datos**. Allí puedes ver la ubicación y versión de la base, comprobar su integridad, crear respaldos, revisar errores e importar o exportar información.

Para cambiar de computadora, crea un respaldo completo y copia el archivo `.donovanbackup` a un medio externo. Instala Donovan POS Pro en el destino y restaura ese archivo desde la bienvenida. Revisa el negocio y las cantidades de registros antes de confirmar. Inicia sesión con las cuentas conservadas y configura la impresora del nuevo equipo.

Los respaldos automáticos funcionan mientras Donovan POS Pro está abierto. Conserva también una copia fuera del disco de la computadora. Reinstalar el programa no recupera un disco perdido sin respaldo. El archivo contiene información del negocio y debe guardarse con acceso restringido.

## Importar productos desde Excel

Usa la plantilla del sistema o selecciona tu archivo desde Administración de datos. Elige hoja y columnas; conserva los códigos como texto si incluyen ceros iniciales. Revisa la vista previa y corrige los errores. Las políticas iniciales conservan precios y existencias existentes; cambiarlas requiere una elección explícita. Solo la confirmación final escribe los productos, en una sola operación.

## Personalizar y resolver un problema

En **Apariencia**, elige tema, modo claro/oscuro/sistema, tamaño de letra, densidad, movimiento y fondo. Los cambios se guardan para tu cuenta. La identidad comercial y el logo de los tickets se cambian desde la configuración del negocio.

Si una operación muestra un error, anota su referencia y revisa el estado antes de repetirla. Una venta con folio ya está registrada. Ante un error de almacenamiento, el administrador debe consultar **Administración de datos → Diagnóstico** y comprobar integridad y espacio disponible. Para recuperar información, usa un respaldo verificado. No borres archivos de la base ni cambies tablas directamente para corregir un mensaje.

SQLiteStudio queda reservado para diagnóstico técnico sobre una copia o con Donovan POS Pro cerrado. La guía técnica **DATOS-Y-PORTABILIDAD** documenta tablas, relaciones, migraciones y archivos.

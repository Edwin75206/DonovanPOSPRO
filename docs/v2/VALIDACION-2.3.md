# Validación de Donovan POS Pro 2.3.0

Fecha: 9 de noviembre de 2026. Cliente: Construrama Chicontla.

## Motor, integridad y equivalencia

La validación usa instalaciones aisladas y datos sintéticos que no se incluyen en el instalador comercial.

- Regresión SQLite: 31 pruebas aprobadas (5 unitarias y 26 de integración).
- Interfaz React: 20 pruebas aprobadas; compilación TypeScript y Vite correcta.
- MySQL 8.4.13 real: migración desde SQLite, comparación SHA-256 por tabla, venta con fracciones, reintento sin duplicados, reinicio y respaldo/restauración entre dos instalaciones independientes, aprobados.
- Compras, costos, devoluciones, crédito, cotizaciones, reportes, Excel y documentos: 13 pruebas de integración aprobadas sobre MySQL.
- Caja y tickets: 3 pruebas sobre MySQL aprobadas; incluyen escaneo exacto, producto agotado, pagos combinados y fallo de impresión sin repetir cobro.
- Fallos: se comprobó que la cuenta de operación no puede alterar tablas, que el vendedor no obtiene credenciales de Workbench, que una tabla modificada se rechaza y que cortar la conexión durante una transacción no deja datos parciales.
- Sesiones, bloqueo, caducidad, permisos de vendedor y personalización: 4 pruebas adicionales aprobadas sobre MySQL. Las mismas 4 se repitieron correctamente en SQLite tras adaptar el reinicio de las pruebas al motor elegido.
- Se corrigió y volvió a probar el reinicio inmediato: el cierre espera a que InnoDB termine y retire su archivo PID antes de volver a abrir.
- El adaptador CUPS compila en la prueba de plataforma y su selección de formatos anunciados por el controlador pasa la prueba unitaria. Esto no valida impresión real en Mac.

Los tests `mysql_portability` y `mysql_failures` se ejecutan explícitamente con `--ignored`; el atributo evita que el conjunto ordinario instale o arranque un motor externo inadvertidamente. `DONOVAN_TEST_MYSQL_RUNTIME` apunta exclusivamente al runtime oficial de validación.

## Paquetes y plataforma

Los cuatro paquetes de Oracle (MySQL y Workbench, Windows x64 y macOS arm64) están fijados mediante SHA-256 en `packaging/dependencies.json`. Las firmas Authenticode del servidor y del MSI Windows se verificaron como válidas de Oracle. Los componentes Visual C++ distribuidos son bibliotecas de ejecución firmadas de Microsoft.

La instalación comercial queda vacía hasta configurar o restaurar el negocio. No contiene contraseñas de prueba, bases de aceptación ni los archivos privados de desarrollo.

La actualización a MySQL conserva las bases SQLite originales y exige una migración autorizada por el administrador. Restaurar o crear un negocio nuevo genera otro esquema; no sobrescribe el anterior.

## Aplicación instalada en Windows

La prueba real se ejecutó desde una instalación NSIS, con datos fuera de la carpeta del programa. La primera apertura creó MySQL y mostró el asistente con la identidad de Construrama Chicontla. Después se prepararon cuentas y productos sintéticos únicamente en la carpeta de aceptación.

Desde la interfaz de Negociante Uno se seleccionó cable en el buscador F2, se agregó cemento mediante su código, se editó el cable a 2.5 metros y se cobró en efectivo. La venta `V-000001` quedó en $276.25, con $500 recibidos y $223.75 de cambio. La caja quedó vacía para la siguiente venta y se abrió el ticket con el logotipo monocromático del negocio.

La comprobación posterior encontró exactamente una venta y las existencias esperadas: 42.5 metros de cable y 23 unidades de cemento. La cuenta técnica destinada a Workbench consultó la misma venta mediante una conexión real. Se creó un respaldo verificado de la información resultante. El tablero del administrador conservó sus indicadores y Administración de datos mostró MySQL 8.4 LTS e integridad comprobada.

Workbench 26.10.0 y su motor MySQL Shell se abrieron desde el paquete instalado. Las rutas excesivamente largas de componentes Windows provocaron un fallo de arranque durante la primera prueba. El instalador ahora rechaza rutas del programa superiores a 90 caracteres antes de instalar archivos; se comprobó el rechazo con código 1602 y sin crear el ejecutable. Se utiliza la ruta corta sugerida para la validación final.

El instalador también rechazó una actualización mientras la aplicación estaba abierta, sin forzar su cierre. Desinstalar los ejecutables conservó el archivo de selección del negocio, la base, el respaldo y la evidencia de la venta.

La revisión previa al ajuste del indicador de tamaño tiene SHA-256 `142b410a2e5547f352050cde6649ac92c8ef432f24a4b0398c7f24971d10248a`. Se instaló y reinstaló correctamente; el ejecutable instalado coincidió con el hash esperado tras el marcador de empaquetado NSIS de Tauri. Tras reinstalar se repitieron las comprobaciones de la venta única, los importes, las existencias y el acceso de la cuenta de Workbench, y se generó otro respaldo válido. Ese registro está en `artifacts/qa/141fc64f-f740-4f3a-8359-80e109bc1cb2/`.

La revisión final comparte una única medición del almacenamiento entre el tablero y Administración de datos. Se añadió una comprobación que exige un tamaño positivo en MySQL y coincidencia entre ambas pantallas. La comprobación pasó con la venta conservada, y se repitieron las pruebas de reportes con SQLite.

El instalador final entregado, después de esa corrección, tiene SHA-256 `b9c52eeb41ebd111eafe0cc7e339cdca8c2f28bab0773e29c0eed69b2df689b5`. Se instaló correctamente (NSIS, código 0), se comprobó el hash del ejecutable instalado y la presencia de MySQL, Workbench, sus bibliotecas de ejecución y documentación. La comprobación de la venta, existencias, cuenta de Workbench y respaldo pasó nuevamente usando el runtime de esta instalación. Al abrir el ejecutable instalado, el tablero mostró la venta única de $276.25, efectivo en caja de $1,276.25 y tamaño de la base de 1.3 MB. La captura, el texto accesible y los registros de esta revisión están en `artifacts/qa/df7a840e-cd65-4b4c-85b9-3446a8ae53be/`.

La impresión física requiere comprobar el modelo y controlador de la ticketera del negocio. En esta sesión se verificó vista previa y funcionamiento sin una impresora térmica disponible; no se afirma haber impreso en hardware no conectado. El instalador Windows generado **no tiene firma Authenticode de Donovan Systems**; la firma comercial queda pendiente de disponer del certificado.

Evidencias locales: `artifacts/qa/7e0ec94f-5084-4d0e-b110-71d8185279c3/` contiene capturas de cobro, venta confirmada, ticket, Workbench y administración de MySQL, además de la comprobación de inventario y el respaldo. No son datos comerciales y no se distribuyen como base inicial.

## Pendientes de macOS

El código, adaptador de impresión, integración con Keychain y script del paquete Apple Silicon están preparados. No hay un `.pkg` generado ni probado desde este equipo Windows. Faltan compilación nativa en la Mac M4, instalación y actualización, prueba de Workbench, impresión con el controlador real y firma/notarización. No se declara terminada la versión Mac.

Edwin solicitó dejar el instalador de Mac para después. Se conserva el trabajo preparado y no se continúa esa compilación hasta que lo indique.

Cada equipo opera una base local independiente. No se ha implementado sincronización entre cajas. El traslado se hace con respaldos verificados.

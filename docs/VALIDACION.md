# Validación de la entrega

Fecha de ejecución: 30 de octubre de 2026. Equipo de desarrollo: Windows 11 x64, .NET SDK 10.0.302. La aplicación se publica con runtime autónomo 10.0.14. Todos los datos utilizados fueron temporales o de demostración.

## Pruebas automatizadas

La suite contiene **31 casos**, incluyendo las dos variantes de ancho de ticket. El resultado y duración exactos se guardan en `artifacts/test-results/commerce.trx`.

```powershell
dotnet test tests/DonovanPOS.Tests -c Release --logger "trx;LogFileName=commerce.trx" --results-directory artifacts/test-results
```

Se comprueba creación inicial sin semilla, permisos, hash de contraseña, venta, impuestos, descuentos, cambio, pagos mixtos, idempotencia, reversión de transacciones, dos ventas concurrentes sobre el mismo stock, presentaciones, fracciones, cotización y conversión única, crédito, abonos, devolución parcial, acumulación exacta de centavos, apertura/corte de caja, datos de reportes, importación válida/duplicada, reintento de importación, búsqueda por acentos/alias/medidas, índice actualizado, catálogo de 2,000 productos, suspendidas recuperadas, precios modificados, falta de stock, respaldos con WAL, respaldo automático, copia dañada, migración desde esquema inicial y logotipo incorporado.

La prueba de jornada completa recibe compra, registra una venta mixta, suspende/recupera/cobra otra, convierte una cotización con descuento, genera una venta a crédito, recibe abono, cancela esa venta, registra gasto, cierra caja, respalda, restaura y abre nuevamente la base. Resultados comprobados: **17 unidades**, **$1,300.00 de efectivo**, **$0.00 de deuda**, **$568.40 de venta neta** y caja cerrada conservada después de restaurar. Son resultados de una prueba sintética, no datos comerciales.

## Interfaz y documentos

La auditoría WPF genera 36 combinaciones de dashboard/POS/inventario, nueve vistas adicionales y cuatro pasos del asistente. Comprueba errores de enlace de datos y excepciones de interfaz; el resultado se escribe en `artifacts/qa/visual-audit.json`. Las imágenes son renders del árbol visual real de WPF, no diseños estáticos inventados.

| Entorno revisado | Método |
|---|---|
| Ventana real de la aplicación | Captura mediante la interfaz de Windows y uso de controles reales |
| 1366 × 768 y 1920 × 1080 | Render WPF con tamaño de ventana configurado |
| 125 % y 150 % de esas resoluciones | Simulación del área lógica disponible: 1093 × 614, 911 × 512, 1536 × 864 y 1280 × 720 |
| Claro / oscuro | Aplicación de recursos reales en ambas variantes |

La simulación de área lógica **no equivale a cambiar el DPI real de Windows**. Se usa para encontrar recortes y verificar acceso al cobro, navegación y desplazamiento. Las capturas detectaron y permitieron corregir el fondo de las gráficas oscuras, un convertidor de visibilidad, iconos en la barra estrecha y el catálogo compacto. Las resoluciones indicadas describen la ventana; las barras de título y bordes reducen el contenido útil.

Se generan y renderizan mediante Poppler tickets de 58 y 80 mm, cotización de una página, cotización con sesenta renglones, etiquetas de diecisiete unidades y un reporte de ventas. Se revisan legibilidad, totales, paginado y recortes. Una regresión del tamaño del ticket dejaba contenido fuera de la página: se corrigió calculando primero la geometría y se añadió una prueba que comprueba las posiciones de texto dentro de sus límites.

## Entrega y alcance de las comprobaciones

El instalador, ZIP y sumas SHA-256 se encuentran en `artifacts`. El registro de prueba del instalador se conserva en esa misma carpeta. Las pruebas de instalación y desinstalación utilizan una carpeta de prueba y no reemplazan una instalación comercial existente.

Se instaló el MSI por línea de comandos con salida 0, se verificaron el ejecutable y los accesos de Inicio, y se abrió el primer inicio con una carpeta de datos separada. La desinstalación terminó con salida 0, retiró el ejecutable y el menú, conservó la base y un archivo testigo, y mantuvo idéntico el SHA-256 de la base. `artifacts/installer-validation.json` y los registros `install.log`/`uninstall.log` conservan esa evidencia.

En el ejecutable autónomo se probó desde la interfaz una venta ficticia de una brocha y un metro de cable: total $66.50, recibido $100.00, cambio $33.50, folio V-000030, brochas de 34 a 33 y cable de 444 a 443 metros. También se abrió la vista previa del ticket. Las capturas de `docs/images` corresponden a esa ejecución real en modo demostración.

La revisión visual del asistente MSI queda pendiente: la herramienta de control de Windows bloqueó el acceso a `msiexec.exe`. Se corrigió una referencia circular de tabulación detectada durante la preparación del asistente; sus tablas se comprueban estáticamente. No se presenta la prueba silenciosa como una revisión visual del instalador.

La consulta de vulnerabilidades de paquetes se realiza con:

```powershell
dotnet list DonovanPOS.sln package --vulnerable --include-transitive
```

Es una revisión puntual del catálogo de avisos disponible, no una garantía permanente de ausencia de vulnerabilidades. Se conservan versiones en `packages.lock.json`; cualquier actualización debe repetir pruebas, auditoría de licencias y empaquetado.

Quedan para validación en el equipo de uso: impresora térmica física y controlador concreto, lector USB real, bandeja/papel de etiquetas, DPI real al 125/150 %, otras versiones de Windows y rendimiento con el catálogo y volumen históricos del negocio. Tampoco se ha probado recuperación frente a pérdida física de energía o de disco. Las transacciones y copias reducen ese riesgo, pero no sustituyen una prueba de operación y restauración en el entorno final.

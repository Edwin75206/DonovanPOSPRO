# Validación 2.2 — caja de mostrador

Fecha: 31 de octubre de 2026. Aplicación Windows x64 con Tauri 2, React/TypeScript y servicios Rust/SQLite. No se usó servidor de desarrollo para la demostración.

## Comportamiento implementado

La venta ocupa una tabla compacta con edición directa, unidad, descripción, precio, descuento cuando aplica e importe. El encabezado conserva vendedor, referencia, caja y cliente; el total y Cobrar/F8 permanecen visibles. F2 abre un buscador con categorías y marcas obtenidas de SQLite, existencias e indicadores compartidos con Inventario. Flechas/Enter seleccionan; Escape cancela. Una selección agrega y cierra, devolviendo el foco al código.

La captura principal consulta códigos exactos sin abrir el buscador. La cola serial evita perder incrementos por lecturas repetidas. Se rechazan productos inactivos, ambiguos, agotados y cantidades mayores al stock. Una respuesta pendiente del buscador no puede agregar después de cerrarlo. Las ráfagas sobre cantidades o cobro no agregan productos ni confirman pagos y conservan los valores anteriores. Los permisos de precios/descuentos siguen comprobándose en Rust.

Al cobrar se conserva la operación transaccional e idempotente existente. La confirmación aparece en una franja con folio, cambio y acceso al ticket; la captura ya está lista para el siguiente cliente. El esquema sigue en versión 5; no se reescriben datos ni historiales por este rediseño. El filtro de marca usa parámetros SQL y no expone costos al vendedor.

## Pruebas automatizadas

- 20 pruebas de interfaz en seis archivos: captura directa, repetición, código desconocido, F2, categorías, marcas, agotados, selección manual/teclado, foco, cancelación de respuesta pendiente, fracciones, sobreventa, confirmación para vaciar, pagos, impresión fallida y nueva venta inmediata.
- 27 pruebas Rust: importes, transacciones y reversión, equivalencia comercial, permisos, sesiones, caja, cotizaciones, crédito, inventario, documentos, impresión, migraciones y respaldo/restauración entre raíces aisladas.
- `tsc -b`, compilación optimizada de Tauri y empaquetado NSIS completados. Los ajustes finales volvieron a pasar las 20 pruebas de interfaz. Registros en `artifacts/v2.2/work/ui-tests-final.log`, `native-tests.log`, `build-final.log` y `bundle-final.log`.

## Demostración desde Windows

Base aislada: `artifacts/v2.2/preview/6126d4c4-31f5-4f6b-ad84-cb49c2f63e1d/data-seller`. El nombre del negocio y la cuenta indican DEMO. No se modificó la instancia original del usuario.

Se comprobó código inexistente, código de cemento repetido, filtro Electricidad, stock bajo y agotado visibles, búsqueda por nombre con F2, flechas y Enter, retorno al lector y edición de cable a 2.5 metros. Se registró la venta **V-000019**:

| Producto | Cantidad | Precio | Importe |
| --- | ---: | ---: | ---: |
| Cemento gris CPC 30R | 2 piezas | $235.00 | $470.00 |
| Cable THW calibre 12 | 2.5 metros | $18.50 | $46.25 |
| Brocha profesional de 2 pulgadas | 1 pieza | $48.00 | $48.00 |

Total $564.25; efectivo recibido $1,000.00; cambio $435.75. Se generó desde la interfaz el PDF de 80 mm con el logo monocromático original. El PDF tiene una página y se revisó renderizado completo, sin cortes de texto ni cantidades.

Después de cerrar la instancia, `pos_ui_qa verify` confirmó `integrity=ok`, 19 ventas (18 de muestra y una nueva), total/efectivo/cambio exactos y descuentos de inventario de 2 piezas, 2.5 metros y 1 pieza. El comprobante está en `ui-sale-verified.json` dentro de la raíz de prueba.

La interfaz se revisó a 1366 × 768 y maximizada en un monitor de 1920 × 1080 (área de aplicación 1920 × 1032, por la barra de tareas), en claro y oscuro. Las capturas solicitadas están en la carpeta `capturas` de esa prueba y se incluyen en la entrega.

## Instalador

Prueba aislada: `artifacts/v2/installation/8ad039a3-ca57-4ea3-937a-9e6c6cf3eec4`. Se instaló el paquete final de 2.2.0, se abrió su ejecutable y se revisaron la caja, el ticket recuperado y el Dashboard administrativo. La revisión del Dashboard conserva indicadores, gráficas, reportes y menús de administrador; Negociante Uno inicia directamente en caja.

| Comprobación | Resultado |
| --- | --- |
| Instalación silenciosa x64 | Código 0; ejecutable y recursos coinciden con los hashes esperados |
| Reinstalación con su copia abierta | Código 1602; no cierra la aplicación por la fuerza |
| Reinstalación después del cierre | Código 0; seis archivos persistentes del vendedor conservan sus hashes |
| Desinstalación de la copia de prueba | Código 0; ejecutable y registro de desinstalación retirados |
| Datos tras desinstalar | Trece archivos de las dos bases de demostración conservan sus hashes |
| Base reabierta después de desinstalar | Integridad correcta; siguen 19 ventas y las existencias verificadas |

El instalador incluye WebView2 offline y los recursos de documentación y licencias. Los archivos del negocio permanecen fuera del directorio del programa. El marcador de tipo NSIS modifica tres bytes de identificación respecto al EXE independiente; el script calcula y verifica el hash esperado de la copia instalada. Se retiró únicamente el valor de registro residual que apuntaba a esta instalación de prueba.

La entrega reproducible está en `artifacts/v2.2/entrega`: instalador, EXE, manual, comprobantes, capturas, ticket PDF y manifiesto SHA-256. `validacion/data-retention.json` resume la conservación comprobada. Los JSON originales del instalador y los registros de compilación se incluyen en esa carpeta.

## Actualización entre versiones

Se verificó adicionalmente la actualización **2.1.0 → 2.2.0**, sin desinstalar previamente, mediante los instaladores de entrega originales. Ejecución: `artifacts/upgrades/ae8b6d8c-80d6-4e33-988c-f5ebbd8efb86/verified.json`.

- El instalador 2.1.0 y la actualización 2.2.0 terminaron con código 0. La versión registrada y el hash del EXE instalado coinciden con sus respectivos paquetes.
- Los cuatro archivos persistentes copiados de la ubicación aislada de documentación mantuvieron exactamente sus hashes después de actualizar y desinstalar. No se utilizó la base real del usuario.
- El intento de instalar 2.1.0 sobre 2.2.0 se rechazó con código 2; permanecieron el ejecutable 2.2.0 y los mismos datos.
- La consulta SQLite de solo lectura confirmó integridad `ok`, cero errores de claves foráneas, esquema 5 y 19 ventas. El último total siguió en 56,425 centavos, con 100,000 recibidos y 43,575 de cambio; la identidad siguió siendo Construrama Chicontla.
- La instalación aislada y su valor de registro temporal se retiraron al finalizar. El comprobador reutilizable está en `scripts/Test-NativeUpgrade.ps1`; exige paquetes con hashes verificados, una ubicación de documentación propia y ausencia de una instalación registrada del cliente.

Esta comprobación cubre esos dos paquetes, que comparten esquema 5. Cada versión futura requiere verificar su actualización y cualquier migración que incorpore. La modalidad vigente es entregar un instalador; no se activó un servicio ni un botón de actualización por internet.

## Alcance de la validación

Los códigos HID se probaron con eventos de teclado automatizados y captura de códigos desde la ventana nativa. Falta probar el lector físico del negocio. Se validaron la generación del ticket/PDF, la ausencia de impresora configurada y errores de impresión simulados sin cobro duplicado. Falta comprobar salida en papel con la térmica, su controlador y sus funciones reales. Esta máquina ya tiene WebView2; no sustituye una prueba en un Windows limpio sin ese runtime. El paquete no tiene firma Authenticode comercial.

La persistencia frente a reinicio del trabajo pendiente se ofrece mediante **Poner en espera**; el carrito sin guardar permanece en memoria y el cierre avisa antes de descartarlo. No se afirma recuperación automática de un carrito no guardado ante un corte de energía.

# Donovan POS Pro 2.3 — MySQL local y macOS Apple Silicon

Cliente: **Construrama Chicontla**. Desarrollado por **Donovan Systems**.

## Estado y alcance

Esta edición incorpora MySQL 8.4.13 LTS como almacenamiento real del negocio. MySQL Workbench 26.10.0 es la herramienta de administración; la base no se almacena dentro de Workbench. La interfaz y los servicios de ventas son los mismos en Windows y macOS. React no ejecuta SQL ni recibe la contraseña de la cuenta de operación.

La compilación macOS se dirige a Apple Silicon, incluido M4, y requiere macOS 15 o posterior. Se prepara desde una Mac con `scripts/Build-MacOS.command`. **No se considera entregado ni validado un instalador Mac hasta compilarlo, instalarlo y probarlo en macOS.** Los resultados en Windows no sustituyen esa comprobación.

Cada equipo tiene una instancia independiente. La instalación no configura sincronización entre cajas ni acceso desde otras computadoras. Para trasladar el negocio entre Windows y Mac se utiliza un respaldo de Donovan POS Pro.

## Instalación y primera apertura

El instalador incluye la aplicación, los binarios oficiales de MySQL y Workbench. En Windows, incluye los componentes de ejecución de Visual C++ junto al motor y WebView2 offline. El cliente no necesita Node, Rust, Visual Studio, SDK de .NET ni Python.

En Windows, utiliza la carpeta sugerida por el instalador. Se limita la ruta del programa a 90 caracteres para conservar compatibilidad con los archivos internos de Workbench y los límites de ruta de sus componentes. La raíz persistente del negocio es independiente de esa carpeta.

Al abrir una instalación nueva, la aplicación configura su instancia privada de MySQL, crea un esquema único, aplica la estructura versionada y comprueba su integridad. Después permite configurar Construrama Chicontla, crear las cuentas e iniciar sesión. También se puede restaurar un respaldo desde la bienvenida.

Si ya existe una base SQLite, la aplicación la conserva activa. El administrador abre **Configuración → Administración de datos → Respaldar y migrar a MySQL** y confirma su contraseña. El sistema crea un respaldo, prepara un esquema nuevo, copia la información, compara cantidades de filas y SHA-256 por tabla, revisa relaciones e importes y solo entonces cambia la conexión activa. El archivo SQLite original se conserva. Al finalizar se vuelve a iniciar sesión.

El proceso requiere espacio libre para el motor, la base y el respaldo. Ante un fallo no se activa una copia incompleta ni se sustituye la base por un negocio vacío.

## Archivos persistentes

| Elemento | Windows | macOS |
|---|---|---|
| Raíz del negocio | `%LOCALAPPDATA%\DonovanSystems\DonovanPOS\data-v2` | `~/Library/Application Support/DonovanSystems/DonovanPOS/data-v2` |
| Motor de datos | `mysql/data/` dentro de la raíz | `mysql/data/` dentro de la raíz |
| Configuración de conexión | `mysql/instance.json` | `mysql/instance.json` |
| Configuración del motor | `mysql/my.cnf` | `mysql/my.cnf` |
| Registro del motor | `mysql/server.log` | `mysql/server.log` |
| Negocio activo | `active.json`, versión 2, backend `mysql` | Igual |
| Respaldos | Carpeta elegida por el administrador | Carpeta elegida por el administrador |
| Secretos | DPAPI del usuario de Windows | Llavero de macOS |

Los datos nunca se guardan en la carpeta del instalador. Actualizar o desinstalar los ejecutables no elimina la raíz del negocio. Las credenciales nativas son específicas del equipo y del usuario del sistema operativo: **no copies la carpeta de MySQL como método de traslado**. Restaura un respaldo en la instalación de destino.

La instancia escucha exclusivamente en `127.0.0.1`, en un puerto local asignado durante la primera configuración y conservado después. No abre el firewall ni modifica otros servicios MySQL. Arranca con Donovan POS Pro y se cierra ordenadamente al salir. Mantén la aplicación abierta si vas a consultar la base con Workbench.

## Administración con Workbench

En **Administración de datos → Ver conexión de Workbench**, el administrador confirma su contraseña de Donovan POS Pro. Aparecen servidor, puerto, usuario, esquema y la contraseña técnica de esa instalación. No son las credenciales del vendedor. La contraseña se muestra en un campo protegido que se puede seleccionar y copiar; se retira al cerrar el panel.

Pulsa **Abrir MySQL Workbench**, crea una conexión `Standard TCP/IP` con esos datos y guarda la contraseña en su almacén seguro. Una vez conectado, el esquema de Construrama Chicontla aparece en la lista de bases. No es necesario importar manualmente las tablas.

La cuenta de aplicación tiene permisos de lectura y escritura de datos, sin permisos para crear o eliminar tablas. La cuenta de Workbench puede administrar el esquema del negocio. La cuenta de mantenimiento permanece en el almacén seguro nativo y no se muestra en la interfaz.

La administración cotidiana se realiza desde Donovan POS Pro. Workbench se reserva principalmente para diagnóstico técnico. Antes de modificar datos o estructura, crea un respaldo y detén la operación de caja. Cambiar índices, columnas, claves o reglas provoca un rechazo de compatibilidad en la aplicación. No cambies usuarios técnicos, contraseñas o `my.cnf` directamente.

## Tablas y relaciones

La estructura incluye 35 tablas InnoDB. Los importes se conservan en centavos enteros y las cantidades en unidades de 1/10,000, respetando las reglas de piezas y fracciones.

| Área | Tablas principales y relaciones |
|---|---|
| Identidad y migraciones | `AppMeta`, `NativeMigration`, `__EFMigrationsHistory`, `__EFMigrationsLock` |
| Usuarios y sesiones | `User`, `UserSession`, `UserPreferences`, `UserAppearance`; las últimas dependen de `User.Id` |
| Configuración comercial | `StoreSettings`, `StorePresentation`, `TicketProfile`; presentación y ticket dependen del negocio |
| Catálogo e inventario | `Product`, `StockMovement`; cada movimiento referencia producto y usuario |
| Clientes y proveedores | `Customer`, `Supplier` |
| Caja | `CashSession`, `CashMovement`; una restricción única impide más de una caja abierta |
| Ventas | `Sale`, `SaleLine`, `PaymentRecord`; cada venta conserva vendedor, caja, cliente opcional, folio y solicitud única |
| Devoluciones | `SaleReturn`, `ReturnLine`; referencian la venta y sus partidas originales |
| Crédito | `CreditEntry`; referencia cliente, venta y usuario |
| Cotizaciones | `Quote`, `QuoteLine`; conservan relación con la venta al convertirlas |
| Ventas en espera | `HeldSale`, `HeldSaleLine`; conservan propietario y productos |
| Compras | `Purchase`, `PurchaseLine`, `PurchaseDetails`, `PurchaseLineDetails`; conservan proveedor, empaque e importes exactos |
| Control | `Sequence`, `AuditEvent`, `RequestReceipt`, `BackupRecord` |

La definición ejecutable está en `desktop/src-tauri/migrations/mysql/001_mysql.sql`. `contract.json` documenta columnas, claves, relaciones y comprobaciones. `schema-hashes.json` fija las definiciones de tabla verificadas contra MySQL 8.4.13; los contadores de autoincremento se excluyen de esa comparación. El generador de desarrollo es `scripts/Generate-MySqlSchema.py` y no se instala en el cliente.

Se conserva el historial de las migraciones SQLite 1–5 y la identidad del negocio. La migración MySQL tiene su archivo versionado y su checksum en `AppMeta`. Una actualización que cambie el esquema necesita una migración explícita y un respaldo preventivo; una versión incompatible debe rechazar la conexión.

## Integridad, ventas y respaldo

Los servicios Rust ejecutan las operaciones críticas en transacciones. MySQL utiliza aislamiento serializable. Antes de confirmar una venta se verifican existencias, cantidades, permisos y pagos. El identificador único de la solicitud evita duplicar cobros al reintentar. Imprimir o reimprimir no vuelve a registrar la venta.

El archivo `.donovanbackup` conserva su formato portable: manifiesto con versión, identidad del negocio, tamaño y SHA-256, más una copia SQLite validada de las 35 tablas. Esa copia es el formato de intercambio del respaldo; la base que atiende las ventas es MySQL. Se incluyen los logotipos y fondos guardados en las tablas.

Para crear el respaldo se mantiene una transacción consistente mientras se copian y comparan las tablas. Se validan la copia y sus relaciones antes de publicar el archivo. Al restaurar, se crea un esquema MySQL diferente y se cambia el puntero activo únicamente después de verificarlo. Las bases anteriores se conservan, se revocan sesiones y se restablece la selección local de impresora.

## Impresión y sesiones en macOS

Windows conserva su adaptador GDI/spooler. macOS utiliza CUPS (`lpstat`, `lpoptions`, `lp`) y el mismo generador PDF de los tickets y cotizaciones. Las llamadas no pasan por un intérprete de comandos. El adaptador identifica colas instaladas, solicita copias y consulta las opciones que anuncia el controlador.

No se asume que el controlador pueda confirmar la conexión física ni que acepte todos los tamaños. La prueba en la impresora real de 58/80 mm forma parte de la validación final. Sin impresora se puede registrar la venta, ver el ticket y guardar PDF. Un fallo de impresión conserva el folio registrado.

La sesión y las credenciales de MySQL se guardan en el llavero de macOS, sin contraseñas en archivos de texto. Los archivos de datos pertenecen al usuario local y se mantienen fuera de `.app`.

## Compilación y entrega

Windows: ejecutar `scripts/Build-MySqlWindows.ps1` en el equipo de desarrollo. Las descargas se fijan mediante SHA-256 en `packaging/dependencies.json`, y se comprueban las firmas de Oracle y Microsoft. La extracción administrativa de Workbench prepara una copia privada; no instala ni modifica un servidor del sistema.

Mac M4: ejecutar `bash scripts/Build-MacOS.command --verify` para compilar y probar el paquete en una Mac de desarrollo. Para distribución comercial, usar `--release`, certificados `Developer ID Application` y `Developer ID Installer`, y un perfil de `notarytool`. El script exige firma y notarización en ese modo. El cliente recibe el `.pkg`; no ejecuta scripts ni instala herramientas de desarrollo.

Las actualizaciones se distribuyen enviando un enlace al nuevo instalador, según la preferencia de Edwin. Debe cerrarse Donovan POS Pro y Workbench antes de instalar. Se conserva la base persistente; no existe un servidor de actualizaciones ni una dependencia de internet durante las ventas.

## Validación obligatoria antes de distribuir en Mac

1. Compilar nativamente para `aarch64-apple-darwin` y ejecutar todas las pruebas, incluidas las de MySQL.
2. Instalar el `.pkg` en una cuenta limpia de macOS, con internet desconectado.
3. Configurar el negocio, vender varios productos con fracciones y efectivo, comprobar cambio y reiniciar.
4. Abrir Workbench y verificar que la conexión consulta la misma venta y las mismas existencias.
5. Crear un respaldo, restaurarlo en otra instalación aislada y comparar identidad, folios, caja, inventario y logotipos.
6. Probar vista previa, PDF, impresión real, impresora desconectada y reimpresión sin duplicar venta.
7. Reinstalar y actualizar conservando información. Comprobar el cierre ordenado del motor y la recuperación tras un cierre inesperado.
8. Verificar firma/notarización y ausencia de avisos de aplicación dañada en Gatekeeper.

Fuentes de componentes: [MySQL Community Server](https://dev.mysql.com/downloads/mysql/8.4.html), [MySQL Workbench](https://dev.mysql.com/downloads/workbench/), [distribución macOS de Tauri](https://tauri.app/distribute/macos-application-bundle/). Las licencias de terceros permanecen junto a sus distribuciones. Conserva las fuentes correspondientes y las condiciones de redistribución al publicar el instalador.

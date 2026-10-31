# Base local y portabilidad — Donovan POS Pro 2

## Decisión de almacenamiento

SQLite se incluye en el ejecutable. Para una caja que opera en una computadora no se necesita instalar ni mantener MySQL Server. SQLiteStudio es una herramienta opcional de diagnóstico, no un requisito de funcionamiento. La operación cotidiana se realiza dentro de Donovan POS Pro.

Esta versión trabaja con una base local por instalación. No sincroniza ventas entre computadoras. Si en el futuro se necesitan varias cajas escribiendo al mismo tiempo sobre un negocio común, debe diseñarse una solución de concurrencia y red; compartir el archivo SQLite mediante una carpeta de red no es ese diseño.

## Ubicación y permanencia

La raíz normal es `%LOCALAPPDATA%\DonovanSystems\DonovanPOS\data-v2`. Pertenece al usuario de Windows que ejecuta el programa y se mantiene separada de los archivos instalados.

| Archivo o carpeta | Contenido |
| --- | --- |
| `active.json` | Ruta relativa de la base administrada que está activa |
| `businesses\<identificador>\donovan.db` | Bases creadas o restauradas por la aplicación |
| `donovan.db` | Nombre de base inicial o heredada cuando aún no existe un selector de base activa |
| `*-wal`, `*-shm` | Archivos de trabajo de SQLite mientras hay conexiones abiertas |
| `backups` | Destino local predeterminado de respaldos |
| `recovery-preserved\<identificador>` | Copias preventivas de la base anterior a una restauración o cambio |
| `before-migration-<identificador>.sqlite` | Copia preventiva antes de migrar una base existente |
| `logs\*.jsonl` | Diagnóstico local con referencias de incidente |
| `session.dpapi` | Token de sesión protegido por Windows; no contiene la contraseña |
| `.instance.lock` | Exclusión de una segunda instancia sobre la misma raíz |
| `staging`, `work` | Directorios administrados para preparar y validar operaciones; no son la base activa |

La ubicación exacta se muestra en **Configuración → Administración de datos**. Las bases anteriores no se borran al activar otra. Actualizar o reinstalar los archivos del programa no requiere trasladar la base. Copiar únicamente el EXE a otra computadora no transporta el negocio.

## Consistencia y migraciones

Las conexiones de escritura activan claves foráneas, WAL y sincronización FULL. Venta, inventario, pagos y caja se confirman en una transacción. Las operaciones críticas tienen identificadores de solicitud para que un reintento no duplique una venta, un abono o un movimiento de caja.

Los importes se guardan en centavos enteros; las cantidades usan 10,000 unidades internas por unidad comercial. El IVA usa puntos base: 1,600 representa 16%. Las cantidades y redondeos se calculan en Rust. El costo faltante se representa como NULL y no como cero.

| Migración | Cambio |
| --- | --- |
| `001_legacy.sql` | Esquema comercial preservado de WPF y sus cuatro registros históricos de Entity Framework |
| `002_desktop.sql` | Identidad de base, migraciones nativas, sesiones, preferencias y recibos de solicitudes; `user_version=2` |
| `003_appearance.sql` | Apariencia individual y fondo embebido; `user_version=3` |
| `004_business_documents.sql` | Presentación del negocio, impresión automática y detalle exacto de nuevas compras; `user_version=4` |

El esquema WPF reconocido usa `user_version=0` y su historial EF exacto. No se acepta cualquier SQLite con tablas de nombres parecidos: se verifican versión, columnas, relaciones, índices relevantes, integridad y claves foráneas. Las migraciones se ejecutan sobre una copia controlada. No se degrada una base de una versión futura.

`NativeMigration` registra versiones aplicadas; `AppMeta` registra producto, versión e identificador de base. Los SQL completos se distribuyen con la documentación técnica.

## Tablas y relaciones

| Tablas | Función y relaciones principales |
| --- | --- |
| `Product`, `ProductSearch` | Catálogo e índice FTS5; disparadores mantienen el índice de búsqueda |
| `Customer`, `Supplier` | Clientes, límites de crédito y proveedores; archivar conserva el historial |
| `User`, `UserSession` | Cuentas, permisos y sesiones vinculadas a la versión de autenticación de cada usuario |
| `UserPreferences`, `UserAppearance` | Tutorial y personalización por `UserId`; el fondo se guarda como BLOB |
| `StoreSettings`, `StorePresentation` | Identidad, logo embebido, impuestos, ticket, impresora y respaldos; relación por `StoreId` |
| `Sale`, `SaleLine`, `PaymentRecord` | Venta con usuario, cliente opcional y jornada de caja; partidas y pagos apuntan a `SaleId` |
| `SaleReturn`, `ReturnLine` | Devolución vinculada a venta; partidas vinculadas a la partida original para limitar cantidades e importes |
| `CashSession`, `CashMovement` | Jornada con responsable y movimientos de efectivo; solo se admite una jornada abierta |
| `StockMovement` | Movimiento de producto, responsable, motivo y saldo resultante |
| `Purchase`, `PurchaseLine` | Recepción, proveedor, responsable y productos recibidos |
| `PurchaseDetails`, `PurchaseLineDetails` | Forma de afectación de caja, empaque y costo original exacto de nuevas compras |
| `CreditEntry` | Cargos, abonos y reducciones de deuda asociados a cliente, venta y usuario |
| `Quote`, `QuoteLine` | Propuesta y partidas; `SaleId` enlaza la conversión única y protege su historial |
| `HeldSale`, `HeldSaleLine` | Venta suspendida sin cobro ni reserva de inventario |
| `Sequence` | Consecutivos por tipo de documento |
| `RequestReceipt` | Resultado persistido y comprobación de reintentos de operaciones |
| `AuditEvent` | Responsable, operación y referencia de cambios de negocio |
| `BackupRecord` | Historial, ruta, tamaño y resultado de respaldos |
| `AppMeta`, `NativeMigration`, `__EFMigrationsHistory`, `__EFMigrationsLock` | Identidad y compatibilidad técnica; no editar manualmente |

Las partidas de venta conservan nombre, clave, precio, IVA y costo del momento. Cambiar un producto no reescribe ventas anteriores. Las ganancias son estimaciones sin IVA y antes de gastos. Una compra por empaque conserva su costo total exacto; el costo unitario promedio se guarda en centavos, por lo que una valuación posterior puede mostrar diferencias de redondeo de centavos.

## Respaldos y cambio de computadora

Un `.donovanbackup` es un contenedor con `manifest.json` y `business.sqlite`. El manifiesto incluye versión de formato, esquema, identificador de base, tamaño, fecha y SHA-256. La copia usa la API de respaldo de SQLite e incorpora operaciones confirmadas que todavía se encuentren en WAL. Logo y fondos están embebidos en la base y viajan con el negocio.

1. En el equipo de origen, entrar como administrador y crear un respaldo desde Administración de datos. Confirmar que la aplicación lo reporta como verificado.
2. Copiar ese archivo completo a un medio externo y al equipo de destino.
3. Instalar la misma versión de Donovan POS Pro en el destino y elegir restaurar un respaldo en la bienvenida.
4. Revisar nombre del negocio, versión y cantidades de registros de la vista previa. Acreditar una cuenta administradora del respaldo y confirmar la restauración.
5. Iniciar sesión con las cuentas conservadas. Revisar caja, últimas ventas, inventario y cuentas por cobrar. Elegir la impresora y la carpeta de respaldos del nuevo equipo.

En una instalación ya configurada se exige autorización del administrador actual y se crea una copia preventiva antes de activar otra base. La restauración valida el contenedor y su tamaño, el hash, la compatibilidad, la integridad y las relaciones. La activación conserva la base anterior ante un fallo. Una selección caduca tras 15 minutos y debe revisarse de nuevo.

Las sesiones se eliminan del negocio restaurado. No se transporta el archivo DPAPI de Windows. La impresora y la impresión automática se restablecen para evitar enviar documentos a un dispositivo equivocado. Un respaldo no incluye los PDF o Excel que se hayan exportado a carpetas externas; son copias de consulta, no registros necesarios para reconstruir las operaciones.

El respaldo contiene información comercial y hashes de contraseñas. El contenedor no está cifrado. Guardarlo en un medio con acceso restringido; conservar una copia fuera del disco del equipo. Los respaldos automáticos se ejecutan mientras la aplicación está abierta, no mediante un servidor o servicio adicional.

## SQLiteStudio para diagnóstico

Cerrar Donovan POS Pro antes de inspeccionar la base activa, o trabajar con una copia extraída de un respaldo. Preferir una conexión de solo lectura. No modificar precios, inventario, folios, usuarios, sesiones o tablas de migración directamente: se omitirían permisos, validaciones y movimientos relacionados.

Consultas de diagnóstico sobre una copia:

```sql
PRAGMA user_version;
PRAGMA integrity_check;
PRAGMA foreign_key_check;
SELECT Version, AppliedAt, Description FROM NativeMigration ORDER BY Version;
SELECT Key, Value FROM AppMeta WHERE Key IN ('product','schema','databaseId');
```

La comprobación de integridad debe devolver `ok`; la de claves foráneas debe devolver cero filas. Nunca eliminar archivos WAL/SHM de una base abierta ni copiar solo el archivo `.sqlite` mientras el programa está operando. Utilizar el respaldo de la aplicación para mover información.

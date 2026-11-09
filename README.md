# Donovan POS Pro

Punto de venta de escritorio para **Construrama Chicontla**, desarrollado por **Donovan Systems — Ing. Edwin Donovan Castañeda Calderón**. Español de México, MXN y operación local sin internet.

## Instaladores

**[Descargar desde GitHub Releases](https://github.com/Edwin75206/DonovanPOSPRO/releases)**

| Plataforma | Estado de 2.3.0 |
|---|---|
| Windows x64 | Instalador `.exe` probado; incluye MySQL local, Workbench y WebView2 offline. Firma Authenticode pendiente. |
| macOS Apple Silicon / M4 | Código y compilación preparados. Instalador pendiente de compilar y validar en Mac. |

Los instaladores son adjuntos de las versiones publicadas. `git clone` descarga el proyecto para desarrollo, no los instaladores ni los datos del negocio. El cliente final no necesita Git, Node, Rust, Visual Studio ni un SDK.

## Continuar en tu Mac

```sh
git clone https://github.com/Edwin75206/DonovanPOSPRO.git
cd DonovanPOSPRO
```

Abre la carpeta clonada en Codex y usa **[PROMPT-PARA-MAC.md](PROMPT-PARA-MAC.md)**. El contexto técnico, decisiones y comprobaciones están en [CONTINUAR-EN-MAC.md](CONTINUAR-EN-MAC.md); [AGENTS.md](AGENTS.md) señala qué leer al continuar.

## Producto

- Caja de mostrador para Negociante Uno: tabla compacta, escaneo directo, buscador **F2**, cantidades editables, existencias y total permanente, cobro **F8** y cálculo de cambio.
- Dashboard ejecutivo para el administrador: KPIs, gráficas, caja, inventario, cartera, reportes y control del negocio.
- Cotizaciones, ventas en espera, compras, crédito, abonos, devoluciones, importación/exportación de productos y roles.
- Tickets con identidad del negocio, 58/80 mm según controlador, vista previa, PDF y reimpresión sin volver a cobrar.
- MySQL local privado, migración explícita desde SQLite y respaldos verificados. Cada equipo mantiene una base independiente; no hay sincronización automática entre cajas.

## Arquitectura

Tauri 2 + React/TypeScript, Tailwind, componentes Radix/shadcn, Motion, Apache ECharts y TanStack Table. **Rust controla reglas comerciales, permisos, SQL, transacciones y operaciones nativas.** React no accede directamente a la base.

MySQL Server almacena el negocio y Workbench permite su administración técnica. SQLite se conserva para compatibilidad, migración y formato portable de respaldos. La aplicación comercial inicia vacía para configurar o restaurar el negocio; los datos sintéticos solo se usan en pruebas aisladas.

| Carpeta | Contenido |
|---|---|
| `desktop/` | Aplicación actual, interfaz, servicios Rust y pruebas |
| `desktop/src-tauri/migrations/` | Estructura versionada y contrato MySQL |
| `assets/` | Logotipos originales, fuentes y avisos distribuidos |
| `packaging/` | Paquetes oficiales fijados por versión y SHA-256 |
| `scripts/` | Compilación, validación y controles del repositorio |
| `docs/` | Operación, arquitectura, pruebas y notas de versiones |
| `licenses/` | Procedencia y licencias de terceros |

La referencia WPF original está conservada en la etiqueta [v1.0.0](https://github.com/Edwin75206/DonovanPOSPRO/tree/v1.0.0) y en el historial; no es la aplicación activa.

## Desarrollo y versiones

- [Preparar y compilar Windows o Mac](desktop/README.md).
- [Flujo de ramas, versiones y publicación](docs/VERSIONES-Y-GITHUB.md).
- [Historial de cambios](CHANGELOG.md).
- [Manual de operación](docs/v2/MANUAL.md).
- [MySQL, Workbench, migración, respaldo y portabilidad](docs/v2/MYSQL-Y-MACOS.md).
- [Validación 2.3 y límites comprobados](docs/v2/VALIDACION-2.3.md).

Las actualizaciones se distribuyen por enlace a un nuevo instalador. La base se guarda fuera del programa y se conserva al actualizar. La impresión física y el escáner deben comprobarse con el hardware del negocio. Los tickets no son CFDI y tarjeta/transferencia se registran manualmente.

El código original conserva los derechos indicados en [PROJECT-NOTICE.txt](licenses/PROJECT-NOTICE.txt); las dependencias mantienen sus licencias. Que el repositorio sea público no cambia esos términos.

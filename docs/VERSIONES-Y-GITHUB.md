# Versiones y trabajo entre Windows y Mac

Repositorio principal: https://github.com/Edwin75206/DonovanPOSPRO

`main` contiene el código actual Tauri/React/Rust y las migraciones. Las plataformas comparten reglas de negocio; los adaptadores nativos se seleccionan por sistema operativo. No mantener dos aplicaciones divergentes.

## Clonar en otro equipo

```sh
git clone https://github.com/Edwin75206/DonovanPOSPRO.git
cd DonovanPOSPRO
```

En la Mac, abrir esta carpeta en Codex y seguir `PROMPT-PARA-MAC.md` y `CONTINUAR-EN-MAC.md`. El nombre de la carpeta puede cambiar; los scripts resuelven la raíz por su ubicación y no dependen de `C:\Users\edwin`.

## Flujo habitual

1. Antes de empezar, revisar `git status` y ejecutar `git pull --ff-only` con el trabajo local ya guardado en un commit. No usar `reset --hard` para solucionar conflictos.
2. Crear una rama para cada cambio: `git switch -c feat/nombre-del-cambio` o `fix/nombre-del-arreglo`.
3. Realizar el cambio, ejecutar las pruebas pertinentes y seleccionar los archivos a incluir. No usar `git add -f` con salidas ignoradas.
4. Ejecutar `python scripts/Check-Repository.py` después de preparar el índice con `git add`. El control revisa archivos versionados y coincidencia de versiones; no sustituye la revisión humana.
5. Crear commits descriptivos, subir la rama y abrir un pull request a `main`. GitHub Actions comprueba la interfaz, reglas de negocio Windows, compatibilidad SQLite e higiene del repositorio.
6. Las comprobaciones reales de MySQL, migración, impresoras e instaladores son adicionales; no quedan cubiertas por un indicador verde de CI.

Para pasar de una computadora a otra, guardar y subir los cambios en una rama; luego obtenerlos con `git fetch origin` y cambiar a esa rama en el otro equipo. Los archivos sin commit de Windows no aparecen automáticamente en Mac.

## Publicar una versión

Usar versiones `MAJOR.MINOR.PATCH` y etiquetas como `v2.3.0`. Mantener alineados `desktop/package.json`, `desktop/package-lock.json`, `desktop/src-tauri/Cargo.toml`, `Cargo.lock` y `tauri.conf.json`. Una corrección posterior que cambie el producto debe tener nueva versión; no reemplazar una etiqueta ya publicada.

Construir y validar el instalador de cada plataforma antes de publicarlo. Crear una etiqueta sobre el commit revisado y una Release de GitHub con notas, instalador y SHA-256. Los `.exe`, `.pkg` y `.dmg` son adjuntos de **Releases**, no blobs del repositorio. Clonar Git descarga el código, no los instaladores; estos se obtienen en https://github.com/Edwin75206/DonovanPOSPRO/releases .

Windows 2.3.0 tiene instalador validado localmente y aún sin firma Authenticode. Mac M4 tiene código y proceso de compilación preparados, **sin instalador validado**. Cuando se complete Mac, publicar su paquete únicamente con el alcance de validación que realmente se haya comprobado; usar una prerelease si todavía falta validación comercial.

Las actualizaciones se envían al negocio como enlace al nuevo instalador. No se necesitan Git, Node ni herramientas de desarrollo en la computadora del cliente. Antes de actualizar, crear un respaldo desde la aplicación, cerrar Donovan POS Pro y Workbench, instalar y verificar la reapertura del negocio.

## Qué se conserva y qué se excluye

Se versionan código, lockfiles, migraciones SQL, pruebas, logotipos, fuentes, licencias, configuración de empaquetado y documentación. Las fuentes `.crate` bajo `assets/licenses/sources/` son material de licencias distribuido intencionalmente, no caché de compilación.

Se excluyen bases, respaldos, secretos, certificados privados, `.env`, dependencias descargadas, runtimes, `target`, `node_modules`, evidencia local, documentos comerciales generados, cotizaciones, instaladores y temporales. Las pruebas conservan generadores de datos sintéticos, nunca bases ya creadas.

El historial anterior se conserva sin reescribirlo. La etiqueta `v1.0.0` contiene la aplicación WPF original con su solución, pruebas, manuales y scripts; esos archivos ya no forman parte del árbol activo de `main`. Para consultarlos, usar GitHub o una clonación aparte y `git switch --detach v1.0.0`. No modificar migraciones históricas para adaptar datos nuevos.

La migración MySQL `001_mysql.sql` conserva los bytes CRLF de la versión Windows validada. `.gitattributes` desactiva su conversión de texto para que clones y archivos de fuentes mantengan el mismo checksum en cualquier plataforma. `packaging/migration-checksums.json` y CI comprueban esos bytes tanto en Git como en el checkout. No normalizar ni reescribir ese archivo; los cambios de esquema requieren una migración nueva.

El repositorio fue creado público por Edwin. No incluir datos reales del negocio en issues, commits, capturas o logs. La licencia del código original se encuentra en `licenses/PROJECT-NOTICE.txt`; las dependencias mantienen sus avisos correspondientes.

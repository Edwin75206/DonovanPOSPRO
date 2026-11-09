# Donovan POS Pro — contexto del proyecto

Antes de continuar este proyecto, lee `CONTINUAR-EN-MAC.md`. Contiene las decisiones de Edwin, el estado comprobado de Windows y los pendientes de macOS. Después consulta `docs/v2/MYSQL-Y-MACOS.md` y `docs/v2/VALIDACION-2.3.md` según la tarea.

El repositorio principal es `Edwin75206/DonovanPOSPRO`. Para continuar en Mac, usa `PROMPT-PARA-MAC.md`; para ramas y entregas, `docs/VERSIONES-Y-GITHUB.md`. Publica instaladores en Releases y conserva fuera de Git datos, secretos y archivos generados. Ejecuta `python scripts/Check-Repository.py` sobre el índice antes de publicar cambios.

La aplicación vigente está en `desktop/`: Tauri 2, React/TypeScript y servicios Rust. El cliente es Construrama Chicontla y el autor es Donovan Systems. Conserva los logotipos existentes en `assets/branding/`.

El contexto documentado no equivale a tener el historial completo del chat. Las instrucciones actuales de Edwin tienen prioridad. No declares terminada ni probada la versión Mac hasta compilarla, instalarla y verificar sus operaciones en macOS. No alteres bases comerciales para las pruebas.

Conserva los cambios locales existentes. Antes de resetear, reemplazar o limpiar archivos, revisa el estado del proyecto; el traslado contiene también archivos nuevos que todavía no estaban confirmados en Git.

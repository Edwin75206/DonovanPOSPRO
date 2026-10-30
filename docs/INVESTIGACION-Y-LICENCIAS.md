# Procedencia y licencias

## Base estudiada

Se revisaron dos proyectos públicos:

| Proyecto | Revisión consultada | Evaluación |
|---|---|---|
| [Sm3th/SupermarketPos](https://github.com/Sm3th/SupermarketPos) | `332d77453cd47ed4ebc9979e53cd15aa024ab1e3` | Licencia MIT y un motor de precios pequeño, separable y reutilizable. Seleccionado como referencia y origen del componente de precios. |
| [mohamedelareeg/WPF-POS](https://github.com/mohamedelareeg/WPF-POS) | `b62185ab4413cb2af6202654f7992084c8d1c6cd` | Base WPF de .NET Framework 4.8 con mayor acoplamiento. No se incorporó su código. |

La selección no convierte este proyecto en una copia completa de SupermarketPos. Se adaptaron únicamente Cart, CartLine, Discount, PricingService, SaleSummary y VatGroup, conservando sus avisos y la licencia en `licenses/SupermarketPos.txt`. Persistencia, casos de uso, identidad Donovan, navegación, inventario, documentos, respaldo y demás módulos se desarrollaron para esta aplicación. No se importaron credenciales, usuarios ni productos de esos repositorios.

MIT permite el uso comercial con conservación de su aviso y texto de licencia. Los componentes nuevos mantienen la autoría del proyecto; no se publica ningún repositorio remoto ni se cambia la titularidad de código de terceros.

## Dependencias distribuidas

`licenses/DEPENDENCIES.csv` contiene el inventario exacto del archivo de dependencias de publicación, con versión, autor, licencia y origen disponible. `licenses/packages` conserva los avisos incluidos en NuGet; `licenses/sources` añade textos originales de los repositorios y `PROVENANCE.txt` identifica sus revisiones/URL. El runtime autónomo conserva sus propios LICENSE y THIRD-PARTY-NOTICES.

| Familia | Versión principal | Licencia declarada |
|---|---|---|
| .NET / Windows Desktop runtime | 10.0.14 | MIT y avisos de terceros del runtime |
| Windows SDK para .NET / proyecciones WinRT | 10.0.19041.57 | Términos de Microsoft Windows SDK; copia RTF incluida |
| Entity Framework Core / Microsoft.Data.Sqlite / Extensions | 10.0.14 | MIT |
| CommunityToolkit.Mvvm | 8.4.2 | MIT |
| LiveCharts2 | 2.0.5 | MIT |
| SkiaSharp | 3.119.4 | MIT más avisos de los componentes nativos |
| SkiaSharp.HarfBuzz | 3.119.0 | MIT más avisos incluidos |
| HarfBuzzSharp | 8.3.1.1 | MIT más avisos incluidos |
| OpenTK / GLWpfControl | 4.3.0 / 4.2.3 | MIT; GLFW conserva su COPYING |
| ClosedXML | 0.105.1 | MIT |
| ClosedXML.Parser | 2.0.0 | MIT |
| Open XML SDK | 3.1.1 | MIT |
| ExcelNumberFormat | 1.1.0 | MIT |
| SixLabors.Fonts | 1.0.0 | Apache-2.0 para esta versión |
| RBush.Signed | 4.0.0 | MIT |
| SQLitePCLRaw | 2.1.14 | Apache-2.0; SQLite conserva su condición de dominio público |
| PDFsharp | **6.2.4** | **MIT para esta versión** |
| ZXing.Net | 0.16.11 | Apache-2.0 |

Las licencias corresponden a las versiones fijadas. En particular, PDFsharp 6.5 cambió su modelo de licencia: no debe actualizarse automáticamente interpretando que conserva la licencia MIT de 6.2.4. La fijación evita introducir esa incompatibilidad en esta entrega, pero deja una dependencia anterior que requiere seguimiento de mantenimiento. Una actualización futura debe revisar licencia y seguridad y, si corresponde, contratar una licencia compatible o sustituir el generador. [Licencia oficial de PDFsharp 6.2.4](https://github.com/empira/PDFsharp/blob/v6.2.4/LICENSE).

No se incorporan logotipos ni capturas de los productos de terceros mostrados en las referencias visuales. Los nombres de marcas de herramientas en demostración sirven únicamente para datos ficticios de ejemplo; no implican afiliación.

El paquete de proyecciones Windows SDK es una dependencia de la variante Windows de MVVM Toolkit. Sus términos son diferentes de MIT: la redistribución comercial requiere cumplir las condiciones de código redistribuible y trasladar la protección contractual correspondiente a distribuidores y usuarios finales. La copia original se incluye en `licenses/sources/Microsoft-Windows-SDK.rtf`, proveniente del [enlace de licencia declarado por el paquete](https://aka.ms/WinSDKLicenseURL). Esta entrega local no establece un contrato de licencia comercial entre Donovan Systems y futuros clientes.

## Instalador

El MSI se genera con las tablas estándar de Windows Installer y un gabinete producido por `makecab.exe`. No contiene acciones personalizadas de borrado ni instala servicios. Los datos viven fuera del directorio de componentes que administra el MSI. La instalación por usuario utiliza el indicador de paquete sin elevación descrito por Microsoft; no altera políticas de ejecución ni deshabilita controles de Windows. [Propiedad Word Count](https://learn.microsoft.com/en-us/windows/win32/msi/word-count-summary), [API de creación de bases MSI](https://learn.microsoft.com/en-us/windows/win32/msi/installer-opendatabase).

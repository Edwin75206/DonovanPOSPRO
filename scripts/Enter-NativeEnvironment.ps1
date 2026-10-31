$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'Instala las herramientas C++ de Visual Studio para compilar. No son necesarias para el cliente.' }
$toolset = Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$compiler = Join-Path $toolset.FullName 'bin/Hostx64/x64'
$sdkHeaders = Join-Path $projectRoot '.tools/microsoft.windows.sdk.cpp/c/Include/10.0.28000.0'
$sdkLibs = Join-Path $projectRoot '.tools/microsoft.windows.sdk.cpp.x64/c'
if (-not (Test-Path -LiteralPath $sdkHeaders)) { throw 'Falta el SDK de Windows. Consulta desktop/README.md para preparar el entorno.' }
$env:PATH = "$compiler;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:LIB = "$(Join-Path $toolset.FullName 'lib/x64');$sdkLibs/um/x64;$sdkLibs/ucrt/x64"
$env:INCLUDE = "$(Join-Path $toolset.FullName 'include');$sdkHeaders/ucrt;$sdkHeaders/shared;$sdkHeaders/um;$sdkHeaders/winrt;$sdkHeaders/cppwinrt"
$env:CC = Join-Path $compiler 'cl.exe'
$env:AR = Join-Path $compiler 'lib.exe'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $compiler 'link.exe'
$env:RUSTFLAGS = '-C target-feature=+crt-static'
$nativeTools = Join-Path $projectRoot '.tools/microsoft.windows.sdk.buildtools/bin/10.0.28000.0/x64'
if(Test-Path -LiteralPath $nativeTools) { $env:PATH = "$nativeTools;$env:PATH"; $env:RC = Join-Path $nativeTools 'rc.exe' }
$node = Join-Path $projectRoot '.tools/node'
if(Test-Path -LiteralPath (Join-Path $node 'node.exe')) { $env:PATH = "$node;$env:PATH" }

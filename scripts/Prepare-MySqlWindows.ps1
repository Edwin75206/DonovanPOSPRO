param([string]$CrtDirectory)
$ErrorActionPreference='Stop'
$project=Split-Path $PSScriptRoot -Parent
$cache=Join-Path $project '.tools/mysql'
$stage=Join-Path $project 'desktop/.runtime'
$packages=Get-Content (Join-Path $project 'packaging/dependencies.json') -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Path $cache,$stage -Force | Out-Null
foreach($name in 'mysqlWindows','workbenchWindows'){
 $package=$packages.$name
 $file=Join-Path $cache $package.file
 if(!(Test-Path -LiteralPath $file)){Invoke-WebRequest -Uri $package.url -OutFile $file -TimeoutSec 300}
 if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $package.sha256){throw "Checksum mismatch: $name"}
}
$server=Join-Path $cache 'mysql-8.4.13-winx64'
if(!(Test-Path -LiteralPath (Join-Path $server 'bin/mysqld.exe'))){Expand-Archive -LiteralPath (Join-Path $cache $packages.mysqlWindows.file) -DestinationPath $cache}
$signature=Get-AuthenticodeSignature -FilePath (Join-Path $server 'bin/mysqld.exe')
if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notlike '*Oracle America*'){throw 'Invalid Oracle MySQL signature'}
$workbench=Join-Path $cache 'workbench-extracted/MySQL/MySQL Workbench'
if(!(Test-Path -LiteralPath (Join-Path $workbench 'MySQL Workbench.exe'))){
 $msi=Join-Path $cache $packages.workbenchWindows.file
 $signature=Get-AuthenticodeSignature -FilePath $msi
 if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notlike '*Oracle America*'){throw 'Invalid Oracle Workbench signature'}
 $extract=Join-Path $cache 'workbench-extracted'
 New-Item -ItemType Directory -Path $extract -Force | Out-Null
 $arguments='/a "'+$msi+'" /qn TARGETDIR="'+$extract+'"'
 $process=Start-Process -FilePath msiexec.exe -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
 if($process.ExitCode -ne 0){throw "Workbench administrative extraction failed: $($process.ExitCode)"}
}
if(!$CrtDirectory){
 $vs=& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
 $redist=Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Redist/MSVC') -Directory | Where-Object Name -Match '^\d' | Sort-Object Name -Descending | Select-Object -First 1
 $CrtDirectory=(Get-ChildItem -LiteralPath (Join-Path $redist.FullName 'x64') -Directory | Where-Object Name -Like '*.CRT' | Select-Object -First 1).FullName
}
if(!(Test-Path -LiteralPath (Join-Path $CrtDirectory 'vcruntime140.dll'))){throw 'VC redistributable runtime directory required (not development/debug DLLs).'}
$mysqlTarget=Join-Path $stage 'mysql'
New-Item -ItemType Directory -Path $mysqlTarget -Force | Out-Null
foreach($name in 'bin','share','docs'){Copy-Item -LiteralPath (Join-Path $server $name) -Destination $mysqlTarget -Recurse -Force}
New-Item -ItemType Directory -Path (Join-Path $mysqlTarget 'lib') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $server 'lib/plugin') -Destination (Join-Path $mysqlTarget 'lib') -Recurse -Force
Copy-Item -LiteralPath (Join-Path $server 'lib/private') -Destination (Join-Path $mysqlTarget 'lib') -Recurse -Force
Copy-Item -LiteralPath (Join-Path $server 'lib/mecab') -Destination (Join-Path $mysqlTarget 'lib') -Recurse -Force
Copy-Item -LiteralPath (Join-Path $server 'LICENSE'),(Join-Path $server 'README') -Destination $mysqlTarget -Force
# Do not ship compiler symbols, static import libraries or debug-only binaries.
$resolvedStage=(Resolve-Path -LiteralPath $stage).Path
Get-ChildItem -LiteralPath $mysqlTarget -File -Recurse | Where-Object {$_.Extension -in '.pdb','.lib' -or $_.Name -like '*-debug.*'} | ForEach-Object {
 if(!$_.FullName.StartsWith($resolvedStage+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Staging path escaped its workspace'}
 Remove-Item -LiteralPath $_.FullName
}
# App-local CRT avoids asking the customer to install developer tools or a global service.
Get-ChildItem -LiteralPath $CrtDirectory -Filter '*.dll' | ForEach-Object {
 if((Get-AuthenticodeSignature -FilePath $_.FullName).Status -ne 'Valid'){throw 'Invalid Microsoft runtime signature'}
 Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $mysqlTarget 'bin') -Force
}
$wbTarget=Join-Path $stage 'workbench'
New-Item -ItemType Directory -Path $wbTarget -Force | Out-Null
Get-ChildItem -LiteralPath $workbench | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination $wbTarget -Recurse -Force}
Get-ChildItem -LiteralPath $CrtDirectory -Filter '*.dll' | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination $wbTarget -Force}
# Workbench starts MySQL Shell as a separate process. Its executable directory
# also needs the CRT; the Electron parent directory is not a DLL search fallback.
$shellBin=Join-Path $wbTarget 'resources/app/shell/bin'
if(!(Test-Path -LiteralPath (Join-Path $shellBin 'mysqlsh.exe'))){throw 'The bundled Workbench layout changed'}
Get-ChildItem -LiteralPath $CrtDirectory -Filter '*.dll' | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination $shellBin -Force}
# MSI administrative extraction preserves read-only flags. They must not prevent
# subsequent Tauri resource copies when producing an update.
Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object IsReadOnly | ForEach-Object {$_.IsReadOnly=$false}
& (Join-Path $mysqlTarget 'bin/mysqld.exe') --no-defaults --version
if($LASTEXITCODE -ne 0){throw 'The staged MySQL runtime could not execute'}
Write-Output 'MySQL and Workbench staged with verified packages. No system database was installed or changed.'

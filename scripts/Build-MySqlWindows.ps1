param([switch]$SkipPrepare)
$ErrorActionPreference='Stop'
$project=Split-Path $PSScriptRoot -Parent
Push-Location $project
try {
 . ./scripts/Enter-NativeEnvironment.ps1
 npm --prefix desktop ci
 if($LASTEXITCODE -ne 0){throw 'Could not install locked frontend dependencies'}
 if(!$SkipPrepare){& ./scripts/Prepare-MySqlWindows.ps1}
 # Older staged Oracle MSI files may already have reached the build directory.
 foreach($relative in 'desktop/.runtime','desktop/src-tauri/target/release/runtime'){
  $resources=Join-Path $project $relative
  if(Test-Path -LiteralPath $resources){Get-ChildItem -LiteralPath $resources -Recurse -File | Where-Object IsReadOnly | ForEach-Object {$_.IsReadOnly=$false}}
 }
 Push-Location desktop
 try {npm run tauri -- build --config src-tauri/tauri.mysql-windows.conf.json; if($LASTEXITCODE -ne 0){throw 'Native installer build failed'}} finally {Pop-Location}
} finally {Pop-Location}

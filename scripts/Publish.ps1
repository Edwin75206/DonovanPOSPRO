param([switch]$SkipTests, [switch]$Installer)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    if (-not $SkipTests) { dotnet test tests/DonovanPOS.Tests -c Release --logger 'trx;LogFileName=commerce.trx' --results-directory artifacts/test-results; if ($LASTEXITCODE -ne 0) { throw 'Las pruebas no pasaron.' } }
    dotnet publish src/DonovanPOS.Presentation -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true -p:PublishTrimmed=false -p:DebugType=None -p:DebugSymbols=false -o artifacts/DonovanPOS
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo publicar.' }
    Get-ChildItem -LiteralPath (Join-Path $projectRoot 'artifacts/DonovanPOS') -Filter '*.pdb' -File -Recurse | ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    & (Join-Path $PSScriptRoot 'Export-Licenses.ps1')
    Copy-Item -LiteralPath 'README.md' -Destination 'artifacts/DonovanPOS/LEEME.md' -Force
    New-Item -ItemType Directory -Path 'artifacts/DonovanPOS/docs','artifacts/DonovanPOS/licenses' -Force | Out-Null
    Copy-Item -Path 'docs/*' -Destination 'artifacts/DonovanPOS/docs' -Recurse -Force
    Copy-Item -Path 'licenses/*' -Destination 'artifacts/DonovanPOS/licenses' -Recurse -Force
    $demo = '@start "" "%~dp0DonovanPOS.exe" --demo'; Set-Content 'artifacts/DonovanPOS/Probar demostracion.cmd' $demo -Encoding ascii
    Compress-Archive -Path 'artifacts/DonovanPOS/*' -DestinationPath 'artifacts/DonovanPOS-1.0.0-portable.zip' -Force
    if ($Installer) { & (Join-Path $PSScriptRoot 'Build-Installer.ps1') }
    Get-ChildItem artifacts -File | Where-Object { $_.Extension -in '.msi','.zip' } | Get-FileHash -Algorithm SHA256 | ForEach-Object { "$($_.Hash)  $([IO.Path]::GetFileName($_.Path))" } | Set-Content 'artifacts/SHA256SUMS.txt' -Encoding ascii
} finally { Pop-Location }

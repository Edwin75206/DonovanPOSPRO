param(
    [Parameter(Mandatory)][ValidateSet('Prepare','RefreshInstaller','InstallA','InstallB','VerifyA','BusyA','BusyB','UninstallA','UninstallB')][string]$Action,
    [string]$RunDirectory
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$qaParent = Join-Path $repo 'artifacts/v2/installation'
$packageVersion = (Get-Content -LiteralPath (Join-Path $repo 'desktop/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$installer = Join-Path $repo "desktop/src-tauri/target/release/bundle/nsis/Donovan POS Pro_$($packageVersion)_x64-setup.exe"
$binary = Join-Path $repo 'desktop/src-tauri/target/release/DonovanPOS.exe'
function ExpectedNsisHash([string]$Path) {
    # Tauri patches this unique marker UNK -> NSS while bundling, then restores
    # the standalone binary. Derive the expected installed bytes independently.
    $bytes = [IO.File]::ReadAllBytes($Path)
    $text = [Text.Encoding]::Latin1.GetString($bytes)
    $marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
    $offset = $text.IndexOf($marker, [StringComparison]::Ordinal)
    if ($offset -lt 0 -or $text.IndexOf($marker, $offset + $marker.Length, [StringComparison]::Ordinal) -ge 0) { throw 'Expected exactly one Tauri standalone bundle marker.' }
    $replacement = [Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS')
    [Buffer]::BlockCopy($replacement, 0, $bytes, $offset, $replacement.Length)
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}
$uninstallKey = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall/Donovan POS Pro'
if ($Action -eq 'Prepare') {
    $existing = @('HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall','HKLM:/Software/Microsoft/Windows/CurrentVersion/Uninstall','HKLM:/Software/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall') |
        ForEach-Object { Get-ChildItem -LiteralPath $_ -ErrorAction SilentlyContinue | Get-ItemProperty -ErrorAction SilentlyContinue } |
        Where-Object { $_.DisplayName -eq 'Donovan POS Pro' }
    if ($existing) { throw 'An existing registered installation must not be modified by this isolated test.' }
    $runId = [guid]::NewGuid().ToString()
    $RunDirectory = Join-Path $qaParent $runId
    New-Item -ItemType Directory -Path $RunDirectory | Out-Null
    $manifest = [ordered]@{ purpose='DonovanPOS native installer QA'; runId=$runId; createdAt=(Get-Date).ToString('o'); installer=$installer; installerSha256=(Get-FileHash -LiteralPath $installer).Hash.ToLowerInvariant(); exeSha256=(Get-FileHash -LiteralPath $binary).Hash.ToLowerInvariant(); installedExeSha256=(ExpectedNsisHash $binary) }
    $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $RunDirectory 'qa-manifest.json') -Encoding utf8
    Write-Output $RunDirectory
    exit 0
}
if (-not $RunDirectory) { throw 'RunDirectory is required.' }
$run = (Resolve-Path -LiteralPath $RunDirectory).Path
$parent = (Resolve-Path -LiteralPath $qaParent).Path
if ((Split-Path $run -Parent) -ne $parent) { throw 'The test must be directly inside the designated QA directory.' }
$parsedId = [guid]::Empty
if (-not [guid]::TryParse((Split-Path $run -Leaf), [ref]$parsedId)) { throw 'Invalid QA directory ID.' }
$manifest = Get-Content -LiteralPath (Join-Path $run 'qa-manifest.json') -Raw | ConvertFrom-Json
if ($manifest.purpose -ne 'DonovanPOS native installer QA') { throw 'Missing QA ownership marker.' }
if ((Get-FileHash -LiteralPath $binary).Hash.ToLowerInvariant() -ne $manifest.exeSha256) { throw 'The standalone release changed during this test.' }
$expectedInstalled = ExpectedNsisHash $binary
if (-not $manifest.PSObject.Properties['installedExeSha256']) {
    $manifest | Add-Member -NotePropertyName installedExeSha256 -NotePropertyValue $expectedInstalled
    $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'qa-manifest.json') -Encoding utf8
}
if ($manifest.installedExeSha256 -ne $expectedInstalled) { throw 'Unexpected Tauri bundle marker hash.' }
if ($Action -eq 'RefreshInstaller') {
    # Permit a packaging-only revision; the standalone and installed EXE hashes
    # above must still match. Preserve the previous package identity as evidence.
    $history = Join-Path $run "package-before-$([guid]::NewGuid()).json"
    $manifest | ConvertTo-Json | Set-Content -LiteralPath $history -Encoding utf8
    $manifest.installerSha256 = (Get-FileHash -LiteralPath $installer).Hash.ToLowerInvariant()
    $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'qa-manifest.json') -Encoding utf8
    $manifest | ConvertTo-Json
    exit 0
}
$a = Join-Path $run 'program-a'
$b = Join-Path $run 'program-b'
$registered = Get-ItemProperty -LiteralPath $uninstallKey -ErrorAction SilentlyContinue
if ($registered -and $registered.InstallLocation.Trim('"').TrimEnd('\') -notin @($a,$b)) { throw 'A different registered installation appeared. Leave it untouched.' }
$destination = if ($Action -match 'B$') { $b } else { $a }
$exe = Join-Path $destination 'DonovanPOS.exe'
$expectedExit = if ($Action -like 'Busy*') { 1602 } else { 0 }
if ($Action -like 'Uninstall*') {
    $program = Join-Path $destination 'uninstall.exe'
    $arguments = "/S _?=$destination"
} else {
    if ((Get-FileHash -LiteralPath $installer).Hash.ToLowerInvariant() -ne $manifest.installerSha256) { throw 'The installer changed after the test was prepared.' }
    $program = $installer
    $arguments = "/S /NS /D=$destination"
}
# NSIS /D= and _?= must be the final argument, without enclosing quotes.
$exitCode = 0
if ($Action -ne 'VerifyA') {
    $process = Start-Process -FilePath $program -ArgumentList $arguments -WindowStyle Hidden -PassThru -Wait
    $exitCode = $process.ExitCode
}
$result = [ordered]@{ action=$Action; at=(Get-Date).ToString('o'); exitCode=$exitCode; expectedExit=$expectedExit; directory=$destination; installerSha256=$manifest.installerSha256 }
if ($exitCode -ne $expectedExit) {
    $result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run "$Action-failed-$([guid]::NewGuid()).json") -Encoding utf8
    $result | ConvertTo-Json
    throw 'Unexpected NSIS exit code.'
}
if ($Action -like 'Uninstall*') {
    if (Test-Path -LiteralPath $exe) { throw 'Uninstall left the application executable.' }
    $result.executableRemoved = $true
} else {
    if ((Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant() -ne $expectedInstalled) { throw 'Installed binary differs from the verified NSIS release.' }
    foreach ($resource in @('licenses/THIRD_PARTY_NOTICES.txt','documentation/MANUAL.md','documentation/DATOS-Y-PORTABILIDAD.md','documentation/migrations/005_ticket_profile.sql')) {
        if (-not (Test-Path -LiteralPath (Join-Path $destination $resource))) { throw "Missing bundled resource: $resource" }
    }
    $result.executableMatched = $true
    $result.resourcesPresent = $true
}
$result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run "$Action-$([guid]::NewGuid()).json") -Encoding utf8
$result | ConvertTo-Json

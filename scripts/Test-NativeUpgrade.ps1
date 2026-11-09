param(
    [string]$PreviousRelease = 'artifacts/v2.1/entrega',
    [string]$NextRelease = 'artifacts/v2.2/entrega',
    [Parameter(Mandatory)][string]$DocumentationRunDirectory
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$artifacts = (Resolve-Path -LiteralPath (Join-Path $repo 'artifacts')).Path
$uninstallKey = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall/Donovan POS Pro'
$productKey = 'HKCU:/Software/Donovan Systems/Donovan POS Pro'

function Get-Release([string]$Relative) {
    $directory = (Resolve-Path -LiteralPath (Join-Path $repo $Relative)).Path
    if (-not $directory.StartsWith($artifacts + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Use locally verified release artifacts only.' }
    $info = Get-Content -LiteralPath (Join-Path $directory 'BUILD-INFO.json') -Raw | ConvertFrom-Json
    if ($info.product -ne 'Donovan POS Pro' -or $info.version -notmatch '^\d+\.\d+\.\d+$') { throw 'Unexpected product or version.' }
    if ($info.installer.file -ne "DonovanPOS-$($info.version)-x64-Setup.exe" -or $info.executable.file -ne 'DonovanPOS.exe') { throw 'Unexpected package filenames.' }
    $setup = Join-Path $directory $info.installer.file
    $binary = Join-Path $directory 'DonovanPOS.exe'
    if ((Get-FileHash -LiteralPath $setup).Hash -ne $info.installer.sha256 -or (Get-FileHash -LiteralPath $binary).Hash -ne $info.executable.sha256) { throw 'Release hashes do not match.' }
    $bytes = [IO.File]::ReadAllBytes($binary)
    $encoded = [Text.Encoding]::Latin1.GetString($bytes)
    $marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
    $offset = $encoded.IndexOf($marker, [StringComparison]::Ordinal)
    if ($offset -lt 0 -or $encoded.IndexOf($marker, $offset + $marker.Length, [StringComparison]::Ordinal) -ge 0) { throw 'Unexpected Tauri bundle marker.' }
    $replacement = [Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS')
    [Buffer]::BlockCopy($replacement, 0, $bytes, $offset, $replacement.Length)
    [pscustomobject]@{ version=$info.version; setup=$setup; setupSha256=$info.installer.sha256; installedSha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)) }
}

$existing = @('HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall','HKLM:/Software/Microsoft/Windows/CurrentVersion/Uninstall','HKLM:/Software/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall') |
    ForEach-Object { Get-ChildItem -LiteralPath $_ -ErrorAction SilentlyContinue | Get-ItemProperty -ErrorAction SilentlyContinue } |
    Where-Object { $_.DisplayName -eq 'Donovan POS Pro' }
if ($existing) { throw 'A registered installation already exists. It must not be modified.' }
$previous = Get-Release $PreviousRelease
$next = Get-Release $NextRelease
if ([version]$next.version -le [version]$previous.version) { throw 'The next version must be newer.' }

# The data source is an owned documentation fixture, never the live business.
$source = (Resolve-Path -LiteralPath $DocumentationRunDirectory).Path
$docParent = (Resolve-Path -LiteralPath (Join-Path $artifacts 'customer-documentation')).Path
if ((Split-Path $source -Parent) -ne $docParent) { throw 'Documentation fixture required.' }
$parsed = [guid]::Empty
if (-not [guid]::TryParse((Split-Path $source -Leaf), [ref]$parsed)) { throw 'Invalid fixture ID.' }
$marker = Get-Content -LiteralPath (Join-Path $source 'manifest.json') -Raw | ConvertFrom-Json
if ($marker.purpose -ne 'Construrama Chicontla documentation') { throw 'Unexpected fixture ownership.' }
$sourceData = Join-Path $source 'data-seller'
if (-not (Test-Path -LiteralPath (Join-Path $sourceData 'donovan.db'))) { throw 'Missing fixture database.' }

$runParent = Join-Path $artifacts 'upgrades'
$run = Join-Path $runParent ([guid]::NewGuid().ToString())
$program = Join-Path $run 'program'
$data = Join-Path $run 'business'
New-Item -ItemType Directory -Path $data -Force | Out-Null
# An exclusive handle prevents copying while the fixture application is open.
$lock = [IO.File]::Open((Join-Path $sourceData '.instance.lock'), [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
try {
    Get-ChildItem -LiteralPath $sourceData -Force | Where-Object Name -ne '.instance.lock' | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $data -Recurse
    }
} finally { $lock.Dispose() }

function Get-DataSnapshot {
    $snapshot = [ordered]@{}
    Get-ChildItem -LiteralPath $data -File -Recurse -Force | Sort-Object FullName | ForEach-Object {
        $snapshot[[IO.Path]::GetRelativePath($data, $_.FullName)] = (Get-FileHash -LiteralPath $_.FullName).Hash
    }
    return ($snapshot | ConvertTo-Json -Compress)
}
function Assert-OwnedRegistry {
    $registered = Get-ItemProperty -LiteralPath $uninstallKey -ErrorAction SilentlyContinue
    if ($registered -and $registered.InstallLocation.Trim('"').TrimEnd('\') -ne $program) { throw 'Another installation appeared; leave it untouched.' }
}
function Assert-Installed($release) {
    Assert-OwnedRegistry
    $registered = Get-ItemProperty -LiteralPath $uninstallKey
    if ($registered.DisplayVersion -ne $release.version) { throw 'Registered version does not match.' }
    if ((Get-FileHash -LiteralPath (Join-Path $program 'DonovanPOS.exe')).Hash -ne $release.installedSha256) { throw 'Installed executable differs from its release.' }
}
function Run-Setup($release) {
    Assert-OwnedRegistry
    if ((Get-FileHash -LiteralPath $release.setup).Hash -ne $release.setupSha256) { throw 'Installer changed during validation.' }
    # NSIS /D must be last and unquoted; no shell command interpolation is used.
    $process = Start-Process -FilePath $release.setup -ArgumentList "/S /NS /D=$program" -WindowStyle Hidden -PassThru -Wait
    return $process.ExitCode
}

$baseline = Get-DataSnapshot
$priorProductPath = if (Test-Path -LiteralPath $productKey) { (Get-Item -LiteralPath $productKey).GetValue('') } else { $null }
$proof = [ordered]@{ purpose='DonovanPOS isolated version upgrade'; root=$run; from=$previous.version; to=$next.version; previousInstallerSha256=$previous.setupSha256; nextInstallerSha256=$next.setupSha256; files=@(($baseline | ConvertFrom-Json).PSObject.Properties).Count }
$proof | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'manifest.json') -Encoding utf8
$baseline | Set-Content -LiteralPath (Join-Path $run 'data-before.json') -Encoding utf8
try {
    $proof.installPreviousExit = Run-Setup $previous
    if ($proof.installPreviousExit -ne 0) { throw 'Previous installer failed.' }
    Assert-Installed $previous
    $proof.upgradeExit = Run-Setup $next
    if ($proof.upgradeExit -ne 0) { throw 'Upgrade failed.' }
    Assert-Installed $next
    $proof.dataPreservedAfterUpgrade = (Get-DataSnapshot) -ceq $baseline
    if (-not $proof.dataPreservedAfterUpgrade) { throw 'Business files changed during installation.' }
    $proof.downgradeExit = Run-Setup $previous
    if ($proof.downgradeExit -eq 0) { throw 'Older installer unexpectedly succeeded.' }
    Assert-Installed $next
    $proof.dataPreservedAfterRejectedDowngrade = (Get-DataSnapshot) -ceq $baseline
    if (-not $proof.dataPreservedAfterRejectedDowngrade) { throw 'Business files changed after rejected downgrade.' }
} finally {
    Assert-OwnedRegistry
    $ownedUninstaller = Join-Path $program 'uninstall.exe'
    if (Test-Path -LiteralPath $ownedUninstaller) {
        $resolved = [IO.Path]::GetFullPath($program)
        if ((Split-Path $resolved -Parent) -ne [IO.Path]::GetFullPath($run)) { throw 'Uninstall target escaped its owned workspace.' }
        $process = Start-Process -FilePath $ownedUninstaller -ArgumentList "/S _?=$program" -WindowStyle Hidden -PassThru -Wait
        $proof.uninstallExit = $process.ExitCode
        if ($proof.uninstallExit -ne 0) { throw 'Owned installation could not be removed.' }
    }
    $writableProductKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Donovan Systems\Donovan POS Pro', $true)
    if ($null -ne $writableProductKey) {
        try {
            if ($writableProductKey.GetValue('') -eq $program) {
                if ($null -eq $priorProductPath) { $writableProductKey.DeleteValue('', $false) }
                else { $writableProductKey.SetValue('', $priorProductPath) }
            }
        } finally { $writableProductKey.Dispose() }
    }
    $proof.dataPreservedAfterUninstall = (Get-DataSnapshot) -ceq $baseline
    $proof | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $run 'result.json') -Encoding utf8
    $proof | ConvertTo-Json -Depth 5
}
if (-not $proof.dataPreservedAfterUninstall -or (Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath (Join-Path $program 'DonovanPOS.exe'))) { throw 'Cleanup or data retention verification failed.' }

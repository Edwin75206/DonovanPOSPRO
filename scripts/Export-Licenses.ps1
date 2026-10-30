$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$licenseRoot = Join-Path $projectRoot 'licenses'
$assetFile = Join-Path $projectRoot 'src/DonovanPOS.Presentation/obj/project.assets.json'
$assets = Get-Content -LiteralPath $assetFile -Raw | ConvertFrom-Json -AsHashtable
$packageRoots = @($assets.packageFolders.Keys)
$inventory = [Collections.Generic.List[object]]::new()
foreach ($key in ($assets.libraries.Keys | Sort-Object)) {
    $library = $assets.libraries[$key]; if ($library.type -ne 'package') { continue }
    $parts = $key -split '/'; $folder = Join-Path $packageRoots[0] $library.path
    $nuspec = Join-Path $folder ($parts[0].ToLowerInvariant()+'.nuspec'); if (-not (Test-Path -LiteralPath $nuspec)) { continue }
    [xml]$spec = Get-Content -LiteralPath $nuspec
    $license = [string]$spec.package.metadata.license.InnerText
    if ($license.Length -eq 0 -and $parts[0] -like 'OpenTK*') { $license = 'MIT (OpenTK); ver avisos GLFW si aplica' }
    $inventory.Add([pscustomobject]@{ Package=$parts[0]; Version=$parts[1]; License=$license; Authors=[string]$spec.package.metadata.authors; Project=[string]$spec.package.metadata.projectUrl; Repository=[string]$spec.package.metadata.repository.url; Commit=[string]$spec.package.metadata.repository.commit })
    foreach ($file in (Get-ChildItem -LiteralPath $folder -File -Recurse | Where-Object { $_.Name -match '(?i)^(licen[sc]e|notice|third.party.notices|copying)' -and $_.Extension -notin '.dll','.pdb','.xml' })) {
        $target = Join-Path $licenseRoot ('packages/' + $parts[0] + '-' + $parts[1]); New-Item -ItemType Directory -Path $target -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $target $file.Name) -Force
    }
}
$inventory | Export-Csv -LiteralPath (Join-Path $licenseRoot 'DEPENDENCIES.csv') -NoTypeInformation -Encoding utf8
# Framework projection distributed by the Windows target, outside the regular NuGet library list.
[pscustomobject]@{ Package='Microsoft.Windows.SDK.NET.Ref'; Version='10.0.19041.57'; License='Microsoft Windows SDK license (see Microsoft-Windows-SDK.rtf)'; Authors='Microsoft'; Project='https://aka.ms/WinSDKProjectURL'; Repository=''; Commit='' } | Export-Csv -LiteralPath (Join-Path $licenseRoot 'DEPENDENCIES.csv') -NoTypeInformation -Encoding utf8 -Append
# The self-contained .NET runtime packs are not regular project dependencies.
foreach ($runtime in @('microsoft.netcore.app.runtime.win-x64','microsoft.windowsdesktop.app.runtime.win-x64')) {
    $folder = Join-Path $packageRoots[0] "$runtime/10.0.14"
    if (Test-Path -LiteralPath $folder) { $target=Join-Path $licenseRoot ('packages/'+$runtime+'-10.0.14'); New-Item -ItemType Directory -Path $target -Force | Out-Null; Get-ChildItem -LiteralPath $folder -File | Where-Object { $_.Name -match '(?i)license|notice' } | Copy-Item -Destination $target -Force }
}
Write-Output "Inventario: $($inventory.Count) paquetes; runtime .NET 10.0.14."

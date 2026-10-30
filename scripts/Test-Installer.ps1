param([string]$Version='1.0.0')
$ErrorActionPreference='Stop'
$projectRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$artifactRoot=Join-Path $projectRoot 'artifacts'
$package=Join-Path $artifactRoot "DonovanPOS-$Version-win-x64.msi"
$destination=Join-Path $artifactRoot 'install-test'
$dataDirectory=Join-Path $artifactRoot 'install-test-data'
$installer=New-Object -ComObject WindowsInstaller.Installer
$database=$installer.OpenDatabase($package,0)
$query=$database.OpenView('SELECT `Value` FROM `Property` WHERE `Property` = ''ProductCode'''); $query.Execute(); $record=$query.Fetch(); $productCode=$record.StringData(1); $query.Close()
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) | Out-Null
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($query) | Out-Null
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer) | Out-Null
[GC]::Collect(); [GC]::WaitForPendingFinalizers()
function RunMsi([string[]]$Arguments) { $process=Start-Process msiexec.exe -ArgumentList $Arguments -PassThru -WindowStyle Hidden; if(-not $process.WaitForExit(55000)){throw "Windows Installer sigue ejecutándose: PID $($process.Id)."}; if($process.ExitCode -ne 0){throw "Windows Installer terminó con $($process.ExitCode)."}; return $process.ExitCode }
$installExit=RunMsi @('/i',('"'+$package+'"'),'/qn','/norestart',('INSTALLDIR="'+$destination+'"'),'/l*v',('"'+(Join-Path $artifactRoot 'install.log')+'"'))
$exe=Join-Path $destination 'DonovanPOS.exe'
$installedHash=(Get-FileHash -LiteralPath $exe).Hash; $publishedHash=(Get-FileHash -LiteralPath (Join-Path $artifactRoot 'DonovanPOS/DonovanPOS.exe')).Hash
if($installedHash -ne $publishedHash){throw 'El ejecutable instalado no coincide con la publicación.'}
$menu=Join-Path $env:APPDATA 'Microsoft/Windows/Start Menu/Programs/Donovan POS Pro'
$menuExists=Test-Path (Join-Path $menu 'Donovan POS Pro.lnk')
$demoShortcutExists=Test-Path (Join-Path $menu 'Probar demostracion.lnk')
$desktopShortcut=Join-Path ([Environment]::GetFolderPath('Desktop')) 'Donovan POS Pro.lnk'
$desktopExists=Test-Path -LiteralPath $desktopShortcut
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
$testProcess=Start-Process -FilePath $exe -ArgumentList @('--data-dir',('"'+$dataDirectory+'"')) -WindowStyle Hidden -PassThru
try { Start-Sleep -Seconds 4; $testProcess.Refresh(); if($testProcess.HasExited){throw "El ejecutable terminó prematuramente: $($testProcess.ExitCode)."} }
finally { $testProcess.Refresh(); if(-not $testProcess.HasExited){Stop-Process -Id $testProcess.Id} }
$dataFile=Join-Path $dataDirectory 'donovan.db'; $before=(Get-FileHash -LiteralPath $dataFile).Hash
$marker=Join-Path $dataDirectory 'keep-after-uninstall.txt'; Set-Content -LiteralPath $marker 'La desinstalacion debe conservar los datos de prueba.'
$uninstallExit=RunMsi @('/x',$productCode,'/qn','/norestart','/l*v',('"'+(Join-Path $artifactRoot 'uninstall.log')+'"'))
$result=[ordered]@{ TestedAt=(Get-Date).ToString('o'); PackageHash=(Get-FileHash -LiteralPath $package).Hash; InstallExit=$installExit; UninstallExit=$uninstallExit; ExeMatched=$true; Started=$true; MenuCreated=$menuExists; DemoShortcutCreated=$demoShortcutExists; DesktopCreated=$desktopExists; ExeRemoved=(-not(Test-Path -LiteralPath $exe)); DataPreserved=(Test-Path -LiteralPath $dataFile); MarkerPreserved=(Test-Path -LiteralPath $marker); DataHashUnchanged=($before -eq (Get-FileHash -LiteralPath $dataFile).Hash); MenuRemoved=(-not(Test-Path -LiteralPath $menu)); DesktopRemoved=(-not(Test-Path -LiteralPath $desktopShortcut)) }
$result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $artifactRoot 'installer-validation.json')
if(@($result.GetEnumerator() | Where-Object { $_.Value -is [bool] -and -not $_.Value }).Count -gt 0){throw 'Una comprobación del instalador falló. Revisa installer-validation.json.'}
$result | ConvertTo-Json

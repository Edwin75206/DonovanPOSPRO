param([string]$Version = '1.0.0')
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$publishRoot = Join-Path $projectRoot 'artifacts/DonovanPOS'
$buildRoot = Join-Path $projectRoot 'artifacts/installer-build'
$output = Join-Path $projectRoot "artifacts/DonovanPOS-$Version-win-x64.msi"
if (-not (Test-Path -LiteralPath (Join-Path $publishRoot 'DonovanPOS.exe'))) { throw 'Primero ejecuta Publish.ps1.' }
New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
if (Test-Path -LiteralPath $output) { Remove-Item -LiteralPath $output }
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase($output, 3)
function ReleaseCom($Object) { if($null -ne $Object) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Object) } }
function Query([string]$Sql) { $view = $database.OpenView($Sql); [void]$view.Execute(); [void]$view.Close(); ReleaseCom $view }
function Row([string]$Table, [string[]]$Columns, [object[]]$Values) {
    $quoted = ($Columns | ForEach-Object { '`' + $_ + '`' }) -join ','
    $placeholders = ($Columns | ForEach-Object { '?' }) -join ','
    $view = $database.OpenView('INSERT INTO `' + $Table + '` (' + $quoted + ') VALUES (' + $placeholders + ')')
    $record = $installer.CreateRecord($Values.Count)
    for ($i = 0; $i -lt $Values.Count; $i++) { if ($null -eq $Values[$i]) { continue }; if ($Values[$i] -is [int] -or $Values[$i] -is [long]) { $record.IntegerData($i + 1) = [int]$Values[$i] } else { $record.StringData($i + 1) = [string]$Values[$i] } }
    try { [void]$view.Execute($record) } catch { $detail = $installer.LastErrorRecord; $reason = if ($null -ne $detail) { $detail.FormatText } else { $_.Exception.Message }; throw "Tabla $Table ($($Columns -join ', ')): $reason. Valores: $($Values -join ' | ')" }; [void]$view.Close(); ReleaseCom $record; ReleaseCom $view
}
function Stream([string]$Table, [string]$Name, [string]$Path) { $view = $database.OpenView('INSERT INTO `' + $Table + '` (`Name`,`Data`) VALUES (?,?)'); $record = $installer.CreateRecord(2); $record.StringData(1) = $Name; $record.SetStream(2, $Path); [void]$view.Execute($record); [void]$view.Close(); ReleaseCom $record; ReleaseCom $view }
function StableGuid([string]$Value) { $hash = [Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes('DonovanPOS:' + $Value.ToLowerInvariant())); return '{' + ([Guid]::new([byte[]]$hash[0..15])).ToString().ToUpperInvariant() + '}' }

# Native Windows Installer tables; no custom action runs scripts or deletes user data.
Query 'CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(0) NOT NULL PRIMARY KEY `Property`)'
Query 'CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72), `DefaultDir` CHAR(255) NOT NULL PRIMARY KEY `Directory`)'
Query 'CREATE TABLE `Component` (`Component` CHAR(72) NOT NULL, `ComponentId` CHAR(38), `Directory_` CHAR(72) NOT NULL, `Attributes` SHORT NOT NULL, `Condition` CHAR(255), `KeyPath` CHAR(72) PRIMARY KEY `Component`)'
Query 'CREATE TABLE `Feature` (`Feature` CHAR(38) NOT NULL, `Feature_Parent` CHAR(38), `Title` CHAR(64), `Description` CHAR(255), `Display` SHORT, `Level` SHORT NOT NULL, `Directory_` CHAR(72), `Attributes` SHORT NOT NULL PRIMARY KEY `Feature`)'
Query 'CREATE TABLE `FeatureComponents` (`Feature_` CHAR(38) NOT NULL, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Feature_`, `Component_`)'
Query 'CREATE TABLE `File` (`File` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL, `FileName` CHAR(255) NOT NULL, `FileSize` LONG NOT NULL, `Version` CHAR(72), `Language` CHAR(20), `Attributes` SHORT, `Sequence` LONG NOT NULL PRIMARY KEY `File`)'
Query 'CREATE TABLE `Media` (`DiskId` SHORT NOT NULL, `LastSequence` LONG NOT NULL, `DiskPrompt` CHAR(64), `Cabinet` CHAR(255), `VolumeLabel` CHAR(32), `Source` CHAR(72) PRIMARY KEY `DiskId`)'
Query 'CREATE TABLE `Shortcut` (`Shortcut` CHAR(72) NOT NULL, `Directory_` CHAR(72) NOT NULL, `Name` CHAR(128) NOT NULL, `Component_` CHAR(72) NOT NULL, `Target` CHAR(255) NOT NULL, `Arguments` CHAR(255), `Description` CHAR(255), `Hotkey` SHORT, `Icon_` CHAR(72), `IconIndex` SHORT, `ShowCmd` SHORT, `WkDir` CHAR(72) PRIMARY KEY `Shortcut`)'
Query 'CREATE TABLE `Icon` (`Name` CHAR(72) NOT NULL, `Data` OBJECT NOT NULL PRIMARY KEY `Name`)'
Query 'CREATE TABLE `RemoveFile` (`FileKey` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL, `FileName` CHAR(255), `DirProperty` CHAR(72) NOT NULL, `InstallMode` SHORT NOT NULL PRIMARY KEY `FileKey`)'
Query 'CREATE TABLE `LaunchCondition` (`Condition` CHAR(255) NOT NULL, `Description` CHAR(255) NOT NULL PRIMARY KEY `Condition`)'
Query 'CREATE TABLE `Upgrade` (`UpgradeCode` CHAR(38) NOT NULL, `VersionMin` CHAR(20), `VersionMax` CHAR(20), `Language` CHAR(255), `Attributes` LONG NOT NULL, `Remove` CHAR(255), `ActionProperty` CHAR(72) NOT NULL PRIMARY KEY `UpgradeCode`, `VersionMin`, `VersionMax`, `Language`, `Attributes`)'
foreach ($table in @('InstallExecuteSequence','InstallUISequence')) { Query ('CREATE TABLE `' + $table + '` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT PRIMARY KEY `Action`)') }
Query 'CREATE TABLE `TextStyle` (`TextStyle` CHAR(72) NOT NULL, `FaceName` CHAR(32) NOT NULL, `Size` SHORT NOT NULL, `Color` LONG, `StyleBits` SHORT PRIMARY KEY `TextStyle`)'
Query 'CREATE TABLE `Dialog` (`Dialog` CHAR(72) NOT NULL, `HCentering` SHORT NOT NULL, `VCentering` SHORT NOT NULL, `Width` SHORT NOT NULL, `Height` SHORT NOT NULL, `Attributes` LONG, `Title` CHAR(128), `Control_First` CHAR(50) NOT NULL, `Control_Default` CHAR(50), `Control_Cancel` CHAR(50) PRIMARY KEY `Dialog`)'
Query 'CREATE TABLE `Control` (`Dialog_` CHAR(72) NOT NULL, `Control` CHAR(50) NOT NULL, `Type` CHAR(20) NOT NULL, `X` SHORT NOT NULL, `Y` SHORT NOT NULL, `Width` SHORT NOT NULL, `Height` SHORT NOT NULL, `Attributes` LONG, `Property` CHAR(50), `Text` CHAR(0), `Control_Next` CHAR(50), `Help` CHAR(50) PRIMARY KEY `Dialog_`, `Control`)'
Query 'CREATE TABLE `ControlEvent` (`Dialog_` CHAR(72) NOT NULL, `Control_` CHAR(50) NOT NULL, `Event` CHAR(50) NOT NULL, `Argument` CHAR(255) NOT NULL, `Condition` CHAR(255), `Ordering` SHORT PRIMARY KEY `Dialog_`, `Control_`, `Event`, `Argument`, `Condition`)'
Query 'CREATE TABLE `EventMapping` (`Dialog_` CHAR(72) NOT NULL, `Control_` CHAR(50) NOT NULL, `Event` CHAR(50) NOT NULL, `Attribute` CHAR(50) NOT NULL PRIMARY KEY `Dialog_`, `Control_`, `Event`)'
Query 'CREATE TABLE `ActionText` (`Action` CHAR(72) NOT NULL, `Description` CHAR(64), `Template` CHAR(128) PRIMARY KEY `Action`)'
[IO.File]::WriteAllText((Join-Path $buildRoot 'codepage.idt'), "`r`n`r`n1252`t_ForceCodepage`r`n", [Text.Encoding]::ASCII)
$database.Import($buildRoot, 'codepage.idt')
$upgradeCode = '{79560BE4-1D4C-492B-9AB0-C2558095BB21}'
$properties = @{ ProductName='Donovan POS Pro'; ProductVersion=$Version; ProductCode=(StableGuid "product-$Version"); UpgradeCode=$upgradeCode; ProductLanguage='2058'; Manufacturer='Donovan Systems'; ALLUSERS='2'; MSIINSTALLPERUSER='1'; ARPNOMODIFY='1'; ARPPRODUCTICON='AppIcon'; ARPCOMMENTS='Punto de venta para ferreteria. Los datos del negocio se conservan al desinstalar.'; DefaultUIFont='Normal'; SecureCustomProperties='OLDPRODUCTS;NEWERPRODUCTS;INSTALLDIR'; REBOOT='ReallySuppress' }
foreach ($key in $properties.Keys) { Row 'Property' @('Property','Value') @($key,$properties[$key]) }
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('TARGETDIR',$null,'SourceDir')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('LocalAppDataFolder','TARGETDIR','.')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('ProgramsDir','LocalAppDataFolder','Programs')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('CompanyDir','ProgramsDir','Donovan Systems')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('INSTALLDIR','CompanyDir','Donovan POS Pro')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('DesktopFolder','TARGETDIR','.')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('ProgramMenuFolder','TARGETDIR','.')
Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @('AppMenuDir','ProgramMenuFolder','Donovan POS Pro')
Row 'Feature' @('Feature','Title','Description','Display','Level','Directory_','Attributes') @('Complete','Donovan POS Pro','Programa, manuales y licencias',1,1,'INSTALLDIR',0)
Row 'LaunchCondition' @('Condition','Description') @('VersionNT64','Se requiere Windows de 64 bits. Sistemas compatibles: Windows 10 (2004 o posterior) y Windows 11.')
Row 'LaunchCondition' @('Condition','Description') @('NOT NEWERPRODUCTS','Ya existe una version mas reciente de Donovan POS Pro.')
Row 'Upgrade' @('UpgradeCode','VersionMax','Attributes','ActionProperty') @($upgradeCode,$Version,1,'OLDPRODUCTS')
Row 'Upgrade' @('UpgradeCode','VersionMin','Attributes','ActionProperty') @($upgradeCode,$Version,2,'NEWERPRODUCTS')
$directoryIds = @{ '' = 'INSTALLDIR' }; $directoryIndex = 0
function DirectoryId([string]$Relative) {
    if ($directoryIds.ContainsKey($Relative)) { return $directoryIds[$Relative] }
    $parent = [IO.Path]::GetDirectoryName($Relative); $parentId = DirectoryId $parent
    $script:directoryIndex++; $id = 'D' + $script:directoryIndex; $directoryIds[$Relative] = $id
    $null = Row 'Directory' @('Directory','Directory_Parent','DefaultDir') @($id,$parentId,[IO.Path]::GetFileName($Relative)); return $id
}
$files = @(Get-ChildItem -LiteralPath $publishRoot -Recurse -File | Where-Object { $_.Extension -ne '.pdb' } | Sort-Object FullName)
$cabLines = [Collections.Generic.List[string]]::new(); foreach ($line in @('.OPTION EXPLICIT','.Set CabinetNameTemplate=payload.cab',('.Set DiskDirectoryTemplate="' + $buildRoot + '"'),'.Set Cabinet=on','.Set Compress=on','.Set CompressionType=MSZIP','.Set MaxDiskSize=0')) { $cabLines.Add($line) }
$sequence = 0; $mainComponent = ''; $seenDirectories = @{}
foreach ($file in $files) {
    $sequence++; $relative = [IO.Path]::GetRelativePath($publishRoot,$file.FullName); $directoryId = DirectoryId ([IO.Path]::GetDirectoryName($relative)); $fileId = 'F' + $sequence; $component = 'C' + $sequence
    if ($directoryId -is [array]) { throw "DirectoryId produjo $($directoryId.Count) valores: $($directoryId | ForEach-Object { $_.GetType().FullName + ': ' + $_ })" }
    Row 'Component' @('Component','ComponentId','Directory_','Attributes','KeyPath') @($component,(StableGuid $relative),$directoryId,256,$fileId)
    Row 'FeatureComponents' @('Feature_','Component_') @('Complete',$component)
    $versionInfo = if ($file.Extension -eq '.exe') { [Diagnostics.FileVersionInfo]::GetVersionInfo($file.FullName).FileVersion } else { $null }
    Row 'File' @('File','Component_','FileName','FileSize','Version','Attributes','Sequence') @($fileId,$component,$file.Name,[int]$file.Length,$versionInfo,512,$sequence)
    $cabLines.Add('"' + $file.FullName + '" ' + $fileId)
    if (-not $seenDirectories.ContainsKey($directoryId)) { Row 'RemoveFile' @('FileKey','Component_','DirProperty','InstallMode') @(('Remove' + $directoryId),$component,$directoryId,2); $seenDirectories[$directoryId] = $true }
    if ($relative -eq 'DonovanPOS.exe') { $mainComponent = $component }
}
foreach ($shortcut in @(@('Desktop','DesktopFolder','Donovan POS Pro',$null),@('Menu','AppMenuDir','Donovan POS Pro',$null),@('Demo','AppMenuDir','Probar demostracion','--demo'))) {
    Row 'Shortcut' @('Shortcut','Directory_','Name','Component_','Target','Arguments','Description','Icon_','IconIndex','ShowCmd','WkDir') @($shortcut[0],$shortcut[1],$shortcut[2],$mainComponent,'[INSTALLDIR]DonovanPOS.exe',$shortcut[3],'Donovan Systems','AppIcon',0,1,'INSTALLDIR')
}
Row 'RemoveFile' @('FileKey','Component_','DirProperty','InstallMode') @('RemoveMenu',$mainComponent,'AppMenuDir',2)
Stream 'Icon' 'AppIcon' (Join-Path $projectRoot 'assets/branding/app.ico')
$execute = @{ FindRelatedProducts=25; LaunchConditions=100; CostInitialize=800; FileCost=900; CostFinalize=1000; InstallValidate=1400; InstallInitialize=1500; RemoveExistingProducts=1510; ProcessComponents=1600; UnpublishFeatures=1800; RemoveShortcuts=3200; RemoveFiles=3500; RemoveFolders=3600; InstallFiles=4000; CreateShortcuts=4500; RegisterUser=6000; RegisterProduct=6100; PublishFeatures=6300; PublishProduct=6400; InstallFinalize=6600 }
foreach ($action in $execute.Keys) { Row 'InstallExecuteSequence' @('Action','Condition','Sequence') @($action, $(if($action -eq 'RemoveExistingProducts'){'OLDPRODUCTS'}else{$null}), $execute[$action]) }
Row 'TextStyle' @('TextStyle','FaceName','Size','Color','StyleBits') @('Normal','Segoe UI',9,3286796,0)
Row 'TextStyle' @('TextStyle','FaceName','Size','Color','StyleBits') @('Heading','Segoe UI',18,3286796,1)
function Control([string]$Dialog,[string]$Id,[string]$Type,[int]$X,[int]$Y,[int]$Width,[int]$Height,[string]$Text,[string]$Property=$null,[string]$Next=$null) { Row 'Control' @('Dialog_','Control','Type','X','Y','Width','Height','Attributes','Property','Text','Control_Next') @($Dialog,$Id,$Type,$X,$Y,$Width,$Height,3,$Property,$Text,$Next) }
Row 'Dialog' @('Dialog','HCentering','VCentering','Width','Height','Attributes','Title','Control_First','Control_Default','Control_Cancel') @('Welcome',50,50,390,250,3,'Instalar Donovan POS Pro','PathLabel','Install','Cancel')
Control 'Welcome' 'Title' 'Text' 22 18 345 32 '{\Heading}Donovan POS Pro'
Control 'Welcome' 'Description' 'Text' 22 60 345 45 'Punto de venta para ferreteria. Se instalara para esta cuenta de Windows, sin descargar componentes adicionales.'
Control 'Welcome' 'PathLabel' 'Text' 22 116 345 17 'Carpeta de instalacion:' $null 'Path'
Control 'Welcome' 'Path' 'PathEdit' 22 135 345 22 '' 'INSTALLDIR' 'Install'
Control 'Welcome' 'Data' 'Text' 22 170 345 35 'Las ventas, clientes y respaldos se guardan por separado. Desinstalar el programa conserva los datos del negocio.'
Control 'Welcome' 'Install' 'PushButton' 205 216 78 22 'Instalar' $null 'Cancel'
Control 'Welcome' 'Cancel' 'PushButton' 291 216 76 22 'Cancelar' $null 'PathLabel'
Row 'ControlEvent' @('Dialog_','Control_','Event','Argument','Condition','Ordering') @('Welcome','Install','SetTargetPath','INSTALLDIR','1',1)
Row 'ControlEvent' @('Dialog_','Control_','Event','Argument','Condition','Ordering') @('Welcome','Install','EndDialog','Return','1',2)
Row 'ControlEvent' @('Dialog_','Control_','Event','Argument','Condition','Ordering') @('Welcome','Cancel','EndDialog','Exit','1',1)
Row 'Dialog' @('Dialog','HCentering','VCentering','Width','Height','Attributes','Title','Control_First','Control_Cancel') @('Progress',50,50,390,170,1,'Donovan POS Pro','Cancel','Cancel')
Control 'Progress' 'Title' 'Text' 22 22 345 32 '{\Heading}Preparando tu negocio'
Control 'Progress' 'Status' 'Text' 22 74 345 24 'Instalando archivos...'
Control 'Progress' 'ProgressBar' 'ProgressBar' 22 110 345 16 ''
Control 'Progress' 'Cancel' 'PushButton' 291 140 76 22 'Cancelar' $null 'Cancel'
Row 'ControlEvent' @('Dialog_','Control_','Event','Argument','Condition','Ordering') @('Progress','Cancel','EndDialog','Exit','1',1)
Row 'EventMapping' @('Dialog_','Control_','Event','Attribute') @('Progress','ProgressBar','SetProgress','Progress')
Row 'EventMapping' @('Dialog_','Control_','Event','Attribute') @('Progress','Status','ActionText','Text')
foreach ($dialogInfo in @(@('Complete','Operacion completada','Puedes abrir Donovan POS Pro desde el escritorio o el menu Inicio.','-1'),@('Cancelled','Instalacion cancelada','No se completo la instalacion.','-2'),@('Failed','No se completo la instalacion','Revisa el espacio disponible y los permisos de la carpeta. Consulta el registro de Windows Installer.','-3'))) {
    Row 'Dialog' @('Dialog','HCentering','VCentering','Width','Height','Attributes','Title','Control_First','Control_Default','Control_Cancel') @($dialogInfo[0],50,50,390,190,3,'Donovan POS Pro','Finish','Finish','Finish')
    Control $dialogInfo[0] 'Title' 'Text' 22 22 345 36 ('{\Heading}' + $dialogInfo[1]); Control $dialogInfo[0] 'Description' 'Text' 22 73 345 50 $dialogInfo[2]; Control $dialogInfo[0] 'Finish' 'PushButton' 287 150 80 22 'Finalizar' $null 'Finish'
    Row 'ControlEvent' @('Dialog_','Control_','Event','Argument','Condition','Ordering') @($dialogInfo[0],'Finish','EndDialog','Return','1',1)
    Row 'InstallUISequence' @('Action','Sequence') @($dialogInfo[0],[int]$dialogInfo[3])
}
foreach ($action in @('FindRelatedProducts','LaunchConditions','CostInitialize','FileCost','CostFinalize')) { Row 'InstallUISequence' @('Action','Sequence') @($action,$execute[$action]) }
Row 'InstallUISequence' @('Action','Condition','Sequence') @('Welcome','NOT Installed',1100)
Row 'InstallUISequence' @('Action','Sequence') @('Progress',1200)
Row 'InstallUISequence' @('Action','Sequence') @('ExecuteAction',1300)
Row 'ActionText' @('Action','Description','Template') @('InstallFiles','Copiando archivos del programa...','[1]')
Row 'ActionText' @('Action','Description','Template') @('CreateShortcuts','Creando accesos directos...','[1]')
Row 'ActionText' @('Action','Description','Template') @('RemoveFiles','Quitando archivos del programa...','[1]')
# Validate every dialog's tab order before packaging (Windows Installer error 2810).
$dialogView=$database.OpenView('SELECT `Dialog`,`Control_First` FROM `Dialog`'); [void]$dialogView.Execute()
while($dialogRecord=$dialogView.Fetch()) {
    $dialogName=$dialogRecord.StringData(1); $first=$dialogRecord.StringData(2); $controls=@{}
    $controlView=$database.OpenView('SELECT `Control`,`Control_Next` FROM `Control` WHERE `Dialog_` = ?'); $parameter=$installer.CreateRecord(1); $parameter.StringData(1)=$dialogName; [void]$controlView.Execute($parameter)
    while($controlRecord=$controlView.Fetch()){ $controls[$controlRecord.StringData(1)]=$controlRecord.StringData(2); ReleaseCom $controlRecord }
    [void]$controlView.Close(); ReleaseCom $parameter; ReleaseCom $controlView; $visited=[Collections.Generic.HashSet[string]]::new(); $next=$first
    do { if(-not $controls.ContainsKey($next) -or -not $visited.Add($next)){throw "Ciclo de tabulación inválido en $dialogName."}; $next=$controls[$next]; if([string]::IsNullOrEmpty($next)){throw "Tabulación incompleta en $dialogName."} } while($next -ne $first)
    foreach($entry in $controls.GetEnumerator()){if($entry.Value -and -not $visited.Contains($entry.Key)){throw "Control fuera del ciclo en ${dialogName}: $($entry.Key)."}}
    ReleaseCom $dialogRecord
}
[void]$dialogView.Close(); ReleaseCom $dialogView
$ddf = Join-Path $buildRoot 'payload.ddf'; [IO.File]::WriteAllLines($ddf,$cabLines,[Text.Encoding]::Default)
& makecab.exe /F $ddf | Out-Null; if ($LASTEXITCODE -ne 0) { throw 'No se pudo crear el gabinete.' }
Row 'Media' @('DiskId','LastSequence','Cabinet') @(1,$sequence,'#payload.cab')
Stream '_Streams' 'payload.cab' (Join-Path $buildRoot 'payload.cab')
$summary = $database.SummaryInformation(20); $summary.Property(1)=1252; $summary.Property(2)='Donovan POS Pro'; $summary.Property(3)='Instalador para Windows x64'; $summary.Property(4)='Donovan Systems'; $summary.Property(7)='x64;2058'; $summary.Property(9)='{' + [Guid]::NewGuid().ToString().ToUpperInvariant() + '}'; $summary.Property(14)=400; $summary.Property(15)=10; $summary.Property(18)='Donovan Systems - Native MSI Builder'; $summary.Persist(); $database.Commit()
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($summary) | Out-Null
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null
[Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer) | Out-Null
[GC]::Collect(); [GC]::WaitForPendingFinalizers()
Write-Output $output

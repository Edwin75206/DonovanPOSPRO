; Tauri includes utils.nsh before this file. Preserve the standard installer,
; but require a normal application close instead of forcing Restart Manager
; shutdown, which would discard an operator's unsaved cart.
!pragma warning error 6000
!ifmacrodef CheckIfAppIsRunning
  !macroundef CheckIfAppIsRunning
!endif

!macro CheckIfAppIsRunning executablePath productName
  !define DONOVAN_CLOSE_ID ${__LINE__}
  Push $R0
  Push $R1
  Push $0
  Push $1
  Push $2
  Push $3

  donovan_retry_${DONOVAN_CLOSE_ID}:
  StrCpy $R1 0
  ${If} ${FileExists} "${executablePath}"
    StrCpy $R1 -1
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        StrCpy $R1 $0
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${EndIf}
  ${If} $R1 = ${ERROR_MORE_DATA}
    IfSilent donovan_cancel_${DONOVAN_CLOSE_ID} 0
    ${If} $PassiveMode = 1
      Goto donovan_cancel_${DONOVAN_CLOSE_ID}
    ${EndIf}
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Guarda las ventas pendientes y cierra ${productName} antes de continuar.$\r$\n$\r$\nLa aplicación de esta carpeta sigue abierta:$\r$\n${executablePath}$\r$\n$\r$\nEl instalador no cerrará tu trabajo por la fuerza." /SD IDCANCEL IDRETRY donovan_retry_${DONOVAN_CLOSE_ID}
    Goto donovan_cancel_${DONOVAN_CLOSE_ID}
  ${ElseIf} $R1 != 0
    IfSilent donovan_cancel_${DONOVAN_CLOSE_ID} 0
    MessageBox MB_OK|MB_ICONSTOP "No pudimos comprobar si la aplicación está cerrada. Cierra Donovan POS Pro y vuelve a ejecutar el instalador." /SD IDOK
    Goto donovan_cancel_${DONOVAN_CLOSE_ID}
  ${EndIf}
  Goto donovan_done_${DONOVAN_CLOSE_ID}
  donovan_cancel_${DONOVAN_CLOSE_ID}:
    SetErrorLevel 1602
    Quit
  donovan_done_${DONOVAN_CLOSE_ID}:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
  Pop $R1
  Pop $R0
  !undef DONOVAN_CLOSE_ID
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$INSTDIR\runtime\workbench\MySQL Workbench.exe"
    CreateShortcut "$SMPROGRAMS\MySQL Workbench - Donovan POS.lnk" "$INSTDIR\runtime\workbench\MySQL Workbench.exe"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "$INSTDIR\runtime\workbench\MySQL Workbench.exe" "MySQL Workbench"
  ${If} ${FileExists} "$INSTDIR\runtime\mysql\bin\mysqld.exe"
    ExecWait '"$INSTDIR\DonovanPOS.exe" --stop-local-database' $0
    ${If} $0 != 0
      SetErrorLevel 1602
      Quit
    ${EndIf}
  ${EndIf}
  Delete "$SMPROGRAMS\MySQL Workbench - Donovan POS.lnk"
!macroend

; Tauri's minimumWebview2Version path invokes the online Edge updater.
; Keep it unset. Also upgrade an existing old runtime with the offline payload.
; This file is included before Tauri defines WEBVIEW2APPGUID. Functions compile
; immediately, unlike the hook macros, so use the documented runtime GUID here.
!define DONOVAN_WEBVIEW_GUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
Function DonovanWebViewVersion
  Push $0
  ${If} ${RunningX64}
    ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${DONOVAN_WEBVIEW_GUID}" "pv"
  ${Else}
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${DONOVAN_WEBVIEW_GUID}" "pv"
  ${EndIf}
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${DONOVAN_WEBVIEW_GUID}" "pv"
  ${EndIf}
  Exch $0
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  Push $0
  Push $1
  ; Workbench includes Python modules with long relative paths. Keep the complete
  ; paths below the Windows component limit, even on hosts with long paths off.
  StrLen $0 "$INSTDIR"
  ${If} $0 > 90
    IfSilent donovan_path_too_long 0
    MessageBox MB_OK|MB_ICONEXCLAMATION "Selecciona una carpeta de instalación más corta, de hasta 90 caracteres. La ubicación sugerida es compatible con MySQL Workbench.$\r$\n$\r$\nNo se ha modificado la información del negocio." /SD IDOK
    donovan_path_too_long:
    SetErrorLevel 1602
    Quit
  ${EndIf}
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  !insertmacro CheckIfAppIsRunning "$INSTDIR\runtime\workbench\MySQL Workbench.exe" "MySQL Workbench"
  ${If} ${FileExists} "$INSTDIR\runtime\mysql\bin\mysqld.exe"
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --stop-local-database' $1
    ${If} $1 != 0
      SetErrorLevel 1602
      Quit
    ${EndIf}
  ${EndIf}
  Call DonovanWebViewVersion
  Pop $0
  ${VersionCompare} "$0" "120.0.0.0" $1
  ${If} $0 == ""
  ${OrIf} $1 = 2
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    InitPluginsDir
    File "/oname=$PLUGINSDIR\DonovanWebView2Offline.exe" "${WEBVIEW2INSTALLERPATH}"
    DetailPrint "Preparando el componente de Windows incluido, sin descargar archivos..."
    ExecWait '"$PLUGINSDIR\DonovanWebView2Offline.exe" /silent /install' $1
    ${If} $1 != 0
      IfSilent donovan_runtime_failed 0
      MessageBox MB_OK|MB_ICONSTOP "Windows no pudo preparar el componente incluido (código $1). Reinicia Windows e intenta instalar de nuevo. Tus datos se conservan." /SD IDOK
      Goto donovan_runtime_failed
    ${EndIf}
    Call DonovanWebViewVersion
    Pop $0
    ${VersionCompare} "$0" "120.0.0.0" $1
    ${If} $0 == ""
    ${OrIf} $1 = 2
      IfSilent donovan_runtime_failed 0
      MessageBox MB_OK|MB_ICONSTOP "El componente de Windows todavía no es compatible. Reinicia Windows y vuelve a instalar. Tus datos se conservan." /SD IDOK
      Goto donovan_runtime_failed
    ${EndIf}
  ${EndIf}
  Goto donovan_runtime_ready
  donovan_runtime_failed:
    SetErrorLevel 1603
    Quit
  donovan_runtime_ready:
  Pop $1
  Pop $0
!macroend

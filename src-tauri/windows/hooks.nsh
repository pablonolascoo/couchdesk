; Pasos extra del instalador de CouchDesk.
; Hasta la 1.1.0 la app se llamaba «Consola» (carpeta %LOCALAPPDATA%\Consola, programa consola.exe).
; Al instalar CouchDesk se retira esa instalación vieja SIN tocar sus datos:
; la propia app los mueve a %LOCALAPPDATA%\CouchDesk la primera vez que arranca.

!macro NSIS_HOOK_PREINSTALL
  ; La consola vieja y xTend (que corre desde la carpeta vieja) se cierran para poder moverlo todo
  nsExec::Exec 'taskkill /IM consola.exe /F'
  nsExec::Exec 'taskkill /IM xtend.exe /F'
  Sleep 600
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Quitar la instalación vieja de «Agregar o quitar programas» y sus accesos
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Consola"
  Delete "$SMPROGRAMS\Consola.lnk"
  Delete "$DESKTOP\Consola.lnk"
  Delete "$LOCALAPPDATA\Consola\consola.exe"
  Delete "$LOCALAPPDATA\Consola\uninstall.exe"

  ; Si Windows arrancaba en la consola vieja, ahora arranca en CouchDesk
  ReadRegStr $0 HKCU "Software\Microsoft\Windows NT\CurrentVersion\Winlogon" "Shell"
  StrCmp $0 '"$LOCALAPPDATA\Consola\consola.exe"' couchdesk_shell 0
  StrCmp $0 "$LOCALAPPDATA\Consola\consola.exe" couchdesk_shell couchdesk_shell_done
  couchdesk_shell:
    WriteRegStr HKCU "Software\Microsoft\Windows NT\CurrentVersion\Winlogon" "Shell" '"$INSTDIR\couchdesk.exe"'
  couchdesk_shell_done:
!macroend

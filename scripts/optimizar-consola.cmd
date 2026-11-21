@echo off
setlocal EnableDelayedExpansion
rem Optimizacion a fondo para usar el PC como consola (CouchDesk). Windows 10 y 11.
rem Clic derecho > Ejecutar como administrador. Todo se puede deshacer con
rem deshacer-optimizacion.cmd (o el boton Deshacer de CouchDesk).
rem Lo que habia antes se apunta en deshacer-registro.cmd, paso a paso y solo la primera vez
rem que se hace cada paso: volver a ejecutarlo no pisa lo guardado.
rem Este archivo tiene que quedarse en ASCII: cmd lo lee con la pagina de codigos del sistema.
net session >nul 2>&1 || (echo Abre este archivo con clic derecho ^> Ejecutar como administrador. & pause & exit /b 1)

set "UNDO=%~dp0deshacer-registro.cmd"
set "APPS=%~dp0apps-quitadas.txt"
set "BACKUP="
if exist "%UNDO%" (
  findstr /b /c:"rem paso:" "%UNDO%" >nul 2>&1
  if errorlevel 1 for %%p in (base-servicios base-gpu base-juegos base-usb base-wu) do >>"%UNDO%" echo rem paso:%%p
) else (
  >"%UNDO%" echo @echo off
  >>"%UNDO%" echo rem Creado por optimizar-consola.cmd: deja Windows como estaba
)

echo.
echo  CouchDesk - Optimizacion a fondo
echo  ================================
echo  Windows queda limpio y rapido para jugar. Todo se puede deshacer.
echo  Primero, dos preguntas. Despues ya no tienes que hacer nada.
echo.
echo  Pregunta 1 de 2. Apps que vienen con Windows y casi nadie usa:
echo     Clipchamp, Noticias, El Tiempo, Solitario, To Do, Teams, Mapas, Peliculas y TV,
echo     Enlace movil, Copilot, Outlook nuevo, Power Automate, Centro de opiniones...
echo     La Tienda, Xbox, Fotos, Calculadora, Bloc de notas, Paint y Recortes NO se tocan.
echo     Se pueden volver a instalar cuando quieras desde la Microsoft Store.
choice /c SN /m "    Quitar esas apps"
if errorlevel 2 (set "QUITAR=") else set "QUITAR=1"
echo.
echo  Pregunta 2 de 2. Bloquear las actualizaciones de Windows.
echo     No recomendado: el PC deja de recibir parches de seguridad.
echo     Si dices que no, siguen llegando, pero Windows no reinicia mientras juegas.
echo     Si en 5 minutos no contestas, se toma como no.
choice /c SN /d N /t 300 /m "    Bloquear las actualizaciones de Windows"
if errorlevel 2 (set "BLOQUEAR=") else set "BLOQUEAR=1"

echo.
echo [1/10] Telemetria: Windows deja de enviar datos de uso a Microsoft
call :paso telemetria
call :servicio DiagTrack disabled 4
call :servicio dmwappushservice disabled 4
set "DCOL=HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection"
call :poner "%DCOL%" "AllowTelemetry" REG_DWORD 0
call :poner "%DCOL%" "DoNotShowFeedbackNotifications" REG_DWORD 1
call :poner "HKCU\Software\Microsoft\Siuf\Rules" "NumberOfSIUFInPeriod" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo" "DisabledByGroupPolicy" REG_DWORD 1
call :poner "HKCU\Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo" "Enabled" REG_DWORD 0
call :poner "HKCU\Software\Microsoft\Windows\CurrentVersion\Privacy" "TailoredExperiencesWithDiagnosticDataEnabled" REG_DWORD 0
call :poner "HKCU\Software\Policies\Microsoft\Windows\CloudContent" "DisableTailoredExperiencesWithDiagnosticData" REG_DWORD 1
echo     Tareas programadas que recogen datos:
call :tareas "\Microsoft\Windows\Customer Experience Improvement Program\" "*"
call :tareas "\Microsoft\Windows\Application Experience\" "Microsoft Compatibility Appraiser*"
call :tareas "\Microsoft\Windows\Application Experience\" "ProgramDataUpdater"
call :tareas "\Microsoft\Windows\Autochk\" "Proxy"
call :tareas "\Microsoft\Windows\DiskDiagnostic\" "Microsoft-Windows-DiskDiagnosticDataCollector"
call :tareas "\Microsoft\Windows\Feedback\Siuf\" "*"

echo [2/10] Anuncios y sugerencias: fuera los consejos, apps sugeridas y "saca mas partido a Windows"
call :paso anuncios
set "CDM=HKCU\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager"
for %%v in (SilentInstalledAppsEnabled SystemPaneSuggestionsEnabled SoftLandingEnabled PreInstalledAppsEnabled OemPreInstalledAppsEnabled RotatingLockScreenOverlayEnabled SubscribedContent-310093Enabled SubscribedContent-338387Enabled SubscribedContent-338388Enabled SubscribedContent-338389Enabled SubscribedContent-338393Enabled SubscribedContent-353694Enabled SubscribedContent-353696Enabled SubscribedContent-353698Enabled) do (
  call :poner "%CDM%" "%%v" REG_DWORD 0
)
call :poner "HKCU\Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement" "ScoobeSystemSettingEnabled" REG_DWORD 0
set "ADV=HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced"
call :poner "%ADV%" "Start_IrisRecommendations" REG_DWORD 0
call :poner "%ADV%" "Start_AccountNotifications" REG_DWORD 0
call :poner "%ADV%" "ShowSyncProviderNotifications" REG_DWORD 0
set "CC=HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent"
call :poner "%CC%" "DisableWindowsConsumerFeatures" REG_DWORD 1
call :poner "%CC%" "DisableSoftLanding" REG_DWORD 1

echo [3/10] Ruido de fondo: sin Widgets ni noticias, sin Copilot, sin Bing en el buscador,
echo        sin Cortana, Edge no se queda abierto por detras y las descargas no se comparten
call :paso ruido
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Dsh" "AllowNewsAndInterests" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\Windows Feeds" "EnableFeeds" REG_DWORD 0
call :poner "%ADV%" "TaskbarDa" REG_DWORD 0 q
call :poner "%ADV%" "ShowCopilotButton" REG_DWORD 0
call :poner "HKCU\Software\Policies\Microsoft\Windows\WindowsCopilot" "TurnOffWindowsCopilot" REG_DWORD 1
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsCopilot" "TurnOffWindowsCopilot" REG_DWORD 1
call :poner "HKCU\Software\Policies\Microsoft\Windows\Explorer" "DisableSearchBoxSuggestions" REG_DWORD 1
call :poner "HKCU\Software\Microsoft\Windows\CurrentVersion\Search" "BingSearchEnabled" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\Windows Search" "AllowCortana" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Edge" "StartupBoostEnabled" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Edge" "BackgroundModeEnabled" REG_DWORD 0
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization" "DODownloadMode" REG_DWORD 0

echo [4/10] Servicios que la consola no usa: busqueda indexada y precarga apagadas,
echo        impresion solo cuando haga falta
call :paso base-servicios
call :servicio WSearch disabled 4
call :servicio SysMain disabled 4
call :servicio Spooler demand 3

echo [5/10] Energia: plan de maximo rendimiento, procesador siempre a tope enchufado,
echo        y ni el USB ni la tarjeta grafica se duermen (el mando no se desconecta)
call :paso energia
set "CDPLAN=c0ced35c-7a3f-4e7b-9c1d-5e0c0de5c0de"
set "prev="
for /f "tokens=2 delims=:(" %%g in ('powercfg /getactivescheme') do for /f %%h in ("%%g") do set "prev=%%h"
set "creado="
powercfg /list | findstr /i /c:"%CDPLAN%" >nul 2>&1
if errorlevel 1 (
  powercfg /duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61 %CDPLAN% >nul 2>&1
  powercfg /list | findstr /i /c:"%CDPLAN%" >nul 2>&1 || powercfg /duplicatescheme 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c %CDPLAN% >nul 2>&1
  powercfg /list | findstr /i /c:"%CDPLAN%" >nul 2>&1 && set "creado=1"
)
if defined creado powercfg /changename %CDPLAN% "CouchDesk - Maximo rendimiento" "Toda la potencia para jugar. Lo crea CouchDesk." >nul 2>&1
if defined BACKUP if defined prev if /i not "!prev!"=="%CDPLAN%" >>"%UNDO%" echo powercfg /setactive !prev! ^>nul 2^>^&1
if defined creado >>"%UNDO%" echo powercfg /delete %CDPLAN% ^>nul 2^>^&1
powercfg /list | findstr /i /c:"%CDPLAN%" >nul 2>&1 && (
  powercfg /setactive %CDPLAN% >nul 2>&1 && echo     Plan activo: CouchDesk - Maximo rendimiento
) || echo     Este PC no deja crear un plan de maximo rendimiento: se ajusta el plan actual
set "act="
for /f "tokens=2 delims=:(" %%g in ('powercfg /getactivescheme') do for /f %%h in ("%%g") do set "act=%%h"
if defined act (
  call :energia !act! SUB_PROCESSOR PROCTHROTTLEMIN 100
  call :energia !act! SUB_PCIEXPRESS ASPM 0
)
call :paso base-usb
set "USBSUB=2a737441-1930-4402-8d77-b2bebba308a3"
set "USBSET=48e6b7a6-50f5-4782-a5d4-53bb8f07e226"
for /f "tokens=2 delims=:(" %%g in ('powercfg /list ^| findstr /r /c:"[0-9a-f]*-[0-9a-f]*-[0-9a-f]*-"') do (
  for /f "tokens=1" %%h in ("%%g") do call :usb %%h
)
powercfg /setactive SCHEME_CURRENT >nul 2>&1

echo [6/10] Juegos primero: la tarjeta grafica organiza su trabajo sola, el juego que tienes
echo        delante manda, la red no se frena y Windows no graba tus partidas por detras
call :paso base-gpu
call :poner "HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers" "HwSchMode" REG_DWORD 2
call :paso base-juegos
set "MM=HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"
call :poner "%MM%" "SystemResponsiveness" REG_DWORD 0
call :poner "%MM%\Tasks\Games" "GPU Priority" REG_DWORD 8
call :poner "%MM%\Tasks\Games" "Priority" REG_DWORD 6
call :poner "%MM%\Tasks\Games" "Scheduling Category" REG_SZ High
call :poner "%MM%\Tasks\Games" "SFIO Priority" REG_SZ High
call :paso rendimiento
call :poner "%MM%" "NetworkThrottlingIndex" REG_DWORD 0xffffffff
call :poner "HKLM\SYSTEM\CurrentControlSet\Control\PriorityControl" "Win32PrioritySeparation" REG_DWORD 0x26
call :poner "HKLM\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling" "PowerThrottlingOff" REG_DWORD 1
call :poner "HKLM\SOFTWARE\Policies\Microsoft\Windows\GameDVR" "AllowGameDVR" REG_DWORD 0

echo [7/10] Efectos visuales de Windows al minimo (se quedan las letras suaves y las miniaturas).
echo        CouchDesk sigue animada igual: no depende de esto.
call :paso visual
call :poner "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects" "VisualFXSetting" REG_DWORD 3
call :poner "HKCU\Control Panel\Desktop" "UserPreferencesMask" REG_BINARY 9012038010000000
call :poner "HKCU\Control Panel\Desktop" "DragFullWindows" REG_SZ 0
call :poner "HKCU\Control Panel\Desktop" "FontSmoothing" REG_SZ 2
call :poner "HKCU\Control Panel\Desktop\WindowMetrics" "MinAnimate" REG_SZ 0
call :poner "%ADV%" "TaskbarAnimations" REG_DWORD 0
call :poner "%ADV%" "ListviewAlphaSelect" REG_DWORD 0
call :poner "%ADV%" "ListviewShadow" REG_DWORD 0
call :poner "%ADV%" "IconsOnly" REG_DWORD 0
call :poner "HKCU\Software\Microsoft\Windows\DWM" "EnableAeroPeek" REG_DWORD 0

echo [8/10] Antivirus: sigue encendido. Solo deja de revisar las carpetas de juegos y emuladores
echo        mientras juegas (cargan mas rapido)
call :paso defender
call :excluir "%USERPROFILE%\Juegos"
call :excluir "%LOCALAPPDATA%\CouchDesk\emuladores"

echo [9/10] Windows Update: horas activas de 8:00 a 2:00 y sin reinicios con la sesion abierta
call :paso base-wu
set "UX=HKLM\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings"
set "AU=HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU"
call :poner "%UX%" "SmartActiveHoursState" REG_DWORD 0
call :poner "%UX%" "ActiveHoursStart" REG_DWORD 8
call :poner "%UX%" "ActiveHoursEnd" REG_DWORD 2
call :poner "%AU%" "NoAutoRebootWithLoggedOnUsers" REG_DWORD 1
if not defined BLOQUEAR goto :sinbloqueo
echo        Has elegido bloquearlas: Windows deja de buscarlas y de instalarlas solo
call :paso wu-bloqueo
call :poner "%AU%" "NoAutoUpdate" REG_DWORD 1
call :poner "%AU%" "AUOptions" REG_DWORD 1
call :servicio UsoSvc disabled 4
call :servicio WaaSMedicSvc disabled 4
rem wuauserv queda en manual (no apagado): la Tienda y la app de Xbox lo necesitan para descargar juegos
call :servicio wuauserv demand 3
sc stop wuauserv >nul 2>&1
call :tareas "\Microsoft\Windows\UpdateOrchestrator\" "*"
call :tareas "\Microsoft\Windows\WaaSMedic\" "*"
call :tareas "\Microsoft\Windows\WindowsUpdate\" "*"
:sinbloqueo

if not defined QUITAR goto :sinapps
echo [10/10] Quitando apps que vienen de serie (solo para tu usuario)
call :paso apps
if defined BACKUP >>"%UNDO%" echo echo Apps quitadas: se pueden volver a instalar desde la Microsoft Store, o con:
call :app Clipchamp.Clipchamp 9P1J8S7CCWWT "Clipchamp"
call :app Microsoft.BingNews 9WZDNCRFHVFW "Noticias"
call :app Microsoft.BingWeather 9WZDNCRFJ3Q2 "El Tiempo"
call :app Microsoft.GetHelp 9PKDZBMV1H3T "Obtener ayuda"
call :app Microsoft.Getstarted 9WZDNCRDTBJJ "Consejos"
call :app Microsoft.MicrosoftSolitaireCollection 9WZDNCRFHWD2 "Solitario"
call :app Microsoft.People 9NBLGGH10PG8 "Contactos"
call :app Microsoft.PowerAutomateDesktop 9NFTCH6J7FHV "Power Automate"
call :app Microsoft.Todos 9NBLGGH5R558 "To Do"
call :app MSTeams XP8BT8DW290MPQ "Teams"
call :app MicrosoftTeams XP8BT8DW290MPQ "Teams chat"
call :app Microsoft.WindowsFeedbackHub 9NBLGGH4R32N "Centro de opiniones"
call :app Microsoft.WindowsMaps 9WZDNCRDTBVB "Mapas"
call :app Microsoft.ZuneVideo 9WZDNCRFJ3P2 "Peliculas y TV"
call :app Microsoft.YourPhone 9NMPJ99VJBWV "Enlace movil"
call :app Microsoft.Copilot 9NHT9RB2F4HD "Copilot"
call :app Microsoft.OutlookForWindows 9NRX63209R7B "Outlook nuevo"
call :app Microsoft.Windows.DevHome 9N8MHTZ6CBH4 "Dev Home"
:sinapps
if not defined QUITAR echo [10/10] Apps de serie: no se tocan

echo.
echo  ================================ Resumen ================================
echo   Telemetria, publicidad, consejos y sugerencias de Windows: apagados
echo   Widgets, Copilot, Cortana y Bing en el buscador: fuera
echo   Busqueda indexada y precarga: apagadas. Impresion: solo si hace falta
echo   Energia: maximo rendimiento, procesador a tope, USB y PCI Express sin dormirse
echo   Juegos: prioridad alta, GPU por hardware, red sin frenos, sin grabacion de fondo
echo   Efectos visuales de Windows: al minimo (letras suaves y miniaturas se quedan)
echo   Antivirus: encendido. No revisa Juegos ni emuladores mientras juegas
if defined BLOQUEAR (echo   Actualizaciones de Windows: BLOQUEADAS. Para volver a recibirlas, usa Deshacer) else echo   Actualizaciones de Windows: siguen llegando, sin reiniciar mientras juegas
if defined QUITAR (echo   Apps de serie: quitadas. Lista en apps-quitadas.txt) else echo   Apps de serie: no se han tocado
echo  ==========================================================================
echo.
echo  Reinicia el PC para que todo haga efecto.
echo  Para deshacerlo todo: boton Deshacer de CouchDesk (o deshacer-optimizacion.cmd).
pause
exit /b 0


rem ======================= Ayudantes =======================

rem ---- :paso NOMBRE -> si este paso nunca se apunto, apunta lo de antes (BACKUP=1) ----
:paso
set "BACKUP="
findstr /b /c:"rem paso:%~1" "%UNDO%" >nul 2>&1 && exit /b 0
set "BACKUP=1"
>>"%UNDO%" echo rem paso:%~1
exit /b 0

rem ---- :guardar CLAVE VALOR TIPO -> apunta en deshacer como estaba el valor ----
:guardar
if not defined BACKUP exit /b 0
set "old=" & set "line="
for /f "tokens=*" %%L in ('reg query "%~1" /v "%~2" 2^>nul ^| findstr /c:"%~3"') do set "line=%%L"
if defined line (
  set "old=!line:*%~3=!"
  for /f "tokens=*" %%v in ("!old!") do set "old=%%v"
)
if defined old (
  >>"%UNDO%" echo reg add "%~1" /v "%~2" /t %~3 /d "!old!" /f ^>nul 2^>^&1
) else (
  >>"%UNDO%" echo reg delete "%~1" /v "%~2" /f ^>nul 2^>^&1
)
exit /b 0

rem ---- :poner CLAVE VALOR TIPO DATO [q] -> apunta lo de antes y pone el nuevo ----
:poner
call :guardar "%~1" "%~2" %~3
reg add "%~1" /v "%~2" /t %~3 /d "%~4" /f >nul 2>&1 && exit /b 0
if "%~5"=="" echo     Windows no deja cambiar %~2 (se sigue con lo demas)
exit /b 0

rem ---- :servicio NOMBRE MODO NUMERO -> apunta como arrancaba y lo cambia ----
:servicio
set "SK=HKLM\SYSTEM\CurrentControlSet\Services\%~1"
reg query "%SK%" /v Start >nul 2>&1 || exit /b 0
if defined BACKUP (
  set "n=" & set "line="
  for /f "tokens=3" %%v in ('reg query "%SK%" /v Start 2^>nul ^| findstr /c:"REG_DWORD"') do set /a "n=%%v"
  set "m="
  if "!n!"=="2" set "m=auto"
  if "!n!"=="3" set "m=demand"
  if "!n!"=="4" set "m=disabled"
  if defined m >>"%UNDO%" echo sc config %~1 start= !m! ^>nul 2^>^&1
  if defined n >>"%UNDO%" echo reg add "%SK%" /v Start /t REG_DWORD /d !n! /f ^>nul 2^>^&1
)
sc config %~1 start= %~2 >nul 2>&1 || reg add "%SK%" /v Start /t REG_DWORD /d %~3 /f >nul 2>&1 || echo     Windows protege el servicio %~1: no se puede cambiar
if "%~2"=="disabled" sc stop %~1 >nul 2>&1
exit /b 0

rem ---- :tareas CARPETA PATRON -> apaga las tareas programadas encendidas y apunta cuales ----
:tareas
powershell -NoProfile -Command "$u=$env:UNDO; $b=$env:BACKUP; Get-ScheduledTask -TaskPath '%~1' -ErrorAction SilentlyContinue | Where-Object { $_.TaskName -like '%~2' -and $_.State -ne 'Disabled' } | ForEach-Object { $n=$_.TaskName; $f=$_.TaskPath+$n; try { Disable-ScheduledTask -TaskPath $_.TaskPath -TaskName $n -ErrorAction Stop | Out-Null; if ($b) { Add-Content -LiteralPath $u -Encoding Ascii -Value ('schtasks /Change /TN '+[char]34+$f+[char]34+' /Enable >nul 2>&1') }; Write-Output ('      apagada: '+$n) } catch { Write-Output ('      Windows la protege, sigue igual: '+$n) } }"
exit /b 0

rem ---- :energia PLAN SUBGRUPO AJUSTE VALOR -> cambia un ajuste de energia enchufado y apunta el de antes ----
:energia
set "p1=" & set "p2="
for /f "tokens=2 delims=:" %%v in ('powercfg /q %1 %2 %3 2^>nul ^| findstr /c:": 0x"') do (
  set "p1=!p2!"
  set /a "p2=%%v"
)
if not defined p1 exit /b 0
if defined BACKUP >>"%UNDO%" echo powercfg /setacvalueindex %1 %2 %3 !p1! ^>nul 2^>^&1
powercfg /setacvalueindex %1 %2 %3 %4 >nul 2>&1
exit /b 0

rem ---- :usb GUID-del-plan -> apunta como estaba y quita el ahorro de USB ----
:usb
set "ac=" & set "dc="
for /f "tokens=2 delims=:" %%v in ('powercfg /q %1 %USBSUB% %USBSET% 2^>nul ^| findstr /c:": 0x"') do (
  if not defined ac (set /a "ac=%%v") else if not defined dc set /a "dc=%%v"
)
if not defined ac exit /b 0
if defined BACKUP (
  >>"%UNDO%" echo powercfg /setacvalueindex %1 %USBSUB% %USBSET% !ac! ^>nul 2^>^&1
  if defined dc >>"%UNDO%" echo powercfg /setdcvalueindex %1 %USBSUB% %USBSET% !dc! ^>nul 2^>^&1
)
powercfg /setacvalueindex %1 %USBSUB% %USBSET% 0 >nul 2>&1
powercfg /setdcvalueindex %1 %USBSUB% %USBSET% 0 >nul 2>&1
exit /b 0

rem ---- :excluir CARPETA -> el antivirus no la revisa; apunta como quitar la excepcion ----
:excluir
set "r="
for /f %%r in ('powershell -NoProfile -Command "try { if ((Get-MpPreference).ExclusionPath -contains '%~1') { 'ya' } else { Add-MpPreference -ExclusionPath '%~1' -ErrorAction Stop; 'ok' } } catch { 'no' }"') do set "r=%%r"
if "!r!"=="ok" (
  echo     Sin revisar: %~1
  if defined BACKUP >>"%UNDO%" echo powershell -NoProfile -Command "Remove-MpPreference -ExclusionPath '%~1'" ^>nul 2^>^&1
)
if "!r!"=="ya" echo     Ya estaba sin revisar: %~1
if "!r!"=="no" echo     El antivirus no deja cambiarlo (quizas usas otro antivirus): %~1
exit /b 0

rem ---- :app PAQUETE ID-TIENDA "NOMBRE" -> la quita para tu usuario y apunta como reinstalarla ----
rem      NOMBRE sin parentesis: va dentro de un bloque if ( ... ) y lo cerraria.
:app
set "r="
for /f %%r in ('powershell -NoProfile -Command "$p=@(Get-AppxPackage -Name '%~1'); if ($p.Count) { try { foreach ($x in $p) { Remove-AppxPackage -Package $x.PackageFullName -ErrorAction Stop }; 'ok' } catch { 'no' } }"') do set "r=%%r"
if "!r!"=="ok" (
  echo     Quitada: %~3
  if defined BACKUP >>"%UNDO%" echo echo     %~3: winget install --id %~2 -s msstore
  >>"%APPS%" echo %~3: Microsoft Store, o winget install --id %~2 -s msstore
)
if "!r!"=="no" echo     No se pudo quitar: %~3
exit /b 0

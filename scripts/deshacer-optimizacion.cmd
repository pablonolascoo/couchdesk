@echo off
rem Deja Windows como estaba antes de optimizar-consola.cmd.
rem Clic derecho > Ejecutar como administrador.
rem Este archivo tiene que quedarse en ASCII: cmd lo lee con la pagina de codigos del sistema.
net session >nul 2>&1 || (echo Abre este archivo con clic derecho ^> Ejecutar como administrador. & pause & exit /b 1)

echo Servicios de busqueda, precarga e impresion: como vienen en Windows
sc config WSearch start= delayed-auto >nul
sc config SysMain start= auto >nul
sc config Spooler start= auto >nul

if exist "%~dp0deshacer-registro.cmd" (
  echo Devolviendo cada cambio a como estaba: registro, servicios, tareas programadas,
  echo plan de energia, antivirus y actualizaciones de Windows...
  echo.
  call "%~dp0deshacer-registro.cmd"
  powercfg /setactive SCHEME_CURRENT >nul 2>&1
  del "%~dp0deshacer-registro.cmd" >nul 2>&1
) else (
  echo No hay mas cambios apuntados que deshacer.
)
del "%~dp0estado-servicios.txt" >nul 2>&1

echo.
echo Arrancando los servicios que vuelven a estar encendidos...
for %%s in (WSearch SysMain Spooler DiagTrack dmwappushservice UsoSvc wuauserv) do sc start %%s >nul 2>&1

if exist "%~dp0apps-quitadas.txt" (
  echo.
  echo Las apps quitadas no vuelven solas. Instalalas desde la Microsoft Store si las quieres.
  echo La lista esta en apps-quitadas.txt, junto a este archivo.
)

echo.
echo Listo. Reinicia el PC para que todo vuelva a ser como antes.
pause

@echo off
rem Ajustes finales para usar el PC solo como consola. Todo se puede deshacer
rem con deshacer-optimizacion.cmd. Clic derecho > Ejecutar como administrador.
net session >nul 2>&1 || (echo Abre este archivo con clic derecho ^> Ejecutar como administrador. & pause & exit /b 1)

echo Guardando como estaba...
sc qc WSearch  | find "START_TYPE" >  "%~dp0estado-servicios.txt"
sc qc SysMain  | find "START_TYPE" >> "%~dp0estado-servicios.txt"
sc qc Spooler  | find "START_TYPE" >> "%~dp0estado-servicios.txt"

echo 1/3 Indexado de busqueda de Windows: apagado (la consola no lo usa)
sc config WSearch start= disabled >nul & sc stop WSearch >nul 2>&1

echo 2/3 SysMain (precarga en disco): apagado (con SSD no aporta y gasta memoria)
sc config SysMain start= disabled >nul & sc stop SysMain >nul 2>&1

echo 3/3 Impresion: solo cuando haga falta
sc config Spooler start= demand >nul

echo.
echo Listo. Reinicia el PC: la consola arrancara sin cargar el escritorio de Windows.
pause

@echo off
rem Instala CouchDesk en el Windows real con el último instalador compilado.
rem Se lanza desde el Programador de tareas para que no quede dentro de otra app.
rem El instalador retira la antigua «Consola» y arregla el arranque; la app mueve sus datos al abrirse.
set "OUT=C:\Users\CASA\Consola\scripts\resultado.txt"
set "KEY=HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon"

echo === antes === > "%OUT%"
reg query "%KEY%" /v Shell >> "%OUT%" 2>&1

rem Siempre el instalador más reciente, tenga la versión que tenga
set "NSIS=C:\Users\CASA\Consola\src-tauri\target\release\bundle\nsis"
for /f "delims=" %%F in ('dir /b /o-d "%NSIS%\CouchDesk_*_x64-setup.exe"') do if not defined SETUP set "SETUP=%NSIS%\%%F"
echo instalador: %SETUP% >> "%OUT%"

taskkill /im couchdesk.exe /f >nul 2>&1
taskkill /im consola.exe /f >nul 2>&1
start "" /wait "%SETUP%" /S
rem Con la misma versión ya instalada el instalador puede no reemplazar nada: se copia el programa recién compilado
taskkill /im couchdesk.exe /f >nul 2>&1
copy /y "C:\Users\CASA\Consola\src-tauri\target\release\couchdesk.exe" "%LOCALAPPDATA%\CouchDesk\couchdesk.exe" >> "%OUT%" 2>&1

rem Si el arranque en CouchDesk ya estaba activo, que apunte al programa nuevo
reg query "%KEY%" /v Shell | find /i "consola" >nul && reg add "%KEY%" /v Shell /t REG_SZ /d "\"%LOCALAPPDATA%\CouchDesk\couchdesk.exe\"" /f >> "%OUT%" 2>&1

rem Windows guarda copias de los iconos: se refrescan para que salga el nuevo
ie4uinit.exe -show >nul 2>&1
del /q /a "%LOCALAPPDATA%\Microsoft\Windows\Explorer\iconcache_*.db" >nul 2>&1

echo === despues === >> "%OUT%"
dir /b "%LOCALAPPDATA%\CouchDesk" >> "%OUT%" 2>&1
echo --- carpeta vieja: >> "%OUT%"
dir /b "%LOCALAPPDATA%\Consola" >> "%OUT%" 2>&1
reg query "%KEY%" /v Shell >> "%OUT%" 2>&1
echo === fin === >> "%OUT%"

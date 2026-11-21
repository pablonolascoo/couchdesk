@echo off
rem Instala la consola en el Windows real y la pone como escritorio de este usuario.
rem Se lanza desde el Programador de tareas para que no quede dentro de otra app.
set "OUT=C:\Users\CASA\Consola\scripts\resultado.txt"
set "KEY=HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon"

echo === antes === > "%OUT%"
reg query "%KEY%" /v Shell >> "%OUT%" 2>&1

taskkill /im consola.exe /f >nul 2>&1
start "" /wait "C:\Users\CASA\Consola\src-tauri\target\release\bundle\nsis\Consola_0.1.0_x64-setup.exe" /S

if exist "%LOCALAPPDATA%\Consola\consola.exe" (
  reg add "%KEY%" /v Shell /t REG_SZ /d "\"%LOCALAPPDATA%\Consola\consola.exe\"" /f >> "%OUT%" 2>&1
) else (
  echo ERROR: no se instalo >> "%OUT%"
)

rem Windows guarda copias de los iconos: se refrescan para que salga el nuevo
ie4uinit.exe -show >nul 2>&1
del /q /a "%LOCALAPPDATA%\Microsoft\Windows\Explorer\iconcache_*.db" >nul 2>&1

echo === despues === >> "%OUT%"
dir /b "%LOCALAPPDATA%\Consola" >> "%OUT%" 2>&1
reg query "%KEY%" /v Shell >> "%OUT%" 2>&1
echo === fin === >> "%OUT%"

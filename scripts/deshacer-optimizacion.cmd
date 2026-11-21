@echo off
rem Deja los servicios como venian. Clic derecho > Ejecutar como administrador.
net session >nul 2>&1 || (echo Abre este archivo con clic derecho ^> Ejecutar como administrador. & pause & exit /b 1)
sc config WSearch start= delayed-auto >nul & sc start WSearch >nul 2>&1
sc config SysMain start= auto >nul & sc start SysMain >nul 2>&1
sc config Spooler start= auto >nul & sc start Spooler >nul 2>&1
echo Servicios restaurados.
pause

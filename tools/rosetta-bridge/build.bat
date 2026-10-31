@echo off
rem Build the RosettaStone bridge for Emberstone difftest.
rem ASCII only: cmd reads .bat files with the OEM code page.
rem
rem Requirements:
rem   - RosettaStone checkout at commit 1690208 in %ROSETTA_ROOT%\RosettaStone
rem   - vcpkg in %VCPKG_ROOT% (default %ROSETTA_ROOT%\vcpkg), CMake and Ninja on PATH
rem     (%ROSETTA_ROOT%\.tools\Scripts is added to PATH if it exists)
rem   - Visual Studio 2022 C++ tools (VCVARS, or found with vswhere)
rem Output: tools\rosetta-bridge\build\rosetta-bridge.exe
setlocal
if "%ROSETTA_ROOT%"=="" set "ROSETTA_ROOT=D:\HearthstoneCollection\rosetta"
if "%VCPKG_ROOT%"=="" set "VCPKG_ROOT=%ROSETTA_ROOT%\vcpkg"
if "%VCVARS%"=="" (
    for /f "usebackq delims=" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath`) do set "VCVARS=%%i\VC\Auxiliary\Build\vcvars64.bat"
)
set "BRIDGE_DIR=%~dp0"
set "SRC=%ROSETTA_ROOT%\RosettaStone"
set "ROSETTA_COMMIT=1690208"
set "PATCHES=0001-build-options.patch;0002-rng-call-sites.patch;0003-fix-adjacent-aura-use-after-free.patch"

rem vcvars may overwrite VCPKG_ROOT with the copy bundled in Visual Studio, so keep ours
set "MY_VCPKG_ROOT=%VCPKG_ROOT%"
call "%VCVARS%" >nul || exit /b 1
set "VCPKG_ROOT=%MY_VCPKG_ROOT%"
if exist "%ROSETTA_ROOT%\.tools\Scripts" set "PATH=%ROSETTA_ROOT%\.tools\Scripts;%PATH%"
cd /d "%SRC%" || exit /b 1

for /f %%h in ('git rev-parse --short HEAD') do set "HEAD=%%h"
if not "%HEAD%"=="%ROSETTA_COMMIT%" (
    echo RosettaStone HEAD is %HEAD%, expected %ROSETTA_COMMIT%
    exit /b 1
)

rem Apply patches (skip the ones already applied)
for %%p in (0001-build-options.patch 0002-rng-call-sites.patch 0003-fix-adjacent-aura-use-after-free.patch) do (
    git apply --check -R --ignore-whitespace "%BRIDGE_DIR%patches\%%p" 2>nul && (
        echo %%p already applied
    ) || (
        git apply --ignore-whitespace "%BRIDGE_DIR%patches\%%p" || exit /b 1
        echo %%p applied
    )
)

cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release ^
  -DCMAKE_C_COMPILER=cl -DCMAKE_CXX_COMPILER=cl ^
  "-DCMAKE_TOOLCHAIN_FILE=%VCPKG_ROOT%\scripts\buildsystems\vcpkg.cmake" ^
  -DVCPKG_TARGET_TRIPLET=x64-windows ^
  "-DROSETTA_SHIM_DIR=%BRIDGE_DIR%shim" || exit /b 1
ninja -C build RosettaStone || exit /b 1

if not exist "%BRIDGE_DIR%build" mkdir "%BRIDGE_DIR%build"
cl /nologo /std:c++latest /O2 /EHa /MD /DNDEBUG /utf-8 /bigobj /permissive- ^
  /D_SCL_SECURE_NO_WARNINGS /D_CRT_SECURE_NO_WARNINGS ^
  /DROSETTA_COMMIT=\"%ROSETTA_COMMIT%\" /DBRIDGE_PATCHES=\"%PATCHES%\" ^
  /I "%BRIDGE_DIR%shim" /I Includes /I build\vcpkg_installed\x64-windows\include ^
  "%BRIDGE_DIR%bridge.cpp" build\lib\RosettaStone.lib ^
  /Fo"%BRIDGE_DIR%build\\" /Fe"%BRIDGE_DIR%build\rosetta-bridge.exe" /link /LTCG || exit /b 1
echo BRIDGE_OK

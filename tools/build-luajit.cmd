@echo off
rem Builds the LuaJIT 2.0 compiler Obsidian uses into target\luajit (needs git and the Visual Studio Build Tools).
setlocal
rem Some shells set this, which stops msvcbuild.bat from running the tools it builds in the build folder.
set NoDefaultCurrentDirectoryInExePath=
set ROOT=%~dp0..
set SRC=%ROOT%\target\luajit-src
set OUT=%ROOT%\target\luajit
rem A Rust cache can bring back the folder without a working .git, and git would then act on Obsidian's own repository.
if not exist "%SRC%\.git\HEAD" if exist "%SRC%" rmdir /s /q "%SRC%"
if not exist "%SRC%\.git\HEAD" git clone --quiet https://github.com/LuaJIT/LuaJIT "%SRC%" || exit /b 1
git -C "%SRC%" checkout --quiet 2410b57 || exit /b 1
for /f "usebackq delims=" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath`) do set VS=%%i
call "%VS%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
pushd "%SRC%\src"
call msvcbuild.bat static || (popd & exit /b 1)
popd
if not exist "%OUT%\lua\jit" mkdir "%OUT%\lua\jit"
copy /y "%SRC%\src\luajit.exe" "%OUT%\" >nul
copy /y "%SRC%\src\jit\*.lua" "%OUT%\lua\jit\" >nul
echo LuaJIT 2.0 is ready in %OUT%

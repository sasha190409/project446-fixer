@echo off
setlocal enabledelayedexpansion
cd /d "%~dp0"

set "PKG_NAME=csgo-legacy-fixer"
set "OUTNAME=.super duper mega fixer 3000 gui"
set "OUTDIR=%~dp0release"
set "STAGEDIR=%~dp0zip-root"
set "ARCHIVEDIR=%~dp0archive"
set "ZIPNAME=csgo-legacy-fixer.zip"
set "META_LOG=%TEMP%\csgo-fixer-cargo-meta.log"
set "LOCK_MARKER=%~dp0target\.cargo-lock.last"

rem --- pick profile ---
if "%FAST%"=="1" (
    set "PROFILE=release-fast"
    set "PROFILE_FLAG=--profile release-fast"
    set "TARGETDIR=%~dp0target\release-fast"
) else (
    set "PROFILE=release"
    set "PROFILE_FLAG=--release"
    set "TARGETDIR=%~dp0target\release"
)

rem --- lock enforcement is on by default; set LOCKED=0 to disable ---
if "%LOCKED%"=="0" (
    set "LOCK_FLAG="
) else (
    set "LOCK_FLAG=--locked"
)

rem --- QUIET=1 suppresses cargo progress lines ---
if "%QUIET%"=="1" (
    set "QUIET_FLAG=--quiet"
) else (
    set "QUIET_FLAG="
)

rem --- sccache: default cache dir next to the project ---
if not defined SCCACHE_DIR set "SCCACHE_DIR=%~dp0.sccache"
if not defined SCCACHE_CACHE_SIZE set "SCCACHE_CACHE_SIZE=20G"

where sccache >nul 2>nul
if not errorlevel 1 goto :sccache_ok
echo [WARN] sccache not found on PATH -- builds will be much slower.
echo [WARN] Install with: cargo install sccache
goto :sccache_done
:sccache_ok
set "RUSTC_WRAPPER=sccache"
echo [INFO] Using sccache, cache: !SCCACHE_DIR!, size: !SCCACHE_CACHE_SIZE!
:sccache_done

echo ============================================
echo  Building %PKG_NAME% ^(%PROFILE%^)
echo ============================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERROR] cargo was not found on PATH.
    pause
    exit /b 1
)

rem ------------------------------------------------------------------
rem  Pre-flight: is Cargo.lock in sync with Cargo.toml?
rem
rem  Fast path: if Cargo.lock is byte-identical to the marker written by
rem  the last successful run, skip the (slow) `cargo metadata` call.
rem  The marker was already being written before; it just was never read.
rem ------------------------------------------------------------------
if not defined LOCK_FLAG goto :skip_lock_check

if not exist "Cargo.lock" goto :lock_check_run
if not exist "%LOCK_MARKER%" goto :lock_check_run

fc /b "Cargo.lock" "%LOCK_MARKER%" >nul 2>nul
if not errorlevel 1 (
    echo [INFO] Cargo.lock unchanged since last build -- skipping metadata check.
    goto :skip_lock_check
)

:lock_check_run
cargo metadata --locked --format-version 1 > "%META_LOG%" 2>&1
if not errorlevel 1 goto :lock_ok

findstr /c:"--locked" /c:"lock file" "%META_LOG%" >nul
if errorlevel 1 goto :lock_unexpected

echo [INFO] Cargo.lock is out of date with Cargo.toml.
echo [INFO] Running cargo update to resync it...
echo.
cargo update
if errorlevel 1 (
    echo.
    echo [ERROR] cargo update failed. See output above.
    del "%META_LOG%" >nul 2>nul
    pause
    exit /b 1
)
echo.
echo [INFO] Cargo.lock updated. Proceeding with the build.
echo.
goto :lock_ok

:lock_unexpected
echo [ERROR] cargo metadata --locked failed unexpectedly:
echo.
type "%META_LOG%"
echo.
del "%META_LOG%" >nul 2>nul
pause
exit /b 1

:lock_ok
del "%META_LOG%" >nul 2>nul
if not exist "%~dp0target" mkdir "%~dp0target"
if exist "Cargo.lock" copy /y "Cargo.lock" "%LOCK_MARKER%" >nul

:skip_lock_check

rem --- actual build ---
cargo build %PROFILE_FLAG% %LOCK_FLAG% %QUIET_FLAG%
if errorlevel 1 (
    echo.
    echo [ERROR] cargo build failed.
    pause
    exit /b 1
)

if not defined RUSTC_WRAPPER goto :skip_stats
echo.
sccache --show-stats
:skip_stats

rem ==================================================================
rem  Packaging (mirrors the CI workflow)
rem ==================================================================

echo.
echo ============================================
echo  Packaging
echo ============================================
echo.

rem --- sanity check on STAGEDIR before we rm -rf it ---
if not defined STAGEDIR (
    echo [ERROR] STAGEDIR is not set. Aborting.
    pause
    exit /b 1
)

if exist "%STAGEDIR%" (
    rmdir /s /q "%STAGEDIR%"
    if exist "%STAGEDIR%" (
        echo [ERROR] Could not remove "%STAGEDIR%".
        pause
        exit /b 1
    )
)
mkdir "%STAGEDIR%"
if errorlevel 1 (
    echo [ERROR] Could not create "%STAGEDIR%".
    pause
    exit /b 1
)

if not exist "%OUTDIR%" mkdir "%OUTDIR%"
if errorlevel 1 (
    echo [ERROR] Could not create "%OUTDIR%".
    pause
    exit /b 1
)

rem --- main EXE ---
set "EXE=%TARGETDIR%\%PKG_NAME%.exe"
if not exist "%EXE%" (
    echo [ERROR] %EXE% not found.
    pause
    exit /b 1
)
copy /y "%EXE%" "%STAGEDIR%\%OUTNAME%.exe" >nul
echo [OK] Staged %OUTNAME%.exe

rem --- PDB ---
set "PDB=%TARGETDIR%\%PKG_NAME%.pdb"
if exist "%PDB%" (
    copy /y "%PDB%" "%STAGEDIR%\%OUTNAME%.pdb" >nul
    echo [OK] Staged %OUTNAME%.pdb
) else (
    echo [INFO] No PDB file produced.
)

rem --- archive/ contents ---
if exist "%ARCHIVEDIR%\" (
    echo [INFO] Copying contents of archive\ into stage...
    robocopy "%ARCHIVEDIR%" "%STAGEDIR%" /E /NFL /NDL /NJH /NJS /NP >nul
    if errorlevel 8 (
        echo [ERROR] robocopy failed with code !errorlevel!.
        pause
        exit /b 1
    )
) else (
    echo [INFO] No archive\ directory -- skipping.
)

rem ------------------------------------------------------------------
rem  sha256sums.txt (UTF-8, no BOM, forward slashes, two-space separator)
rem  Generated via a small PowerShell helper written to %TEMP%.
rem ------------------------------------------------------------------
set "PS1=%TEMP%\csgo-fixer-sums.ps1"

> "%PS1%" echo param([Parameter(Mandatory=$true)][string]$Stage)
>> "%PS1%" echo $ErrorActionPreference = 'Stop'
>> "%PS1%" echo $sums = Join-Path $Stage 'sha256sums.txt'
>> "%PS1%" echo $lines = New-Object System.Collections.Generic.List[string]
>> "%PS1%" echo $allFiles = Get-ChildItem -LiteralPath $Stage -Recurse -File -Force
>> "%PS1%" echo $files = $allFiles ^| Sort-Object FullName ^| Where-Object { $_.FullName -ne $sums }
>> "%PS1%" echo foreach ($f in $files) {
>> "%PS1%" echo $h = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLower()
>> "%PS1%" echo $r = $f.FullName.Substring($Stage.Length).TrimStart('\','/') -replace '\\','/'
>> "%PS1%" echo $lines.Add($h + '  ' + $r)
>> "%PS1%" echo }
>> "%PS1%" echo $utf8 = New-Object System.Text.UTF8Encoding($false)
>> "%PS1%" echo [System.IO.File]::WriteAllLines($sums, $lines, $utf8)

powershell -NoProfile -ExecutionPolicy Bypass -File "%PS1%" "%STAGEDIR%"
if errorlevel 1 (
    echo [ERROR] Failed to generate sha256sums.txt.
    del "%PS1%" >nul 2>nul
    pause
    exit /b 1
)
del "%PS1%" >nul 2>nul

echo.
echo === sha256sums.txt ===
type "%STAGEDIR%\sha256sums.txt"
echo.

rem --- pick 7z binary (PATH, then common install locations) ---
set "SEVENZIP="
where 7z >nul 2>nul
if not errorlevel 1 set "SEVENZIP=7z"
if not defined SEVENZIP (
    where 7za >nul 2>nul
    if not errorlevel 1 set "SEVENZIP=7za"
)
if not defined SEVENZIP if exist "%ProgramFiles%\7-Zip\7z.exe" set "SEVENZIP=%ProgramFiles%\7-Zip\7z.exe"
if not defined SEVENZIP if exist "%ProgramFiles(x86)%\7-Zip\7z.exe" set "SEVENZIP=%ProgramFiles(x86)%\7-Zip\7z.exe"
if not defined SEVENZIP (
    echo [ERROR] 7z was not found.
    echo [ERROR] Install 7-Zip: https://www.7-zip.org/
    pause
    exit /b 1
)
echo [INFO] Using 7z: %SEVENZIP%

rem --- create zip ---
set "ZIP=%OUTDIR%\%ZIPNAME%"
if exist "%ZIP%" del "%ZIP%" >nul 2>nul

pushd "%STAGEDIR%"
"%SEVENZIP%" a -tzip -mx=9 -y "%ZIP%" "*" >nul
set "SEVENZIP_RC=!errorlevel!"
popd

if not "!SEVENZIP_RC!"=="0" (
    echo [ERROR] 7z failed with code !SEVENZIP_RC!.
    pause
    exit /b 1
)

echo [OK] Created %ZIP%

echo.
echo === ZIP CONTENTS ===
"%SEVENZIP%" l -ba "%ZIP%"

echo.
echo ============================================
echo  Done. Output: %OUTDIR%
echo ============================================
endlocal
exit /b 0

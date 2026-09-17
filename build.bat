@echo off
setlocal enabledelayedexpansion
cd /d "%~dp0"

set "PKG_NAME=csgo-legacy-fixer"
set "OUTNAME=.super duper mega fixer 3000 gui"
set "OUTDIR=%~dp0release"
set "TARGETDIR=%~dp0target\release"
set "META_LOG=%TEMP%\csgo-fixer-cargo-meta.log"
set "LOCK_MARKER=%~dp0target\.cargo-lock.last"

rem --- pick profile ---
if "%FAST%"=="1" (
    set "PROFILE=release-fast"
    set "PROFILE_FLAG=--profile release-fast"
) else (
    set "PROFILE=release"
    set "PROFILE_FLAG=--release"
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
rem  Skipped when Cargo.lock is byte-identical to the last run.
rem ------------------------------------------------------------------
if not defined LOCK_FLAG goto :skip_lock_check
set "SKIP_CHECK=0"

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

rem --- Prepare output folder ---
if not exist "%OUTDIR%" mkdir "%OUTDIR%"
if errorlevel 1 (
    echo [ERROR] Could not create "%OUTDIR%".
    pause
    exit /b 1
)

set "EXE=%TARGETDIR%\%PKG_NAME%.exe"
set "OUTEXE=%OUTDIR%\%OUTNAME%.exe"
if exist "%EXE%" (
    copy /y "%EXE%" "%OUTEXE%" >nul
    echo [OK] Copied %OUTNAME%.exe
) else (
    echo [WARN] %PKG_NAME%.exe not found in "%TARGETDIR%".
)

set "PDB=%TARGETDIR%\%PKG_NAME%.pdb"
set "OUTPDB=%OUTDIR%\%OUTNAME%.pdb"
if not exist "%PDB%" goto :no_pdb
copy /y "%PDB%" "%OUTPDB%" >nul
echo [OK] Copied %OUTNAME%.pdb
goto :pdb_done
:no_pdb
echo [INFO] No PDB file produced. This is normal for some profiles.
:pdb_done

echo.
echo ============================================
echo  Done. Output: %OUTDIR%
echo ============================================
endlocal
exit /b 0
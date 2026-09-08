@echo off
chcp 65001 >nul
title Rubik - CFOP Solver
cd /d "%~dp0"

where cargo >nul 2>&1
if errorlevel 1 (
    echo.
    echo [LOI] Khong tim thay Cargo/Rust tren may nay.
    echo Hay cai Rust tai: https://rustup.rs
    echo Tren Windows con can Visual C++ Build Tools:
    echo   https://visualstudio.microsoft.com/visual-cpp-build-tools/
    echo   ^(chon workload "Desktop development with C++"^)
    echo.
    pause
    exit /b 1
)

echo ================================================
echo   RUBIK - CFOP Solver
echo ================================================
echo.
echo Lan dau chay se bien dich (khoang 1-3 phut).
echo Cac lan sau se nhanh hon nhieu.
echo.

cargo run --release -p rubik-app

if errorlevel 1 (
    echo.
    echo [LOI] Chay that bai. Xem thong bao loi o tren.
    echo Neu loi lien quan den "link.exe not found", hay cai
    echo Visual C++ Build Tools roi khoi dong lai may.
    echo.
    pause
)

@echo off
chcp 65001 >nul
title Rubik - CFOP Solver
cd /d "%~dp0"

where cargo >nul 2>&1
if errorlevel 1 goto NOCARGO

:MENU
cls
echo ==================================================
echo    RUBIK - CFOP Solver
echo ==================================================
echo.
echo   [1] Ban THUONG  (khuyen nghi)
echo       Quet 6 anh, moi anh 1 mat. Dung che do
echo       "Hinh hoc (mac dinh)" trong man hinh quet.
echo       Build nhanh, khong dung den ONNX Runtime.
echo.
echo   [2] Bat them lua chon "ML (thu nghiem)"
echo       CHI them 1 lua chon do vi tri trong man hinh
echo       quet. Bo do do tra ve HOP VUONG, khong bieu
echo       dien duoc mat cube nhin nghieng.
echo       DA DO: kem hon "Hinh hoc" mac dinh.
echo       LAN DAU build lau: crate ort phai tai ONNX
echo       Runtime ve va bien dich them.
echo.
set "CHON="
set /p "CHON=Chon 1 hoac 2 roi Enter (de trong = 1): "
if "%CHON%"=="2" goto ML
goto THUONG

:THUONG
echo.
echo --- Dang chay BAN THUONG ---
echo.
cargo run --release -p rubik-app
if errorlevel 1 goto FAIL
goto END

:ML
echo.
echo --- Dang chay ban CO lua chon ML ---
echo.
echo Neu bao loi khong tai duoc ONNX Runtime tu cdn.pyke.io
echo (mang bi chan), tai ban ONNX Runtime 1.28.x cho Windows
echo roi dat 2 bien moi truong truoc khi chay lai:
echo     set ORT_PREFER_DYNAMIC_LINK=1
echo     set ORT_LIB_LOCATION=D:\duong\dan\chua\onnxruntime.dll
echo PHAI dung ban 1.28.x - ban cu hon se bien dich duoc nhung
echo chay se bao "requested API version 27".
echo.
cargo run --release -p rubik-app --features ml_detect
if errorlevel 1 goto MLFAIL
goto END

:MLFAIL
echo.
echo [LOI] Ban co ML khong chay duoc. Xem thong bao loi o tren.
echo Ban THUONG khong dung den ort nen thuong van chay binh thuong.
echo.
set "LUI="
set /p "LUI=Chay ban THUONG de khong bi ket? (c/k): "
if /i "%LUI%"=="c" goto THUONG
goto FAIL

:NOCARGO
echo.
echo [LOI] Khong tim thay Cargo/Rust tren may nay.
echo Hay cai Rust tai: https://rustup.rs
echo Tren Windows con can Visual C++ Build Tools:
echo   https://visualstudio.microsoft.com/visual-cpp-build-tools/
echo   (chon workload "Desktop development with C++")
echo.
pause
exit /b 1

:FAIL
echo.
echo Neu loi "link.exe not found": cai Visual C++ Build Tools.
echo Neu loi doi Rust moi hon: chay rustup update roi thu lai.
echo.
pause
exit /b 1

:END

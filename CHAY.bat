@echo off
chcp 65001 >nul
title Rubik - CFOP Solver
cd /d "%~dp0"

where cargo >nul 2>&1
if errorlevel 1 goto NOCARGO

:: --1 tren dong lenh = chon thang ban THUONG, bo qua menu hoi.
:: (VD: CHAY.bat --1). Khong co co nay thi mac dinh la ban ML.
for %%A in (%*) do if "%%A"=="--1" set "FORCE1=1"
if defined FORCE1 goto THUONG

:: ================================================================
:: Menu hoi [1]/[2] dang TAT -- chay thang ban 2 (ML) ben duoi.
:: Can hoi lai nhu truoc thi: xoa/comment dong "goto ML" ngay ben
:: duoi day, roi bo dau ":: " o dau moi dong trong khoi MENU phia
:: duoi (tu ":MENU" toi het "goto ML" cua khoi do).
:: ================================================================
goto ML

:: :MENU
:: cls
:: echo ==================================================
:: echo    RUBIK - CFOP Solver
:: echo ==================================================
:: echo.
:: echo   [1] Ban THUONG
:: echo       Quet 6 anh, moi anh 1 mat. Dung che do
:: echo       "Hinh hoc (mac dinh)" trong man hinh quet.
:: echo       Build nhanh, khong dung den ONNX Runtime.
:: echo.
:: echo   [2] Bat them lua chon "ML (thu nghiem)"  (mac dinh)
:: echo       CHI them 1 lua chon do vi tri trong man hinh
:: echo       quet. Bo do do tra ve HOP VUONG, khong bieu
:: echo       dien duoc mat cube nhin nghieng.
:: echo       DA DO: kem hon "Hinh hoc" mac dinh.
:: echo       LAN DAU build lau: crate ort phai tai ONNX
:: echo       Runtime ve va bien dich them.
:: echo.
:: echo   (Meo: chay "CHAY.bat --1" de vao thang ban THUONG,
:: echo    khong can qua man hinh hoi nay.)
:: echo.
:: set "CHON="
:: set /p "CHON=Chon 1 hoac 2 roi Enter (de trong = 2): "
:: if "%CHON%"=="1" goto THUONG
:: goto ML

:THUONG
echo.
echo --- Dang bien dich BAN THUONG (neu da build roi thi rat nhanh) ---
echo.
cargo build --release -p rubik-app
if errorlevel 1 goto FAIL
:: Mo ung dung nhu 1 tien trinh doc lap roi DONG LUON cua so cmd nay --
:: rubik-app.exe (release) khong tu mo console rieng (xem
:: #![windows_subsystem = "windows"] trong main.rs), nen sau dong nay
:: chi con lai cua so 3D cua ung dung, khong con cua so den nao ca.
start "" "%~dp0target\release\rubik-app.exe"
exit /b 0

:ML
echo.
echo --- Dang bien dich ban CO lua chon ML (neu da build roi thi rat nhanh) ---
echo.
echo Neu bao loi khong tai duoc ONNX Runtime tu cdn.pyke.io
echo (mang bi chan), tai ban ONNX Runtime 1.28.x cho Windows
echo roi dat 2 bien moi truong truoc khi chay lai:
echo     set ORT_PREFER_DYNAMIC_LINK=1
echo     set ORT_LIB_LOCATION=D:\duong\dan\chua\onnxruntime.dll
echo PHAI dung ban 1.28.x - ban cu hon se bien dich duoc nhung
echo chay se bao "requested API version 27".
echo.
cargo build --release -p rubik-app --features ml_detect
if errorlevel 1 goto MLFAIL
start "" "%~dp0target\release\rubik-app.exe"
exit /b 0

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

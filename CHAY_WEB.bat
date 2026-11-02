@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo ============================================
echo   Rubik Web (ban demo local)
echo   Lan dau build mat 1-3 phut, cac lan sau nhanh.
echo   Trinh duyet se tu mo tai http://127.0.0.1:8080
echo   Tat server: dong cua so nay hoac bam Ctrl+C
echo ============================================
where cargo >nul 2>nul
if errorlevel 1 (
  echo.
  echo Chua cai Rust. Cai tai https://rustup.rs roi chay lai file nay.
  pause
  exit /b 1
)
start "" cmd /c "timeout /t 12 >nul & start http://127.0.0.1:8080"
cargo run --release -p rubik-web
pause

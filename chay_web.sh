#!/usr/bin/env sh
# Chạy bản demo local: ./chay_web.sh  rồi mở http://127.0.0.1:8080
cd "$(dirname "$0")" || exit 1
echo "Lần đầu build mất 1-3 phút. Mở http://127.0.0.1:8080 khi thấy dòng 'Rubik web chạy tại'."
exec cargo run --release -p rubik-web

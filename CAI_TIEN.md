# Các cải tiến trong lần cập nhật này

Tóm tắt những điểm yếu đã tìm thấy và cách sửa. Toàn bộ test cũ vẫn đạt; có thêm test mới cho từng lỗi.

## 1. Lỗ hổng làm treo / sập máy chủ và trang web (nghiêm trọng nhất)
**Vấn đề.** Cú pháp lặp `(...)n` trong ô công thức không có giới hạn:
- `(R)99999999999999999999` → tràn số khi đọc số lặp → **panic**. Ở bản GitHub Pages, panic trong WebAssembly làm
  worker chết và mọi nút bấm sau đó **treo vĩnh viễn** (chỉ còn cách tải lại trang).
- `(R U R' U')999999999` → bung ra chuỗi hàng GB → **máy chủ hết RAM và bị hệ điều hành giết**. Route `/api/apply`
  không bị giới hạn tốc độ, và `/api/game/finish` đọc lời giải *trước* khi kiểm tra độ dài, nên chỉ một yêu cầu là đủ.

**Sửa** (`rubik-core/src/cube.rs`): giới hạn 5000 nước sau khi bung, kiểm tra độ dài *trước* khi cấp phát, số lặp quá
lớn trả lỗi thay vì panic. Thông báo lỗi ký hiệu sai đổi từ tiếng Anh (`Unknown: 'Q'`) sang tiếng Việt.
Test: `rubik-core/tests/parse_limits.rs`, `rubik-api/tests/api.rs` (`apply_chan_cong_thuc_doc_hai`).

## 2. Bản WebAssembly: lỗi bộ giải không còn làm treo trang
`worker.js` bắt lỗi khi tải và khi tính; trang báo lỗi ngay cho lời gọi đang chờ và **tự khởi động lại worker**
(tối đa 3 lần/30 giây để tránh lặp vô hạn). Trước đây lỗi chỉ hiện một dòng chữ, còn các lời gọi đang chờ treo mãi.

## 3. Máy chủ axum (`rubik-web`)
| Trước | Sau |
|---|---|
| Thư mục giao diện gắn cứng đường dẫn lúc build → chép file chạy sang máy khác/Docker là mất giao diện | Tự tìm `static/` cạnh file chạy, hoặc đặt `RUBIK_STATIC`; in đường dẫn đang dùng khi khởi động |
| Phải tự đặt `RUBIK_ADDR=0.0.0.0:10000` trên Render | Tự đọc biến `PORT` (Render/Heroku) |
| Không có health check | `GET /api/health` |
| Không có header bảo mật | CSP, `X-Frame-Options: DENY` (chống clickjacking), `nosniff`, `Referrer-Policy`, `Permissions-Policy` |
| Trang HTML ~70 KB không nén; trình duyệt có thể giữ bản cũ sau khi cập nhật | Nén gzip (còn ~24 KB); `Cache-Control: no-cache` cho giao diện, `no-store` cho API |
| Route nhẹ và khung hình camera không giới hạn theo IP | Giới hạn rộng tay (1200 và 1500 lần/phút/IP); khung hình > 1920×1080 bị từ chối |
| Tắt server (SIGTERM/Ctrl+C) cắt ngang yêu cầu đang chạy | Tắt êm: hoàn tất yêu cầu rồi mới thoát |

## 4. Giao diện web
- Gọi server báo lỗi tiếng Việt rõ ràng khi mất mạng, hoặc khi server/proxy trả trang lỗi không phải JSON
  (trước đây hiện lỗi khó hiểu kiểu `Unexpected token '<'`).
- Tên thế, nhãn gợi ý và công thức trong thư viện đều được escape trước khi chèn vào HTML.

## 5. Khác
- API `scramble` không truyền `seed` giờ xáo ngẫu nhiên thật (trước đây mặc định seed = 1 nên luôn ra cùng một đề).
- App desktop: sửa 20 cảnh báo `float_literal_f32_fallback` — Rust ghi rõ sẽ thành **lỗi biên dịch** ở bản tương lai.
- CI: kiểm thử chạy cả khi mở pull request (không đăng trang); tài liệu deploy cập nhật theo các thay đổi trên
  (bỏ ghi chú cũ "server chưa giới hạn số yêu cầu" và "không chép riêng file exe").

---

# Đợt cải tiến thứ hai

Giải quyết 10 điểm yếu còn lại của đợt trước.

| # | Vấn đề | Đã làm |
|---|---|---|
| 1 | Mỗi nước xoay phải chờ server (giải 60 nước = 60 lần gọi `apply`) | Trình duyệt tự tính nước bằng bảng hoán vị 54 ô `web/move_table.json`, **sinh từ engine Rust** (`rubik-core/examples/gen_move_table.rs`) và đã đối chiếu khớp 72/72 nước với bảng trong `schema.sql`. Test Rust báo lỗi nếu file lệch engine; test JS đối chiếu vectơ chuẩn do Rust tính. Server chỉ còn được hỏi để giải/gợi ý, kiểm tra công thức nhập tay và nộp ván |
| 2 | Không có test cho máy chủ HTTP | Router tách ra `rubik-web/src/lib.rs` (`router`, `Config`, `App::new`); `tests/http.rs` có 19 test: giao diện tĩnh, header bảo mật, gzip, mọi route, giới hạn tốc độ từng nhóm, `X-Forwarded-For`, cookie, luồng ván xếp hạng, body quá lớn. Đã thử phá code (bỏ header, đổi cách lấy IP) để chắc test bắt được |
| 3 | Ván bỏ dở và dữ liệu cũ không được dọn | `Accounts::cleanup` chạy lúc khởi động và mỗi giờ: ván "đang chơi" quá 1 giờ → bỏ dở; ván bỏ dở quá 7 ngày → xoá; phiên hết hạn → xoá. Trang gửi `sendBeacon` huỷ ván ngay khi đóng tab. Bản Supabase có hàm `cleanup_old_games()` |
| 4 | Sao lưu SQLite phải tắt server | `rubik-web backup <file>` và `RUBIK_BACKUP_DIR` (tự sao lưu, xoay vòng) dùng `VACUUM INTO`, không cần tắt server, không ghi đè file có sẵn |
| 5 | Chưa chạy được ngoại tuyến | PWA: `manifest.webmanifest`, biểu tượng, `sw.js` (bản GitHub Pages). Đã thử trên Chromium thật: cài được, tắt mạng rồi tải lại vẫn mở |
| 6 | Font tải từ Google | Be Vietnam Pro nhúng sẵn (`web/fonts`, 6 file woff2 ≈ 100 KB, giấy phép OFL); bỏ Google khỏi trang và khỏi CSP; 0 yêu cầu ra ngoài |
| 7 | Thiếu hỗ trợ bàn phím | Menu "Thêm" theo mẫu WAI-ARIA (↑↓ Home End Esc Tab); cửa sổ nổi nhận con trỏ khi mở chủ động, giữ Tab trong cửa sổ, trả con trỏ khi đóng. **Sửa thêm một lỗi phát hiện được:** phím tắt Enter/Space trước đây nuốt phím Enter/Space khi đang chọn nút bằng bàn phím |
| 8 | File quá lớn | `rubik-app/src/main.rs` 1941 → 171 dòng, `scan_ui.rs` 1135 dòng → thư mục `scan_ui/` (không file nào > 364 dòng); `cargo check` sạch, 31 test đạt. Mẫu web tách thành `web/src/{index.html,style.css,app.js}` và định dạng bằng prettier |
| 9 | Thiếu `render.yaml` | Có `render.yaml` (Blueprint, build bằng Rust, không dùng Docker; xem `HUONG_DAN_DEPLOY.md`, Cách B) |
| 10 | Dọn dẹp | Xoá hai thư mục `_sao_luu_*` và các file `.pt`/`.png` trong `train_detect/` (còn lại `face_detector.onnx` và mã huấn luyện); gộp `DEPLOY.md` vào `HUONG_DAN_DEPLOY.md` |

## Bổ sung sau đợt hai
- Service worker: mã chương trình (`index.html`, `config.js`, `worker.js`, `pkg/*.js`, `pkg/*.wasm`) giờ thử mạng trước, chỉ ngoại tuyến mới dùng bản đã lưu, để không ghép nhầm `.js` mới với `.wasm` cũ sau khi cập nhật. Font/biểu tượng vẫn trả bản đã lưu ngay. Đã thử lại ngoại tuyến trên Chromium.
- Thêm `LICENSE` (MIT) — **hãy thay dòng tên tác giả**, hoặc đổi sang giấy phép bạn muốn.
- Đã chạy `cargo fmt` toàn dự án và sửa các gợi ý clippy có thể sửa an toàn; CI giờ **chặn** nếu rustfmt hoặc clippy báo lỗi. Bốn lint thuần phong cách (`needless_range_loop`, `manual_memcpy`, `len_without_is_empty`, `doc_lazy_continuation`) được tắt có chủ đích để khỏi viết lại các vòng lặp chỉ số trong bộ giải.
- **Đổi mật khẩu và xoá tài khoản** (cả bản server lẫn Supabase): cần nhập lại mật khẩu hiện tại; đổi mật khẩu đăng xuất các thiết bị khác; xoá tài khoản xoá luôn phiên, ván và thành tích. Dùng chung cơ chế khoá tạm với đăng nhập. Bản Supabase cần chạy lại `schema.sql` (thêm hàm `delete_my_account`).
- **Trợ năng khối 3D:** canvas có `role="img"` kèm mô tả bằng lời 54 ô, vùng `aria-live` thông báo sau mỗi lượt xoay ("Đã xoay: R U. Khối đã giải xong."), sơ đồ 6 mặt trùng lặp được ẩn khỏi trình đọc màn hình.

## Chưa kiểm chứng được trong môi trường làm việc
- Bản WebAssembly (`wasm-pack`) chưa chạy được ở đây; service worker đã thử với bản `www/` thiếu `pkg/`. Hãy xem kết quả CI sau lần push đầu.
- Chưa có cách chọn mặt bằng bàn phím ngoài các phím tắt (sơ đồ 6 mặt chỉ bấm bằng chuột).
- Bản `rubik-app` với feature `ml_detect` không build được (không tải được `ort-sys`); phần đó không bị đụng tới.

## Kích thước
Trang HTML tăng từ ~76 KB lên ~98 KB vì mã giờ được định dạng dễ đọc thay vì dồn dòng (gzip còn khoảng 30 KB).

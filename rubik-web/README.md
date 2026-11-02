# rubik-web

Bản web của ứng dụng Rubik: backend axum (dùng lại `rubik-core`, `rubik-vision`) + giao diện tĩnh.

Chạy từ thư mục gốc của workspace:

    cargo run --release -p rubik-web

Cách nhanh nhất: Windows bấm đúp CHAY_WEB.bat; Linux/macOS chạy ./chay_web.sh. Hoặc tự chạy lệnh trên rồi mở http://127.0.0.1:8080 (đổi địa chỉ bằng biến môi trường RUBIK_ADDR).
Camera chỉ hoạt động trên localhost hoặc HTTPS.

## Tài khoản, thành tích và bảng xếp hạng

Bản server có thêm đăng ký/đăng nhập, lưu thành tích và bảng xếp hạng (bản WebAssembly trên GitHub Pages dùng Supabase thay thế, xem `supabase/HUONG_DAN.md`).

- **Database:** SQLite, một file `rubik.db` tạo tự động ở thư mục chạy lệnh (đổi bằng biến môi trường `RUBIK_DB`). SQLite được biên dịch kèm nên không cần cài thêm gì. Server tự dọn dữ liệu cũ mỗi giờ: ván "đang chơi" quá 1 giờ (đóng tab giữa chừng) thành "bỏ dở", ván bỏ dở quá 7 ngày bị xoá, phiên đăng nhập hết hạn bị xoá; ván đã giải xong và tài khoản không bị đụng tới. Trang cũng báo server huỷ ván ngay khi đóng tab.
- **Sao lưu khi server đang chạy (không cần tắt):** `rubik-web backup sao-luu.db` (dùng `VACUUM INTO` của SQLite, nhất quán và đã nén gọn; từ chối ghi đè file có sẵn). Muốn tự động: đặt `RUBIK_BACKUP_DIR=/duong/dan` (mặc định mỗi 24 giờ, giữ 7 bản; chỉnh bằng `RUBIK_BACKUP_HOURS`, `RUBIK_BACKUP_KEEP`). Khôi phục: tắt server, chép bản sao lưu đè lên `rubik.db`, bật lại.
- **Bảo mật:** mật khẩu băm bằng Argon2; phiên đăng nhập là cookie `HttpOnly` sống 30 ngày, server chỉ lưu bản băm SHA-256 của mã phiên; sai mật khẩu 5 lần thì khoá tên đó 60 giây. Khi chạy sau HTTPS, đặt `RUBIK_SECURE_COOKIE=1` để cookie có cờ `Secure`.
- **Ván xếp hạng:** bấm 🏆 Xếp hạng → "Chơi ván xếp hạng". Server phát đề xáo 25 nước và bấm giờ; khi khối được giải, trình duyệt gửi danh sách nước đã đi, server áp lại lên đề xáo và chỉ ghi nhận nếu khối giải xong thật. Thời gian do server đo. Dùng gợi ý, giải tự động, bảng công thức, xáo lại, quét camera hoặc điền màu sẽ huỷ ván. Ván quá 1 giờ không được tính.
- **Bảng xếp hạng:** xếp theo nhanh nhất, ít nước nhất hoặc giải nhiều nhất; tab "Của tôi" cho số ván, lượt giải, kỷ lục, trung bình và 10 ván gần nhất.

| Route | Việc |
|---|---|
| `POST /api/auth/register`, `POST /api/auth/login` | `{username, password}` → đặt cookie phiên |
| `POST /api/auth/logout`, `GET /api/auth/me` | Đăng xuất / người dùng hiện tại (`user: null` nếu chưa đăng nhập) |
| `POST /api/auth/change_password` | `{old_password, new_password}` → đổi mật khẩu, đăng xuất các phiên khác |
| `POST /api/auth/delete` | `{password}` → xoá vĩnh viễn tài khoản, phiên và ván |
| `POST /api/game/start` | Bắt đầu ván xếp hạng → `{game_id, scramble, state}` |
| `POST /api/game/finish` | `{game_id, moves}` → `{time_ms, move_count, personal_best}` |
| `POST /api/game/abandon` | `{game_id}` → huỷ ván |
| `GET /api/leaderboard?by=time\|moves\|solves` | Bảng xếp hạng (tối đa 50 dòng) |
| `GET /api/profile` | Thành tích của người đang đăng nhập |

Kiểm thử: `cargo test --release -p rubik-web`.
- `src/accounts.rs` (9 test): đăng ký/đăng nhập, khoá tạm, kiểm tra lời giải, bảng xếp hạng, dọn dẹp, sao lưu.
- `tests/http.rs` (19 test): dựng cả router trong bộ nhớ (hàm `rubik_web::router`) và kiểm tra giao diện tĩnh, header bảo mật,
  nén gzip, mọi route, giới hạn tốc độ từng nhóm, `X-Forwarded-For` có/không tin proxy, thuộc tính cookie, luồng ván xếp hạng, body quá lớn.
- `tests/ui.test.js`: giao diện trong jsdom (phím tắt, cửa sổ nổi, bàn phím cho menu, nước đi tính tại chỗ).

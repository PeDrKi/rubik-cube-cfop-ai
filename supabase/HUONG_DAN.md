# Bật tài khoản & bảng xếp hạng cho bản GitHub Pages (Supabase)

Bản WebAssembly chạy hoàn toàn trong trình duyệt nên cần một database bên ngoài để lưu tài khoản và thành tích. Supabase có gói miễn phí đủ dùng. Làm một lần, khoảng 10 phút.

## 1. Tạo project Supabase
1. Vào https://supabase.com, đăng nhập (có thể bằng GitHub), bấm **New project**.
2. Đặt tên, đặt mật khẩu database (lưu lại), chọn vùng gần nhất (ví dụ Singapore), bấm **Create**. Đợi 1–2 phút.

## 2. Tạo bảng và hàm
1. Trong project, mở **SQL Editor** → **New query**.
2. Mở file `supabase/schema.sql`, chép toàn bộ, dán vào, bấm **Run**. Thấy "Success" là xong. Chạy lại nhiều lần cũng không sao.

## 3. Tắt xác nhận email
Tài khoản dùng **tên tài khoản** thay vì email: trang web đổi tên thành một email nội bộ dạng `ten@rubik.local`, không email nào được gửi đi. Vì vậy phải tắt bước xác nhận:
- **Authentication → Sign In / Providers → Email** → tắt **Confirm email** → **Save**.

## 4. Điền cấu hình vào trang web
1. Mở **Project Settings → API** (hoặc nút **Connect** ở trên cùng). Chép **Project URL** và khoá **anon / publishable**.
2. Mở `rubik-wasm/www/config.js`, điền:
   ```js
   url: 'https://xxxx.supabase.co',
   anonKey: 'eyJ...',
   ```
3. Commit và push lên `main`. GitHub Actions tự build và đăng lại trang.

Khoá anon được phép công khai trong mã nguồn: nó chỉ cho gọi các hàm đã cấp quyền trong `schema.sql`. **Tuyệt đối không** dán khoá `service_role` / `secret` vào `config.js`.

## Cách chống gian lận
- Trình duyệt không gửi thời gian. Hàm `start_game` tự sinh đề xáo và ghi giờ bắt đầu bằng đồng hồ của database; `finish_game` tự tính thời gian.
- `finish_game` áp lại đề xáo + các nước người chơi gửi lên bằng bảng hoán vị `cube_moves` (sinh từ chính engine Rust của dự án, đã đối chiếu khớp 100%), chỉ ghi nhận khi khối giải xong thật.
- Bảng `games` bật RLS và không có policy nào, nên client không đọc/sửa trực tiếp được; mọi thứ đi qua hàm.
- Ván quá 1 giờ không được tính. Không nộp hộ được ván của người khác, không nộp lại được ván đã xong.
- Giới hạn còn lại: người chơi có thể nhờ một chương trình giải khác rồi gõ lời giải vào ô công thức. Thời gian vẫn tính từ lúc nhận đề nên khó đạt kỷ lục bằng cách này, nhưng không thể ngăn hoàn toàn.

## Đổi đuôi email nội bộ
Nếu lúc đăng ký Supabase báo email không hợp lệ, đổi `emailDomain` trong `config.js` (ví dụ `'nguoichoi.rubik.app'`). Chỉ đổi trước khi có người đăng ký, vì đổi sau thì tài khoản cũ không đăng nhập được.

## Dọn dữ liệu cũ
Ván bị bỏ dở không tự biến mất. Hàm `cleanup_old_games()` (có trong `schema.sql`, trình duyệt không gọi được) chuyển ván
"đang chơi" quá 1 giờ thành "bỏ dở" và xoá ván bỏ dở quá 7 ngày; ván đã giải xong được giữ mãi. Chạy tay trong SQL Editor:
`select public.cleanup_old_games();` hoặc đặt lịch hằng ngày bằng pg_cron (Database → Extensions → bật `pg_cron`):
`select cron.schedule('rubik-cleanup', '17 3 * * *', 'select public.cleanup_old_games()');`

## Xem dữ liệu
**Table Editor** → bảng `profiles` (người chơi) và `games` (các ván). Xoá người chơi: **Authentication → Users** → xoá, ván của họ tự xoá theo.

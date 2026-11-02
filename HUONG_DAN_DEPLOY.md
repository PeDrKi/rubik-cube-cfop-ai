# Hướng dẫn deploy Rubik CFOP

Tài liệu này hướng dẫn đưa dự án lên internet, từ đầu đến cuối, cho người chưa từng làm.

| Cách | Chi phí | Tài khoản & bảng xếp hạng | Nên dùng khi |
|---|---|---|---|
| **A. GitHub Pages + Supabase** (khuyên dùng) | Miễn phí | Có (Supabase) | Muốn có link công khai cho mọi người chơi |
| B. Server axum trên Render / VPS | Từ khoảng 7 USD/tháng nếu cần giữ dữ liệu | Có (SQLite) | Muốn tự giữ database trên máy chủ riêng |
| C. Chạy trên máy mình | Miễn phí | Có (SQLite) | Thử nghiệm, dùng trong mạng nhà |

Phần lớn mọi người chỉ cần **cách A**. Làm lần đầu mất khoảng 30 phút.

---

## Cách A. GitHub Pages + Supabase

Trang web chạy hoàn toàn trong trình duyệt (bộ giải Rust biên dịch sang WebAssembly), GitHub lo phần đăng trang, Supabase lo tài khoản và bảng xếp hạng.

### Bước 0. Chuẩn bị
- Tài khoản GitHub: https://github.com/signup
- Cài **Git for Windows**: https://git-scm.com/download/win (bấm Next đến hết là được).
- Không cần cài Rust trên máy: GitHub tự build.

### Bước 1. Tạo database Supabase
1. Vào https://supabase.com → **Start your project** → đăng nhập bằng GitHub.
2. **New project**: đặt tên (ví dụ `rubik`), đặt mật khẩu database (lưu lại), vùng chọn **Southeast Asia (Singapore)** → **Create new project**. Đợi 1–2 phút.
3. Menu trái → **SQL Editor** → **New query**. Mở file `supabase/schema.sql` trong thư mục dự án bằng Notepad, chép **toàn bộ**, dán vào, bấm **Run**. Thấy *Success* là xong.
4. Menu trái → **Authentication** → **Sign In / Providers** → **Email** → **tắt "Confirm email"** → **Save**.
   Giữ nguyên giới hạn tốc độ đăng ký/đăng nhập mặc định của Supabase (Authentication → Rate Limits) để hạn chế tài khoản rác. Đừng bật CAPTCHA nếu chưa sửa trang web gửi kèm `captchaToken`, vì đăng ký sẽ bị lỗi.
   (Người chơi đăng ký bằng tên tài khoản, trang đổi tên thành email nội bộ `ten@rubik.local`, không gửi email nào, nên phải tắt bước xác nhận.)
5. Lấy 2 giá trị: bấm nút **Connect** ở trên cùng (hoặc **Project Settings → API**), chép **Project URL** và khoá **anon public** (có thể tên là *publishable key*).
   ⚠️ **Không** dùng khoá `service_role` / `secret`.

### Bước 2. Điền cấu hình
Mở `rubik-wasm\www\config.js` bằng Notepad, điền 2 giá trị vừa chép:

```js
window.RUBIK_SUPABASE = {
  url: 'https://abcdxyz.supabase.co',
  anonKey: 'eyJhbGciOi...',
  emailDomain: 'rubik.local',
};
```

Lưu file. Khoá anon được phép công khai: quyền thật do các hàm trong `schema.sql` kiểm soát.

### Bước 3. Tạo repo GitHub
1. Vào https://github.com/new
2. **Repository name**: ví dụ `rubik`. Chọn **Public** (GitHub Pages miễn phí chỉ cho repo công khai).
3. **Không** tick "Add a README", "Add .gitignore", "license" (dự án đã có sẵn). Bấm **Create repository**.

### Bước 4. Đưa mã nguồn lên
Mở thư mục `C:\Users\admin\Downloads\rubik_web`, bấm vào thanh địa chỉ, gõ `cmd`, Enter. Trong cửa sổ đen, chạy lần lượt (thay `TEN_BAN` và `rubik` bằng tên tài khoản GitHub và tên repo của bạn):

```bat
git init
git config user.name "Ten cua ban"
git config user.email "email@cua.ban"
git add .
git commit -m "Rubik CFOP: ban dau"
git branch -M main
git remote add origin https://github.com/TEN_BAN/rubik.git
git push -u origin main
```

Lần đầu `git push` sẽ mở cửa sổ đăng nhập GitHub, cứ đăng nhập.

> File `.gitignore` đã loại sẵn `target/`, `pkg/`, `node_modules/`, database `rubik.db` và các thư mục sao lưu `_sao_luu_*`. Kiểm tra trước khi commit bằng `git status`: không được có thư mục `target`.

### Bước 5. Bật GitHub Pages
1. Trên trang repo → **Settings** → **Pages**.
2. **Build and deployment → Source**: chọn **GitHub Actions**.
3. Vào tab **Actions**: workflow **Deploy to GitHub Pages** chạy (nếu chưa chạy, bấm vào nó → **Run workflow**). Lần đầu mất khoảng 3–5 phút.
4. Xong sẽ có dấu ✓ xanh và link dạng `https://TEN_BAN.github.io/rubik/`.

### Bước 6. Kiểm tra
Mở link trên, lần lượt:
- [ ] Thanh trạng thái báo *Sẵn sàng*, khối Rubik hiện ra, bấm **Xáo** và **Giải tự động** chạy được.
- [ ] Có nút 🏆 **Xếp hạng** và 👤 **Đăng nhập** (nếu không có: `config.js` chưa điền đúng).
- [ ] Đăng ký một tài khoản → chơi **ván xếp hạng** → giải xong thấy thông báo *Giải xong* và tên mình trên bảng.
- [ ] Trong Supabase → **Table Editor** → bảng `profiles` và `games` có dữ liệu.
- [ ] Mở trên điện thoại: giao diện co lại, camera hỏi quyền (GitHub Pages là HTTPS nên camera dùng được).

### Cập nhật về sau
Sửa file trên máy, rồi trong `cmd` tại thư mục dự án:

```bat
git add .
git commit -m "Mo ta thay doi"
git push
```

GitHub tự build và đăng lại sau vài phút. Nếu sửa `supabase/schema.sql` thì chạy lại file đó trong SQL Editor (chạy lại nhiều lần không sao, dữ liệu người chơi được giữ).

---

## Cách B. Server axum (Render / VPS)

Dùng khi muốn tự giữ database SQLite thay vì Supabase. Server có giới hạn số phép giải chạy cùng lúc (`RUBIK_MAX_JOBS`), giới hạn số yêu cầu theo IP, giới hạn độ dài công thức và khung hình camera, nên chịu được truy cập công khai ở mức nhóm vừa. **Lưu ý**: gói miễn phí của Render **xoá ổ đĩa mỗi lần khởi động lại**, nên muốn giữ tài khoản phải dùng gói trả phí có **Persistent Disk** (`render.yaml` đã chọn gói `starter`).

### B1. Render bằng Blueprint (vài cú bấm)
1. Làm Bước 3–4 của cách A để mã nguồn có trên GitHub (repo có sẵn `render.yaml`).
2. https://render.com → đăng nhập bằng GitHub → **New → Blueprint** → chọn repo → **Apply**.
3. Đợi build lần đầu 5–10 phút. Link dạng `https://rubik-web-xxxx.onrender.com`.

`render.yaml` đã đặt sẵn: ổ đĩa 1 GB gắn tại `/var/data`, database `RUBIK_DB=/var/data/rubik.db`, `RUBIK_SECURE_COOKIE=1`, `RUBIK_TRUST_PROXY=1` (**bắt buộc** sau Render/Nginx/Caddy: thiếu thì mọi người dùng bị tính chung một IP và giới hạn tốc độ chặn nhầm), tự sao lưu mỗi 24 giờ vào `/var/data/backups` (giữ 7 bản), health check `/api/health`. `PORT` do Render cấp, server tự nghe trên `0.0.0.0:PORT`.

### B2. Render thủ công (không dùng Blueprint)
**New → Web Service** → chọn repo → Runtime **Rust**, Build `cargo build --release -p rubik-web`, Start `./target/release/rubik-web`, gói Starter trở lên, thêm Disk (mount `/var/data`, 1 GB), đặt các biến môi trường như B1, Health Check Path `/api/health`.

### B3. VPS (Ubuntu)
Cài Rust (`curl https://sh.rustup.rs -sSf | sh`), clone repo, `cargo build --release -p rubik-web`, chạy với các biến môi trường như B1 (thêm `RUBIK_ADDR=127.0.0.1:8080` khi đứng sau Nginx/Caddy), đặt sau Nginx/Caddy để có HTTPS và chạy bằng `systemd` để tự khởi động lại. Server tắt êm khi nhận SIGTERM/Ctrl+C: hoàn tất các yêu cầu đang chạy rồi mới thoát.

**Chép riêng file chạy**: chép `rubik-web` (hoặc `rubik-web.exe`) **cùng thư mục `static`** (lấy từ `rubik-web/static`) đặt cạnh nó. Server tự tìm `static` cạnh file chạy; muốn để chỗ khác thì đặt `RUBIK_STATIC=<đường dẫn>`. Dòng khởi động in ra đường dẫn giao diện đang dùng.

### Sao lưu và dọn dẹp database (bản server)
- **Sao lưu khi đang chạy, không cần tắt server:** `rubik-web backup sao-luu.db`. Dùng `VACUUM INTO` của SQLite nên bản sao nhất quán, đã nén gọn, và không bao giờ ghi đè file có sẵn.
- **Tự động:** đặt `RUBIK_BACKUP_DIR=/duong/dan` (chu kỳ `RUBIK_BACKUP_HOURS`, mặc định 24; giữ `RUBIK_BACKUP_KEEP` bản, mặc định 7). Nhớ chép các bản này ra khỏi máy chủ định kỳ: sao lưu cùng ổ đĩa không cứu được khi mất ổ.
- **Khôi phục:** tắt server, chép bản sao lưu đè lên file `RUBIK_DB`, bật lại.
- **Dọn dẹp tự động mỗi giờ:** ván "đang chơi" quá 1 giờ (người chơi đóng tab) thành "bỏ dở", ván bỏ dở quá 7 ngày bị xoá, phiên đăng nhập hết hạn bị xoá. Ván đã giải xong và tài khoản được giữ mãi. Bản Supabase dùng hàm `cleanup_old_games()` (xem `supabase/HUONG_DAN.md`).

---

## Cách C. Chạy trên máy mình

- **App desktop**: bấm đúp `CHAY_THUONG.bat` (cần cài Rust: https://rustup.rs và *Visual C++ Build Tools*).
- **Bản web có tài khoản (SQLite)**: bấm đúp `CHAY_WEB.bat` rồi mở http://127.0.0.1:8080. Database nằm ở `rubik.db` trong thư mục dự án.
- **Thử đúng bản sẽ lên GitHub Pages** (cần `wasm-pack`):
  ```bat
  wasm-pack build rubik-wasm --release --target web --out-dir www/pkg
  cd rubik-wasm\www
  python -m http.server 8000
  ```
  rồi mở http://localhost:8000.

---

## Cài như ứng dụng và dùng ngoại tuyến (bản GitHub Pages)
Bản GitHub Pages là PWA: trình duyệt hiện nút **Cài đặt** (Chrome/Edge trên máy tính) hoặc **Thêm vào màn hình chính** (điện thoại). Sau lần mở đầu, trang và bộ giải được lưu lại nên mở được cả khi không có mạng; xoay khối, xáo, giải và gợi ý vẫn chạy ngoại tuyến, chỉ tài khoản/xếp hạng cần mạng. Bản mới tải ở nền và hiện ra ở lần mở kế tiếp. Muốn buộc mọi máy bỏ bản cũ: đổi `VERSION` trong `web/sw.js` rồi `node web/build.js`. Font Be Vietnam Pro nằm ngay trong dự án (`web/fonts`, giấy phép OFL), không tải từ Google.

## Gặp lỗi

| Hiện tượng | Cách xử lý |
|---|---|
| Actions báo đỏ ở bước *Build WebAssembly* | Bấm vào bước đó xem log. Thường do sửa code Rust bị lỗi: chạy `cargo build -p rubik-wasm` trên máy để xem lỗi rõ hơn. |
| Actions báo lỗi *Pages not enabled* / 404 | Chưa làm Bước 5.2 (Source phải là **GitHub Actions**). |
| Trang mở được nhưng không có nút Xếp hạng | `config.js` còn trống hoặc sai dấu nháy. Sửa, commit, push lại. |
| Đăng ký báo *xác nhận email* | Chưa tắt **Confirm email** (Bước 1.4). |
| Đăng ký báo *email không hợp lệ* | Đổi `emailDomain` trong `config.js` (ví dụ `'nguoichoi.rubik.app'`). Chỉ đổi khi chưa có ai đăng ký. |
| Báo *permission denied for function* | Chưa chạy `schema.sql`, hoặc chạy chưa hết. Chạy lại toàn bộ file. |
| Bảng xếp hạng trống dù đã giải | Chỉ **ván xếp hạng** mới được tính. Dùng gợi ý / giải tự động / xáo lại sẽ huỷ ván. |
| Camera không hoạt động | Chỉ chạy trên HTTPS hoặc localhost, và phải cho phép quyền camera trong trình duyệt. |
| `git push` bị từ chối | Repo trên GitHub đã có file (do tick README lúc tạo). Chạy `git pull origin main --allow-unrelated-histories`, rồi push lại. |
| Supabase báo project *paused* | Gói miễn phí tạm dừng project sau khoảng 1 tuần không ai dùng. Vào dashboard bấm **Restore**. |

## Bảo mật cần nhớ
- Chỉ khoá **anon** được nằm trong `config.js`. Khoá `service_role` cho toàn quyền database, không bao giờ đưa vào mã nguồn.
- Mật khẩu database Supabase cũng không đưa vào repo.
- Trước khi push, xem lại `git status` để chắc không có file `rubik.db` (chứa tài khoản người chơi của bản server).

---

# Ghi chú kỹ thuật

Chạy thử bản wasm trên máy (cần Rust bản mới và wasm-pack):

    wasm-pack build rubik-wasm --release --target web --out-dir www/pkg
    cd rubik-wasm/www && python -m http.server 8000     # mở http://localhost:8000

Ghi chú:
- `rubik-core` có thêm feature `parallel` (mặc định BẬT, app Rust không đổi). Bản wasm tắt nó vì trình duyệt không có rayon, nên các phép tìm kiếm chạy tuần tự, kết quả giống hệt, chỉ chậm hơn.
- Camera chỉ chạy trên HTTPS hoặc localhost. GitHub Pages là HTTPS nên dùng được.
- Thêm `rubik-wasm/www/pkg/` vào `.gitignore`.

## Chức năng giao diện web (đã khớp app Rust)
Animation xoay lớp 3D theo từng cubie (kể cả nước wide, M/E/S, x/y/z, nước đúp), chọn mặt rồi dùng phím mũi tên,
phím tắt giống main.rs (Shift nghịch, Ctrl wide, Alt đúp), gợi ý "thử hướng khác" khi bấm H lần nữa,
bảng công thức, thống kê OLL/PLL (lưu trong localStorage, xuất TSV cùng định dạng file của app),
điền màu thủ công, quét camera (tự mở bảng điền màu khi quét thất bại), nút Dừng huỷ được phép tính đang chạy.

## Cấu trúc workspace sau cập nhật
Thêm 3 thành viên vào `members` của `Cargo.toml` gốc: `"rubik-api"`, `"rubik-wasm"`, `"rubik-web"`.
- `rubik-api`: toàn bộ route và xử lý quét ảnh, dùng chung cho cả hai bản.
- `rubik-wasm`: lớp mỏng cho trình duyệt (bản deploy miễn phí).
- `rubik-web`: server axum mỏng (tuỳ chọn, chạy cục bộ hoặc VPS): `cargo run --release -p rubik-web`.

## Quét camera (khớp scan_ui.rs)
Dò mặt khối bằng hình học (`detect_cube_face`), hiệu chỉnh phối cảnh (`warp_to_square`), lấy mẫu có độ tin cậy,
gộp tới 10 khung hình khi giữ yên (`aggregate_frames`), chụp thủ công (Space) hoặc tự động sau 5 khung ổn định,
nạp ảnh từ file, hướng dẫn tư thế cho từng mặt, sửa màu từng ô khi ghép thất bại, chọn phương án khi có nhiều cách ghép.
Gợi ý: H chỉ tính và hiển thị, chạy bằng nút "Chạy gợi ý" hoặc nhấn Alt (từng nước), giống app.
Bộ dò ONNX được bỏ qua theo yêu cầu (chính app cũng ghi nó kém hơn bộ dò hình học).

## Lưu ý về feature "parallel"
`rubik-vision/Cargo.toml` đã đổi sang `default-features = false` cho `rubik-core`. Nếu không, Cargo gộp feature và vẫn bật rayon trong bản wasm.
App Rust và bản server không bị ảnh hưởng vì chúng khai báo `rubik-core` với feature mặc định.
Kiểm tra bản wasm không còn rayon: `cargo tree -p rubik-wasm --target wasm32-unknown-unknown -e features | grep -i rayon` (không được có kết quả).

## Kiểm thử giao diện (không cần Rust)
`rubik-web/tests/ui.test.js` chạy trang thật trong jsdom với backend giả, kiểm tra phím tắt (Shift, Ctrl, Alt, Ctrl/Alt bấm rồi thả),
luồng gợi ý, thống kê, nhập TSV, bảng hoán vị nước đi (đối chiếu vectơ chuẩn do Rust tính), cửa sổ nổi và bàn phím cho menu/cửa sổ:
`cd rubik-web/tests && npm install && npm test`. Không kiểm tra hiển thị 3D, camera hay thuật toán giải.

## Kiểm thử phía Rust (chạy được trên máy thường, không cần wasm)
    cargo test -p rubik-api --release -- --nocapture   # đầu-cuối ở đúng cấu hình wasm (không rayon)
    cargo test -p rubik-core --release                 # bộ test gốc, chạy được cả với --no-default-features
    cargo test -p rubik-web --release                  # server: tài khoản + test HTTP toàn bộ router

## Chạy demo local (không cần deploy)
- Windows: bấm đúp `CHAY_WEB.bat` (cần cài Rust từ https://rustup.rs). Linux/macOS: `./chay_web.sh`.
- Bản này dùng server axum, chạy ở http://127.0.0.1:8080, có đủ chức năng và dùng rayon nên giải nhanh hơn bản wasm.
- Muốn thử đúng bản wasm sẽ deploy: xem mục "Chạy thử trên máy" ở trên (wasm-pack + http.server).

## Cửa sổ nổi (popover)
Bảng công thức và Thư viện công thức là cửa sổ nổi trên giao diện:
- Kéo tiêu đề để di chuyển; kéo 4 cạnh hoặc 4 góc để đổi kích thước (tối thiểu 280×150, không vượt khỏi màn hình).
- Nhấp đúp tiêu đề (hoặc nút –) để thu gọn; nút ✕ hoặc phím Esc để đóng; bấm vào cửa sổ để đưa lên trên cùng.
- Vị trí và kích thước được nhớ trong trình duyệt (localStorage, khoá `rubik_windows_v1`).
- Bảng công thức tự mở khi giải hoặc xin gợi ý. Mở/đóng thủ công bằng nút "Bảng" và "Công thức" trên thanh trên cùng (hoặc menu "Thêm"). Ctrl+J mở thư viện và đưa con trỏ vào ô tìm kiếm.
- Trên điện thoại (≤860 px) cửa sổ trở thành tấm trượt từ đáy màn hình, không kéo hay đổi kích thước.

## Tài khoản & bảng xếp hạng (Supabase)
Bản GitHub Pages có đăng ký/đăng nhập, ván xếp hạng và bảng xếp hạng, dữ liệu lưu trên Supabase (gói miễn phí).
Làm theo `supabase/HUONG_DAN.md`: tạo project, chạy `supabase/schema.sql`, tắt "Confirm email", điền `rubik-wasm/www/config.js`.
Khi `config.js` còn để trống, các nút Xếp hạng/Tài khoản tự ẩn và trang chạy như cũ.
Bản server axum (`rubik-web`) vẫn có tài khoản riêng của nó, lưu trong SQLite `rubik.db` (xem `rubik-web/README.md`); hai nơi không dùng chung dữ liệu.

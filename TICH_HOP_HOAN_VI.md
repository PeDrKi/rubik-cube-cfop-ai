# Tích hợp: khung "Sơ đồ hoán vị"

Thêm vào bản web một khung nằm ngay dưới sơ đồ 6 mặt (góc trên bên trái của khung 3D), vẽ 54 ô của khối trên 9 vòng tròn.
Khi chọn một mặt ở sơ đồ 6 mặt (bấm vào mặt đó), 9 ô của mặt ấy trong sơ đồ hoán vị cũng sáng lên (viền trắng, to hơn), các ô còn lại mờ đi; bấm lại để bỏ chọn.
Bật/tắt bằng phím **P** hoặc mục "Sơ đồ hoán vị" trong menu Thêm; lựa chọn được nhớ (mặc định bật).

- Hình học đo từ video gốc: 3 họ vòng tròn đồng tâm (mỗi trục x/y/z một họ, mỗi vòng là một lớp). Mỗi ô dán nằm ở giao điểm của 2 vòng.
- Chấm lấy màu từ `state` của app và tự cập nhật sau mỗi nước (hook trong `paint()`).
- Khi xoay (`setPose`), vòng của lớp đang xoay được tô theo màu 12 ô trên nó; các chấm chạy dọc vòng, 8 ô trên mặt nhảy quanh tâm mặt. Nước 180° đi hai chặng.
- Hỗ trợ mọi ký hiệu của app: U D F B L R, M E S, u d f b l r, x y z, kèm ' và 2.

Sửa ở `web/src/{index.html,style.css,app.js}` (mã trong app.js nằm trong khối `const PV`), rồi chạy `node web/build.js` để sinh 2 file HTML (đã chạy sẵn).

Kiểm thử: `cd rubik-web/tests && npm install && npm run perm` (không cần server). Test đối chiếu hình học với `MT` cho cả 54 nước trong bảng, 54 vị trí khác nhau, 9 vòng, màu chấm theo trạng thái, phím P và mục menu. `npm test` cũ vẫn đạt 83/83.

## Bản app desktop (egui)

Cùng tính năng cho `rubik-app`: khung "Sơ đồ hoán vị" nằm trong thẻ "Sơ đồ 6 mặt", ngay dưới sơ đồ 6 mặt.

- Bật/tắt: phím **P** hoặc ô tích "Sơ đồ hoán vị (phím P)" trong thẻ (mặc định bật). Có thêm dòng trong bảng phím tắt (⚙).
- Mã: `rubik-app/src/perm_view.rs` (vẽ bằng egui::Painter), nối ở `ui_panel.rs`, `input.rs`, `app.rs`, `main.rs`, `ui_windows.rs`.
- Hiệu ứng chạy theo `PendingMove` đang xoay của app, nên nhận mọi nước: U D F B L R, M E S, u d f b l r, x y z, kèm ' và 2.
- Kiểm thử: `cargo test -p rubik-app perm_view` đối chiếu phép xoay hình học với `CubeState::do_move` cho cả 54 nước trên một khối đã xáo, và kiểm tra 54 ô có 54 vị trí khác nhau.
- Chưa mở cửa sổ app để xem bằng mắt (môi trường thử không có màn hình/GPU); phần hình học đã kiểm chứng bằng test, và cùng thuật toán với bản web đã xem bằng mắt.

## Cải thiện rubik-vision (phân loại màu)

Thay đổi trong `rubik-vision/src/color.rs`:
1. Chuẩn hoá sắc độ theo cường độ (a,b chia theo độ sáng, L giảm trọng số 0.35) → bớt nhạy với bóng đổ/độ sáng.
2. Cân bằng trắng theo từng mặt (3 vòng, median phần dư a,b) → bù ám màu/đèn vàng.
3. Phạt "chói → trắng" trong `balanced_assign` (`sticker_cost`) → ô bị loá khó bị gán sai.

Đo bằng `cargo run --release -p rubik-vision --example bench_synth 300` (dữ liệu **giả lập**, 300 ca/mức):

| mức | đúng trước → sau |
|---|---|
| vừa | 49.7% → 90.3% |
| chỉ đổi sáng | 83% → 99.3% |
| chỉ bóng đổ | 65% → 99.3% |
| khó (tổng hợp) | còn thấp, 19.3% (chủ yếu báo lỗi, 2.7% sai âm thầm) |

Chưa kiểm chứng với camera thật; `rubik-vision-ml` chưa build được trong sandbox (ort-sys cần tải nhị phân).

## Quét camera chậm trên máy yếu

Đo bằng `cargo run [--release] -p rubik-vision --example bench_frame 320 240` (dò mặt + duỗi phối cảnh + lấy mẫu, mỗi khung hình):

| Chế độ | ms / khung |
|---|---|
| release | ~7 |
| debug (trước) | ~89 (vượt ngân sách 66 ms của 15 fps -> giật) |
| debug (sau khi thêm `[profile.dev.package]` opt-level=3) | ~13 |

Khuyến nghị: dùng `cargo run --release -p rubik-app`. Nếu vẫn chạy debug thì Cargo.toml nay đã tối ưu sẵn các thư viện nặng.

## Chế độ tập trung (Focus)

Phím **V** (hoặc menu "Thêm" → "Chế độ tập trung" trên web; mục "Thoát chế độ tập trung" trong popover Công cụ trên app): toàn màn hình, ẩn mọi thứ trừ khối Rubik ở giữa (vẫn kéo xoay, cuộn thu phóng, phím xoay mặt như thường).

Mở chức năng dưới dạng popover:
- Đưa chuột vào **mép trái** (hoặc phím **,**) → *Điều khiển* (sơ đồ 6 mặt, sơ đồ hoán vị, nút xáo/giải/gợi ý, ô công thức…).
- Đưa chuột vào **mép phải** (hoặc phím **.**) → *Công cụ* (web: sơ đồ + menu "Thêm"; app: bảng công thức, thống kê, thư viện, quét camera, điền màu, phím tắt).
- Mở bằng chuột thì tự đóng khi chuột rời đi; mở bằng phím thì ghim cho tới khi bấm lại phím, **Esc**, hoặc bấm/kéo vào khối.
- Web: thoát toàn màn hình bằng Esc/F11 cũng thoát luôn chế độ tập trung.

Kiểm thử: web `node focus.test.js` (jsdom); app `cargo test -p rubik-app` (+ chạy thật dưới Xvfb: V, ,, ., Esc, hover mép trái/phải đều đúng). Chưa thử trên máy có màn hình thật / cảm ứng.

# Rubik's Cube — Rust

Ứng dụng Rubik's Cube 3D tương tác kèm bộ giải CFOP tự động, port từ dự
án Python gốc sang Rust.

## Cấu trúc

```
rubik/
├── rubik-core/       Thư viện: toàn bộ thuật toán giải (không phụ thuộc giao diện)
│   ├── src/          17 module lõi
│   └── tests/        21 test tích hợp (+19 test trong lib, +1 doctest)
├── rubik-vision/     Đọc màu cube từ ảnh: dò vị trí, lấy mẫu, phân loại màu (23 test)
├── rubik-vision-ml/  Bộ dò bằng mô hình ONNX (tuỳ chọn, sau cờ `ml_detect`)
└── rubik-app/        Ứng dụng: cửa sổ 3D, GUI, quét camera, nhật ký OLL/PLL (29 test)
    └── src/          main.rs (vòng lặp sự kiện) + mesh.rs (dựng hình
                       học) + solve_flow.rs (điều phối job/kế hoạch giải
                       tự động) + ui_widgets.rs (vẽ/soi UI) tách riêng
```

Trước đây lõi bị **sao chép** giữa 2 project riêng, phải sửa lỗi 2 lần và
code đã bắt đầu lệch nhau. Nay gộp thành Cargo workspace — chỉ còn 1
nguồn sự thật duy nhất.

## Chạy

**Cách nhanh nhất (Windows):** bấm đúp vào **`CHAY.bat`**. File này tự
kiểm tra Rust đã cài chưa và báo lỗi rõ ràng nếu thiếu, rồi build bản
release và mở app — không mở cửa sổ dòng lệnh đứng lại phía sau.

Mặc định `CHAY.bat` build thẳng **bản có thêm chế độ ML thử nghiệm**
(không hỏi gì, menu chọn hiện đang tắt — có ghi chú cách bật lại ngay
trong file). Muốn vào **bản THƯỜNG** (không có ML, build nhanh hơn):
bấm đúp **`CHAY_THUONG.bat`**, hoặc chạy `CHAY.bat --1` từ dòng lệnh.

Trên Linux/macOS (chưa có script tương ứng CHAY.bat), chạy thủ công:

```bash
cargo run --release -p rubik-app   # mở ứng dụng
cargo test --release               # chạy toàn bộ test (~7 phút, xem số lượng ở mục Cấu trúc)
```

Lần đầu build mất 1-3 phút. Pattern database được xây trên 1 thread nền
ngay từ lúc mở app (không chặn cửa sổ hiện ra) nên không còn cảm giác
khựng ở lần gợi ý/giải đầu tiên.

**Yêu cầu**: Rust (https://rustup.rs). Trên Windows cần thêm Visual C++
Build Tools (chọn workload "Desktop development with C++").

## Giao diện

Giao diện tiếng Việt có dấu (nhúng font DejaVuSans vì font mặc định của
egui thiếu ký tự tiếng Việt), chủ đề tối với màu nhấn cam, các nhóm chức
năng tách thành thẻ riêng: sơ đồ 6 mặt (có chữ tên mặt, viền vàng đánh
dấu mặt đang chọn), điều khiển, nhập công thức, kết quả gợi ý, và thanh
trạng thái kèm tiến trình khi khối đang xoay.

Giao diện **co giãn theo kích thước cửa sổ**: chữ, nút và ô sơ đồ tự
phóng to/thu nhỏ (mốc chuẩn cửa sổ cao 800px). Panel bên trái **kéo giãn
được bằng tay** (280–640pt, kéo mép phải panel) và **cuộn được** khi cửa
sổ thấp. Các cửa sổ nổi (bảng công thức, thư viện, thống kê, phím tắt)
đều resize được ở 4 cạnh/góc, nội dung tự xuống dòng theo bề rộng.

## Điều khiển

| Phím / thao tác | Chức năng |
|---|---|
| Kéo chuột trái | Xoay góc nhìn |
| Cuộn chuột | Thu phóng |
| `U D F B L R` | Xoay lớp tương ứng |
| `Shift` + phím trên | Nước nghịch đảo (`U'`, `R'`…) |
| `Ctrl` + phím trên | Nước wide, xoay 2 lớp (`u`, `r`…) |
| `Alt` + phím trên | Nước đúp 180° (`U2`, `R2`…) |
| `M` `E` `S` | Xoay lớp giữa |
| `X` `Y` `Z` | Xoay cả khối |
| Click sticker → phím mũi tên | Chọn mặt rồi xoay (↑→ thuận, ↓← nghịch) |
| `Space` | Xáo ngẫu nhiên 25 nước |
| `Enter` (hoặc nút "▶ Giải tự động") | Tính nguyên lời giải 1 lần rồi chạy liền mạch, panel gợi ý tự đổi công thức theo từng chặng (Cross → F2L × 4 cặp → OLL → PLL) đúng lúc cube chạy tới đó |
| `H` | Gợi ý 1 bước tiếp theo (không giải hết) |
| `Ctrl` (bấm rồi thả, không kèm phím khác) | Thực hiện gợi ý đang hiển thị (thay cho bấm chuột) |
| `Alt` (bấm rồi thả, không kèm phím khác) | Chạy từng NƯỚC một của gợi ý; hết thì tự tính gợi ý tiếp theo |
| `Ctrl+T` | Bảng công thức: lời giải ván hiện tại, tách theo từng chặng CFOP |
| `Ctrl+J` | Tra công thức chuẩn: toàn bộ 57 OLL + 21 PLL, có tìm kiếm |
| `Ctrl+Y` | Thống kê: số lần đã gặp từng thế OLL/PLL (đọc từ nhật ký) |
| `Ctrl+K` | Quét trạng thái khối từ camera |
| `Esc` | Huỷ khi đang tính; hoặc dừng "Giải tự động" giữa chừng |
| `Ctrl+Z` | Hoàn tác (dừng luôn "Giải tự động" nếu đang chạy) |
| `F11` | Toàn màn hình |
| `[` `]` | Giảm/tăng tốc độ animation (chậm nhất ~2s/nước, nhanh nhất ~0.05s/nước) |
| Thanh nhập | Công thức Singmaster, VD `R U R' U'`, hỗ trợ `(...)3` |

## Nhật ký OLL/PLL

Mỗi lần bảng công thức (`Ctrl+T`) nhận diện được thế OLL và PLL của ván
hiện tại, ứng dụng ghi thêm một dòng vào file nhật ký đặt **cạnh file
thực thi**:

```
target/release/lich_su_oll_pll.tsv
epoch_giay  thoi_gian_utc        loai  ten_the  so_nuoc
1789023176  2026-09-10 06:52:56  OLL   OLL27    7
1789023176  2026-09-10 06:52:56  PLL   T        14
```

Định dạng TSV thuần, mở được bằng Notepad lẫn Excel. Cột thời gian là
**UTC** (không phải giờ máy) vì `std::time` không cho biết lệch múi giờ.
Dòng hỏng hoặc dòng sửa tay bị bỏ qua chứ không làm hỏng phép đếm.

`Ctrl+Y` mở bảng thống kê: hai tab OLL/PLL, mỗi dòng là tên thế + số lần
+ thanh so sánh, sắp giảm dần, **liệt kê cả những thế chưa gặp lần nào**.

Một ván chỉ được ghi một lần: bảng công thức nhớ kèm trạng thái đã dùng
để tính, mở đi mở lại cùng một ván sẽ không tính lại và không đếm trùng.

Vì file nằm cạnh file thực thi nên **`cargo clean` sẽ xoá nhật ký**.

## Thuật toán

Pipeline CFOP đầy đủ:

| Giai đoạn | Phương pháp |
|---|---|
| **Cross** | Leo dốc tham lam trên pattern database (khoảng cách chính xác) |
| **F2L** | A* với heuristic pair-PDB + phạt phá vỡ phần đã giải |
| **OLL** | Bảng tra đủ 57 thế → macro OCLL → search 2-look (dự phòng) |
| **PLL** | Bảng tra 287 mục (21 ca × AUF trước/sau) → macro (dự phòng) |

Nếu một giai đoạn bí, hệ thống tự thử lại với thứ tự tìm kiếm xáo trộn.
Kết quả cuối được rút gọn (`move_simplify`) để bỏ nước thừa.

**Tỉ lệ giải trọn vẹn: 100%** (200/200 scramble ngẫu nhiên, đo bằng test
`batch_200_success_rate`; lời giải trung bình 56 nước). Trước khi sửa hai
bảng OLL/PLL thì tỉ lệ này là 99% (198/200).

**Hiệu năng:** các "tầng" ngân sách tìm kiếm tăng dần (F2L: node theo độ
sâu 8/10/12/14; OLL: 3 tầng mỗi pha) chạy **song song bằng rayon**
(`find_map_first`) thay vì tuần tự — vẫn ra đúng kết quả của tầng nhỏ
nhất thành công, chỉ nhanh hơn trên máy nhiều lõi. Ngân sách của tầng
nặng nhất cũng đã được đo lại và giảm bớt phần dư thừa (F2L 400k→250k
node, OLL 300k→150k node) — xem comment tại hằng số `F2L_NODE_BUDGET`
(`hint.rs`) để biết cách đo lại nếu cần chỉnh tiếp.

## Độ tin cậy

Toàn bộ lõi được kiểm chứng bằng cách **đối chiếu số liệu chính xác với
bản Python gốc**, không chỉ "chạy không lỗi":

- Bảng hoán vị cạnh/góc cho từng nước đi — khớp từng phần tử
- Kích thước pattern database: 190.080 / 136.080 / 576 trạng thái, và
  giá trị khoảng cách tối đa của mỗi bảng
- Trạng thái facelet sau các chuỗi nước đi cụ thể — khớp từng ô

Riêng hai bảng OLL/PLL thì **không** đối chiếu với bản Python nữa, vì bản
Python sai: nó thiếu 2 thế OLL, và chuỗi `E` của nó thực ra là một A-perm
chép nhầm. Thay vào đó chúng được kiểm bằng cách liệt kê TOÀN BỘ trạng
thái có thể xảy ra rồi dựng lại từng trạng thái và áp thật công thức đã
lưu:

- OLL: phủ đủ 57/57 thế, 57 tên ứng 1-1 với 57 thế, mọi công thức áp vào
  đều định hướng xong lớp U mà không phá Cross/F2L, không công thức nào
  dài quá 14 nước
- PLL: phủ 287/288 trạng thái hoán vị lớp cuối (cái còn lại là khối đã
  giải), mọi lời giải áp vào đều giải xong, không hai tên nào trùng ca
- Lưới an toàn `solve_oll_search` giải được cả 57 thế

Các bảng hoán vị **không được chép tay** mà suy ra thực nghiệm lúc chạy
(áp từng nước lên cube đã giải rồi đọc kết quả), nên nếu cube engine
đúng thì các bảng tự động đúng.

## Giới hạn đã biết

- Trên 200 scramble đo được thì giải hết, nhưng ngưỡng test đặt ở 95%
  nên vẫn có thể gặp ván bí — bấm `Space` xáo lại.
- `full_state::apply_move` chỉ hiểu 18 nước cơ bản, không hiểu `M`/`r`/`x`.
  Giao diện và thanh nhập thì hiểu đủ; giới hạn này chỉ nằm ở đường giải,
  và các bảng công thức đã được viết lại cho phù hợp.
- Lựa chọn "ML (kém hơn)" trong màn hình quét đúng như tên: bộ dò đó chỉ
  trả hộp vuông nên không hiệu chỉnh được phối cảnh. Đo trên bộ ảnh
  Bielefeld: Hình học 97,3% so với 32,9%. Chỉ để so sánh, đừng dùng thật.
- Chưa từng đo được độ chính xác khi đưa TỪNG MẶT vuông góc vào webcam —
  bộ ảnh dùng để đo không có kiểu ảnh đó.

## Chạy offline & độc lập

**Lúc chạy: hoàn toàn offline.** Ứng dụng không có bất kỳ mã mạng nào —
không tải gì, không gửi gì. Đã kiểm chứng: chép riêng file thực thi sang
một thư mục trống (không có mã nguồn, không có thư mục `assets`, không
cần cargo) và nó chạy đầy đủ, kể cả font tiếng Việt (font được nhúng
thẳng vào file thực thi qua `include_bytes!`).

Pattern database cũng được dựng lúc chạy (trên thread nền, không chặn
cửa sổ) chứ không tải về.

**Lúc biên dịch: cần mạng đúng một lần** để cargo tải các thư viện phụ
thuộc (three-d, egui, winit…). Sau lần đó, cargo lưu vào bộ đệm máy và
có thể build offline bằng `cargo build --release --offline`. File
`Cargo.lock` đi kèm giúp bản dựng lặp lại được đúng phiên bản.

Muốn build hoàn toàn offline ngay từ đầu (máy chưa từng tải), chạy
`cargo vendor` trên một máy có mạng rồi mang cả thư mục `vendor/` sang.

**Phân phối cho người khác:** chỉ cần gửi file thực thi. Trên Windows là
`rubik-app.exe` (một file duy nhất). Máy nhận không cần cài Rust.

**Thư viện hệ thống cần có** (đều là thứ sẵn có trên máy thường):
- Windows: `opengl32.dll` — có sẵn trong Windows
- Linux: OpenGL (mesa) + X11 hoặc Wayland
- Không cần cài thêm gì cho người dùng cuối

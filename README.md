# Rubik's Cube — Rust

Ứng dụng Rubik's Cube 3D tương tác kèm bộ giải CFOP tự động, port từ dự
án Python gốc sang Rust.

## Cấu trúc

```
rubik/
├── rubik-core/     Thư viện: toàn bộ thuật toán (không phụ thuộc giao diện)
│   ├── src/        16 module lõi
│   └── tests/      21 test đối chiếu với bản Python gốc
└── rubik-app/      Ứng dụng: cửa sổ 3D, GUI (chỉ 1 file main.rs)
```

Trước đây lõi bị **sao chép** giữa 2 project riêng, phải sửa lỗi 2 lần và
code đã bắt đầu lệch nhau. Nay gộp thành Cargo workspace — chỉ còn 1
nguồn sự thật duy nhất.

## Chạy

**Cách nhanh nhất:** bấm đúp vào **`CHAY.bat`** (Windows) hoặc chạy
`./chay.sh` (Linux/macOS). File này tự kiểm tra Rust đã cài chưa và báo
lỗi rõ ràng nếu thiếu.

Hoặc chạy thủ công:

```bash
cargo run --release -p rubik-app   # mở ứng dụng
cargo test --release               # chạy 21 test (~5 phút)
```

Lần đầu build mất 1-3 phút. Khi cửa sổ mở có ~1 giây khựng nhẹ (đang
dựng pattern database, chỉ 1 lần).

**Yêu cầu**: Rust (https://rustup.rs). Trên Windows cần thêm Visual C++
Build Tools (chọn workload "Desktop development with C++").

## Giao diện

Giao diện tiếng Việt có dấu (nhúng font DejaVuSans vì font mặc định của
egui thiếu ký tự tiếng Việt), chủ đề tối với màu nhấn cam, các nhóm chức
năng tách thành thẻ riêng: sơ đồ 6 mặt (có chữ tên mặt, viền vàng đánh
dấu mặt đang chọn), điều khiển, nhập công thức, kết quả gợi ý, và thanh
trạng thái kèm tiến trình khi khối đang xoay.

Giao diện **co giãn theo kích thước cửa sổ**: chữ, nút và ô sơ đồ tự
phóng to/thu nhỏ (mốc chuẩn cửa sổ cao 800px), bề rộng panel bám theo tỉ
lệ cửa sổ trong khoảng 300–430pt. Panel **cuộn được** khi cửa sổ thấp.

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
| `Enter` | AI tự giải từ trạng thái hiện tại |
| `H` | Gợi ý 1 bước tiếp theo (không giải hết) |
| `T` | Bảng công thức: lời giải ván hiện tại, tách theo từng chặng CFOP |
| `V` | Tra công thức chuẩn: toàn bộ 55 OLL + 21 PLL, có tìm kiếm |
| `Esc` | Hủy khi đang tính |
| `Ctrl+Z` | Hoàn tác |
| `F11` | Toàn màn hình |
| `[` `]` | Giảm/tăng tốc độ animation |
| Thanh nhập | Công thức Singmaster, VD `R U R' U'`, hỗ trợ `(...)3` |

## Thuật toán

Pipeline CFOP đầy đủ:

| Giai đoạn | Phương pháp |
|---|---|
| **Cross** | Leo dốc tham lam trên pattern database (khoảng cách chính xác) |
| **F2L** | A* với heuristic pair-PDB + phạt phá vỡ phần đã giải |
| **OLL** | Bảng tra 55 công thức → macro OCLL → search 2-look (dự phòng) |
| **PLL** | Bảng tra 47 mục → macro (T/Y-perm + 3-cycle × AUF) |

Nếu một giai đoạn bí, hệ thống tự thử lại với thứ tự tìm kiếm xáo trộn.
Kết quả cuối được rút gọn (`move_simplify`) để bỏ nước thừa.

**Tỉ lệ giải trọn vẹn: 99%** (198/200 scramble ngẫu nhiên, đo bằng test
`batch_200_success_rate`).

## Độ tin cậy

Toàn bộ lõi được kiểm chứng bằng cách **đối chiếu số liệu chính xác với
bản Python gốc**, không chỉ "chạy không lỗi":

- Bảng hoán vị cạnh/góc cho từng nước đi — khớp từng phần tử
- Kích thước pattern database: 190.080 / 136.080 / 576 trạng thái, và
  giá trị khoảng cách tối đa của mỗi bảng
- Kích thước bảng công thức OLL (55) và PLL (47)
- Trạng thái facelet sau các chuỗi nước đi cụ thể — khớp từng ô

Các bảng hoán vị **không được chép tay** mà suy ra thực nghiệm lúc chạy
(áp từng nước lên cube đã giải rồi đọc kết quả), nên nếu cube engine
đúng thì các bảng tự động đúng.

## Giới hạn đã biết

- ~1% trường hợp AI không tìm được lời giải — bấm `Space` xáo lại.
- Chưa có phím tắt riêng cho M/E/S, nước wide, hay xoay toàn khối
  (`x`/`y`/`z`); nhưng công thức chứa chúng vẫn chạy được qua thanh nhập.

## Chạy offline & độc lập

**Lúc chạy: hoàn toàn offline.** Ứng dụng không có bất kỳ mã mạng nào —
không tải gì, không gửi gì. Đã kiểm chứng: chép riêng file thực thi sang
một thư mục trống (không có mã nguồn, không có thư mục `assets`, không
cần cargo) và nó chạy đầy đủ, kể cả font tiếng Việt (font được nhúng
thẳng vào file thực thi qua `include_bytes!`).

Pattern database cũng được dựng lúc chạy (~1 giây) chứ không tải về.

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

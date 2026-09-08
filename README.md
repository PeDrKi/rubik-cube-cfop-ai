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
| `Shift` + phím trên | Nước nghịch đảo (`U'`, `R'`...) |
| Click sticker → phím mũi tên | Chọn mặt rồi xoay (↑→ thuận, ↓← nghịch) |
| `Space` | Xáo ngẫu nhiên 25 nước |
| `Enter` | AI tự giải từ trạng thái hiện tại |
| `H` | Gợi ý 1 bước tiếp theo (không giải hết) |
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

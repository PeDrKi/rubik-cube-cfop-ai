# So sánh `rubik-rs` và `rubik-render-windowed`

Tài liệu này giải thích hai file zip hiện có là gì, khác nhau ra sao, và
vì sao nên gộp chúng lại.

---

## 1. Tóm tắt nhanh

| | `rubik-rs` | `rubik-render-windowed` |
|---|---|---|
| **Bản chất** | Solver + bộ test (phòng thí nghiệm) | Ứng dụng hoàn chỉnh (sản phẩm) |
| **Chạy ra gì** | In kết quả ra terminal | Mở cửa sổ 3D tương tác |
| **Đồ hoạ** | Không | Có (three-d / OpenGL) |
| **Bộ test** | 20 test đối chiếu với bản Python | Không có |
| **Số dòng code** | ~2.700 | ~3.350 |
| **Số file nguồn** | 13 | 17 |
| **Dùng khi nào** | Kiểm chứng thuật toán, đo hiệu năng | Thực sự dùng để chơi/xem cube |

**Nếu chỉ muốn dùng app:** chỉ cần `rubik-render-windowed`.
**Giữ `rubik-rs` khi:** muốn chạy lại test tự kiểm chứng, hoặc benchmark
khi sửa thuật toán.

---

## 2. Phần dùng chung: lõi thuật toán

Cả hai đều chứa **cùng một bộ 12 file** tạo nên lõi giải Rubik:

| File | Vai trò |
|---|---|
| `cube.rs` | Cube engine: trạng thái 6 mặt, 18+ nước đi, scramble, parser Singmaster |
| `edge_model.rs` / `corner_model.rs` | Mô hình 12 cạnh / 8 góc + bảng hoán vị suy ra thực nghiệm |
| `full_state.rs` | Trạng thái cubie đầy đủ + các hàm kiểm tra giai đoạn (cross_ok, pair_ok...) |
| `pdb.rs` | Pattern database (BFS) — heuristic chính xác cho tìm kiếm |
| `cross_solver.rs` | Giải Cross (leo dốc tham lam trên PDB) |
| `f2l_solver.rs` | Giải F2L (A* với heuristic pair-PDB) |
| `oll_algorithms.rs` / `pll_algorithms.rs` | Bảng tra 55 công thức OLL / 21 công thức PLL |
| `oll_solver.rs` | Search dự phòng cho OLL khi bảng tra miss |
| `macro_solver.rs` | Lưới an toàn: duyệt trên không gian "chiêu" CFOP thật |
| `search.rs` | Engine A* tổng quát |

Đây là phần đã được kiểm chứng khớp 100% với bản Python gốc (bảng hoán
vị, kích thước PDB 190.080 / 136.080 / 576, giá trị khoảng cách tối đa).

---

## 3. Phần chỉ có ở `rubik-rs`

- **`main.rs` dạng benchmark**: chạy thử batch nhiều scramble, in thời
  gian, số node đã duyệt, tỉ lệ giải thành công.
- **20 unit test** — đây là giá trị thật của bản này:
  - Đối chiếu bảng hoán vị cạnh/góc với output Python
  - Đối chiếu kích thước và khoảng cách tối đa của mọi PDB
  - Kiểm tra pipeline CFOP giải trọn vẹn cube
  - Kiểm tra tỉ lệ giải thành công ≥ 95% trên 200 scramble

Chạy: `cargo test --release` (mất khoảng 2-3 phút).

---

## 4. Phần chỉ có ở `rubik-render-windowed`

### 4.1 Giao diện & tương tác
- **`main.rs`** (rất lớn): cửa sổ winit, vòng lặp render, camera orbit,
  panel egui, khung 6 mặt, thanh nhập công thức, xử lý toàn bộ phím tắt.

### 4.2 Các module chức năng bổ sung
| File | Vai trò |
|---|---|
| `solve.rs` | Gộp Cross→F2L→OLL→PLL thành 1 hàm cho UI gọi, có retry |
| `hint.rs` | Gợi ý 1 bước tiếp theo (không giải hết) |
| `cancel.rs` | Cờ hủy job đang chạy (Esc) |
| `move_simplify.rs` | Rút gọn chuỗi nước đi thừa |

### 4.3 Tính năng người dùng
Xáo (Space), AI giải (Enter), Gợi ý (H), Hủy (Esc), Undo (Ctrl+Z),
toàn màn hình (F11), tốc độ animation (`[` `]`), nhập công thức
Singmaster, click sticker chọn mặt + phím mũi tên xoay.

---

## 5. VẤN ĐỀ: lõi bị sao chép, và đã bắt đầu lệch nhau

Bản app **không tham chiếu** tới `rubik-rs` — nó chứa **bản sao** các file
lõi. Đây là nợ kỹ thuật thật, và hậu quả đã xảy ra rồi.

Kiểm tra thực tế bằng `diff` giữa 2 thư mục cho thấy **3 trong 12 file lõi
đã lệch nhau**:

| File | Tình trạng |
|---|---|
| `f2l_solver.rs` | **Lệch** — bản app có thêm điểm kiểm tra hủy (Esc) |
| `search.rs` | **Lệch** — bản app có thêm điểm kiểm tra hủy (Esc) |
| `oll_solver.rs` | **Lệch** — bản app có thêm tham số `retry_seed` để xáo hướng tìm kiếm |
| 9 file còn lại | Giống hệt |

Ba chỗ lệch này **là cố ý** (tính năng chỉ app mới cần), nhưng chúng cho
thấy đúng cái rủi ro: hai bản đang trôi xa nhau, và không có cơ chế nào
đảm bảo bản sửa lỗi ở bên này được áp sang bên kia.

**Điều này đã thực sự gây rắc rối:** khi sửa 2 lỗi gần đây (bug logic F2L
và ngân sách tìm kiếm), phải sửa thủ công ở **cả hai nơi**. Nếu quên một
bên, app sẽ chạy code cũ có lỗi trong khi test ở bản kia vẫn báo xanh —
đây là kiểu lỗi rất khó phát hiện.

---

## 6. ĐỀ XUẤT: gộp thành Cargo workspace

Cấu trúc mục tiêu:

```
rubik/
├── Cargo.toml           <- khai báo workspace
├── rubik-core/          <- THƯ VIỆN: toàn bộ lõi + 20 test
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs       (thay cho main.rs, khai báo các module public)
│       ├── cube.rs
│       ├── f2l_solver.rs
│       └── ... (12 file lõi)
└── rubik-app/           <- ỨNG DỤNG: giao diện 3D
    ├── Cargo.toml       (dependencies: rubik-core = { path = "../rubik-core" })
    └── src/
        ├── main.rs
        └── solve.rs, hint.rs, cancel.rs, move_simplify.rs
```

### Lợi ích
- **Một nguồn sự thật duy nhất**: sửa lõi 1 lần, cả hai cùng có. Không
  còn nguy cơ lệch phiên bản như hiện tại.
- **Test vẫn chạy được**: `cargo test` ở thư mục gốc chạy toàn bộ test
  của `rubik-core`.
- **Chỉ 1 file zip** thay vì 2.
- **Mở rộng dễ**: sau này muốn thêm bản dòng lệnh, bản web (WASM), hay
  bind sang Python — chỉ cần thêm crate mới dùng chung `rubik-core`.

### Cách xử lý 3 chỗ đang lệch
- `cancel.rs` chuyển vào `rubik-core` (lõi cần biết cách kiểm tra hủy).
- `retry_seed` trong `oll_solver.rs` giữ nguyên trong lõi — tham số này
  vô hại với bản benchmark (chỉ cần truyền `None`).

### Công việc cần làm
Chủ yếu là di chuyển file và sửa khai báo:
1. Tạo cấu trúc thư mục workspace
2. Đổi `main.rs` của lõi thành `lib.rs`, khai báo các module là `pub`
3. Trong app, đổi `crate::cube` thành `rubik_core::cube` (và tương tự)
4. Build lại, chạy 20 test, kiểm chứng app vẫn hoạt động

Rủi ro thấp (không đổi logic), nhưng cần chạy đủ test và mở app kiểm tra
để chắc chắn không hỏng gì.

### Cách chạy sau khi gộp
```bash
cargo run --release -p rubik-app   # chạy app
cargo test --release               # chạy toàn bộ test
```

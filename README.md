# Rubik's Cube — Rust

Ứng dụng Rubik's Cube 3D tương tác kèm bộ giải CFOP tự động, port từ dự
án Python gốc (mảng "app mô phỏng + solver") sang Rust. Gồm thêm phần
nhận diện trạng thái cube từ ảnh chụp/webcam.

> Đây là bản port phần **công cụ**. Toàn bộ mảng **nghiên cứu** của dự án
> gốc (`research/`: HLI, A* thiên vị trigger, thí nghiệm, khảo sát) chưa
> được port — xem `DANH_GIA_SO_VOI_GOC.md` để biết chi tiết đã port được
> gì, còn thiếu gì.

## Nội dung thư mục

```
v47_final/
├── Cargo.toml              Khai báo workspace (4 thành viên) + profile release (lto, opt-level 3)
├── Cargo.lock              Khoá phiên bản để bản dựng lặp lại được
├── CHAY.bat                Windows: kiểm tra Rust rồi chạy `cargo run --release -p rubik-app`
├── README.md               File này
├── DANH_GIA_SO_VOI_GOC.md  Đánh giá trung thực bản Rust so với Python gốc (đã port gì / thiếu gì)
├── SO_SANH_2_BAN.md        Lịch sử: vì sao gộp 2 project rời (rubik-rs + rubik-render-windowed) thành workspace
│
├── rubik-core/     Thư viện lõi — toàn bộ thuật toán, không phụ thuộc giao diện (~2.700 dòng)
│   ├── Cargo.toml          Phụ thuộc: rand, rustc-hash
│   ├── src/               17 module (xem "Bản đồ module lõi" bên dưới)
│   └── tests/
│       └── solver_tests.rs  21 test đối chiếu số liệu với bản Python gốc
│
├── rubik-vision/   Thư viện — nhận diện trạng thái cube từ 6 ảnh chụp (~2.000 dòng)
│   ├── Cargo.toml          Phụ thuộc: rubik-core, image
│   └── src/
│       ├── lib.rs          Luồng xử lý: sample → color → scan
│       ├── sample.rs       Lấy màu trung bình lưới 3x3 từ 1 vùng vuông trên ảnh
│       ├── detect.rs       Tự dò khung mặt cube trong khung hình (thuần Rust, không OpenCV)
│       ├── perspective.rs  Hiệu chỉnh phối cảnh khi chụp lệch góc (mặt cube thành hình thang)
│       ├── color.rs        Phân loại 54 ô thành 6 nhóm màu theo khoảng cách Lab tới màu ô tâm
│       └── scan.rs         Ghép 6 mặt thành CubeState hợp lệ (tự thử tối đa 4⁶ tổ hợp xoay)
│
├── rubik-app/      Ứng dụng — cửa sổ 3D + GUI (~3.200 dòng)
│   ├── Cargo.toml          Phụ thuộc: rubik-core, rubik-vision, three-d, egui, winit, nokhwa,
│   │                       image, rand; rubik-vision-ml (optional, sau feature `ml_detect`)
│   ├── assets/
│   │   ├── DejaVuSans.ttf       Font tiếng Việt (nhúng vào binary — xem theme.rs)
│   │   └── DejaVuSans-Bold.ttf
│   └── src/
│       ├── main.rs         Cửa sổ winit, vòng lặp render, camera orbit, panel egui, phím tắt (~1.570 dòng)
│       ├── theme.rs        Nhúng font tiếng Việt + bảng màu/style egui
│       ├── paint_ui.rs     Cửa sổ "Điền màu thủ công": tô 48 ô để mô phỏng trạng thái không cần camera
│       ├── scan_ui.rs      Cửa sổ "Quét từ camera": chụp 6 mặt (webcam/file) → CubeState
│       └── camera_capture.rs  Đọc webcam trên thread nền, chỉ giữ frame mới nhất
│
├── rubik-vision-ml/  THỬ NGHIỆM — dò mặt cube bằng CNN nhỏ (ONNX). Tắt mặc định.
│   ├── Cargo.toml          Phụ thuộc: rubik-vision, image, ort (2.0-rc), ndarray
│   ├── Cargo.lock
│   ├── README.md           Hướng dẫn bật, build, xử lý lỗi `ort`
│   ├── src/lib.rs          MlDetector: nạp model, tiền xử lý 96×96, suy luận qua `ort`
│   ├── examples/try.rs     Chạy model trên 1 ảnh, lưu ket_qua.png có khung đỏ
│   └── models/
│       └── face_detector.onnx   ~551 KB, nhúng vào binary qua include_bytes!
│
└── train_detect/   Script + dữ liệu huấn luyện model ML (cần PyTorch, KHÔNG cần để chạy app)
    ├── prepare_data.py    Sinh dữ liệu huấn luyện (tăng cường cắt-dán nền)
    ├── train.py           Huấn luyện CNN, xuất ONNX
    ├── train_state.pt     Checkpoint PyTorch
    ├── face_detector.pt   Model PyTorch gốc (đối chiếu với bản ONNX)
    ├── face_detector.onnx  Bản ONNX xuất ra (bản sao của model trong rubik-vision-ml)
    ├── train_log.txt      Nhật ký huấn luyện
    └── *_sanity.png, full_frame_test*.png, sanity_grid.png   Ảnh kiểm tra bằng mắt
```

Trước đây lõi bị **sao chép** giữa 2 project riêng (`rubik-rs` và
`rubik-render-windowed`), phải sửa lỗi 2 lần và code đã bắt đầu lệch nhau
(3/12 file lõi đã khác nhau). Nay gộp thành workspace — chỉ còn 1 nguồn
sự thật, dùng chung 1 `Cargo.lock`. Chi tiết: `SO_SANH_2_BAN.md`.

## Chạy

**Windows — cách nhanh nhất:** bấm đúp vào **`CHAY.bat`**. File này tự
kiểm tra Rust đã cài chưa và báo lỗi rõ ràng nếu thiếu.

Chạy thủ công (mọi hệ điều hành):

```bash
cargo run --release -p rubik-app   # mở ứng dụng
cargo test --release               # chạy 21 test (~5 phút, gồm 1 test batch 200 scramble)

cargo run --release -p rubik-app --features ml_detect   # bật thêm chế độ dò cube bằng ML
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
trạng thái kèm tiến trình khi khối đang xoay. Nút bánh răng ⚙ mở cửa sổ
tóm tắt phím tắt ngay trong app; nút "🎨 Điền màu thủ công" mở cửa sổ tô
màu từng ô.

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
| `M` `E` `S` | Xoay lớp giữa (kèm `Shift` để nghịch đảo) |
| `X` `Y` `Z` | Xoay cả khối (kèm `Shift` để nghịch đảo) |
| Click sticker → phím mũi tên | Chọn mặt rồi xoay (↑→ thuận, ↓← nghịch) |
| `Space` | Xáo ngẫu nhiên 25 nước |
| `Enter` | AI tự giải từ trạng thái hiện tại |
| `H` | Gợi ý 1 bước tiếp theo (không giải hết) |
| `Ctrl+T` | Bảng công thức: lời giải ván hiện tại, tách theo từng chặng CFOP |
| `Ctrl+J` | Tra công thức chuẩn: toàn bộ 55 OLL + 21 PLL, có tìm kiếm |
| `Ctrl+K` | Quét trạng thái cube từ camera/ảnh (chỉ vào bằng phím tắt này) |
| `Esc` | Đóng bảng tra / hủy khi đang tính |
| `Ctrl+Z` | Hoàn tác |
| `F11` | Toàn màn hình |
| `[` `]` | Giảm/tăng tốc độ animation |
| Thanh nhập | Công thức Singmaster, VD `R U R' U'`, hỗ trợ `(...)3` |

## Nhập trạng thái cube từ bên ngoài

Ngoài việc gõ công thức, có 2 cách đưa 1 trạng thái cube bất kỳ vào app:

**Điền màu thủ công** (nút 🎨) — cửa sổ hiển thị 6 mặt dạng khai triển,
6 ô tâm điền sẵn (không sửa được, vì tâm không đổi vị trí), 48 ô còn lại
tự chọn 1 trong 6 màu rồi bấm để tô. Giới hạn tối đa 9 ô mỗi màu.

**Quét từ camera** (`Ctrl+K`) — hướng dẫn chụp lần lượt 6 mặt (U, D, F,
B, L, R) qua webcam hoặc nạp ảnh file. Cơ chế (crate `rubik-vision`):

- Không giả định "trắng là màu gì": lấy màu **ô tâm** của 6 ảnh làm 6 màu
  chuẩn, phân loại 54 ô còn lại theo khoảng cách Lab gần nhất — tự thích
  nghi mọi color-scheme và điều kiện ánh sáng.
- Tự dò khung mặt cube trong ảnh (`detect.rs`, thuần Rust) và hiệu chỉnh
  phối cảnh nếu chụp lệch góc (`perspective.rs`).
- Tự thử tối đa 4⁶ = 4096 tổ hợp xoay hướng cầm cube để tìm tổ hợp cho
  trạng thái hợp lệ và giải được.
- Có chế độ dò khung bằng **ML (thử nghiệm)** khi build với `--features
  ml_detect` — xem `rubik-vision-ml/README.md`.

## Bản đồ module lõi (`rubik-core/src/`)

| Module | Vai trò |
|---|---|
| `cube.rs` | Cube engine: 6 mặt × 9 ô, 18+ nước đi, scramble, parser Singmaster |
| `edge_model.rs` / `corner_model.rs` | Mô hình 12 cạnh / 8 góc; bảng hoán vị **suy ra thực nghiệm** lúc chạy (không hard-code) |
| `full_state.rs` | Trạng thái cubie đầy đủ + kiểm tra giai đoạn (cross_ok, pair_ok, cross_f2l_ok…) |
| `pdb.rs` | Pattern database (BFS ngược) — heuristic admissible chính xác, mảng phẳng thay HashMap |
| `search.rs` | Engine A* tổng quát (goal_fn / heuristic_fn dạng closure) |
| `cross_solver.rs` | Giải Cross — leo dốc tham lam trên PDB khoảng cách chính xác |
| `f2l_solver.rs` | Giải F2L — A* với pair-PDB `[u16;576]` + phạt phá vỡ phần đã giải |
| `oll_algorithms.rs` / `pll_algorithms.rs` | Bảng tra 55 OLL / 21 PLL, mỗi mục tự kiểm chứng bằng cách áp thật lên cube |
| `oll_solver.rs` | Search dự phòng cho OLL (2 pha cạnh→góc) khi bảng tra + macro đều miss (~2,5%) |
| `macro_solver.rs` | Lưới an toàn: duyệt trên không gian "chiêu" CFOP thật thay vì từng nước đơn |
| `move_simplify.rs` | Rút gọn chuỗi nước đi thừa (giao hoán mặt đối diện + cộng dồn cùng mặt) |
| `solve.rs` | Gộp Cross→F2L→OLL→PLL thành 1 hàm cho UI, có retry khi bí |
| `breakdown.rs` | Như `solve` nhưng trả lời giải **tách theo từng chặng** kèm trạng thái (bảng `Ctrl+T`) |
| `hint.rs` | Chỉ tính **1 bước tiếp theo** (Cross / 1 cặp F2L / OLL-cạnh / OLL-góc / PLL), không thực thi |
| `cancel.rs` | Cờ hủy job toàn cục (phím `Esc`) — kiến trúc chỉ chạy 1 job AI tại 1 thời điểm |

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

**Tỉ lệ giải trọn vẹn: ~99%** (đo thực tế 198/200 scramble ngẫu nhiên
bằng test `batch_200_success_rate`; ngưỡng test đặt ở 95% để chừa dư địa
cho biến động seed/timing). Nhanh hơn bản Python ~2,4× ở phần solver
(1,05s so với 2,47s mỗi scramble).

## Độ tin cậy

`rubik-core/tests/solver_tests.rs` — 21 test **đối chiếu số liệu chính
xác với bản Python gốc**, không chỉ "chạy không lỗi":

- Bảng hoán vị cạnh/góc cho từng nước đi — khớp từng phần tử
- Kích thước pattern database: 190.080 / 136.080 / 576 trạng thái, và
  giá trị khoảng cách tối đa của mỗi bảng
- Kích thước bảng công thức OLL (55) và PLL (47)
- Trạng thái facelet sau các chuỗi nước đi cụ thể — khớp từng ô
- Pipeline CFOP giải trọn vẹn cube; tỉ lệ thành công ≥ 95% trên 200 scramble

Các bảng hoán vị **không được chép tay** mà suy ra thực nghiệm lúc chạy
(áp từng nước lên cube đã giải rồi đọc kết quả), nên nếu cube engine
đúng thì các bảng tự động đúng.

Trong quá trình port đã tìm ra và sửa **1 bug logic F2L** vốn có trong
bản Python gốc (khối F2L đã xong không được bảo vệ, bị phá lại).

## Giới hạn đã biết

- ~1% trường hợp AI không tìm được lời giải — bấm `Space` xáo lại.
- Quét camera và nhận diện ML còn ở mức thử nghiệm; chất lượng phụ thuộc
  webcam, ánh sáng và cách cầm cube.
- **Chưa port mảng nghiên cứu** (`research/` của bản gốc, ~2.700 dòng:
  HLI, A* thiên vị trigger, thí nghiệm, khảo sát phase-3). Bộ test cũng
  mỏng hơn (21 so với 152) và chưa có test tầng app/UI. Chi tiết và
  khuyến nghị (giữ Python cho research, bind qua PyO3): xem
  `DANH_GIA_SO_VOI_GOC.md`.

## Chạy offline & độc lập

**Lúc chạy: hoàn toàn offline.** Ứng dụng không có bất kỳ mã mạng nào —
không tải gì, không gửi gì. Đã kiểm chứng: chép riêng file thực thi sang
một thư mục trống (không có mã nguồn, không có thư mục `assets`, không
cần cargo) và nó chạy đầy đủ, kể cả font tiếng Việt (font được nhúng
thẳng vào file thực thi qua `include_bytes!`).

Pattern database cũng được dựng lúc chạy (~1 giây) chứ không tải về.
(Bản gốc cache PDB ra đĩa `.pkl`; bản Rust dựng lại mỗi lần chạy.)

**Lúc biên dịch: cần mạng đúng một lần** để cargo tải các thư viện phụ
thuộc (three-d, egui, winit, nokhwa…). Sau lần đó, cargo lưu vào bộ đệm
máy và có thể build offline bằng `cargo build --release --offline`. File
`Cargo.lock` đi kèm giúp bản dựng lặp lại được đúng phiên bản. (Feature
`ml_detect` là ngoại lệ: `ort` tải sẵn thư viện onnxruntime lúc build.)

Muốn build hoàn toàn offline ngay từ đầu (máy chưa từng tải), chạy
`cargo vendor` trên một máy có mạng rồi mang cả thư mục `vendor/` sang.

**Phân phối cho người khác:** chỉ cần gửi file thực thi. Trên Windows là
`rubik-app.exe` (một file duy nhất). Máy nhận không cần cài Rust.

**Thư viện hệ thống cần có** (đều là thứ sẵn có trên máy thường):
- Windows: `opengl32.dll` — có sẵn trong Windows
- Linux: OpenGL (mesa) + X11 hoặc Wayland
- Quét webcam cần quyền truy cập camera của hệ điều hành
- Không cần cài thêm gì cho người dùng cuối

## Tài liệu kèm theo

| File | Nội dung |
|---|---|
| `DANH_GIA_SO_VOI_GOC.md` | Đánh giá trung thực bản Rust so với Python gốc: đã port gì, thiếu gì, chỗ nào tốt/kém hơn, khuyến nghị ưu tiên |
| `SO_SANH_2_BAN.md` | Lịch sử 2 project rời (`rubik-rs` solver+test, `rubik-render-windowed` app) và lý do gộp thành workspace |
| `rubik-vision-ml/README.md` | Bật/build/khắc phục lỗi chế độ dò cube bằng ML; hướng dẫn huấn luyện lại |

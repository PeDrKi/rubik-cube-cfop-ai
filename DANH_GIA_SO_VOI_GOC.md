# Đánh giá bản Rust so với dự án Python gốc

Bản đánh giá trung thực: đã port được gì, còn thiếu gì, và chỗ nào kém hơn.

---

## 0. Tóm tắt trong một câu

Bản Rust đã port **tốt phần app mô phỏng + solver CFOP** (khoảng 5.500
dòng Python), nhưng **chưa đụng tới toàn bộ mảng nghiên cứu** (~2.700
dòng) — vốn là phần tạo nên giá trị học thuật của dự án gốc (paper, chỉ
số HLI, thí nghiệm) — và **bộ test còn mỏng hơn nhiều** (21 so với 152).

| Mảng | Python gốc | Bản Rust | Đánh giá |
|---|---|---|---|
| Cube engine | ~360 dòng | Đã port | Tương đương |
| Solver CFOP | ~1.900 dòng | Đã port | Tương đương, nhanh hơn |
| App mô phỏng | ~2.300 dòng | Đã port phần lớn | Thiếu vài tính năng UI |
| **Nghiên cứu (research/)** | **~2.700 dòng** | **CHƯA CÓ GÌ** | **Thiếu hoàn toàn** |
| Bộ test | 152 test | 21 test | Kém hơn đáng kể |
| Tài liệu | 6 file docs + paper | 1 README | Kém hơn nhiều |

---

## 1. Thiếu hoàn toàn: mảng nghiên cứu (`research/`)

Đây là **khoảng trống lớn nhất**. Dự án gốc không chỉ là app giải Rubik —
nó là một dự án nghiên cứu có paper (`paper/main.tex`, `main_vi.tex`).
Toàn bộ phần này chưa được port:

| Module | Dòng | Nội dung |
|---|---|---|
| `hli_metrics.py` | 119 | **Human-Likeness Index** — 4 chỉ số con đo mức "giống người" của lời giải |
| `trigger_biased_search.py` | 91 | Thuật toán A* thiên vị trigger (đóng góp thuật toán chính của paper) |
| `finger_tricks.py` | 63 | Thư viện các "trigger" CFOP mà người chơi thật hay dùng |
| `baseline_kociemba.py` | 57 | Đường cơ sở để so sánh (thuật toán tối ưu, không giống người) |
| `bootstrap_ci.py` | 106 | Khoảng tin cậy bootstrap (thống kê) |
| `run_experiment.py` | 173 | Chạy thí nghiệm chính |
| `run_pareto_sweep.py` | 112 | Quét biên Pareto (đánh đổi độ dài ↔ giống người) |
| `weight_sensitivity.py` | 130 | Phân tích độ nhạy trọng số |
| `ml_exploration/` | 121 | Sinh dữ liệu huấn luyện ML |
| `phase3_survey/` | ~600 | Khảo sát người thật: ICC, tương quan, xử lý stimuli |
| Còn lại | ~1.100 | Các script phân tích, vẽ biểu đồ Pareto |

**Đây không phải thiếu sót nhỏ.** Nếu mục tiêu của bạn là công bố khoa
học, phần này mới là cốt lõi — bản Rust hiện tại chỉ là "công cụ", chưa
có "nghiên cứu".

Lưu ý thực tế: mảng này dùng nhiều thư viện thống kê/khoa học Python
(numpy, scipy, pandas, matplotlib). Port sang Rust sẽ **rất tốn công và
lợi ích thấp** — Python vẫn là lựa chọn hợp lý hơn cho phần này. Cách
tốt hơn: giữ nghiên cứu bằng Python, gọi solver Rust qua binding (PyO3)
để hưởng tốc độ.

---

## 2. Bộ test kém hơn rõ rệt: 21 so với 152

| | Python gốc | Rust |
|---|---|---|
| Tổng số test | **152** | **21** |
| Test cube engine | Có (`test_cube_engine.py`) | Có (một phần) |
| Test solver | Có (`test_solver.py`) | Có |
| **Test tầng app/UI** | **Có** (`test_app_logic.py`, `test_app_logic2.py`) | **Không có** |
| **Test cơ chế huỷ job** | **Có** (`test_cancellation.py`) | **Không có** |

Bản Python còn có `fake_pygame_stub/` — pygame giả lập để test tầng UI
mà không cần màn hình thật. Bản Rust **không có test UI nào**; toàn bộ
kiểm chứng giao diện là chụp ảnh màn hình thủ công, không tự động hoá,
nên không chạy lại được trong CI.

---

## 3. Tính năng app còn thiếu hoặc kém hơn

| Tính năng | Python | Rust | Ghi chú |
|---|---|---|---|
| Phím tắt M/E/S, wide, x/y/z | Có | **Chỉ qua thanh công thức** | Không có phím riêng |
| Nước đúp (`2`) bằng phím | Có | **Không** | Phải gõ công thức |
| Bảng công thức đầy đủ | Có (`formula_panel.py`, 270 dòng) | **Chỉ có gợi ý 1 bước** | Chưa hiện breakdown cả 4 chặng cùng lúc |
| Thông báo kết quả bằng lời | Có (`solve_result_message`) | Rút gọn | Kém tự nhiên hơn |
| Cache PDB ra đĩa | Có (`solver/cache/*.pkl`) | **Không** | Rust build lại mỗi lần chạy (~1s, chấp nhận được) |

---

## 4. Tài liệu kém hơn nhiều

Python gốc có 6 file tài liệu chuyên biệt:
`HUONG_DAN_CHAY.md`, `CFOP_AI_README.md`, `REPRODUCE.md` (tái lập số
liệu paper), `README_TEST_APP.md`, `REFACTOR_NOTES.md`,
`CHANGELOG_SESSION.md` — cộng với paper LaTeX song ngữ.

Bản Rust chỉ có **1 file README**. Thiếu đặc biệt:
- Tài liệu kiến trúc thuật toán chi tiết
- Hướng dẫn tái lập kết quả
- Changelog / ghi chú refactor

---

## 5. Chỗ bản Rust LÀM TỐT HƠN

Để công bằng:

| Điểm | Chi tiết |
|---|---|
| **Tốc độ** | Nhanh hơn ~2,4× ở phần solver (đo thật: 1,05s so với 2,47s mỗi scramble) |
| **Độ tin cậy giải** | 99% giải trọn vẹn — trong quá trình port đã **tìm ra và sửa 1 bug logic F2L** vốn có trong bản gốc (khối F2L đã xong không được bảo vệ, bị phá lại) |
| **Render** | GPU thật (OpenGL) thay vì rasterizer phần mềm bằng numpy |
| **Kiến trúc** | Workspace 1 nguồn sự thật, tách thư viện/ứng dụng rõ ràng |
| **An toàn bộ nhớ** | Không có lỗi kiểu null/dangling pointer |

---

## 6. Khuyến nghị ưu tiên

**Nếu mục tiêu là nghiên cứu/paper:**
1. **Giữ nguyên Python cho `research/`** — không port. Thay vào đó tạo
   binding PyO3 để Python gọi solver Rust (nhanh hơn 2,4×, giúp chạy
   thí nghiệm nhanh hơn nhiều).
2. Đây là cách tận dụng cả hai: tốc độ của Rust + hệ sinh thái khoa học
   của Python.

**Nếu mục tiêu là ứng dụng hoàn chỉnh:**
1. Bổ sung test cho tầng app (hiện đang trống hoàn toàn)
2. Thêm bảng công thức đầy đủ + các phím tắt còn thiếu
3. Viết thêm tài liệu kiến trúc

**Điều KHÔNG nên làm:** port `research/` sang Rust. Công sức lớn, lợi
ích nhỏ, và mất đi hệ sinh thái numpy/scipy/pandas vốn không có bản
thay thế tương đương trong Rust.

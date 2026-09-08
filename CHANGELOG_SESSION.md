# Tóm tắt thay đổi trong phiên làm việc này

So với bản `rubik_simulation_2P_CFOP_AI.zip` gốc bạn upload, đây là tổng
hợp đầy đủ mọi thay đổi/thêm mới. Cache PDB (`solver/cache/*.pkl`) đã bị
xoá khỏi gói để giảm dung lượng — sẽ **tự động build lại** trong lần
chạy đầu tiên.

---

## PHẦN 1 — Sửa lỗi & cải tiến tính năng gốc

- **`solver/pll_algorithms.py`** — thêm 2 công thức PLL còn thiếu: `Ab`
  (nghịch đảo chính xác của Aa) và `Z` (bản không dùng M-slice). Bảng PLL
  giờ đủ **21/21**, tất cả đã verify bằng code.
- **`main.py`** — thêm nút "Copy → bar" cạnh dòng gợi ý (hint), bấm vào
  chép chuỗi nước đi gợi ý vào thanh Singmaster để chạy/sửa ngay.
- **`demo_ai_search.py`**, **`benchmark.py`** (file mới ở gốc project) —
  minh hoạ/benchmark nhanh cách AI search Cross/F2L hoạt động.

## PHẦN 2 — Hạ tầng nghiên cứu (`research/`) cho đề tài NCKH

**Hướng nghiên cứu đã chốt:** đo lường & cải thiện "tính người"
(human-likeness) của lời giải AI-CFOP so với solver tối ưu thuần tuý
(computer-like), dùng khung lý thuyết Human-Aware AI Planning
(explicability) + phương pháp validation kiểu BotPrize.

### Solver — thêm field phục vụ đo lường (không đổi hành vi mặc định)
- `solver/oll_solver.py`, `solver/pll_solver.py` — `solve_oll()`/
  `solve_pll()` trả thêm field `source`/`*_source` (`'named'` hay
  `'search'`/`'macro'`) để đo Pattern-conformity.
- `solver/cfop_ai.py` — `full_solve()` gộp thành
  `oll_pll_stage_count`/`oll_pll_named_count`.
- `solver/f2l_solver.py` — thêm tham số tuỳ chọn `trigger_bonus_fn`,
  `lam` (λ) — cơ chế **Trigger-Biased A\* Search**. Mặc định `lam=0.0`
  → hành vi giống hệt bản gốc (176 test cũ vẫn pass).

### `research/` — các module cốt lõi
- `hli_metrics.py` — công thức Human-Likeness Index (HLI): Segmentability,
  Pattern-conformity, Trigger-overlap, Move-count ratio.
- `finger_tricks.py` — corpus 11 trigger/finger-trick phổ biến (seed
  corpus thủ công, nên mở rộng bằng dữ liệu thật nếu có).
- `baseline_kociemba.py` — baseline "computer-like" (Kociemba two-phase).
- `trigger_biased_search.py` — hàm tính trigger bonus + wrapper gọi
  `solve_f2l()` với λ tuỳ chỉnh.
- `scramble_utils.py` — sinh scramble **random-state chuẩn WCA** (random
  walk dài + nghịch đảo lời giải Kociemba gần-tối-ưu), thay thế cho
  random-move scramble ban đầu.
- `weight_sensitivity.py` — phân tích độ nhạy của HLI theo bộ trọng số
  $(w_S, w_P, w_O)$.
- `run_experiment.py` — Experiment 1: so sánh Nhóm A (AI-CFOP) vs Nhóm B
  (Kociemba), batch N scramble, có timeout thật (SIGALRM) chống treo.
- `run_pareto_sweep.py` — Experiment 2: quét nhiều λ, đo trade-off
  HLI/số nước/thời gian.
- `plot_pareto.py` — vẽ biểu đồ Pareto (matplotlib) từ CSV kết quả.

### Kết quả thực nghiệm THẬT đã chạy (không phải pilot/giả lập)
- **Experiment 1** (`results_n100.csv`, N=100, random-move): Nhóm A
  trung bình 67.2 nước (sd 11.1, HLI=0.655) vs Nhóm B 20.6 nước (sd 1.1).
  78% giải thành công trong ngân sách/timeout, 22% timeout (hiện tượng
  "scramble pathological" — xem seed=1013 hang >5 phút, đã ghi nhận là
  phát hiện nghiên cứu, không che giấu). Wilcoxon signed-rank:
  p=1.65×10⁻¹⁴, effect size r=0.87.
- **Robustness check** (`results_randomstate_n60.csv`, N=60,
  random-state chuẩn WCA): kết quả gần như giống hệt random-move
  (80% thành công, 66.3 nước, HLI=0.658) → xác nhận kết quả gốc không bị
  thiên vị bởi cách sinh scramble.
- **Experiment 2** (`pareto_n60.csv`, N=60, λ∈{0,0.5,1.0}): λ=1.0 nâng
  Trigger-overlap từ 9.0%→25.5% (+2.8 lần), đổi lấy +7.4% số nước.
  Wilcoxon: trigger-overlap p=8.25×10⁻⁸ (r=0.83), số nước p=0.0014
  (r=0.60).
- **Phát hiện phụ quan trọng (trái ngược giả thuyết ban đầu)**: λ càng
  cao, tỷ lệ THẤT BẠI hội tụ search càng GIẢM (15.0%→1.7%→0%), không
  tăng như suy đoán lúc đầu — đã sửa lại phần Discussion theo đúng dữ
  liệu thay vì giữ giả thuyết sai.
- **Weight-sensitivity với dữ liệu Nhóm A+B đầy đủ**
  (`AB_components_for_wS_sensitivity.csv`): đo trực tiếp Trigger-overlap
  THẬT của Nhóm B (Kociemba, không giả định) = 0.9% (gần 0, xác nhận
  thực nghiệm). Khoảng cách HLI(A)−HLI(B) luôn dương mạnh (0.397–1.000)
  ở MỌI bộ trọng số hợp lý → kết luận cốt lõi bài báo robust.

## PHẦN 3 — Giai đoạn 3: khảo sát validation kiểu BotPrize
(`research/phase3_survey/`)

- `DESIGN.md` — protocol nghiên cứu đầy đủ: câu hỏi nghiên cứu
  (RQ-V1/V2/V3), tiêu chí tuyển người (15-20 CFOP-solver), thiết kế mù
  (blinding), đạo đức nghiên cứu, kế hoạch phân tích thống kê.
- `prepare_stimuli.py` — ĐÃ CHẠY THẬT, sinh `stimuli.json` với 18
  stimuli thật (6 Nhóm A / 6 A+ / 6 B) từ chính solver, có timeout tự
  bỏ qua scramble pathological.
- `add_human_stimulus.py` — công cụ CLI thêm bài giải người thật (Nhóm
  H) vào `stimuli.json` AN TOÀN (tự kiểm tra cú pháp Singmaster, tự gán
  ID, tự xáo trộn vị trí) — không cần sửa JSON bằng tay.
- `rating_tool.html` — công cụ khảo sát chạy offline hoàn toàn trong
  trình duyệt (không cần server): màn hình đồng ý tham gia → hỏi thông
  tin không định danh → chấm từng bài giải ẩn danh 1-5 → xuất CSV.
- `analyze_results.py` — phân tích độ đồng thuận giám khảo, so sánh
  điểm theo nhóm (RQ-V2), tương quan Spearman HLI vs điểm người (RQ-V1).
  Đã test bằng dữ liệu giả lập, chạy đúng.
- `stimuli.json` — **18/24 stimuli đã có sẵn** (Nhóm A/A+/B), CHỈ THIẾU
  6 Nhóm H (người thật) — việc duy nhất còn lại thuộc về bạn.
- `results/` — thư mục rỗng, đặt CSV người tham gia vào đây.

## PHẦN 4 — Bài báo quốc tế (`paper/`)

- `main.tex` — bản thảo LaTeX đầy đủ cấu trúc (Abstract → Conclusion +
  10 trích dẫn thật: Korf 1997, Rokicki 2014, Kociemba, McAleer 2018,
  Kulkarni 2016, Chakraborti 2019, Hingston 2009 (BotPrize), Świechowski
  & Ślęzak 2025 (AAMAS), Lakkaraju 2022 (ALLURE, AAAI), Vanacore 2025
  (preprint, đã ghi rõ trạng thái chưa bình duyệt)).
- **Đã điền số liệu thật** vào Abstract, Table 1, Figure Pareto,
  Discussion (bao gồm cả phát hiện λ↔timeout ngược giả thuyết), Threats
  to Validity, weight-sensitivity, Conclusion — không còn `\todo{}` nào
  thiếu SỐ LIỆU, chỉ còn quyết định cá nhân (tên tác giả, chọn bảng
  chính) và phần phụ thuộc Giai đoạn 3.
- **Đã sửa lỗi LaTeX**: gói `fontenc[T5]` không có sẵn trong TeXLive tối
  giản của môi trường build → đổi sang `fontenc[T1]`, build sạch 0 lỗi,
  11 trang.
- `main.pdf` — bản build sẵn, đã biên dịch kiểm chứng nhiều lần.

## PHẦN 5 — Hệ thống nhận diện case OLL + bảng công thức đầy đủ

Bổ sung sau khi hoàn tất Giai đoạn 3, giải quyết điểm yếu "OLL chỉ
2-look" đã tự phát hiện trong phần tự đánh giá đề tài:

- **`solver/oll_recognition.py`** — nhận diện case OLL dựa THUẦN TUÝ trên
  toán tổ hợp (định luật parity: tổng hướng 4 cạnh U chia hết 2, tổng
  hướng 4 góc U chia hết 3 — đã kiểm chứng bằng code), không phụ thuộc
  trí nhớ công thức. Tự chuẩn hoá AUF (nhận ra 2 case chỉ lệch vài độ
  xoay U là "cùng 1 case"). **Đã phát hiện và sửa 1 bug quan trọng**
  trong quá trình làm việc cùng người dùng: `cp`/`co` trong engine là
  piece-indexed (theo dõi từng viên cụ thể), không phải slot-indexed
  (theo vị trí hiện tại) — bug từng bị che giấu trong test tự viết ban
  đầu (vô tình rơi vào trường hợp permutation=identity), chỉ lộ ra khi
  test với 1 case OLL thật có hoán vị khác identity do người dùng cung
  cấp làm ví dụ.
- **`solver/oll_algorithms.py`** — bảng công thức OLL đầy đủ (không giới
  hạn OCLL), dùng lại cơ chế nhận diện ở trên. Sửa đúng 1 hiểu nhầm quan
  trọng: OLL KHÔNG cần giữ nguyên hoán vị (khác PLL) — công thức OLL
  thật (vd Sune) vẫn hoán vị, chỉ định hướng đúng, PLL sửa hoán vị sau.
  Key tra bảng = NGHỊCH ĐẢO (mod 2/3) của delta đọc được khi áp công
  thức lên cube đã giải. Hiện có 3 công thức (Sune, Anti-Sune, 1 case
  Dot) — **mở rộng dễ dàng**: thêm 1 dòng vào `_RAW_ALGS`, chạy lại
  `python3 -m solver.oll_algorithms` để tự kiểm chứng đúng/sai.
- **`solver/oll_solver.py`** — `solve_oll()` giờ ưu tiên tra bảng đầy đủ
  trước (giải gọn 1 bước như CFOP thật), fallback về kiến trúc 2-look cũ
  khi case chưa có trong bảng. Đã xác nhận THẬT trên pipeline đầy đủ:
  2/21 scramble thật được giải qua bảng mới (Sune/Anti-Sune, giờ tra
  bảng thay vì brute-force AUF như code cũ).

**Không cần chạy lại số liệu Experiment 1-3 trong bài báo** — logic đếm
`corner_source == 'named'` trong `cfop_ai.py` không đổi ý nghĩa, tương
thích ngược hoàn toàn (176/176 test vẫn pass sau mỗi bước sửa).

## CẬP NHẬT MỚI NHẤT 4 — Đổi bảng màu R/L theo yêu cầu (SỬA ĐỦ CẢ 2 NƠI)

Theo yêu cầu người dùng: F=Xanh lá → suy ra R=Cam, L=Đỏ (đã kiểm chứng
bằng toán: đây là phép xoay 180° hợp lệ quanh trục F-B, định thức +1,
không phải đối xứng gương bất khả thi). Bảng màu đầy đủ mới:
U=Vàng, D=Trắng, F=Xanh lá, B=Xanh dương, R=Cam, L=Đỏ.

**Lần sửa đầu chỉ sửa `move_recorder.html`** — sau đó người dùng gửi ảnh
chụp app `main.py` (3D View) cho thấy vẫn hiển thị màu cũ (R=Đỏ,
L=Cam), phát hiện lần tìm kiếm đầu bị SÓT `constants.py` (do lệnh grep
ban đầu không khớp đúng từ khóa). Đã sửa nốt:
- **`constants.py`** — `COL3D` (màu 3D) và `FACE_LC` (màu panel 6-mặt),
  đây mới là nguồn màu THẬT của app chính `main.py`.
- **`research/phase3_survey/move_recorder.html`** — đã sửa từ trước.

Không còn nơi nào khác trong dự án định nghĩa màu sticker (đã grep lại
toàn bộ `main.py`/`draw_helpers.py`/`layout.py` xác nhận không có text
gán cứng tên màu theo mặt). 176/176 test vẫn pass, đã smoke-test headless
xác nhận app vẽ đúng màu mới không lỗi.

## CẬP NHẬT MỚI NHẤT 5 — Sửa UI: danh sách phím tắt bị khuất

Phát hiện qua ảnh chụp màn hình người dùng gửi: dòng cuối ("H — AI gợi
ý bước tiếp" và dòng ký hiệu U/u D/d...) bị đè lên/khuất bởi thanh
Singmaster bar phía dưới. Nguyên nhân: khoảng trống dành cho panel này
trong `main.py` bị **hardcode cứng** (`max(60, int(123*lo.s))`), không
khớp với chiều cao nội dung thật (9 dòng phím tắt + 1 dòng ký hiệu = 10
dòng). Đã sửa: tính khoảng trống **theo đúng số dòng thật**
(`shortcuts_reserved_h`), dùng CHUNG 1 biến ở cả nơi tính giới hạn
History (`hy_limit`) lẫn nơi đặt vị trí bảng phím tắt
(`shortcuts_top_y`) và vòng lặp vẽ — tránh 3 nơi tính riêng dễ lệch
nhau về sau như bug cũ. Đã mô phỏng headless xác nhận toàn bộ layout
(History → CFOP box → Shortcuts → Bar) không còn chồng lấn, còn dư
12px trước thanh bar.

## CẬP NHẬT MỚI NHẤT 6 — Thanh Singmaster nâng cấp thành ô nhập liệu đầy đủ

Trước đây thanh Singmaster chỉ hỗ trợ gõ nối đuôi + Backspace (không có
con trỏ thật, không click chuột định vị, không copy/paste). Đã nâng
cấp toàn diện trong `main.py` + `draw_helpers.py`:

- **Con trỏ thật** (`bar_cursor`) — không còn ký tự `|` giả nối cuối
  text như trước (sai vị trí khi con trỏ không ở cuối).
- **Click chuột định vị con trỏ** — hàm mới `_bar_index_at_x()` tính
  đúng vị trí ký tự gần nhất với toạ độ x của chuột (dùng
  `font.size()` đo độ rộng từng đoạn text).
- **Kéo chuột để chọn** (`bar_dragging`) — giữ chuột kéo trong lúc gõ
  sẽ tô sáng vùng chọn, xử lý qua `MOUSEMOTION`.
- **Phím tắt chuẩn**: mũi tên trái/phải di chuyển con trỏ, Shift+mũi
  tên mở rộng vùng chọn, Home/End (+Shift) nhảy đầu/cuối, Delete xoá
  ký tự sau con trỏ (khác Backspace xoá ký tự trước).
- **Copy/Paste hệ thống thật**: Ctrl+A (chọn tất cả), Ctrl+C (copy),
  Ctrl+X (cắt), Ctrl+V (dán) — dùng `pygame.scrap` (clipboard chuẩn
  OS), có `try/except` an toàn nếu môi trường không hỗ trợ clipboard
  (không crash app).
- **`draw_bar()`** (trong `draw_helpers.py`) viết lại: nhận thêm tham
  số `cursor_pos`, `sel_range`, vẽ đúng vị trí con trỏ nhấp nháy +
  highlight vùng chọn (thay vì chỉ nối `|` vào cuối như trước).

**Đã kiểm chứng bằng cách bơm sự kiện thật** (không chỉ chạy suông):
mô phỏng click focus → gõ ký tự → Home → Shift+End (chọn hết) →
Ctrl+C → Ctrl+V → mũi tên trái → Backspace → Delete → kéo chuột chọn —
toàn bộ chuỗi chạy qua vòng lặp event thật của `main.py`, không có
exception nào. 176/176 test solver vẫn pass (không liên quan tới thay
đổi UI này).

## CẬP NHẬT MỚI NHẤT 7 — Bổ sung test suite, verify độc lập số liệu Giai đoạn 3, dọn dẹp repo

Phiên làm việc riêng, không sửa hành vi solver/UI — tập trung vào tính
tái lập, kiểm chứng độc lập, và vệ sinh gói nộp.

**1. Bổ sung test suite (trước đây thiếu hoàn toàn):**
- `test_solver.py` (mới, 113 test) — bao phủ `solver/` theo đúng thứ tự
  CFOP (edge/corner model → full_state → cross → f2l → oll → pll →
  cfop_ai) + `move_simplify`. Có kiểm tra bất biến nhóm cube (tổng
  hướng góc/cạnh bất biến), đối chiếu PDB Cross bằng tổ hợp học độc lập
  (190.080 trạng thái, max depth 8), đối chiếu 288 trạng thái PLL hợp
  lệ, round-trip tự-sinh-tự-giải cho bảng OLL (224/228 ~98.2% — CHƯA
  100%, xem `TestOLLAlgorithms::test_own_generating_set_round_trip`).
- `test_cube_engine.py` (trước đây RỖNG 0 byte trong gói cũ, nay 206
  test) — bất biến nhóm ($m^4=e$, $m \cdot m'=e$) cho cả 18 base move
  (outer/slice/rotation/wide), đối chiếu công thức wide-move tài liệu
  hoá (`u=U+E'`...), parser Singmaster (group lồng nhau, chuẩn hoá
  suffix, case-sensitivity).
- **Tổng: 319 passed, 0 failed** (`pytest test_solver.py
  test_cube_engine.py`) — con số "176 passed" nhắc ở các bản ghi cũ bên
  trên không còn đúng (bộ test cũ đã thất lạc trước phiên này, không rõ
  lý do; 319 là bộ MỚI viết lại từ đầu, không phải khôi phục bản cũ).

**2. Merge dữ liệu thô Giai đoạn 3 + verify độc lập toàn bộ số liệu paper:**
- Nhận `research.zip` chứa 28 file CSV thô (kết quả thật, 1 giám khảo/
  file) — merge vào `research/phase3_survey/results/` (giữ nguyên 4
  file cũ không có trong zip mới: `analysis_summary_exact.csv`,
  `exact_components.json`, `human_solves_log.txt`,
  `recompute_hli_exact.py`; cập nhật `move_recorder.html` theo bản đã
  sửa lỗi màu — bản cũ gán nhầm F=xanh lá, bản mới đúng quy ước
  Trắng-Vàng/Đỏ-Cam/Xanh lá-Xanh dương đối diện).
- Chạy `analyze_results.py` có sẵn trên dữ liệu thật → khớp
  **chính xác** với paper: proxy ρ=0.440 (p=0.031), trigger-overlap
  ρ=0.605 (p=0.0017).
- **Viết mới `research/phase3_survey/compute_icc.py`** — tự cài công
  thức ICC(2,1)/ICC(2,k) chuẩn (Shrout & Fleiss 1979, ANOVA 2 chiều,
  không cần `pingouin`/`krippendorff`). Kết quả: **ICC(2,1)=0,2563,
  ICC(2,k)=0,9061** — khớp chính xác 2 chữ số thập phân với paper
  (0,26 / 0,91). Đây là phép verify độc lập đầu tiên cho con số mà
  chính paper tự ghi "chưa được tính lại độc lập từ ma trận đánh giá
  tho".
- **Viết mới `research/phase3_survey/compute_exact_correlation.py`** —
  paper dùng **ρ=0,487 (p=0,016)** làm kết quả CHÍNH cho RQ-V1 (bản đã
  sửa lỗi giả định Pattern-conformity=0 cho Nhóm H trong
  `recompute_hli_exact.py`), nhưng trước đây KHÔNG có script nào nối
  `analysis_summary_exact.csv` → phép tính tương quan này (phải tự chạy
  tay). Đã verify: khớp chính xác (0,4867 ≈ 0,487).

**3. Sửa `requirements.txt`** — thiếu `scipy`, `matplotlib`, `kociemba`
(dùng thật trong `research/` nhưng không khai báo — ai làm đúng theo
`HUONG_DAN_CHAY.md` sẽ không chạy được các script này).

**4. Dọn dẹp repo:**
- Xoá `solver/oll_recognition.py.bak`, toàn bộ `__pycache__/` bị đóng
  gói nhầm trong zip nộp.
- Chuyển 4 file dữ liệu/hình đã bị thay thế vào `research/archive/`
  (kèm `README.md` giải thích): `pareto_test.csv`, `results_test.csv`
  (output smoke-test dev, không dùng phân tích), `pareto_n20.csv`,
  `pareto_figure.png` (pilot N=20, paper dùng N=60 —
  `pareto_n60.csv`/`pareto_figure_n60.png`).
- **Gộp logic scramble bị copy-paste ở 5 nơi** (`cube_engine.scramble_cube`,
  `demo_ai_search.py`, `benchmark.py`, `research/scramble_utils.py`,
  `research/run_experiment.py`) thành 1 hàm gốc duy nhất
  `cube_engine.random_scramble_moves(st, n, rng=random)` — nhận `rng`
  tuỳ chọn nên vẫn giữ đúng tính chất "độc lập với `random.seed()` toàn
  cục" mà `scramble_utils.py` cần (đã verify riêng). `scramble_cube()`
  cũ giữ nguyên API (không trả về gì) để không phá `main.py`.

**5. Compile-test cả 2 bản paper** (chưa từng làm trước đây) —
`main.tex` qua `pdflatex` (2 lần, sạch, 15 trang) và `main_vi.tex` qua
**`xelatex`** (bắt buộc, `pdflatex` báo lỗi fatal ngay vì dùng
`fontspec` — phát hiện mới, đã thêm vào `HUONG_DAN_CHAY.md` mục 7).

Toàn bộ 319 test + cả 2 script verify ICC/correlation đã chạy lại lần
cuối sau khi refactor xong — không có gì bị phá vỡ.

## CẬP NHẬT MỚI NHẤT 8 — Đọc lại prose 2 bản paper, sửa 3 chỗ số liệu lỗi thời, tăng độ phủ nhãn OLL 35→40

Đọc toàn bộ 829+867 dòng của cả `main_vi.tex`/`main.tex`, đối chiếu
từng con số với code/CSV thô. Tìm và sửa 3 vấn đề — **lặp lại giống hệt
ở cả 2 bản** (rủi ro copy-paste 2 ngôn ngữ song song):

**1. Claim "55/57 case OLL, 43 đối chiếu số cộng đồng" — sai lệch
thật.** Kiểm tra trực tiếp `PRETTY_CASE_NAME` trong
`solver/oll_algorithms.py`: trước khi sửa chỉ có **35** số OLL chính
thức duy nhất được gán nhãn đúng (37 entry, 2 trùng — OLL 26, 27),
KHÔNG PHẢI 55; 22 số OLL vắng mặt hoàn toàn, không chỉ 2. Đã tra 57
thuật toán chuẩn cộng đồng (nguồn SpeedCubeDB), lọc ra 14 công thức
**thuần** (không nước lát/rộng) cho các số còn thiếu, tự tính pattern
bằng chính engine của dự án (dùng phép "conjugate" — thêm nghịch đảo
phép xoay `y`/`y2` ở cuối chuỗi — để xử lý đúng tiền tố xoay cả khối,
tránh lỗi `cross_f2l_ok` sai do hệ quy chiếu tuyệt đối), đối chiếu với
bảng hiện có. Tìm được **5 case khớp thật** (OLL 24, 29, 36, 39, 40 →
`case_auto_01/44/30/13/16`, trước đó gắn nhãn chung chung
`OCLL`/`Angle`/`Line`) → cập nhật `PRETTY_CASE_NAME`. Kết quả: **35→40
số OLL chính thức được gán nhãn đúng**, 17 công thức còn lại (đã kiểm
chứng hợp lệ về chức năng) chưa đối chiếu được số cộng đồng cụ thể.
Paper đã sửa lại theo đúng con số 40/57 hiện tại.

**2. Claim "đã chứng minh bằng tìm kiếm hai chiều gặp giữa" cho OLL 2
và OLL 20 — không có bằng chứng.** Grep toàn repo: không có script nào
tên/nội dung liên quan "bidirectional"/"meet-in-middle". Claim này
được làm mềm lại thành đúng bản chất: "quan sát thực nghiệm từ nguồn
cộng đồng công khai (mọi thuật toán thuần được công bố cho 2 case này
đều dùng nước lát/rộng), KHÔNG PHẢI chứng minh hình thức".

**3. Đã sửa 2 vấn đề nêu ở CẬP NHẬT MỚI NHẤT 7** (chưa áp dụng vào
paper lúc đó): claim "176 unit test" → đổi thành trích dẫn cụ thể
`test_solver.py::TestF2LSolver::test_lam_zero_is_backward_compatible`
(1 trong 319 test hiện có); caveat ICC "chưa verify độc lập, cần xác
minh trước khi nộp" → đổi thành đã verify, khớp chính xác (0,2563 /
0,9061).

Đã đối chiếu 9 cặp số liệu chính giữa 2 bản VI/EN (điểm nhóm khảo sát,
ρ, số nước...) — khớp 1-1 hoàn toàn, không phát hiện lỗi lệch dịch.
Compile lại sạch cả 2 bản sau khi sửa (pdflatex 15 trang / xelatex 17
trang, 0 lỗi). Chạy lại 319 test sau khi sửa `oll_algorithms.py` —
không có gì bị phá vỡ.

## CẬP NHẬT MỚI NHẤT 9 — Fresh-clone test (giải nén sạch, chạy lại từ đầu)

Giải nén lại chính file zip sẽ giao cho người dùng vào thư mục hoàn
toàn sạch (không cache, không `__pycache__`, không `.aux`/`.log` sót
lại), chạy đúng theo từng bước `HUONG_DAN_CHAY.md` từ đầu đến cuối.

**Phát hiện 1 chỗ docs sai:** mục §3 khẳng định `solver/cache/` "rỗng,
đã xoá `.pkl`" — thực tế gói nộp **có sẵn** ~13MB cache. Đã sửa lại
đúng thực tế, đồng thời **verify luôn cả đường build-từ-đầu** (xoá hết
`.pkl`, build lại thật): Cross PDB build từ rỗng mất **~6 giây**, ra
đúng 190.080 trạng thái / max depth 8 (khớp con số đã verify trước đó
— xác nhận cache cũ và cache build-lại-từ-đầu cho kết quả giống hệt
nhau, không có sai lệch ẩn).

**Đã chạy lại toàn bộ, từ môi trường sạch:**
- 319 test (`test_solver.py` + `test_cube_engine.py`) — pass 100%
- `python3 -m solver.oll_algorithms` — in đúng `57/57`
- `demo_ai_search.py`, `weight_sensitivity.py` — chạy xong, số liệu khớp
- `analyze_results.py`, `compute_icc.py`, `recompute_hli_exact.py`,
  `compute_exact_correlation.py` — tất cả khớp chính xác với paper
- Compile lại cả `main.tex` (pdflatex) và `main_vi.tex` (xelatex) từ
  đầu — sạch, đúng 15/17 trang
- Xác nhận đúng bẫy đã cảnh báo trong docs: chạy nhầm `pdflatex
  main_vi.tex` thật sự báo lỗi fatal `fontspec` như mô tả

Không phát hiện thêm lỗi nào khác ngoài mục cache nói trên. Đây là lần
chạy thử cuối cùng mô phỏng đúng trải nghiệm người nhận gói lần đầu.

## Việc còn lại (đã cập nhật sau CẬP NHẬT MỚI NHẤT 7)

Giai đoạn 3 **đã xong** (28 CSV thật, đã phân tích, đã verify độc lập —
xem CẬP NHẬT MỚI NHẤT 7 phía trên; 3 mục cũ liệt kê ở đây trước đó
không còn đúng).

Việc còn mở, theo mức ưu tiên:
1. **Điều tra 4 case OLL round-trip thất bại** (224/228, xem
   `test_solver.py::TestOLLAlgorithms::test_own_generating_set_round_trip`)
   — chưa rõ là bug thật hay giới hạn thiết kế đã biết. (Lưu ý: KHÁC
   với việc gán nhãn số cộng đồng đã cải thiện ở CẬP NHẬT MỚI NHẤT 8 —
   đây là lỗi round-trip solve, chưa đụng tới.)
2. Xác định `human_solves_log.txt` thu thập trước hay sau bugfix màu
   trong `move_recorder.html` (xem CẬP NHẬT MỚI NHẤT 4 và mục 2 của
   CẬP NHẬT MỚI NHẤT 7) — nếu không xác định được, cần ghi chú thận
   trọng trong paper.
3. ~~Thêm `LICENSE` nếu định public source code.~~ **Đã xong** — MIT,
   xem `LICENSE` (đứng tên Pham Dang Khue theo `paper/main.tex`).
4. ~~Đọc lại toàn bộ prose 2 bản paper lần cuối trước khi nộp chính
   thức.~~ **Đã xong** — xem CẬP NHẬT MỚI NHẤT 8 (sửa 3 chỗ số liệu
   lỗi thời, tăng độ phủ nhãn OLL 35→40/57).
5. (Tuỳ chọn, không bắt buộc) Đối chiếu nốt 15 công thức OLL còn lại
   (hợp lệ về chức năng nhưng chưa gán được số cộng đồng — xem CẬP
   NHẬT MỚI NHẤT 8) nếu muốn paper nêu con số cao hơn 40/57.
6. ~~"Fresh-clone test": giải nén gói ở máy sạch, chạy đúng theo từng
   bước `HUONG_DAN_CHAY.md` từ đầu đến cuối.~~ **Đã xong** — xem CẬP
   NHẬT MỚI NHẤT 9 (1 lỗi docs tìm được và đã sửa: mục cache).

Không còn việc "code/phân tích tự động" nào bắt buộc phải làm trước khi
nộp — các mục còn lại (1, 2, 5) chủ yếu là quyết định nội dung/điều tra
sâu thêm, không còn gì chặn việc nộp bài.

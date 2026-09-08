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

## CẬP NHẬT MỚI NHẤT 10 — Sửa lỗi số liệu OLL/thống kê, thí nghiệm
capped-bonus admissibility, dọn bibliography, hạ tầng mở rộng Exp 3

**Bối cảnh:** phiên làm việc dài, đánh giá dự án như 1 đề tài NCKH rồi
đối chiếu từng con số trong paper với code/dữ liệu thô (không chỉ đọc
prose). Tìm và sửa nhiều lỗi thật, không chỉ lỗi văn phong.

**1. Lỗi trùng lặp trong bảng OLL (đã sửa code, không chỉ paper):**
- `case_auto_03` và `case_auto_05` (2/57 raw algorithm) được xác minh
  bằng thực nghiệm (áp nghịch đảo lên cube đã giải, so khớp dưới 4 phép
  AUF) là **bản sao lệch-AUF của Sune/AntiSune**, không phải case mới —
  ladder solver nội bộ hội tụ nhầm về 2 seed này khi sinh tự động.
- Đã gỡ 2 entry này khỏi `solver/oll_algorithms.py`
  (`_RAW_ALGS`/`PRETTY_CASE_NAME`). Bảng OLL giờ là **55 case** (không
  phải 57) — khớp đúng 57 case chuẩn CFOP trừ OLL 2 và OLL 20 (2 case
  không có thuật toán thuần, đã biết từ trước, fallback 2-look).
- Trong 55 case, **40 đã khớp số hiệu cộng đồng, 15 chưa khớp**. Con số
  "40" trùng ngẫu nhiên với con số cũ trong paper (vốn tính đúng theo
  số hiệu duy nhất) nhưng "57"/"17" trong paper cũ là sai — đã sửa
  paper thành "55"/"15" cho nhất quán.
- Sửa test `test_all_57_cases_present_in_raw_table` (hard-code sai
  giả định) → `test_all_55_addressable_cases_present_in_raw_table`.
- **319/319 test pass** sau thay đổi.

**2. Đối chiếu toàn bộ số liệu thống kê trong paper với dữ liệu thô
(không chỉ OLL):**
- Exp1 (N=60, N=100), Exp2 (Wilcoxon W/p), Exp3 (ICC, Spearman ρ, 24
  stimuli, 28 rater): **tất cả khớp tuyệt đối** khi chạy lại script gốc
  trong repo (`compute_icc.py`, `compute_exact_correlation.py`).
- **Tìm được 1 lỗi thật:** effect size `r` của Wilcoxon ở Exp2 sai
  (paper ghi r=0,83 và r=0,60; công thức đúng z/√n — đã xác nhận đúng
  công thức này qua việc nó tái lập chính xác r=0,87 ở Exp1 — cho ra
  r=0,75 và r=0,45). Đã sửa cả `main.tex` và `main_vi.tex`. W và p
  không sai, chỉ sai ở bước z→r (khả năng lỗi chép tay).
- Sửa câu diễn đạt "20% (12/60) exceeded timeout" — thực ra chỉ 10/12
  là timeout thật, 2 là `f2l_partial` (dừng trước mốc 20s). Đã sửa câu
  chữ ở cả 2 bản cho chính xác.

**3. Thí nghiệm capped-bonus admissibility (KHÔNG chỉ đề xuất, đã cài
đặt + chạy thật + đưa số liệu vào paper):**
- Thêm tham số `cap` vào `trigger_bonus_count()`/
  `solve_f2l_trigger_biased()` trong `research/trigger_biased_search.py`
  (mặc định `cap=None` = hành vi cũ, không đổi).
- Script mới `research/run_capped_bonus_sweep.py`, tự-kiểm-chứng khớp
  120/120 với `pareto_n60.csv` gốc trước khi tin dữ liệu mới (từng bị
  lệch 1 lần do dùng nhầm 60k thay vì đúng 40k node/depth như Exp2 —
  đã phát hiện và sửa).
- Chạy đủ N=60 × 2λ × 4 cap (1,2,4,none) = 480 case, lưu
  `research/capped_bonus_n60.csv`.
- **Kết quả chính:** cap=4 cho kết quả giống hệt uncapped (không khác
  biệt thống kê nào) — đạt cận admissibility λ·4 "miễn phí". Cap=1,2
  giảm trigger-overlap có ý nghĩa NHƯNG cũng giảm tỷ lệ giải thành công
  (53-58/60 so với 59-60/60) — phát hiện bất ngờ: bonus trigger đôi khi
  giúp search thoát bế tắc, không chỉ là thiên lệch thẩm mỹ.
- Đã viết kết quả này (kèm bảng) vào Discussion của cả `main.tex` và
  `main_vi.tex`, thay cho câu "để dành cho tương lai" cũ. Cả 2 file
  biên dịch sạch (pdflatex/xelatex), bảng không tràn lề.

**4. Dọn bibliography (cả 2 bản):**
- `humanlike2025`: sửa label `[Author(s)(2025)]` → tên tác giả thật đã
  điền sẵn nhưng label bị lệch.
- `kociemba`: năm để trống → `(n.d.)` (xác nhận qua tìm kiếm: website
  đang duy trì liên tục, không có 1 năm xuất bản cố định).
- `vanacore2025`: xác nhận qua dblp + tìm kiếm — đã **chính thức công
  bố tại IEEE ICHI 2025** (không còn "unpublished manuscript" như paper
  cũ ghi). Cập nhật thành trích dẫn proceedings đầy đủ (trang 251-260),
  sửa cả câu in-text ở Related Work.

**5. Bỏ hướng "chọn venue quốc tế" theo yêu cầu — coi là đề tài nghiên
cứu thuần túy:**
- Sửa comment đầu `main.tex` (bỏ TODO đổi documentclass theo venue, bỏ
  ghi chú double-blind).

**6. Hạ tầng mở rộng Exp 3 (đã chuẩn bị, KHÔNG áp dụng vào bản chính
thức — xem lý do rollback bên dưới):**
- **Phát hiện lỗi nghiêm trọng tiềm ẩn** trong `add_human_stimulus.py`/
  `prepare_stimuli.py` cũ: xáo trộn + đánh số lại TOÀN BỘ `display_id`
  mỗi lần chạy. Vì `analyze_results.py` tra nội dung theo `display_id`
  từ `stimuli.json` hiện tại (không lưu trong CSV), chạy lại 2 script
  này sẽ làm sai lệch âm thầm 28 rating đã thu. Đã sửa
  `add_human_stimulus.py` để chỉ APPEND, không bao giờ đụng entry cũ
  (xác nhận `rating_tool.html` đã tự xáo thứ tự hiển thị phía client
  rồi nên việc xáo ở khâu sinh file là thừa).
- Viết `expand_stimuli.py` (sinh thêm Nhóm A/A+ tự động) và
  `fill_missing_group_b.py` (điền Nhóm B — cần thư viện `kociemba`).
  Đã test an toàn trên bản sao, xác nhận 0 sai lệch với 24 entry cũ.
- **Đã thử mở rộng thật** (+6 seed → +12 stimuli A/A+, tổng 36) nhưng
  **ROLLBACK về đúng 24 stimuli gốc theo yêu cầu** vì không cài được
  `kociemba` trên máy Windows của người dùng (lỗi thiếu MSVC Build
  Tools, thử 2 hướng khắc phục không cần compiler đều vướng — pip build
  isolation vẫn cố compile `cffi`). `stimuli.json` hiện tại = đúng bản
  gốc, đã xác nhận khớp 0 sai lệch với 28 rating đã có
  (`analyze_results.py` cho lại đúng ρ=0,440, N=24 như cũ).
- 2 script `expand_stimuli.py`/`fill_missing_group_b.py` vẫn còn trong
  repo, sẵn sàng dùng khi có máy cài được `kociemba` — KHÔNG cần sửa gì
  thêm lúc đó.

**7. Vấn đề đã biết, kiểm tra lại — vẫn còn nguyên, không phải lỗi mới:**
- Mục "còn mở" #1 phía trên (round-trip OLL, cũ ghi 224/228) vẫn còn,
  giờ là **216/220 (55 case × 4 AUF, cùng tỷ lệ 98,2%)** — đã xác định
  chính xác hơn: **toàn bộ 4 case thất bại đều là `Dot_OLL1`** (trước
  đây không rõ case nào). Đã xác nhận **không ảnh hưởng khả năng giải
  thực tế** — `solve_oll()` có fallback graceful về kiến trúc 2-look cũ
  khi bảng full-OLL không khớp, không có solve nào thất bại vì lý do
  này. Vẫn là việc điều tra sâu tuỳ chọn, không chặn gì.

Không còn việc "code/phân tích tự động" nào bắt buộc phải làm trước khi
dùng bản này. Việc mở (không bắt buộc): điều tra `Dot_OLL1` (mục 7),
mở rộng Exp 3 thật khi có máy cài được `kociemba` (mục 6).

## CẬP NHẬT MỚI NHẤT 11 — Hiệu chỉnh multiple-comparison cho bảng
capped-bonus, sửa nhãn "giống hệt" sai ở 1 hàng

- Bảng capped-bonus (CẬP NHẬT 10, mục 3) có 10 kiểm định Wilcoxon (2λ ×
  3 cap × 2 chỉ số, bỏ 2 ô phương sai=0) chạy trên cùng bộ dữ liệu mà
  chưa hiệu chỉnh — đúng kiểu "gia đình kiểm định" cần Holm-Bonferroni.
  Đã tính lại: **5/10 vẫn có ý nghĩa sau hiệu chỉnh** (đánh dấu $^\dagger$
  trong bảng) — toàn bộ khác biệt trigger-overlap đứng vững, nhưng phần
  lớn khác biệt số nước (trừ λ=1,0/cap=1) không còn ý nghĩa. Kết luận
  chính của thí nghiệm KHÔNG đổi (cap chặt tốn trigger-overlap thật
  nhưng không chắc tiết kiệm được nước đi) — hiệu chỉnh làm câu chuyện
  chặt chẽ hơn, không đảo ngược.
- Phát hiện thêm khi soát lại: nhãn "giống hệt" (identical) ở hàng
  λ=1,0/cap=4 trong bảng cũ là **sai** — kiểm tra từng seed cho thấy
  2-3/59 seed THỰC SỰ khác nhau (không phải 0 như λ=0,5/cap=4), chỉ là
  chênh lệch không có ý nghĩa thống kê (p=0,18 và p=1,0). Đã sửa bảng
  để ghi rõ p-value thật + số seed khác nhau thay vì gộp chung là
  "giống hệt" — chỉ hàng λ=0,5/cap=4 mới thực sự là 0/59 khác biệt.
- Đã sửa cả `main.tex` và `main_vi.tex`, biên dịch sạch cả 2
  (pdflatex/xelatex), không overfull mới. 319/319 test vẫn pass (không
  đụng code solver, chỉ đụng bảng/prose trong paper).

## CẬP NHẬT MỚI NHẤT 12 — Rà phụ lục, phát hiện thiếu trích dẫn nguồn
cho corpus finger-tricks

- Đối chiếu Phụ lục "Seed corpus T (|T|=11)" với source thật
  (`research/finger_tricks.py`): **11 trigger khớp 100%**, không lỗi
  số liệu.
- Nhưng chính docstring của file này ghi rõ khuyến nghị "khi viết báo
  cáo/luận văn, NÊN trích nguồn tổng quan (vd. J Perm 'Finger tricks'
  video series, speedsolving.com Wiki) thay vì coi đây là đóng góp gốc
  của bạn" — paper trước đó KHÔNG có trích dẫn nào cho thuật ngữ
  finger-tricks/tên các trigger (sexy move, sledgehammer...), dù đây
  không phải đóng góp gốc của dự án.
- Tìm được nguồn chính xác qua tìm kiếm: `jperm.net/3x3/fingertricks`
  (trang "Finger Tricks" chính thức của J Perm/Dylan Wang). Đã thêm
  bibitem `jperm-fingertricks` và trích dẫn tại 2 chỗ: nơi giới thiệu
  thuật ngữ trong Method, và đầu Phụ lục — ở cả `main.tex` và
  `main_vi.tex`.
- Biên dịch 2 lần mỗi bản (cần pass 2 để resolve citation mới) — sạch,
  không còn "undefined citation" warning nào. 319/319 test pass.

## CẬP NHẬT MỚI NHẤT 13 — Rà Related Work, sửa 1 lỗi gán nhầm tên
phương pháp cho sai citation

- Kiểm tra từng claim gán cho các paper trích dẫn (không chỉ tên/năm
  như trước, mà cả nội dung mô tả) — tìm được: câu văn gọi phương pháp
  của **McAleer et al. 2018** là **"DeepCubeA"** và nói nó "đạt độ dài
  lời giải gần tối ưu" — cả hai đều SAI. Xác minh qua tìm kiếm: paper
  2018 (arXiv:1805.07470) giới thiệu thuật toán tên
  **"Autodidactic Iteration"**, độ dài lời giải trung vị **30 nước**
  (cao hơn khá nhiều so với tối ưu 20 nước) — không phải "gần tối ưu".
  "DeepCubeA" là tên của **paper khác, năm 2019** (Agostinelli, McAleer,
  Shmakov, Baldi — Nature Machine Intelligence, đúng là near-optimal:
  giải 100% test, tìm được đường ngắn nhất 60,3% số lần).
- Sửa: thêm bibitem `agostinelli2019` (đúng paper DeepCubeA 2019), viết
  lại câu văn thành 2 câu tách bạch — 1 câu đúng cho McAleer 2018
  (Autodidactic Iteration, ~30 nước), 1 câu đúng cho Agostinelli 2019
  (DeepCubeA, near-optimal) — ở cả `main.tex` và `main_vi.tex`.
- Biên dịch sạch cả 2 (2-3 pass để resolve citation mới), 319/319 test
  pass (không đụng code, chỉ prose+bib).

## CẬP NHẬT MỚI NHẤT 14 — Điều tra sâu Dot_OLL1: phát hiện lớn hơn dự
kiến, QUYẾT ĐỊNH KHÔNG SỬA CODE trong phiên này

**Bối cảnh:** mục "còn mở" cũ (round-trip OLL, hiện 216/220, riêng
`Dot_OLL1` fail cả 4 AUF) — điều tra để hiểu nguyên nhân gốc.

**Phát hiện (quan trọng, thay đổi hiểu biết về scope vấn đề):**
1. Test round-trip cũ (`if solve_oll_with_auf(full) is not None`) chỉ
   kiểm tra "khớp được VỚI BẤT KỲ entry nào trong bảng", KHÔNG kiểm tra
   "khớp lại đúng CHÍNH NÓ" — yếu hơn giả định ban đầu đáng kể.
2. Đào theo hướng "vì sao piece-indexed (dùng trong `_delta_of`/
   `_table_key_for`) lại khác slot-indexed thật (dùng trong
   `_slot_indexed`, dùng lúc tra cứu)": phát hiện giả định "Lesson 2"
   trong docstring đầu `oll_algorithms.py` ("piece-indexed đọc trực
   tiếp = slot-indexed thật, vì bắt đầu từ identity") **chỉ đúng khi
   permutation là identity** — mà thực tế đo được **52/55 case có
   piece-indexed KHÁC slot-indexed thật**, không phải hiện tượng riêng
   của `Dot_OLL1` như tưởng ban đầu. Đây là phát hiện về CHÍNH PHƯƠNG
   PHÁP xây toàn bộ bảng, không phải lỗi cục bộ 1 case.
3. **Kiểm chứng thực nghiệm để đánh giá mức độ nghiêm trọng thực sự**:
   chạy pipeline đầy đủ Cross→F2L→OLL trên 40 scramble ngẫu nhiên thật
   (không phải seq áp lên cube đã giải như round-trip test) — **38/38
   ca hoàn thành F2L đều được OLL giải ĐÚNG, 0 lỗi thật, 0 case OLL
   không tìm được thuật toán**. Kết hợp với 319 test hiện có và hàng
   trăm scramble thật ở Exp1-3 (chưa từng báo lỗi liên quan OLL), có
   bằng chứng thực nghiệm mạnh rằng **bảng OLL vẫn đúng trong thực tế
   sử dụng**, dù công thức xây bảng "không sạch" về lý thuyết nhóm.

**Quyết định (có chủ đích, không phải bỏ sót):** KHÔNG sửa
`_delta_of`/`_table_key_for` trong phiên này. Lý do: đây là vấn đề sâu
chạm đến cách xây TOÀN BỘ 55 entry (không phải 1 case), sửa vội có rủi
ro thật (VD gây trùng key mới giữa các case khi đổi công thức) mà
không đủ thời gian kiểm chứng lại toàn diện (rebuild + so sánh key cũ/
mới cho cả 55 case + rerun toàn bộ 319 test + đối chiếu lại Exp1-3) —
việc này nên dành 1 phiên riêng, tập trung hoàn toàn vào việc này.
Bằng chứng thực nghiệm hiện tại đủ mạnh để yên tâm DÙNG bảng hiện tại,
chỉ là chưa nên ĐỘNG vào công thức xây bảng ngay bây giờ.

**Nếu muốn theo đuổi tiếp trong 1 phiên riêng:** hướng sửa đúng là đổi
`_delta_of()` để tính qua `_slot_indexed(ep,eo)`/`_slot_indexed(cp,co)`
thay vì đọc thẳng `eo[0:4]`/`co[0:4]`, rebuild bảng, kiểm tra không có
2 case nào đụng key nhau sau khi đổi công thức, rồi rerun toàn bộ test
+ đối chiếu Exp1-3 trước khi tin kết quả mới.

## CẬP NHẬT MỚI NHẤT 15 — Tăng khả năng tái lập: sửa
`weight_sensitivity.py` không khớp với chính phân tích nó mô tả

- Paper (đoạn "Composite HLI") báo cáo 1 sensitivity analysis cụ thể:
  khoảng cách HLI Nhóm A-B dao động [0,397 ; 1,000] tùy trọng số, gap
  tại trọng số hiện tại (0,3/0,4/0,3) = 0,653. Kiểm tra thấy
  `research/weight_sensitivity.py` **không thể tái lập được chính con
  số này** — script cũ chỉ đọc component 1 nhóm (Nhóm A từ
  `results_n100.csv`, cột `A_segmentability`...) và báo cáo biến thiên
  HLI nội bộ 1 nhóm, khác hoàn toàn phép tính GAP giữa 2 nhóm mà paper
  thực sự dùng.
- Tìm ra file đúng đã được chuẩn bị riêng cho phân tích này
  (`research/AB_components_for_wS_sensitivity.csv`, N=78, cột
  `A_S/A_P/A_O/B_S/B_P/B_O`) nhưng chưa có script nào dùng đến.
- Viết lại `weight_sensitivity.py` để đọc đúng file này, tính GAP =
  mean(HLI_A) - mean(HLI_B) qua 5 bộ trọng số đặt tên + sweep wS, và
  **tự đối chiếu kết quả với số trong paper** (in ra "KHOP"/"KHONG
  KHOP"). Chạy thử: cho đúng range [0,397 ; 1,000] và gap=0,653 —
  khớp tuyệt đối.
- Thêm 1 câu trong `main.tex`/`main_vi.tex` trỏ đến lệnh chạy
  (`python3 -m research.weight_sensitivity`) ngay tại đoạn báo cáo kết
  quả, để người đọc/reviewer có thể tái lập ngay lập tức. Biên dịch
  sạch cả 2 bản, 319/319 test pass (không đụng code solver).

## CẬP NHẬT MỚI NHẤT 16 — Hạ tầng tái lập toàn bộ paper (`REPRODUCE.md`)

**Phát hiện khi kiểm kê:**
1. `results_randomstate_n60.csv` và `results_n100.csv` (dữ liệu thô
   Experiment 1) **KHÔNG có script sinh** trong repo — tìm ra
   `research/run_experiment.py` (đã có sẵn nhưng chưa từng dùng đến
   trong phiên này). Test regenerate thử: chỉ ~50% seed khớp lại với
   code hiện tại (solver đã tinh chỉnh nhiều lần từ lúc thu dữ liệu) —
   đây là giới hạn THẬT, đã ghi nhận công khai trong `REPRODUCE.md`
   (Tầng 3), không che giấu.
2. **Không có script phân tích thống kê nào được lưu** cho Exp1, Exp2,
   capped-bonus — toàn bộ Wilcoxon/Holm-Bonferroni trước đó chỉ chạy
   tay (`python3 -c`) trong hội thoại, chưa từng persist thành file.
   Nếu không sửa, không ai tái lập được chính các con số thống kê
   trong paper dù có đủ dữ liệu thô.

**Đã làm:**
- Viết 3 script mới: `research/analyze_experiment1.py`,
  `analyze_experiment2.py`, `analyze_capped_bonus.py` — mỗi script đọc
  đúng CSV có sẵn, tính lại toàn bộ thống kê, và **tự đối chiếu với
  con số trong paper** (in "KHỚP"/"LỆCH" cho từng mục, giống pattern
  đã dùng ở `weight_sensitivity.py`). Cả 3 chạy thử đều báo "TẤT CẢ
  KHỚP VỚI PAPER".
- Trong lúc viết `analyze_experiment2.py`, phát hiện và sửa 1 lỗi nhỏ
  trong chính kiểm tra: con số "+7.4%" trong paper tính trên TRUNG
  BÌNH TOÀN NHÓM mỗi λ riêng biệt (n=51 và n=60), không phải trên tập
  con đã ghép cặp (n=51) — cách làm này hợp lệ (mô tả từng nhóm đúng
  cỡ mẫu của nó, chỉ ghép cặp khi cần kiểm định Wilcoxon), chỉ là cần
  sửa lại cách script tự-kiểm-chứng cho khớp đúng phương pháp gốc.
- Viết `REPRODUCE.md` ở gốc repo — bản đồ đầy đủ mọi con số/bảng trong
  paper → đúng lệnh tái lập, phân 3 Tầng rõ ràng (chính xác tuyệt đối
  / cần môi trường đặc biệt / không đảm bảo khớp lại — công khai lý
  do). Đây là điểm reviewer "artifact evaluation" thường tìm kiếm.
- 319/319 test pass, không đụng code solver (chỉ thêm file mới trong
  `research/` + `REPRODUCE.md`, sửa 1 câu trong 2 file `.tex`).

## CẬP NHẬT MỚI NHẤT 17 — Bootstrap CI cho các ước lượng chính (tăng
độ sâu thống kê)

- Paper trước đó chỉ báo cáo p-value/effect size, chưa có khoảng tin
  cậy (CI) cho bản thân độ lớn ước lượng — 1 khoảng chuẩn thống kê
  hiện đại thường yêu cầu đủ cả 2 (p-value nói về ý nghĩa, CI nói về
  độ lớn + độ chính xác).
- Viết `research/bootstrap_ci.py` (BCa, 9999 lần lấy mẫu lại,
  `scipy.stats.bootstrap`, seed cố định để tự nó tái lập được) —
  tính CI 95% cho: chênh lệch số nước Exp1 (A-B), HLI Nhóm A, chênh
  lệch trigger-overlap và số nước Exp2 (λ=1.0 so λ=0), Spearman ρ
  Exp3.
- **Kết quả đáng chú ý nhất**: CI của ρ Exp3 **rất rộng**
  [0,19 ; 0,84] — phản ánh đúng, định lượng hóa giới hạn N=24 (pilot
  scale) đã được disclose từ trước trong Threats to Validity, không
  mâu thuẫn gì, chỉ làm nó cụ thể hơn thay vì chỉ nói suông "cỡ mẫu
  nhỏ". CI của Exp2 (số nước) cũng khá rộng so với ước lượng điểm
  [0,59 ; 2,86], trong khi CI Exp1 và CI trigger-overlap Exp2 tương
  đối hẹp/chắc chắn.
- Đưa toàn bộ CI này vào `main.tex`/`main_vi.tex` ngay tại vị trí
  từng con số liên quan (không tạo mục riêng, giữ gần ngữ cảnh). Thêm
  1 câu diễn giải trung thực cho CI rộng của Exp3, nối lại đúng với
  giới hạn N=24 đã nêu ở Threats to Validity — cần thêm `\label{sec:threats}`
  (chưa có từ trước) để trỏ tới được, đã thêm ở cả 2 bản.
- Biên dịch sạch cả 2 (3 pass để resolve label + CI mới), không
  overfull mới, 319/319 test pass.

## CẬP NHẬT MỚI NHẤT 18 — Adversarial sanity check cho HLI: tìm ra lỗ
hổng thật của Trigger-overlap, đã disclose có trách nhiệm

- Quan sát: `segmentability()` chỉ là cờ nhị phân ("có giải xong qua
  pipeline CFOP không"), không phạt số nước dư. Đặt câu hỏi: liệu
  Trigger-overlap có thể bị "chơi xấu" (gamed) bằng cách chèn nước
  thừa không tiến triển gì hay không?
- Xác nhận bằng tính toán: `sexy_move + sexy_move_inv`
  (`R U R' U'` rồi `U R U' R'`) triệt tiêu hoàn toàn về identity (cube
  không đổi trạng thái) — hợp lệ để chèn vào bất kỳ đâu trong 1 lời
  giải mà không phá nó.
- Đo thực nghiệm trên 1 lời giải thật (63 nước, sinh từ chính
  Trigger-Biased Search): chèn 10 cặp no-op này (thêm 80 nước vô
  nghĩa) làm Trigger-overlap tăng từ **0,333 lên 0,706** — hơn gấp
  đôi, chỉ nhờ padding không tiến triển gì. Đây là lỗ hổng THẬT của
  công thức đo hiện tại.
- **Kiểm tra quan trọng để không hoảng loạn quá mức**: xác nhận
  Trigger-Biased Search (tạo ra Nhóm A/A+ đã báo cáo) dùng A* tối
  thiểu hóa `g(n)` = số nước THẬT, cộng bonus heuristic **bị chặn** ở
  `λc` — không có động cơ chèn padding (chỉ tốn thêm cost, không lợi
  gì thêm sau khi đã cap). => Lỗ hổng có thật ở METRIC, nhưng KHÔNG
  ảnh hưởng đến TÍNH TOÀN VẸN của kết quả đã báo cáo (không có cơ chế
  nào trong pipeline thực tế tạo ra padding).
- Viết đầy đủ finding này (kèm cả 2 mặt: lỗ hổng thật + lý do không
  ảnh hưởng kết quả đã có) vào mục Construct validity trong Threats to
  Validity, cả `main.tex` và `main_vi.tex`. Đề xuất hướng khắc phục
  cho công việc tương lai (thêm số hạng phạt hiệu quả/số nước vào
  Trigger-overlap nếu dùng HLI làm mục tiêu tối ưu trực tiếp).
- Biên dịch sạch cả 2, không overfull mới, 319/319 test pass.

## CẬP NHẬT MỚI NHẤT 19 — ĐÍNH CHÍNH mục 14: `Dot_OLL1` KHÔNG PHẢI lỗi
thật, bảng OLL đúng 100% — kết luận trước đó của chính mình bị sai

**Bối cảnh:** quay lại điều tra `Dot_OLL1` sau khi tạm dừng ở mục 14
(lúc đó kết luận "vấn đề sâu hơn dự kiến, 52/55 case, cố tình chưa
sửa"). Lần này làm đến cùng, và phát hiện **kết luận ở mục 14 chính nó
mới là sai** — không phải do thận trọng thừa, mà do so sánh nhầm đối
tượng cần so sánh.

**Sai lầm ở mục 14 (đính chính):** lúc đó so sánh "piece-indexed đọc
từ áp `seq` THEO CHIỀU THUẬN lên cube đã giải" với "slot-indexed của
CHÍNH trạng thái đó" — nhưng đây không phải là thứ cần so sánh để biết
bảng có đúng không! Theo đúng lý thuyết nhóm, key của bảng cần đại
diện cho trạng thái mà `seq` **được thiết kế để giải** — tức là
`nghịch_đảo(seq)` áp lên cube đã giải, KHÔNG PHẢI `seq` áp thuận.

**Kiểm tra lại đúng (ground-truth):** với cả 55 case, tính
slot-indexed của trạng thái `nghịch_đảo(seq)` áp lên cube đã giải
(= trạng thái THẬT SỰ mà `seq` giải được), so với key hiện đang lưu
trong bảng → **55/55 khớp tuyệt đối**. Công thức xây bảng
(`_delta_of`/`_table_key_for`, dùng "Lesson 2" trong docstring) **luôn
luôn đúng về mặt toán học** — không có case nào sai, kể cả `Dot_OLL1`.

**Vậy vì sao test round-trip cũ báo lỗi?** Vì chính bài test đó kiểm
tra sai đối tượng — nó áp công thức THEO CHIỀU THUẬN rồi hỏi "trạng
thái kết quả có tình cờ khớp được với bảng không" (một câu hỏi không
thật sự có ý nghĩa để đánh giá tính đúng đắn), thay vì "công thức này
có giải ĐÚNG trạng thái mà nó được thiết kế để giải hay không".

**Đã sửa:**
- Viết lại `test_own_generating_set_round_trip` trong `test_solver.py`
  dùng đúng trạng thái nghịch đảo (ground-truth), và kiểm tra MẠNH hơn
  hẳn bản cũ: không chỉ "tìm thấy 1 mục khớp trong bảng" mà "áp dụng
  lời giải trả về THẬT SỰ giải đúng OLL" (`oll_solved()` sau khi áp
  dụng). Kết quả: **220/220 = 100%**, siết ngưỡng từ `>= 0.90` (che
  giấu vấn đề tiềm ẩn) thành `== 220` (đòi hỏi hoàn hảo tuyệt đối,
  đúng vì giờ đã xác nhận đạt được).
- Sửa `HUONG_DAN_CHAY.md` (còn ghi "98,2%, chưa rõ 4 case còn lại là
  bug hay giới hạn thiết kế") và `REPRODUCE.md` (còn liệt kê đây là 1
  hạn chế đã biết) — cả 2 giờ phản ánh đúng: đã giải quyết dứt điểm,
  không còn hạn chế nào ở đây.
- 319/319 test pass (bao gồm bản test mới, nghiêm ngặt hơn).

**Bài học rút ra (đáng ghi lại cho phiên sau):** quyết định "không sửa
vội, cần điều tra kỹ hơn" ở mục 14 là ĐÚNG về mặt quy trình (không
rush 1 fix khi chưa chắc chắn) — nhưng bản thân KẾT LUẬN đưa ra lúc đó
(dựa trên phép so sánh sai) lại sai. Điều này cho thấy: ngay cả khi
thận trọng và minh bạch, vẫn cần tự nghi ngờ lại chính kết luận của
mình trước khi coi nó là sự thật cuối cùng — "thận trọng" và "đúng"
là 2 việc khác nhau, không suy ra lẫn nhau.

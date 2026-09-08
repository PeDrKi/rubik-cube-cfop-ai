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

## CẬP NHẬT MỚI NHẤT 4 — Đổi bảng màu move_recorder.html theo yêu cầu

Theo yêu cầu người dùng: F=Xanh lá → suy ra R=Cam, L=Đỏ (đã kiểm chứng
bằng toán: đây là phép xoay 180° hợp lệ quanh trục F-B, định thức +1,
không phải đối xứng gương bất khả thi). Bảng màu đầy đủ mới:
U=Vàng, D=Trắng, F=Xanh lá, B=Xanh dương, R=Cam, L=Đỏ.

Chỉ sửa `research/phase3_survey/move_recorder.html` (`FACE_COLOR` +
chú thích hướng dẫn) — đây là nơi DUY NHẤT trong dự án có định nghĩa
màu sticker cụ thể (app `main.py` chính chỉ dùng ký tự U/D/L/R/F/B,
không render màu). Logic xoay cube (JS engine) không đổi, đã test lại
khớp Python và không phá vỡ gì (176/176 test vẫn pass).

## Venue dự kiến — xem chi tiết mục 9 trong `HUONG_DAN_CHAY.md`

Hầu hết deadline 2026 (CoG, ICAPS/HAXP) đã qua tại thời điểm dự án thực
hiện. Khuyến nghị nhắm chu kỳ 2027, ưu tiên workshop **HAXP @ ICAPS
2027** (đổi tên từ XAIP, đúng cộng đồng học thuật được trích dẫn nhiều
nhất).

## Việc còn lại — chỉ còn phụ thuộc Giai đoạn 3

1. Thêm 6 bài giải người thật vào `stimuli.json` (dùng
   `add_human_stimulus.py`).
2. Tuyển 15-20 người biết CFOP, gửi khảo sát, thu thập CSV.
3. Chạy `analyze_results.py`, gửi lại `analysis_summary.csv` để điền
   nốt phần Results Giai đoạn 3 vào bài báo.

Không còn việc "code/phân tích tự động" nào khác có thể làm tiếp mà
không cần dữ liệu con người thật.

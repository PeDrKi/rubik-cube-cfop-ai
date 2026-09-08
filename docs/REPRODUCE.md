# Hướng dẫn tái lập paper/main.tex và paper/main_vi.tex

Tài liệu này liệt kê **mọi con số/bảng/hình quan trọng** trong paper và
lệnh chính xác để tái lập nó. Mỗi mục ghi rõ Tầng (Tier) để không ai
hiểu nhầm mức độ đảm bảo:

- **Tầng 1 (chính xác tuyệt đối)**: script đọc dữ liệu THÔ đã có sẵn
  trong repo và tính lại thống kê/số liệu — đã tự-kiểm-chứng khớp
  100% với paper. Chạy lại sẽ luôn ra đúng số trong paper.
- **Tầng 2 (tái lập được nhưng cần môi trường đặc biệt)**: script sinh
  dữ liệu THÔ từ đầu (cần thư viện `kociemba`, xem README chính) —
  đã tự-kiểm-chứng khớp 100% khi có đủ môi trường.
- **Tầng 3 (KHÔNG đảm bảo khớp lại — công khai, không phải lỗi)**: 2
  file CSV của Experiment 1 (`results_randomstate_n60.csv`,
  `results_n100.csv`) là **snapshot lịch sử** thu thập ở một phiên bản
  code trước đó. Solver đã được tinh chỉnh nhiều lần kể từ đó (xem
  CHANGELOG_SESSION.md) — kiểm chứng thực tế: chạy lại
  `run_experiment.py` với đúng seed hiện cho kết quả khớp khoảng
  50% số seed, không phải 100%. Đây là thực hành bình thường trong
  nghiên cứu thực nghiệm lặp lại (tương tự cách paper đã công khai xử
  lý khác biệt OLL 2-look lúc thu dữ liệu vs bảng 55-case hiện tại) —
  **phần THỐNG KÊ tính từ 2 file CSV này (Tầng 1) vẫn tái lập chính
  xác**, chỉ riêng việc SINH LẠI file CSV từ đầu là không đảm bảo.

## 0. Bắt buộc trước khi chạy bất kỳ lệnh nào

```
cd RUBIK        # thư mục gốc, nơi có cube_engine.py
pip install -r requirements.txt
```

Một số lệnh (đánh dấu 🔒) cần thêm `kociemba` — xem hướng dẫn cài đặt
không cần compiler trong lịch sử hội thoại hoặc cài
`Microsoft C++ Build Tools` rồi `pip install kociemba`.

## 1. Test suite (319 test)

```
pytest -q
```
Kỳ vọng: `319 passed`.

## 2. Bảng 1 — Experiment 1 (baseline human-likeness gap)

**Tầng 1** (thống kê từ CSV có sẵn):
```
python3 -m research.analyze_experiment1
```
Tái lập: mean moves A=66.3/B=20.6, mean HLI A=0.658, N=48 cặp,
Wilcoxon W=0, p=1.61e-9, r=0.87, 20% (12/60) không hoàn thành.

**Tầng 3** (sinh lại CSV từ đầu — không đảm bảo khớp, cần 🔒):
```
python3 -m research.run_experiment --n 60 --scramble-mode random-state \
    --base-seed 5000 --out research/results_randomstate_n60.csv
python3 -m research.run_experiment --n 100 --scramble-mode random-move \
    --base-seed 1000 --out research/results_n100.csv
```

## 3. Bảng/Hình Pareto — Experiment 2 (Trigger-Biased Search trade-off)

**Tầng 1**:
```
python3 -m research.analyze_experiment2
```
Tái lập: trigger-overlap 0.090→0.255 (λ 0→1.0), +7.4% moves, Wilcoxon
trigger-overlap (W=29.0, p=8.25e-8, r=0.75), moves (W=209.5, p=0.0014,
r=0.45).

**Tầng 2** (sinh lại từ đầu, cần 🔒, đã tự-kiểm-chứng khớp 100%):
```
python3 -m research.run_pareto_sweep --n 60 --lambdas 0.0 0.5 1.0 \
    --nodes-per-depth 40000 --base-seed 2000 --out research/pareto_n60.csv
```

## 4. Bảng 2 — Capped-bonus admissibility

**Tầng 1**:
```
python3 -m research.analyze_capped_bonus
```
Tái lập: toàn bộ Bảng 2, gồm hiệu chỉnh Holm-Bonferroni (5/10 kiểm
định còn ý nghĩa sau hiệu chỉnh).

**Tầng 2** (sinh lại từ đầu — KHÔNG cần 🔒, không dùng Kociemba, chạy
lâu ~15-20 phút, nên chia lô như đã làm trong CHANGELOG mục 10):
```
python3 -m research.run_capped_bonus_sweep --n 60 \
    --lambdas 0.5 1.0 --caps 1 2 4 none --nodes-per-depth 40000 \
    --base-seed 2000 --out research/capped_bonus_n60.csv
```

## 5. Experiment 3 — ICC, Spearman ρ (human-judge validation)

**Tầng 1** (dữ liệu 24 stimuli + 28 rater đã thu, cố định vĩnh viễn —
xem CHANGELOG mục 6 về lý do KHÔNG được sinh lại `stimuli.json`):
```
python3 -m research.phase3_survey.compute_icc
python3 -m research.phase3_survey.compute_exact_correlation
python3 -m research.phase3_survey.analyze_results
```
Tái lập: ICC(2,1)=0.26, ICC(2,k)=0.91, ρ=0.440 (proxy) / 0.487
(exact), N=24.

## 6. Sensitivity analysis — trọng số HLI (w_S, w_P, w_O)

**Tầng 1**:
```
python3 -m research.weight_sensitivity
```
Tái lập: khoảng cách HLI A-B dao động [0.397, 1.000] tùy trọng số,
gap=0.653 tại trọng số hiện dùng (0.3/0.4/0.3).

## 7. Số liệu OLL/PLL (55 case, 40 khớp số hiệu, 21/21 PLL)

```
python3 -c "
from solver.oll_algorithms import PRETTY_CASE_NAME, VERIFIED_ALG_NAMES
from solver.pll_algorithms import VERIFIED_ALG_NAMES as PLL_V
matched = sum(1 for v in PRETTY_CASE_NAME.values() if v.startswith('OLL'))
print('OLL total:', len(PRETTY_CASE_NAME), '| matched:', matched, '| unmatched:', len(PRETTY_CASE_NAME)-matched)
print('PLL verified:', len(PLL_V))
"
```
Kỳ vọng: `OLL total: 55 | matched: 40 | unmatched: 15`, `PLL verified: 21`.

## Đã biết chưa tái lập 100% từ code hiện tại — công khai, không giấu

- `results_randomstate_n60.csv`, `results_n100.csv` (xem Tầng 3 ở trên).

(Mục "Dot_OLL1 round-trip 216/220" từng liệt kê ở đây đã được điều
tra lại và giải quyết dứt điểm — xem CHANGELOG mục 19: đây là lỗi ở
chính bài test tự viết, không phải lỗi trong bảng OLL. Bảng OLL đã
xác nhận đúng 100% bằng ground-truth theo lý thuyết nhóm.)

Mọi mục "Tầng 1"/"Tầng 2" ở trên đã chạy thử và xác nhận khớp 100%
với paper tại thời điểm viết tài liệu này (CHANGELOG_SESSION.md mục
15). Nếu một script báo "KHÔNG KHỚP" khi bạn tự chạy, đó là tín hiệu
đáng tin cậy để điều tra — không phải false positive.

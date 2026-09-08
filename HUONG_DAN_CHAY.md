# HƯỚNG DẪN CHẠY DỰ ÁN

File này gộp toàn bộ lưu ý setup, để bạn không cần lục lại lịch sử trò
chuyện mỗi khi cần chạy lại project.

## 1. Yêu cầu môi trường

- Python 3.10+ (đã test trên 3.12).
- Bất kỳ hệ điều hành nào — riêng `main.py` (giao diện pygame) cần màn
  hình hiển thị được; các script `research/` không cần GUI.

## 2. Cài đặt thư viện

```bash
pip install numpy pytest          # bắt buộc cho solver/test cốt lõi
pip install pygame                 # bắt buộc nếu chạy main.py
pip install kociemba matplotlib scipy   # bắt buộc cho research/
```

Nếu dùng Linux gặp lỗi `externally-managed-environment`, thêm cờ
`--break-system-packages`, hoặc dùng virtualenv:
```bash
python3 -m venv venv && source venv/bin/activate
pip install numpy pytest pygame kociemba matplotlib scipy
```

## 3. Chạy lần đầu — build cache Pattern Database (PDB)

`solver/cache/` trong gói này **rỗng** (đã xoá `.pkl` để gói nhẹ). Lần
chạy đầu tiên sẽ tự build lại cache (vài giây đến vài phút, chỉ 1 lần).

## 4. Chạy giao diện chính

```bash
python3 main.py
```
Phím tắt: `H` = gợi ý bước tiếp theo, `/` hoặc `T` = mở thanh Singmaster.
Nút **"Copy → bar"** cạnh dòng gợi ý chép công thức gợi ý vào thanh nhập.

## 5. Chạy test suite

```bash
python3 -m pytest test_solver.py test_cube_engine.py -q
```
Kết quả mong đợi: `176 passed`.

## 5b. Mở rộng bảng công thức OLL đầy đủ

`solver/oll_algorithms.py` hiện chỉ có 3 công thức (Sune, Anti-Sune, 1
case Dot). Muốn thêm công thức mới (chỉ chấp nhận nước thuần
R/L/U/D/F/B, không lát/rộng):

```python
# mo file solver/oll_algorithms.py, them 1 dong vao dict _RAW_ALGS:
_RAW_ALGS['TenCaseMoi'] = "chuoi nuoc di ban tim duoc"
```
Chạy `python3 -m solver.oll_algorithms` để tự kiểm chứng — công thức
sai sẽ tự động bị loại (in ra `REJECTED_ALG_NAMES`), không cần lo làm
hỏng bảng hiện có.

Muốn xem case OLL hiện tại là dạng nào (Dot/Line/Angle/AllOriented) để
biết cần tìm công thức gì:
```python
from solver.oll_recognition import identify_oll_case
r = identify_oll_case(full)   # full = solver.full_state.from_facelets(state)
print(r['shape'], r['normalized_key'])
```

## 6. Chạy các script nghiên cứu (`research/`)

Luôn chạy từ **thư mục gốc project** bằng `python3 -m`:

```bash
# Demo nhanh: xem AI search Cross/F2L hoạt động ra sao
python3 demo_ai_search.py

# Experiment 1: so sánh AI-CFOP (Nhóm A) vs Kociemba (Nhóm B)
python3 -m research.run_experiment --n 100 --f2l-nodes-per-depth 40000 \
    --timeout-s 20 --out research/results_n100.csv
#   Thêm --scramble-mode random-state để dùng scramble chuẩn WCA
#   (mặc định random-move; cả 2 đã kiểm chứng cho kết quả tương đương)

# Experiment 2: quét lambda (Trigger-Biased Search)
python3 -m research.run_pareto_sweep --n 60 --lambdas 0.0 0.5 1.0 \
    --nodes-per-depth 40000 --out research/pareto_n60.csv
#   CANH BAO: lambda >= 2 co the mat 15-60+ giay/ca

# Vẽ biểu đồ Pareto
python3 -m research.plot_pareto --csv research/pareto_n60.csv \
    --out research/pareto_figure_n60.png

# Weight-sensitivity analysis cho HLI (dùng lại CSV Experiment 1, không cần chạy lại solver)
python3 -m research.weight_sensitivity --csv research/results_n100.csv
```

**Lưu ý về timeout:** một số scramble hiếm gặp khiến F2L search "treo"
(pathological, đã phát hiện thực tế, vd seed=1013). `run_experiment.py`
có cơ chế timeout thật (`--timeout-s`, mặc định 25s) để tự động bỏ qua
các case này thay vì treo cả batch — đây là hành vi bình thường, không
phải lỗi.

## 6b. Khảo sát validation Giai đoạn 3 (BotPrize-style)

```bash
# 1. (Đã chạy sẵn, có file kết quả trong gói) sinh lại stimuli nếu muốn:
python3 -m research.phase3_survey.prepare_stimuli

# 2. Thêm 6 bài giải NGƯỜI THẬT (Nhóm H) -- CÓ 2 CÁCH:
#
#    Cách A (khuyên dùng, không cần tự gõ ký hiệu Singmaster):
#    mở research/phase3_survey/move_recorder.html bằng trình duyệt --
#    tool cho bấm nút để ghi nước đi khi bạn giải cube thật, cuối cùng
#    tự sinh sẵn câu lệnh terminal để copy-dán chạy luôn.
#
#    Cách B (nếu đã có sẵn chuỗi nước đi, vd từ CSTimer):
python3 -m research.phase3_survey.add_human_stimulus --moves "R U R' U' ..."
#    (hoặc chạy không có --moves để nhập tương tác từng bước)
#    Cả 2 cách đều tự báo "Nhóm H: x/6" sau mỗi lần chạy, tự kiểm tra cú
#    pháp Singmaster trước khi ghi file (không sợ làm hỏng JSON).

# 3. Gửi 2 file research/phase3_survey/{rating_tool.html, stimuli.json}
#    (giữ cùng thư mục, có thể nén chung thành 1 zip nhỏ) cho người tham
#    gia khảo sát -- họ chỉ cần mở rating_tool.html bằng trình duyệt,
#    không cần cài gì, làm được cả trên điện thoại.

# 4. Thu thập CSV kết quả (mỗi người 1 file) vào research/phase3_survey/results/, rồi:
python3 -m research.phase3_survey.analyze_results
```
Chi tiết đầy đủ (câu hỏi nghiên cứu, tiêu chí tuyển người, đạo đức nghiên
cứu, kế hoạch phân tích thống kê) xem `research/phase3_survey/DESIGN.md`.

## 7. Biên dịch bài báo (`paper/main.tex`)

Cần TeXLive (hoặc Overleaf — kéo thả `main.tex` vào, không cần cài gì):

```bash
cd paper
pdflatex -interaction=nonstopmode main.tex
pdflatex -interaction=nonstopmode main.tex   # chạy 2 lần để resolve citation
```
`main.pdf` trong gói là bản build sẵn (11 trang, biên dịch sạch 0 lỗi) —
chỉ cần build lại nếu bạn sửa `main.tex`.

**Về font:** file dùng `fontenc[T1]` (không dùng `T5` vì gói này không có
sẵn trong nhiều bản TeXLive tối giản, từng gây lỗi encoding âm thầm — đã
sửa và xác nhận build sạch). Nếu build trên Overleaf (TeXLive đầy đủ), có
thể thêm `\usepackage{lmodern}` để hiển thị dấu tên tác giả Ba Lan
(Świechowski/Ślęzak) chuẩn hơn khi copy-paste text từ PDF.

## 8. Các `\todo{}` còn lại trong `paper/main.tex`

Tìm `\todo{` (tô đỏ khi build PDF) để thấy toàn bộ việc còn thiếu — hiện
tại chủ yếu là quyết định cá nhân (tên tác giả, chọn bảng chính dùng
scramble mode nào) và phần phụ thuộc Giai đoạn 3, không còn thiếu số
liệu thực nghiệm nào nữa.

## 9. Venue dự kiến nộp bài

Hầu hết deadline 2026 (IEEE CoG, ICAPS/HAXP) đã qua tính đến thời điểm dự
án này thực hiện. Khuyến nghị nhắm chu kỳ 2027:
1. **Ưu tiên**: Workshop HAXP (Human-Aware and Explainable Planning,
   trước đây tên XAIP) @ ICAPS 2027 — đúng cộng đồng học thuật được
   trích dẫn nhiều nhất trong bài.
2. **Dự phòng**: IEEE CoG 2027, dạng auxiliary/short paper (4 trang).
Nên kiểm tra lại CFP chính xác khoảng cuối 2026.

## 10. Xử lý lỗi thường gặp

| Lỗi | Nguyên nhân | Cách sửa |
|---|---|---|
| `ModuleNotFoundError: No module named 'kociemba'` | Chưa cài | `pip install kociemba` |
| `externally-managed-environment` khi pip install | Ubuntu/Debian mới chặn pip cài global | thêm `--break-system-packages` hoặc dùng venv |
| Chạy script trong `research/` báo lỗi import | Chạy sai thư mục / chạy trực tiếp `.py` thay vì `-m` | luôn chạy từ gốc project bằng `python3 -m research.ten_script` |
| Test/thực nghiệm chạy rất lâu ở lần đầu | Đang build cache PDB | bình thường, chỉ xảy ra 1 lần |
| λ lớn (≥2) trong `run_pareto_sweep.py` treo lâu | Đã biết | dùng `--nodes-per-depth` nhỏ hơn hoặc λ nhỏ hơn |
| `add_human_stimulus.py` báo lỗi cú pháp chuỗi nước đi | Gõ sai ký hiệu Singmaster | xem thông báo lỗi cụ thể, sửa lại chuỗi (chỉ chấp nhận R/L/U/D/F/B + `'`/`2`) |

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
Kết quả mong đợi: `319 passed` (113 trong `test_solver.py` + 206 trong
`test_cube_engine.py`). Mất khoảng 10-15 giây (không tính lần đầu build
cache PDB, xem mục 3).

## 5b. Bảng công thức OLL (57/57 đã kiểm chứng)

`solver/oll_algorithms.py` hiện có **đủ 57/57 case OLL chuẩn** (đã tự
kiểm chứng khi import module — xem `VERIFIED_ALG_NAMES`/`REJECTED_ALG_NAMES`).
Kiểm tra lại bất kỳ lúc nào bằng:
```bash
python3 -m solver.oll_algorithms
```
sẽ in ra `57/57 cong thuc OLL hop le`. Muốn thêm/sửa 1 công thức (chỉ
chấp nhận nước thuần R/L/U/D/F/B, không lát/rộng):
```python
# mo file solver/oll_algorithms.py, them 1 dong vao dict _RAW_ALGS:
_RAW_ALGS['TenCaseMoi'] = "chuoi nuoc di ban tim duoc"
```
Công thức sai sẽ tự động bị loại (rơi vào `REJECTED_ALG_NAMES`), không
lo làm hỏng bảng hiện có.

**Lưu ý còn treo:** round-trip tự-sinh-tự-giải (mỗi công thức × 4 AUF =
228 case) hiện đạt 224/228 (~98.2%), chưa phải 100% — xem
`test_solver.py::TestOLLAlgorithms::test_own_generating_set_round_trip`.
Chưa xác định rõ 4 case còn lại là bug thật hay giới hạn thiết kế đã
biết.

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

# 5. Tinh lai DOC LAP tu du lieu tho (ICC + tuong quan da bao cao trong paper) --
#    khong can thu vien ngoai (pingouin/krippendorff), tu cai cong thuc chuan:
python3 -m research.phase3_survey.compute_icc                  # ICC(2,1)/ICC(2,k)
python3 -m research.phase3_survey.recompute_hli_exact           # sua loi P=0/1 gia dinh theo nhom
python3 -m research.phase3_survey.compute_exact_correlation     # tuong quan Spearman ban "exact"
```
28 file CSV thô (kết quả thật, 1 người/file) đã có sẵn trong
`research/phase3_survey/results/` — không cần tự thu thập lại để chạy
thử 3 lệnh trên.

Chi tiết đầy đủ (câu hỏi nghiên cứu, tiêu chí tuyển người, đạo đức nghiên
cứu, kế hoạch phân tích thống kê) xem `research/phase3_survey/DESIGN.md`.

## 7. Biên dịch bài báo (`paper/main.tex`, `paper/main_vi.tex`)

Cần TeXLive (hoặc Overleaf — kéo thả file `.tex` vào, không cần cài gì).

**Bản tiếng Anh (`main.tex`) — dùng `pdflatex`:**
```bash
cd paper
pdflatex -interaction=nonstopmode main.tex
pdflatex -interaction=nonstopmode main.tex   # chạy 2 lần để resolve citation
```
`main.pdf` trong gói là bản build sẵn (15 trang, biên dịch sạch 0 lỗi/
warning sau 2 lần chạy) — chỉ cần build lại nếu bạn sửa `main.tex`.

**Bản tiếng Việt (`main_vi.tex`) — BẮT BUỘC dùng `xelatex` (KHÔNG dùng
được `pdflatex`):**
```bash
cd paper
xelatex -interaction=nonstopmode main_vi.tex
xelatex -interaction=nonstopmode main_vi.tex   # chạy 2 lần để resolve citation
```
Lý do bắt buộc `xelatex`: file dùng gói `fontspec` (để chọn font hỗ trợ
tiếng Việt có dấu) — `fontspec` chỉ tương thích XeTeX/LuaTeX, chạy
`pdflatex main_vi.tex` sẽ báo lỗi fatal ngay dòng đầu
(`! Fatal Package fontspec Error: ... requires either XeTeX or LuaTeX`).
`main_vi.pdf` trong gói là bản build sẵn (17 trang, biên dịch sạch).

**Về font (áp dụng cho `main.tex`):** file dùng `fontenc[T1]` (không
dùng `T5` vì gói này không có sẵn trong nhiều bản TeXLive tối giản, từng
gây lỗi encoding âm thầm — đã sửa và xác nhận build sạch). Nếu build
trên Overleaf (TeXLive đầy đủ), có thể thêm `\usepackage{lmodern}` để
hiển thị dấu tên tác giả Ba Lan (Świechowski/Ślęzak) chuẩn hơn khi
copy-paste text từ PDF.

## 8. Các `\todo{}` còn lại trong `paper/main.tex`

Tìm `\todo{` (tô đỏ khi build PDF) để thấy toàn bộ việc còn thiếu — hiện
tại chủ yếu là quyết định cá nhân (tên tác giả, chọn bảng chính dùng
scramble mode nào) và phần phụ thuộc Giai đoạn 3, không còn thiếu số
liệu thực nghiệm nào nữa.

## 9. Xử lý lỗi thường gặp

| Lỗi | Nguyên nhân | Cách sửa |
|---|---|---|
| `ModuleNotFoundError: No module named 'kociemba'` | Chưa cài | `pip install kociemba` |
| `externally-managed-environment` khi pip install | Ubuntu/Debian mới chặn pip cài global | thêm `--break-system-packages` hoặc dùng venv |
| Chạy script trong `research/` báo lỗi import | Chạy sai thư mục / chạy trực tiếp `.py` thay vì `-m` | luôn chạy từ gốc project bằng `python3 -m research.ten_script` |
| Test/thực nghiệm chạy rất lâu ở lần đầu | Đang build cache PDB | bình thường, chỉ xảy ra 1 lần |
| λ lớn (≥2) trong `run_pareto_sweep.py` treo lâu | Đã biết | dùng `--nodes-per-depth` nhỏ hơn hoặc λ nhỏ hơn |
| `add_human_stimulus.py` báo lỗi cú pháp chuỗi nước đi | Gõ sai ký hiệu Singmaster | xem thông báo lỗi cụ thể, sửa lại chuỗi (chỉ chấp nhận R/L/U/D/F/B + `'`/`2`) |
| `! Fatal Package fontspec Error` khi build `main_vi.tex` | Dùng nhầm `pdflatex` | dùng `xelatex` (xem mục 7) |

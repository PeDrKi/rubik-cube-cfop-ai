# CFOP AI — Kiến trúc & Ghi chú đồ án

## Tổng quan

Module `solver/` bổ sung một AI giải Rubik theo phương pháp **CFOP**
(Cross → F2L → OLL → PLL). Bản hiện tại (MVP) triển khai đầy đủ **Cross**,
**F2L** và **OLL** (2-look); **PLL** là phase kế tiếp (interface đã sẵn sàng
để nối thêm, xem `solver/cfop_ai.py`, làm y hệt cách OLL đã được nối vào).

Có 2 chế độ dùng trong app (`main.py`):
- **Phím `A`** — Auto-solve: AI tự tính và animate toàn bộ Cross+F2L+OLL.
- **Phím `H`** — Hint: AI gợi ý bước **tiếp theo** (Cross / 1 cặp F2L /
  OLL-định hướng cạnh / OLL-định hướng góc), hiển thị dạng text Singmaster,
  **không tự thực thi** — người dùng tự xoay.

Cả hai chạy trong **background thread** (không đứng hình UI khi tính toán).

## Vì sao gọi là "AI" (kỹ thuật dùng)

Đây là **tìm kiếm có heuristic học từ dữ liệu (Pattern Database)** — đúng kỹ
thuật kinh điển trong AI classic search (Culberson & Schaeffer 1998; dùng lại
trong solver Rubik tối ưu của Korf 1997), **không phải** thuật toán cứng viết
tay từng trường hợp, và **không phải** bảng 57 công thức OLL học thuộc lòng
chép từ nguồn ngoài — toàn bộ công thức đều do chính AI tự tìm ra bằng search:

1. **`solver/edge_model.py`, `solver/corner_model.py`** — mô hình hoá cube ở
   mức "cubie" (vị trí + hướng của từng miếng trong 12 khe cạnh / 8 khe góc),
   suy luận **thực nghiệm** bảng hoán vị của mỗi nước đi (không hard-code).

2. **`solver/pdb_builder.py`** — xây **Pattern Database**: BFS từ trạng thái
   đã giải (hoặc từ nhiều trạng thái đích cùng lúc — "multi-goal"/"any-perm"
   BFS, dùng cho OLL vì không quan tâm quân kết thúc ở khe nào). Đây chính là
   "dữ liệu học được":
   - Cross: ~190,080 trạng thái, build ~5 giây.
   - F2L: PDB ghép đôi (corner+edge) 576 trạng thái mỗi slot.
   - OLL: PDB "gộp" 4 cạnh U (190,080 trạng thái) và 4 góc U (136,080 trạng
     thái), mỗi cái build ~4-5 giây.

3. **`solver/cross_solver.py`** — vì PDB cho khoảng cách **chính xác**, chỉ
   cần leo dốc tham lam theo bảng là ra lời giải Cross **tối ưu** (≤8 nước).

4. **`solver/f2l_solver.py`** — **A\*** (best-first search) trên không gian
   cubie đầy đủ, dùng **Pattern Database ghép đôi** (corner+edge cùng lúc)
   làm heuristic, cộng thêm penalty để ưu tiên tránh phá vỡ phần đã giải.

5. **`solver/oll_solver.py`** — OLL theo kỹ thuật **"2-Look"** thật của CFOP
   (tách thành 2 pha, không tìm cả 8 quân cùng lúc — đã thử và quá chậm vì
   không gian tương tác giữa cạnh+góc quá lớn để làm PDB đúng nghĩa):
   - **Pha A (Orient Edges)** — A* + PDB gộp 4 cạnh (chính xác), thường
     <1 giây.
   - **Pha B (Orient Corners)** — A* + PDB gộp 4 góc + penalty giữ cạnh/
     Cross/F2L nguyên vẹn. Đa số trường hợp <2 giây; một số case hiếm (dạng
     "H-case") có thể mất tới ~1-2.5 phút vì phải leo thang ngân sách tìm
     kiếm (`_solve_phase_B_ladder`) — **luôn đúng khi tìm ra**, chỉ đánh đổi
     thời gian, không đánh đổi độ chính xác. Chạy nền nên không đứng hình UI.

Tất cả **không đảm bảo tối ưu tuyệt đối số nước** (đổi lấy tốc độ — phù hợp
MVP), nhưng **luôn đúng** khi tìm ra lời giải (đã stress-test hàng chục scramble
ngẫu nhiên, 100% chính xác trên facelet thật — xem `test_solver.py`).

## File map

```
solver/
  edge_model.py       12 cạnh: bảng hoán vị + hướng (suy luận thực nghiệm)
  corner_model.py      8 góc:  bảng hoán vị + hướng mod 3
  full_state.py        trạng thái cube đầy đủ dạng cubie (không còn phụ
                        thuộc facelet -> tránh bug "thông tin ẩn")
  facelets.py           đọc/kiểm tra facelet (cross_solved, pair_solved,
                        oll_solved, ...)
  pdb_builder.py        BFS xây Pattern Database (đơn/ghép đôi/multi-goal/
                        "any-perm" nhóm nhiều quân)
  cross_solver.py       giải Cross tối ưu
  f2l_solver.py         giải F2L (A* + PDB ghép đôi)
  oll_solver.py         giải OLL 2-look (A* + PDB gộp theo pha)
  cfop_ai.py            API cấp cao: full_solve(state), hint(state), stage_of(state)
cache/                  PDB đã build, cache ra đĩa (.pkl) — build 1 lần
test_solver.py           unit + stress test
```

## Giới hạn hiện tại (rõ ràng cho báo cáo đồ án)

- PLL: **chưa triển khai** (bước tiếp theo hợp lý — có thể làm bằng đúng kỹ
  thuật đã dùng cho OLL: PDB "gộp" cho hoán vị 4 cạnh + 4 góc lớp U, thay vì
  21 công thức học thuộc lòng).
- F2L/OLL dùng A* có trọng số (không tối ưu tuyệt đối số nước) để đổi lấy
  tốc độ chạy thực tế.
- OCLL (pha B của OLL) có đuôi thời gian dài (long tail): đa số <2s, hiếm khi
  tới ~1-2.5 phút cho case khó nhất. Đây là đánh đổi kỹ thuật đã biết, không
  phải bug.
- Cache PDB nằm trong `solver/cache/*.pkl`; xoá thư mục này nếu muốn build lại
  từ đầu (ví dụ sau khi sửa `edge_model.py`/`corner_model.py`).

## Cách test nhanh

```bash
pip install -r requirements.txt --break-system-packages
python -m pytest test_solver.py -q   # co the mat vai phut do OCLL kho
python main.py     # trong app: Space=scramble, A=auto-solve, H=hint
```

# CFOP AI — Kiến trúc & Ghi chú đồ án

## Tổng quan

Module `solver/` là một AI giải Rubik **đầy đủ theo phương pháp CFOP**
(Cross → F2L → OLL → PLL) — toàn bộ 4 bước đã triển khai.

Có 2 chế độ dùng trong app (`main.py`):
- **Phím `A`** — Auto-solve: AI tự tính và animate **toàn bộ** Cross+F2L+OLL+PLL.
- **Phím `H`** — Hint: AI gợi ý bước **tiếp theo** (Cross / 1 cặp F2L /
  OLL-cạnh / OLL-góc / PLL-góc / PLL-cạnh), hiển thị dạng text Singmaster,
  **không tự thực thi** — người dùng tự xoay.

Cả hai chạy trong **background thread** (không đứng hình UI khi tính toán).

## Vì sao gọi là "AI" (kỹ thuật dùng)

Đây là **tìm kiếm có heuristic học từ dữ liệu (Pattern Database)** — kỹ
thuật kinh điển trong AI classic search (Culberson & Schaeffer 1998; dùng
lại trong solver Rubik tối ưu của Korf 1997), **không phải** thuật toán
cứng viết tay từng trường hợp, và **không phải** bảng công thức OLL/PLL học
thuộc lòng chép từ nguồn ngoài — toàn bộ công thức đều do chính AI tự tìm ra
bằng search.

1. **`solver/edge_model.py`, `solver/corner_model.py`** — mô hình hoá cube ở
   mức "cubie" (vị trí + hướng của từng miếng trong 12 khe cạnh / 8 khe góc),
   suy luận **thực nghiệm** bảng hoán vị của mỗi nước đi (không hard-code).

2. **`solver/pdb_builder.py`** — xây **Pattern Database**: BFS từ trạng thái
   đã giải (đơn/ghép đôi), hoặc từ nhiều trạng thái đích cùng lúc
   ("multi-goal"/"any-perm" BFS — dùng cho OLL vì không quan tâm quân kết
   thúc ở khe nào). Đây chính là "dữ liệu học được".

3. **`solver/cross_solver.py`** — PDB chính xác (~190K trạng thái) + leo
   dốc tham lam → lời giải Cross **tối ưu** (≤8 nước).

4. **`solver/f2l_solver.py`** — A* + PDB ghép đôi (corner+edge) mỗi slot,
   cộng penalty tránh phá vỡ Cross/các slot đã xong.

5. **`solver/oll_solver.py`** — **2-Look OLL** thật của CFOP (tách 2 pha:
   định hướng cạnh rồi định hướng góc — đã thử tìm cả 8 quân cùng lúc và
   quá chậm, không gian tương tác quá lớn để làm PDB đúng nghĩa). Mỗi pha
   dùng PDB "gộp" (any-perm) riêng + penalty giữ Cross/F2L.

6. **`solver/pll_solver.py`** — **2-Look PLL** (tách 2 pha: hoán vị góc rồi
   hoán vị cạnh). Vì mục tiêu là MỘT trạng thái đích cụ thể (không phải
   "bất kỳ hoán vị nào" như OLL), dùng lại đúng kỹ thuật PDB chính xác của
   Cross (`build_pdb_edges`/`build_pdb_corners`) — heuristic mạnh hơn hẳn
   PDB any-perm của OLL.

Tất cả **không đảm bảo tối ưu tuyệt đối số nước** (đổi lấy tốc độ — phù hợp
MVP), nhưng **luôn đúng** khi tìm ra lời giải (đã stress-test hàng chục
scramble ngẫu nhiên + test tất định, 100% chính xác trên facelet thật —
xem `test_solver.py`).

## File map

```
solver/
  edge_model.py       12 cạnh: bảng hoán vị + hướng (suy luận thực nghiệm)
  corner_model.py      8 góc:  bảng hoán vị + hướng mod 3
  full_state.py        trạng thái cube đầy đủ dạng cubie
  facelets.py           đọc/kiểm tra facelet (cross_solved, pair_solved,
                        oll_solved, ...)
  pdb_builder.py        BFS xây Pattern Database (đơn/ghép đôi/multi-goal/
                        "any-perm" nhóm nhiều quân)
  cross_solver.py       giải Cross tối ưu
  f2l_solver.py         giải F2L (A* + PDB ghép đôi)
  oll_solver.py         giải OLL 2-look (A* + PDB gộp any-perm)
  pll_solver.py         giải PLL 2-look (A* + PDB chính xác)
  cfop_ai.py            API cấp cao: full_solve(state), hint(state), stage_of(state)
cache/                  PDB đã build, cache ra đĩa (.pkl) — build 1 lần
test_solver.py           unit + stress test
```

## Cập nhật: IDA* thuần gây "treo" bất ngờ ở vài case → chuyển sang lai A*+IDA*

Sau khi chuyển hẳn sang IDA*, phát hiện thêm vấn đề: IDA* **không lưu
trạng thái đã duyệt** (memoization), nên với heuristic chưa thật sát, một
số case *nhìn có vẻ đơn giản* (h ban đầu nhỏ) lại khiến IDA* phải **duyệt
lại toàn bộ cây tìm kiếm ở mỗi mức ngưỡng độ sâu** — nhược điểm lý thuyết
kinh điển của IDA* ("re-expansion"). Thực tế đã gặp: 1 case OLL-cạnh có
h₀=4 (trông rất dễ) khiến IDA* thuần treo hơn 4 phút không ra kết quả.

**Giải pháp cuối cùng — kết hợp cả hai:**
- **A* (có nhớ) làm ưu tiên chính** — nhanh, dự đoán được thời gian, nhờ
  memoization không duyệt lại. Ngân sách giới hạn ở mức an toàn RAM đã
  kiểm chứng (~300k node ~3GB).
- **IDA* chỉ là phương án dự phòng cuối** — dùng khi A* hết ngân sách mà
  vẫn chưa ra, đảm bảo **không bao giờ crash** dù chậm.

Đã test: 15/15 scramble ngẫu nhiên khác đều được A* giải trong <0.5s (tier
đầu tiên) — xác nhận đây là hành vi **phổ biến**, còn case treo lâu chỉ là
**ngoại lệ hiếm** với các scramble có tương tác cạnh/Cross/F2L đặc biệt
khó tách rời. Đây là giới hạn thực sự của cách tiếp cận "tìm kiếm từ đầu"
so với cách người chơi thật dùng 57 công thức OLL đã thuộc lòng sẵn cho
từng trường hợp cụ thể — xem mục "Giới hạn hiện tại" bên dưới.

## Cập nhật quan trọng: A* → IDA* (khắc phục OOM và tăng tỉ lệ giải được)

Trong quá trình tối ưu, phát hiện A* (heapq + dict `best_g` lưu mọi trạng
thái đã thăm) làm bộ nhớ tăng **tuyến tính theo số node duyệt** — máy chạy
chỉ có **~4GB RAM** (đã kiểm tra qua `/proc/meminfo`), nên ~300k node đã
chiếm ~3GB, có nguy cơ bị **kernel OOM-kill cả tiến trình** (crash toàn bộ
app) nếu tăng ngân sách tìm kiếm để giải case khó hơn.

**Đã chuyển sang IDA\* (Iterative Deepening A\*)** — kỹ thuật kinh điển
Korf (1997) dùng cho chính solver Rubik tối ưu, lý do gốc cũng là vì
không gian trạng thái quá lớn để lưu toàn bộ visited-set:
- Bộ nhớ chỉ **O(độ sâu tìm kiếm)** (vài chục phần tử), không phụ thuộc số
  node đã duyệt → **không bao giờ OOM**, đã kiểm chứng thực tế: chạy hơn
  4 phút liên tục, bộ nhớ giữ nguyên ~65-500MB (so với A* cũ có thể chạm
  3GB+ chỉ sau vài chục giây).
- Kết hợp với **heuristic mạnh hơn hẳn**: tái sử dụng chính PDB của Cross
  (`cross_solver.py`) và PDB từng cặp F2L (`f2l_solver.py`) làm cận dưới
  **admissible thực sự** (không phải cờ phạt 0/1 tùy ý như trước) cho việc
  "Cross/F2L có đang bị phá hay không, và phá bao nhiêu bước để sửa".
- Kết quả: case OCLL khó nhất từng gặp (2 góc xoay ngược chiều — **A\* cũ
  thất bại hoàn toàn dù đã dùng tới ~3GB RAM**) nay **giải được trong
  109 giây, chỉ dùng 121MB**. Case PLL cạnh (U-perm) từ ~45 giây (A\*)
  xuống còn **~9 giây** (IDA\* + heuristic mới).
- Xem `solver/search_utils.py` (IDA* dùng chung) và
  `solver/oll_solver.py::_cross_f2l_lower_bound()` (heuristic mới).

## Tối ưu đã thực hiện

- **Tốc độ:** các hàm kiểm tra trạng thái trong hot-path của A* (`cross_ok`,
  `pair_ok`, `cross_f2l_ok`, `u_edges_oriented`, ...) được viết lại thành so
  sánh chỉ số nguyên trực tiếp (dựa vào thứ tự `SLOT_NAMES` cố định) thay vì
  vòng lặp tra cứu theo tên qua dict — nhanh hơn ~1.5x (đã benchmark, xem
  `solver/full_state.py`), vì đây là hàm được gọi nhiều nhất trong toàn bộ
  quá trình tìm kiếm.
- **Tốc độ (thiết kế):** `hint()` trước đây gọi `solve_oll()`/`solve_pll()`
  **nguyên khối** (tính cả 2 pha) dù chỉ cần 1 pha — khiến hint bị "ăn theo"
  độ khó của pha không liên quan. Đã tách `solve_oll_edges_only()` /
  `solve_oll_corners_only()` / `solve_pll_corners_only()` /
  `solve_pll_edges_only()` để `hint()` chỉ tính đúng phần cần thiết.
- **UI:** panel trạng thái AI được neo vị trí **cố định** từ đáy màn hình
  (độc lập với chiều cao thay đổi của History phía trên) để không bao giờ
  đè lên danh sách phím tắt; có khung nền riêng biệt và 4 badge tiến trình
  Cross/F2L/OLL/PLL (✓ xong / ● đang làm / ○ chưa tới) để nhìn là biết ngay
  đang ở bước nào.

## Giới hạn hiện tại & đánh đổi hiệu năng (rõ ràng cho báo cáo đồ án)

- F2L/OLL/PLL dùng A* có trọng số (không tối ưu tuyệt đối số nước) để đổi
  lấy tốc độ chạy thực tế.
- **Đuôi thời gian dài (long tail):** đa số trường hợp Cross/F2L/OLL/PLL
  giải trong vài giây. Nhưng vì bài toán "giữ nguyên phần đã giải trong khi
  sửa phần còn lại" vốn khó về mặt tổ hợp, một số ít case hiếm (đặc biệt là
  OCLL — pha góc của OLL, và pha hoán vị cạnh của PLL) có thể mất tới
  **~1-2.5 phút**. Đây là đánh đổi kỹ thuật đã biết và đã kiểm chứng, không
  phải bug — chạy nền (thread) nên không đứng hình UI. Một số case cực hiếm
  có thể không tìm ra trong ngân sách cho phép; AI sẽ báo rõ trạng thái
  (`'reached': 'oll_partial'`, `'pll_partial'`, ...) thay vì treo máy hoặc
  báo sai.
- Cache PDB nằm trong `solver/cache/*.pkl`; xoá thư mục này nếu muốn build
  lại từ đầu (ví dụ sau khi sửa `edge_model.py`/`corner_model.py`).

## Cách test nhanh

```bash
pip install -r requirements.txt --break-system-packages
python -m pytest test_solver.py -q   # co the mat vai phut do 1-2 case OCLL/PLL kho
python main.py     # trong app: Space=scramble, A=auto-solve, H=hint
```

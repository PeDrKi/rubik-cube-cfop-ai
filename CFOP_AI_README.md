# CFOP AI — Kiến trúc & Ghi chú đồ án

## Tổng quan

Module `solver/` bổ sung một AI giải Rubik theo phương pháp **CFOP**
(Cross → F2L → OLL → PLL). Bản hiện tại (MVP) triển khai đầy đủ **Cross** và
**F2L**; OLL/PLL là phase kế tiếp (interface đã sẵn sàng để nối thêm, xem
`solver/cfop_ai.py`).

Có 2 chế độ dùng trong app (`main.py`):
- **Phím `A`** — Auto-solve: AI tự tính và animate toàn bộ Cross+F2L.
- **Phím `H`** — Hint: AI gợi ý bước **tiếp theo** (Cross hoặc 1 cặp F2L),
  hiển thị dạng text Singmaster, **không tự thực thi** — người dùng tự xoay.

Cả hai chạy trong **background thread** (không đứng hình UI khi tính toán).

## Vì sao gọi là "AI" (kỹ thuật dùng)

Đây là **tìm kiếm có heuristic học từ dữ liệu (Pattern Database)** — đúng kỹ
thuật kinh điển trong AI classic search (Culberson & Schaeffer 1998; dùng lại
trong solver Rubik tối ưu của Korf 1997), **không phải** thuật toán cứng viết
tay từng trường hợp:

1. **`solver/edge_model.py`, `solver/corner_model.py`** — mô hình hoá cube ở
   mức "cubie" (vị trí + hướng của từng miếng trong 12 khe cạnh / 8 khe góc),
   suy luận **thực nghiệm** bảng hoán vị của mỗi nước đi (không hard-code).

2. **`solver/pdb_builder.py`** — xây **Pattern Database**: BFS từ trạng thái
   đã giải, ghi lại "còn bao nhiêu bước tối thiểu" cho từng cấu hình rút gọn.
   Đây chính là "dữ liệu học được" (bảng ~190,080 trạng thái cho Cross, build
   ~5 giây, cache ra đĩa).

3. **`solver/cross_solver.py`** — vì PDB cho khoảng cách **chính xác**, chỉ
   cần leo dốc tham lam theo bảng là ra lời giải Cross **tối ưu** (≤8 nước).

4. **`solver/f2l_solver.py`** — **A\*** (best-first search) trên không gian
   cubie đầy đủ, dùng **Pattern Database ghép đôi** (corner+edge cùng lúc,
   576 trạng thái) làm heuristic, cộng thêm penalty để ưu tiên tránh phá vỡ
   phần đã giải. Không đảm bảo tối ưu tuyệt đối (đổi lấy tốc độ — phù hợp
   MVP), nhưng **luôn đúng** (đã stress-test 100/100 scramble ngẫu nhiên).

## File map

```
solver/
  edge_model.py      12 cạnh: bảng hoán vị + hướng (suy luận thực nghiệm)
  corner_model.py     8 góc:  bảng hoán vị + hướng mod 3
  full_state.py       trạng thái cube đầy đủ dạng cubie (không còn phụ
                       thuộc facelet -> tránh bug "thông tin ẩn")
  facelets.py          đọc/kiểm tra facelet (cross_solved, pair_solved, ...)
  pdb_builder.py       BFS xây Pattern Database (cross / single-piece / pair)
  cross_solver.py      giải Cross tối ưu
  f2l_solver.py        giải F2L (A* + PDB ghép đôi)
  cfop_ai.py           API cấp cao: full_solve(state), hint(state), stage_of(state)
cache/                 PDB đã build, cache ra đĩa (.pkl) — build 1 lần
test_solver.py          unit + stress test
```

## Giới hạn hiện tại (rõ ràng cho báo cáo đồ án)

- OLL/PLL: **chưa triển khai** (bước tiếp theo hợp lý — OLL có 57 case, PLL
  21 case, có thể làm bằng lookup-table tương tự cách CFOP thật, hoặc mở
  rộng full_state.py sang PDB tương tự F2L).
- F2L dùng A* có trọng số (không tối uu tuyệt đối số nước) để đổi lấy tốc độ
  chạy thực tế (trung bình <1s/scramble, số ít trường hợp khó tới ~10-13s).
- Cache PDB nằm trong `solver/cache/*.pkl`; xoá thư mục này nếu muốn build lại
  từ đầu (ví dụ sau khi sửa `edge_model.py`/`corner_model.py`).

## Cách test nhanh

```bash
pip install -r requirements.txt --break-system-packages
python -m pytest test_solver.py -q
python main.py     # trong app: Space=scramble, A=auto-solve, H=hint
```

# README_TEST_APP.md

Hướng dẫn chạy các bộ test cho **tầng app** (bổ sung cho
`test_cube_engine.py` + `test_solver.py` vốn đã có, chỉ phủ tầng
engine/solver): `test_app_logic.py`, `test_app_logic2.py` và
`test_cancellation.py`.

## `test_app_logic2.py` — không cần pygame (kể cả stub)

Test cho `app_logic.py` (logic quyết định/tính toán thuần tách từ
`main.py` ở vòng sửa thứ 2 — resize/zoom, thông báo kết quả AI, retry
hint, undo trim, tốc độ animation...). Hoàn toàn không đụng `pygame`,
chạy trực tiếp:

```bash
python3 test_app_logic2.py
```

## `test_cancellation.py` — không cần pygame

Test cơ chế huỷ job AI (`Esc` trong lúc AI đang tính). Chỉ dùng
`cube_engine` + `solver/`, KHÔNG import `pygame`, nên chạy trực tiếp:

```bash
python3 test_cancellation.py
# hoặc, nếu có pytest thật:
python3 -m pytest test_cancellation.py -q
```

Mất khoảng vài giây (1 test cố tình dùng scramble "chậm" ~3.3s để kiểm
tra huỷ giữa chừng có thực sự dừng sớm, xem comment trong file).

## `test_app_logic.py` — cần `pygame` (thật hoặc giả)

Test các hàm THUẦN trong `formula_panel.py` (`build_formula_lines`,
`looks_like_moves`, `formula_row_data`, `formula_lines_to_text`). File
này `import formula_panel`, mà `formula_panel.py` có `import pygame` ở
đầu (dùng cho phần vẽ, dù các hàm được test ở đây không đụng gì tới
`pygame.Surface` thật) — nên cần `pygame` import được, dù không cần
mở cửa sổ thật.

**Nếu máy bạn đã cài `pygame` thật** (máy dev bình thường chạy được
`python3 main.py`):

```bash
python3 test_app_logic.py
# hoặc:
python3 -m pytest test_app_logic.py -q
```

**Nếu môi trường KHÔNG có pygame** (vd CI tối giản, sandbox không có
Internet để `pip install pygame`) — dùng bản `pygame` GIẢ đi kèm trong
`fake_pygame_stub/`:

```bash
PYTHONPATH=./fake_pygame_stub:. python3 test_app_logic.py
```

`fake_pygame_stub/pygame/` chỉ định nghĩa tối thiểu để `import` không
lỗi: `Rect`, `Surface` (no-op), `font.SysFont` giả (đo độ rộng chuỗi bằng
`len(text) * 7` — đủ để logic wrap/hit-test không crash, KHÔNG chính xác
như font thật), hằng số trong `pygame.locals`. **Không dùng stub này để
chạy `main.py` thật** — `main()` cần cửa sổ/renderer pygame thật 100% để
có ý nghĩa, stub chỉ tồn tại để cho phép `import formula_panel` thành
công trong lúc test các hàm không đụng tới rendering.

## Chạy toàn bộ 5 bộ test cùng lúc

```bash
# Neu co pytest that + pygame that:
python3 -m pytest test_cube_engine.py test_solver.py test_cancellation.py test_app_logic.py test_app_logic2.py -q

# Neu KHONG co pygame that (dung stub cho rieng test_app_logic.py):
python3 -m pytest test_cube_engine.py test_solver.py test_cancellation.py test_app_logic2.py -q
PYTHONPATH=./fake_pygame_stub:. python3 -m pytest test_app_logic.py -q
```

Kỳ vọng: `51 + 50 + 8 + 11 + 23 = 143 passed`.

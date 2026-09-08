# README_TEST_APP.md

Hướng dẫn chạy các bộ test cho **tầng app** (bổ sung cho
`tests/test_cube_engine.py` + `tests/test_solver.py` vốn đã có, chỉ phủ
tầng engine/solver): `tests/test_app_logic.py`, `tests/test_app_logic2.py`
và `tests/test_cancellation.py`.

> Từ vòng sắp xếp lại thư mục: toàn bộ file test nằm trong `tests/`, mỗi
> file đều tự thêm thư mục gốc repo vào `sys.path` khi chạy (xem vài dòng
> đầu mỗi file) — nên các lệnh dưới đây chạy đúng dù bạn gọi từ thư mục
> gốc repo hay `cd` thẳng vào `tests/`.

## `tests/test_app_logic2.py` — không cần pygame (kể cả stub)

Test cho `app_logic.py` (logic quyết định/tính toán thuần tách từ
`main.py` — resize/zoom, thông báo kết quả AI, retry hint, undo trim,
tốc độ animation, cuộn/cắt text tránh tràn UI...). Hoàn toàn không đụng
`pygame`, chạy trực tiếp:

```bash
python3 tests/test_app_logic2.py
```

## `tests/test_cancellation.py` — không cần pygame

Test cơ chế huỷ job AI (`Esc` trong lúc AI đang tính). Chỉ dùng
`cube_engine` + `solver/`, KHÔNG import `pygame`, nên chạy trực tiếp:

```bash
python3 tests/test_cancellation.py
# hoặc, nếu có pytest thật:
python3 -m pytest tests/test_cancellation.py -q
```

Mất khoảng vài giây (1 test cố tình dùng scramble "chậm" ~3.3s để kiểm
tra huỷ giữa chừng có thực sự dừng sớm, xem comment trong file).

## `tests/test_app_logic.py` — cần `pygame` (thật hoặc giả)

Test các hàm THUẦN trong `formula_panel.py` (`build_formula_lines`,
`looks_like_moves`, `formula_row_data`, `formula_lines_to_text`). File
này `import formula_panel`, mà `formula_panel.py` có `import pygame` ở
đầu (dùng cho phần vẽ, dù các hàm được test ở đây không đụng gì tới
`pygame.Surface` thật) — nên cần `pygame` import được, dù không cần
mở cửa sổ thật.

**File này TỰ ĐỘNG phát hiện có pygame thật hay không** (thử `import
pygame`, nếu lỗi thì tự thêm `tests/fake_pygame_stub/` vào `sys.path`) —
không cần tự set `PYTHONPATH` nữa:

```bash
python3 tests/test_app_logic.py
# hoặc, nếu có pytest thật (pytest KHÔNG chạy được bootstrap tự chọn
# stub ở đầu file theo cùng cách -- neu may khong co pygame that, van
# can tu set PYTHONPATH nhu ben duoi khi dung pytest):
python3 -m pytest tests/test_app_logic.py -q
```

**Nếu dùng `pytest` thật trên máy KHÔNG có `pygame` thật** (vd CI tối
giản) — vẫn cần tự set `PYTHONPATH` trỏ tới stub (bootstrap tự động
trong file chỉ chạy khi gọi trực tiếp bằng `python3`, không chắc chạy
trước khi `pytest` tự import module):

```bash
PYTHONPATH=./tests/fake_pygame_stub:. python3 -m pytest tests/test_app_logic.py -q
```

`tests/fake_pygame_stub/pygame/` chỉ định nghĩa tối thiểu để `import`
không lỗi: `Rect`, `Surface` (no-op), `font.SysFont` giả (đo độ rộng
chuỗi bằng `len(text) * 7` — đủ để logic wrap/hit-test không crash,
KHÔNG chính xác như font thật), hằng số trong `pygame.locals`. **Không
dùng stub này để chạy `main.py` thật** — `main()` cần cửa sổ/renderer
pygame thật 100% để có ý nghĩa, stub chỉ tồn tại để cho phép `import
formula_panel` thành công trong lúc test các hàm không đụng tới rendering.

## Chạy toàn bộ 5 bộ test cùng lúc

```bash
# Neu co pytest that + pygame that (cach don gian nhat):
python3 -m pytest tests/ -q

# Neu KHONG co pygame that (dung stub cho rieng test_app_logic.py):
python3 -m pytest tests/test_cube_engine.py tests/test_solver.py tests/test_cancellation.py tests/test_app_logic2.py -q
PYTHONPATH=./tests/fake_pygame_stub:. python3 -m pytest tests/test_app_logic.py -q

# Hoac chay tung file rieng (2 file dau BAT BUOC can pytest that cai san,
# vi dung @pytest.mark.parametrize -- KHAC 3 file sau, tu viet co runner
# noi bo `if __name__ == '__main__'` nen chay duoc ma khong can pytest):
python3 -m pytest tests/test_cube_engine.py -q   # can pytest that
python3 -m pytest tests/test_solver.py -q        # can pytest that
python3 tests/test_cancellation.py               # KHONG can pytest
python3 tests/test_app_logic.py                  # KHONG can pytest, tu chon stub neu can
python3 tests/test_app_logic2.py                 # KHONG can pytest
```

Kỳ vọng: `51 + 50 + 8 + 11 + 32 = 152 passed`.

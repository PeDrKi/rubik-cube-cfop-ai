# REFACTOR_NOTES.md

Ghi lại cách từng **hạn chế/rủi ro** nêu ra khi đánh giá app mô phỏng
(`main.py` + `renderer_3d.py` + `layout.py` + `draw_helpers.py` +
`solver/`) đã được xử lý. Không đụng tới phần nghiên cứu (`research/`,
`paper/`).

## 1. `main.py` monolithic (1372 dòng, 1 hàm `main()` với hàng chục biến `nonlocal`)

**Đã làm:**
- Tách `formula_panel.py` ra khỏi `main.py`: toàn bộ logic dựng nội dung
  bảng công thức CFOP (`build_formula_lines`, `_stage_row`,
  `formula_lines_to_text`, `formula_row_data`, `looks_like_moves`) +
  phần vẽ (`draw_formula_panel_content`, `wrap_text`) + cửa sổ OS riêng
  (`try_open_formula_window`, `sdl_window_available`).
- Module chia rõ 2 nhóm: **THUẦN** (không đụng `pygame.Surface`/`Font`
  thật → test được độc lập) và **CẦN RENDERER** (chỉ chạy đúng trong app
  thật). Xem docstring đầu file.
- `main.py` giờ `import formula_panel as fp` và gọi `fp.build_formula_lines(...)`,
  `fp.draw_formula_panel_content(...)` v.v. thay vì định nghĩa tại chỗ.

**Chưa làm hết (còn lại, ghi nhận trung thực):** `main.py` vẫn còn lớn
(~1200 dòng) vì phần game loop (xử lý sự kiện chuột/phím, vẽ 2 panel 6-mặt
+ 3D, animation queue) vốn dĩ là 1 khối trạng thái liên kết chặt (yaw/pitch/
zoom, anim_queue, bar_text...) — tách tiếp thành class/module riêng (vd
`AppState`, `InputHandler`) là bước hợp lý tiếp theo nếu có thời gian,
nhưng rủi ro hồi quy cao hơn nhiều so với tách `formula_panel.py` (vốn đã
khá độc lập) nên **cố tình để lại phạm vi sau** thay vì làm ẩu.

## 2. AI dao động 0.2s–7.7s, không có progress/cancel

**Đã làm** (xem `solver/search_utils.py`, `solver/f2l_solver.py`,
`solver/cfop_ai.py`, `main.py`):

- `search_utils.py`: thêm `SearchCancelled` (exception), `set_cancel_event()`
  /`clear_cancel_event()`/`check_cancel()` dùng 1 biến **module-level**
  (không phải tham số xuyên suốt mọi hàm) — hợp lý vì kiến trúc app chỉ
  chạy **1 job AI/thời điểm** (`cfop_busy` khoá job mới cho tới khi job cũ
  xong). `a_star()` và `ida_star()` gọi `check_cancel()` mỗi 2048 node
  (không phải mỗi node, tránh chi phí `Event.is_set()` làm chậm tìm kiếm).
- `f2l_solver.py::_solve_pair()` (vòng lặp heap riêng, thường là pha chậm
  nhất) cũng gọi `check_cancel()` tương tự.
- `cfop_ai.py`: `full_solve()`, `hint()`, `full_solve_breakdown()` nhận
  thêm tham số **tuỳ chọn** `cancel_event=None` — không truyền thì hành vi
  y hệt bản gốc (đã test riêng, xem `test_cancellation.py`). Khi bị huỷ
  giữa chừng, trả về dict có `'cancelled': True` thay vì để exception rơi
  ra worker thread.
- `main.py`: `start_cfop_job()` tạo 1 `threading.Event()` mới cho mỗi job
  (`cfop_cancel_event`), lưu `cfop_start_time` để hiển thị số giây đã trôi
  qua. Phím **Esc trong lúc `cfop_busy`** gọi `cancel_cfop_job()` (ưu tiên
  huỷ AI, KHÔNG thoát app luôn — Esc khi rảnh mới thoát). Thanh trạng thái
  hiển thị `"AI đang tính... (2.3s, Esc để huỷ)"` thay vì chỉ "đang tính"
  im lặng như trước.

**Vì sao không làm progress bar % thay vì chỉ elapsed-time:** `a_star`/
`ida_star`/A* trong F2L không biết trước tổng số node cần duyệt (đó là bản
chất bài toán tìm kiếm heuristic, không phải vòng lặp `for i in range(N)`
biết trước N) — hiển thị % giả sẽ gây hiểu lầm còn tệ hơn không có gì. Số
giây thực tế + nút Huỷ là thông tin trung thực nhất có thể cho loại
tìm kiếm này.

## 3. Không có test tự động cho tầng app/renderer

**Đã làm:**
- `test_app_logic.py`: 11 test cho các hàm THUẦN trong `formula_panel.py`
  (không đụng pygame display thật) — bao gồm cả test bất biến quan trọng
  `test_build_formula_lines_does_not_mutate_input` (đảm bảo hàm không có
  side-effect, vì `main()` gọi lại hàm này nhiều lần với cùng dữ liệu từ
  queue kết quả AI).
- `test_cancellation.py`: 8 test cho cơ chế huỷ mới (mục 2) — bao gồm cả
  test đảm bảo **hành vi cũ không đổi** khi không truyền `cancel_event`
  (`test_full_solve_without_cancel_event_unchanged`,
  `test_cancel_event_is_cleared_between_jobs`).
- `fake_pygame_stub/`: 1 bản `pygame` giả tối thiểu (chỉ `Rect`, `Surface`
  no-op, `font.SysFont` giả, hằng số `pygame.locals`) để `test_app_logic.py`
  chạy được trong CI/máy không có pygame thật (vd sandbox review này —
  không có Internet để `pip install pygame`). Xem `README_TEST_APP.md`.

**Không test được (và không nên cố):** phần game loop thật của `main()`
(vẽ 3D, animation, input chuột) — cần `pygame.display` thật với 1 cửa sổ
thật, không có ý nghĩa mock. Đây là giới hạn cố hữu của việc test app
pygame, không phải thiếu sót có thể vá bằng thêm test giả.

## 4. Renderer software-only (numpy, không GPU)

**Đã làm:** thêm overlay hiệu năng chẩn đoán, bật/tắt bằng phím **P**
(`show_perf` trong `main.py`) — hiện FPS thực tế (`clock.get_fps()`) và
thời gian frame vừa vẽ (ms) ở góc trên-trái, không che UI khác.

**Cố tình KHÔNG làm:** viết lại renderer bằng OpenGL/GPU. Đây là thay đổi
kiến trúc lớn, rủi ro cao (phải viết lại toàn bộ `renderer_3d.py`), và
**không cần thiết ở quy mô hiện tại** (1 cube, ~2000 quad) — software
rasterizer bằng numpy đã đủ nhanh (xem overlay P để tự đo trên máy cụ
thể). Overlay hiệu năng cho phép người dùng/reviewer TỰ kiểm chứng thay vì
chỉ tin lời khẳng định "đủ nhanh" suông.

## 5. Lời giải CFOP dài (55-93 nước) dễ bị hiểu lầm là AI "kém"

**Đã làm:** thông báo `cfop_note` sau khi auto-solve xong giờ có ghi chú
rõ AI ưu tiên tính "tự nhiên" (giống người chơi thật, Trigger-Biased
search) nên số nước **thường nhiều hơn** lời giải tối ưu tuyệt đối một
cách CÓ CHỦ Ý, không phải lỗi — xem đoạn `msgs = {...}` trong `main.py`
ngay sau khi `apply_cfop_solution()` được gọi cho `kind == 'solve'`.

## 6. (Vòng sửa thứ 2) `main.py` vẫn còn nhiều logic inline chưa test được

**Bối cảnh:** ở vòng sửa trước, tôi cố tình KHÔNG rewiring toàn bộ state
quản lý cube (state/undo_stack/move_count/timer/scrambled) ra khỏi
`main()` vì việc đó đụng tới ~100+ điểm tham chiếu rải khắp vòng lặp sự
kiện 800 dòng, rủi ro cao, **trong khi môi trường làm việc không cài được
`pygame` thật để tự kiểm chứng bằng mắt sau khi sửa** (đã thử lại
`pip install pygame` và `apt-get install python3-pygame` ở vòng này —
cả 2 đều thất bại vì sandbox không có Internet, xác nhận lại giới hạn
này vẫn tồn tại, không phải tôi bỏ qua).

**Đã làm (thỏa hiệp an toàn hơn):** thay vì di dời QUYỀN SỞ HỮU state
(rủi ro cao), tôi tách các QUYẾT ĐỊNH/TÍNH TOÁN THUẦN (không cần biết gì
về pygame, không mutate biến ngoài) ra `app_logic.py`, và cho `main.py`
gọi các hàm này thay vì lặp lại logic inline — rủi ro thấp hơn nhiều vì
mỗi thay đổi chỉ thay 1 biểu thức bằng 1 lời gọi hàm, không đổi cấu trúc
biến `nonlocal`:

- `bar_index_at_x()` — định vị con trỏ trong Singmaster bar theo toạ độ
  chuột (trước đây là `_bar_index_at_x` viết thẳng trong `main.py`,
  KHÔNG có test).
- `solve_result_message()` — nội dung thông báo sau auto-solve (trước là
  1 dict `msgs` viết tay inline, không test được cả 5 case).
- `should_retry_hint()` — logic quyết định "bấm H lại có nên xáo trộn thứ
  tự tìm kiếm không".
- `clamp_window_size()` / `rescale_zoom()` — logic resize/fullscreen giữ
  tỉ lệ zoom. **Phát hiện thêm 1 bug có sẵn** (không phải do vòng sửa này
  gây ra): phím F11 (fullscreen) gán lại `lo = make_layout()` TRƯỚC khi
  tính tỷ lệ zoom, khiến phép chia luôn vô nghĩa (`zoom/lo.ZOOM0` chia
  cho chính `ZOOM0` MỚI) — zoom giữ nguyên giá trị tuyệt đối thay vì theo
  tỷ lệ khi bật/tắt fullscreen, khác hành vi đúng ở nhánh resize cửa sổ
  thường. Đã sửa bằng cách chụp `old_zoom0` trước khi đổi `lo`, dùng
  chung `rescale_zoom()` cho nhất quán.
- `speed_idx_up()` / `speed_idx_down()` — điều khiển tốc độ animation.
- `trim_undo_stack()` — giới hạn độ sâu undo (logic `pop(0)` khi vượt
  `UNDO_MAX`).

Tổng cộng thêm **23 test mới** (`test_app_logic2.py`), tất cả THUẦN
Python — không cần `pygame` thật hay stub giả, chạy trực tiếp
`python3 test_app_logic2.py`. Đã verify `main.py` vẫn `import` sạch với
`fake_pygame_stub/` sau khi rewiring (module-level code không lỗi cú
pháp/tham chiếu).

**Còn lại (thật sự chưa giải quyết, nói thẳng):** phần SỞ HỮU state (cục
`state`/`undo_stack`/`move_count`/`timer_start`/`scrambled` là biến
`nonlocal` của `main()`, không phải thuộc tính 1 class) và toàn bộ luồng
render/animation/input 3D **vẫn y nguyên cấu trúc cũ**. Đây KHÔNG phải
tái cấu trúc kiến trúc triệt để — chỉ là kéo phần logic quyết định ra
ngoài để test được, còn "khung xương" nonlocal-heavy vẫn còn đó. Muốn
giải quyết dứt điểm cần 1 phiên làm việc có khả năng chạy `python3
main.py` thật để kiểm chứng UI bằng mắt sau mỗi bước rewiring lớn — điều
sandbox review hiện tại không đáp ứng được.

## 7. (Vòng sửa thứ 2) Độ hạt của cơ chế hủy (2048 node)

**Đã đo thực tế** (không chỉ suy đoán): huỷ ngay lập tức (set() event
ngay khi thread bắt đầu, không chờ) trên 20 scramble ngẫu nhiên, sau khi
đã "làm nóng" cache PDB — độ trễ đo được **130ms–252ms**, KHÔNG PHẢI do
khoảng cách 2048 node như tôi từng phỏng đoán, mà chủ yếu đến từ overhead
NGOÀI vòng lặp search: khởi động thread, `solve_cross()` luôn chạy hết
(≤20 bước, không có checkpoint hủy — nhưng đã bounded nên không rủi ro),
và build PDB theo cặp F2L cụ thể nếu scramble đó chưa từng cache case
tương ứng.

**Đã làm:** vẫn giảm `_CANCEL_CHECK_EVERY` từ 2048 → 512 (ở cả
`search_utils.py` và `f2l_solver.py`) làm biện pháp phòng thủ cho các pha
sâu hơn (OLL/PLL IDA* có thể tốn nhiều công việc/node hơn F2L), đã verify
KHÔNG làm chậm thời gian giải bình thường (chạy lại `test_solver.py`:
10.5s so với 10.6s trước đó, sai khác trong nhiễu đo đạc bình thường).

**Không làm** (và giải thích tại sao): không thêm checkpoint hủy vào
`solve_cross()` vì nó đã bounded chặt (`max_moves=20`, ~microsecond mỗi
bước) — thêm `check_cancel()` vào đó sẽ chỉ tăng độ phức tạp mà gần như
không giảm được gì trong 130-250ms đã đo (phần lớn latency không nằm ở
đó). Nói cách khác: đã đo trước khi "tối ưu" để tránh sửa nhầm chỗ không
phải nút thắt cổ chai thật sự.

---

**Tổng kết kiểm chứng:** toàn bộ thay đổi trên (cả 2 vòng sửa) đã chạy
qua: `test_cube_engine.py` (51 test), `test_solver.py` (50 test),
`test_cancellation.py` (8 test), `test_app_logic.py` (11 test),
`test_app_logic2.py` (23 test) — **143/143 pass**, không có test cũ nào
bị hỏng do các thay đổi này (đã chạy thật qua `/tmp/runtests.py` vì môi
trường review không cài được pytest thật, và đã thử lại `pip install
pygame`/`apt-get install python3-pygame` ở vòng 2 để xác nhận vẫn không
có Internet — không phải khẳng định suông).

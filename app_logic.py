"""
app_logic.py
=============
Các hàm LOGIC THUẦN tách ra từ main.py: không đụng pygame.display/Surface,
không sở hữu state (không mutate biến ngoài trừ tham số truyền vào rõ
ràng) -- test được độc lập, xem test_app_state_logic.py.

Đây là bước tiếp theo của việc tách main.py (sau formula_panel.py, xem
REFACTOR_NOTES.md mục 1). KHÔNG di chuyển quyền SỞ HỮU state (cube state,
undo_stack, timer...) ra khỏi main() -- việc đó đòi hỏi rewiring ~100+
điểm tham chiếu rải khắp vòng lặp sự kiện 800 dòng, rủi ro cao trong khi
môi trường phát triển hiện tại KHÔNG cài được pygame thật để tự kiểm
chứng bằng mắt sau khi sửa (xem README_TEST_APP.md). Thay vào đó, các
QUYẾT ĐỊNH/TÍNH TOÁN thuần túy (không cần biết gì về pygame) được kéo ra
đây để test được, còn main.py giữ nguyên cấu trúc nonlocal hiện tại và
chỉ GỌI các hàm này thay vì lặp lại logic inline.
"""


def bar_index_at_x(text, mouse_x, text_start_x, font_size_fn):
    """Trả về vị trí ký tự (0..len(text)) gần nhất với toạ độ x của chuột,
    dùng khi bấm/kéo chuột trong Singmaster bar để đặt con trỏ/chọn text.

    font_size_fn: hàm nhận 1 str, trả về (width, height) -- thường là
    `font.size` của pygame.font.Font, nhưng để THUẦN/test được không cần
    pygame, hàm này nhận thẳng callable thay vì đối tượng Font.
    """
    best_i = len(text)
    best_dist = abs(text_start_x + font_size_fn(text)[0] - mouse_x)
    for i in range(len(text) + 1):
        w = font_size_fn(text[:i])[0]
        dist = abs(text_start_x + w - mouse_x)
        if dist < best_dist:
            best_dist = dist
            best_i = i
    return best_i


# ── Thông báo kết quả auto-solve (phím A) ────────────────────────────────
# Ghi chú "ưu tiên tự nhiên" -- xem REFACTOR_NOTES.md: AI dùng CFOP kiểu
# người chơi thật (Trigger-Biased search) nên số nước THƯỜNG NHIỀU HƠN
# lời giải tối ưu (God's Number ~20) -- đây là CHỦ ĐÍCH, không phải AI
# "kém". Ghi rõ trong thông báo để người dùng không hiểu lầm.
def solve_result_message(res):
    """Trả về (text, is_generic) cho thông báo hiển thị sau khi 1 job
    'solve' (auto-solve, phím A) trả kết quả. `res` là dict trả về từ
    cfop_ai.full_solve() (đã có key 'reached' và 'all_moves').
    is_generic=True nếu 'reached' không khớp case nào đã biết (fallback)."""
    n = len(res['all_moves'])
    msgs = {
        'solved':      f"AI: 🎉 GIẢI XONG cả cube ({n} nước, ưu tiên tự nhiên chứ không phải tối thiểu)!",
        'pll_partial': "AI: xong Cross+F2L+OLL, PLL còn dở (case khó, thử lại 'A')",
        'oll_partial': "AI: xong Cross+F2L, OLL còn dở (case khó, thử lại 'A')",
        'f2l_done':    f"AI: đã giải Cross + F2L ({n} nước)",
        'f2l_partial': "AI: giải được Cross + một phần F2L (cặp khó, thử lại 'A' lần nữa)",
    }
    reached = res.get('reached')
    if reached in msgs:
        return msgs[reached], False
    return f"AI: đã đi {n} nước", True


# ── Xác định có nên "thử lại" (retry=True, xáo trộn thứ tự search) khi ──
#    người dùng bấm Hint (H) hay không ─────────────────────────────────────
def should_retry_hint(last_failed, last_stage, last_move_count, cur_stage, cur_move_count):
    """True nếu người dùng bấm H lại cho CÙNG 1 giai đoạn vừa thất bại mà
    cube CHƯA đổi gì (move_count không đổi) -- tìm kiếm vốn tất định nên
    chỉ 'thử lại' có ý nghĩa khi xáo trộn thứ tự duyệt (retry=True), gọi
    lại y hệt với cùng state/thứ tự sẽ cho kết quả GIỐNG HỆT lần trước."""
    return bool(last_failed and last_stage == cur_stage and last_move_count == cur_move_count)


# ── Resize cửa sổ: giữ nguyên tỉ lệ zoom hiện tại khi Layout đổi ─────────
def clamp_window_size(req_w, req_h, min_w, min_h):
    """Trả về (w, h) đã kẹp về [min_w, min_h] tối thiểu -- tránh UI vỡ hình
    khi người dùng kéo cửa sổ quá nhỏ."""
    return max(min_w, req_w), max(min_h, req_h)


def rescale_zoom(old_zoom, old_zoom0, new_zoom0):
    """Giữ NGUYÊN tỉ lệ zoom/zoom0 hiện tại khi zoom0 đổi (vd sau resize
    cửa sổ, Layout tính lại zoom0 theo kích thước mới) -- tránh cube đột
    ngột to/nhỏ khi resize."""
    if old_zoom0 == 0:
        return new_zoom0
    ratio = old_zoom / old_zoom0
    return new_zoom0 * ratio


# ── Điều khiển tốc độ animation (phím [ và ]) ────────────────────────────
# ── Cuộn ngang thanh Singmaster bar theo con trỏ ─────────────────────────
def bar_scroll_offset(text, cursor_pos, available_w, font_size_fn):
    """Trả về offset ngang (px) sao cho con trỏ (cursor_pos) LUÔN nằm
    trong vùng nhìn thấy rộng `available_w`, giống hành vi ô nhập liệu
    thông thường -- khắc phục lỗi chuỗi công thức dài tràn ra ngoài thanh
    bar (khi cursor_pos=None, coi như cursor ở cuối text -- tự nhiên hiện
    phần ĐUÔI chuỗi, thường là phần vừa gõ/dán vào).

    font_size_fn: callable nhận str, trả (width, height) -- xem
    bar_index_at_x() ở trên về lý do dùng callable thay vì đối tượng Font.
    """
    if cursor_pos is None:
        cursor_pos = len(text)
    full_w = font_size_fn(text)[0]
    if full_w <= available_w:
        return 0
    cursor_w = font_size_fn(text[:cursor_pos])[0]
    if cursor_w > available_w:
        return cursor_w - available_w
    return 0


def truncate_with_ellipsis(text, max_w, font_size_fn, ellipsis='…'):
    """Cắt `text` (nếu cần) sao cho vừa `max_w` khi render, thêm `ellipsis`
    ở cuối nếu bị cắt. Dùng cho vùng CHIỀU CAO CỐ ĐỊNH (không wrap được,
    vd hộp gợi ý AI) -- khác với wrap_text() trong formula_panel.py (dùng
    cho vùng có thể cuộn dọc). Trả về (text_da_cat, bi_cat: bool)."""
    if font_size_fn(text)[0] <= max_w:
        return text, False
    ell_w = font_size_fn(ellipsis)[0]
    budget = max_w - ell_w
    if budget <= 0:
        return ellipsis, True
    lo_i, hi_i = 0, len(text)
    while lo_i < hi_i:
        mid = (lo_i + hi_i + 1) // 2
        if font_size_fn(text[:mid])[0] <= budget:
            lo_i = mid
        else:
            hi_i = mid - 1
    return text[:lo_i] + ellipsis, True


def speed_idx_down(idx):
    """Giảm 1 bậc tốc độ, kẹp ở 0 (không âm)."""
    return max(0, idx - 1)


def speed_idx_up(idx, n_steps):
    """Tăng 1 bậc tốc độ, kẹp ở n_steps-1 (chỉ số hợp lệ cuối cùng)."""
    return min(n_steps - 1, idx + 1)


# ── Giới hạn độ sâu undo_stack (dùng trong push_undo()) ──────────────────
def trim_undo_stack(stack, max_len):
    """Cắt bớt PHẦN ĐẦU của `stack` (các bản snapshot cũ nhất) nếu vượt
    quá `max_len`, MUTATE `stack` tại chỗ (giống list.pop(0) gốc) để giữ
    nguyên hành vi/độ phức tạp bộ nhớ như code cũ trong main.py. Trả về
    chính `stack` để tiện dùng dạng biểu thức nếu cần."""
    while len(stack) > max_len:
        stack.pop(0)
    return stack

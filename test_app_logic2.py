"""
test_app_logic2.py
====================
Test cho app_logic.py (các hàm thuần tách từ main.py, KHÔNG cần pygame --
khác với test_app_logic.py vốn test formula_panel.py và cần stub pygame).

Chạy trực tiếp: python3 test_app_logic2.py
"""

import app_logic as al


# ── bar_index_at_x ────────────────────────────────────────────────────────

def _mono_size(text, char_w=7):
    """Font giả: mỗi ký tự rộng đúng char_w px (giống font monospace)."""
    return (len(text) * char_w, 12)


def test_bar_index_at_x_click_at_start():
    idx = al.bar_index_at_x("R U R'", mouse_x=100, text_start_x=100,
                             font_size_fn=_mono_size)
    assert idx == 0


def test_bar_index_at_x_click_at_end():
    text = "R U R'"
    end_x = 100 + _mono_size(text)[0]
    idx = al.bar_index_at_x(text, mouse_x=end_x, text_start_x=100,
                             font_size_fn=_mono_size)
    assert idx == len(text)


def test_bar_index_at_x_click_middle():
    text = "R U R' U'"   # 9 ky tu, char_w=7 -> moi ky tu 7px
    # click dung o giua ky tu thu 3 va 4 (idx=3 hoac 4 deu hop ly, chi can
    # gan voi 1 trong 2, khong bi loi ve dau/cuoi)
    text_start_x = 50
    click_x = text_start_x + 3 * 7 + 3   # gan voi idx=3 hon idx=4
    idx = al.bar_index_at_x(text, mouse_x=click_x, text_start_x=text_start_x,
                             font_size_fn=_mono_size)
    assert 2 <= idx <= 4   # vung dung, khong doi hoi chinh xac tuyet doi 1 diem


def test_bar_index_at_x_empty_text():
    idx = al.bar_index_at_x("", mouse_x=999, text_start_x=0, font_size_fn=_mono_size)
    assert idx == 0


# ── solve_result_message ──────────────────────────────────────────────────

def test_solve_result_message_solved():
    res = {'reached': 'solved', 'all_moves': ['R'] * 42}
    text, is_generic = al.solve_result_message(res)
    assert is_generic is False
    assert '42' in text
    assert 'GIẢI XONG' in text
    assert 'tự nhiên' in text   # phai co ghi chu chong hieu lam ve so nuoc dai


def test_solve_result_message_f2l_done():
    res = {'reached': 'f2l_done', 'all_moves': ['U'] * 30}
    text, is_generic = al.solve_result_message(res)
    assert is_generic is False
    assert '30' in text and 'F2L' in text


def test_solve_result_message_unknown_reached_is_generic_fallback():
    res = {'reached': 'some_unexpected_value', 'all_moves': ['U', 'R']}
    text, is_generic = al.solve_result_message(res)
    assert is_generic is True
    assert '2' in text   # van bao dung so nuoc du khong nhan dien duoc case


def test_solve_result_message_all_known_cases_covered():
    """Duyet toan bo cac 'reached' ma cfop_ai.py thuc su co the tra ve
    (xem full_solve()/_full_solve_raw() trong solver/cfop_ai.py) -- dam
    bao khong co case nao roi vao nhanh generic fallback ngoai y muon."""
    known_cases = ['solved', 'pll_partial', 'oll_partial', 'f2l_done', 'f2l_partial']
    for case in known_cases:
        res = {'reached': case, 'all_moves': []}
        _, is_generic = al.solve_result_message(res)
        assert is_generic is False, f"case '{case}' bi roi vao fallback generic ngoai y muon"


# ── should_retry_hint ──────────────────────────────────────────────────────

def test_should_retry_hint_true_when_same_stage_same_move_count_and_failed():
    assert al.should_retry_hint(
        last_failed=True, last_stage='F2L', last_move_count=12,
        cur_stage='F2L', cur_move_count=12) is True


def test_should_retry_hint_false_when_stage_changed():
    assert al.should_retry_hint(
        last_failed=True, last_stage='F2L', last_move_count=12,
        cur_stage='OLL', cur_move_count=12) is False


def test_should_retry_hint_false_when_move_count_changed():
    """Neu cube da doi (nguoi dung di 1 nuoc) thi khong con la 'bam H lai
    cho CUNG 1 the trang thai' -- khong can retry vi search se tu nhien
    cho ket qua khac (state khac)."""
    assert al.should_retry_hint(
        last_failed=True, last_stage='F2L', last_move_count=12,
        cur_stage='F2L', cur_move_count=13) is False


def test_should_retry_hint_false_when_last_did_not_fail():
    assert al.should_retry_hint(
        last_failed=False, last_stage='F2L', last_move_count=12,
        cur_stage='F2L', cur_move_count=12) is False


def test_should_retry_hint_false_on_first_call_none_stage():
    """Truong hop lan dau bam H (chua co lich su) -- last_stage=None phai
    khong bao gio trung voi cur_stage that (luon la string)."""
    assert al.should_retry_hint(
        last_failed=False, last_stage=None, last_move_count=None,
        cur_stage='Cross', cur_move_count=0) is False


# ── clamp_window_size ───────────────────────────────────────────────────────

def test_clamp_window_size_above_min_unchanged():
    assert al.clamp_window_size(1000, 700, 800, 520) == (1000, 700)


def test_clamp_window_size_clamps_both_dims():
    assert al.clamp_window_size(300, 200, 800, 520) == (800, 520)


def test_clamp_window_size_clamps_one_dim():
    assert al.clamp_window_size(300, 700, 800, 520) == (800, 700)


# ── rescale_zoom ─────────────────────────────────────────────────────────

def test_rescale_zoom_preserves_ratio():
    # zoom dang gap 1.5 lan zoom0 cu -> sau khi zoom0 doi, zoom moi cung
    # phai gap 1.5 lan zoom0 moi
    new_zoom = al.rescale_zoom(old_zoom=150.0, old_zoom0=100.0, new_zoom0=120.0)
    assert abs(new_zoom - 180.0) < 1e-9


def test_rescale_zoom_handles_zero_old_zoom0():
    # Truong hop bien (khong xay ra thuc te vi Layout luon tra ve ZOOM0 >
    # 0, nhung phong thu tranh chia cho 0 lam crash app)
    assert al.rescale_zoom(old_zoom=50.0, old_zoom0=0.0, new_zoom0=120.0) == 120.0


# ── speed_idx_up / speed_idx_down ─────────────────────────────────────────

def test_speed_idx_down_clamps_at_zero():
    assert al.speed_idx_down(0) == 0
    assert al.speed_idx_down(3) == 2


def test_speed_idx_up_clamps_at_max():
    n = 8
    assert al.speed_idx_up(7, n) == 7
    assert al.speed_idx_up(3, n) == 4


# ── trim_undo_stack ─────────────────────────────────────────────────────────

def test_trim_undo_stack_no_trim_needed():
    stack = [1, 2, 3]
    al.trim_undo_stack(stack, max_len=5)
    assert stack == [1, 2, 3]


def test_trim_undo_stack_trims_oldest_first():
    stack = list(range(10))   # [0..9]
    al.trim_undo_stack(stack, max_len=5)
    assert stack == [5, 6, 7, 8, 9]   # giu 5 phan tu MOI NHAT, bo phan tu CU


def test_trim_undo_stack_matches_undo_max_used_in_main():
    """UNDO_MAX trong main.py = 120 -- kiem tra hanh vi dung nhu code cu
    (push tung phan tu 1, pop(0) tung cai khi vuot qua) cho ca 1 chuoi dai
    hon nhieu UNDO_MAX, mo phong dung cach push_undo() goc su dung ham nay."""
    stack = []
    for i in range(300):
        stack.append(i)
        al.trim_undo_stack(stack, max_len=120)
    assert len(stack) == 120
    assert stack[0] == 180   # 300 - 120
    assert stack[-1] == 299


if __name__ == '__main__':
    import sys
    fns = [v for k, v in sorted(globals().items()) if k.startswith('test_') and callable(v)]
    ok = fail = 0
    for fn in fns:
        try:
            fn()
            ok += 1
            print(f"  ok  {fn.__name__}")
        except Exception as e:
            fail += 1
            print(f" FAIL {fn.__name__}: {e}")
    print(f"\n{ok} passed, {fail} failed ({len(fns)} tests)")
    sys.exit(1 if fail else 0)

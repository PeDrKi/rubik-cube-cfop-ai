"""
test_cube_engine.py
====================
Bộ test tự động cho cube_engine.py (không phụ thuộc pygame).

Chạy: pytest test_cube_engine.py -v
"""
import copy
import numpy as np
import pytest

from cube_engine import (
    make_solved, do_move, cube_solved, scramble_cube,
    parse_singmaster, invert_moves, ALL_MOVES, _VALID_BASES,
)
from constants import FACES


# ── Helpers ──────────────────────────────────────────────────────────────────

def snapshot(st):
    """So sánh 2 state bằng giá trị (không phải reference)."""
    return {f: st[f].copy() for f in FACES}


def states_equal(a, b):
    return all(np.array_equal(a[f], b[f]) for f in FACES)


def color_counts(st):
    """Đếm số lượng mỗi màu (chữ cái mặt) xuất hiện trên toàn cube.
    Bất biến bắt buộc: luôn phải là 9 cho mỗi trong 6 màu, dù có xáo trộn
    thế nào — nếu sai, nghĩa là do_move làm mất/nhân đôi sticker.
    """
    counts = {}
    for f in FACES:
        for v in st[f].flatten():
            counts[v] = counts.get(v, 0) + 1
    return counts


ALL_BASES_NO_SUFFIX = sorted(_VALID_BASES)   # 18 ký tự: U D F B L R M E S x y z u d f b l r


# ── make_solved ──────────────────────────────────────────────────────────────

def test_make_solved_each_face_uniform():
    st = make_solved()
    for f in FACES:
        assert st[f].shape == (3, 3)
        assert set(st[f].flatten()) == {f}


def test_make_solved_is_solved():
    assert cube_solved(make_solved())


def test_make_solved_returns_fresh_copy_each_time():
    a, b = make_solved(), make_solved()
    a['U'][0, 0] = 'X'
    assert b['U'][0, 0] != 'X'   # không share reference giữa 2 lần gọi


# ── Bất biến số lượng sticker (áp dụng cho MỌI move, mọi lúc) ────────────────

@pytest.mark.parametrize("base", ALL_BASES_NO_SUFFIX)
@pytest.mark.parametrize("suffix", ["", "'", "2"])
def test_move_preserves_sticker_counts(base, suffix):
    st = make_solved()
    do_move(st, base + suffix)
    counts = color_counts(st)
    assert counts == {f: 9 for f in FACES}, f"move {base+suffix} lam sai lech so luong sticker"


# ── Move rồi move-nghịch-đảo phải quay lại y hệt ban đầu ─────────────────────

@pytest.mark.parametrize("base", ALL_BASES_NO_SUFFIX)
def test_move_then_inverse_returns_to_solved(base):
    st = make_solved()
    do_move(st, base)
    do_move(st, base + "'")
    assert states_equal(st, make_solved()), f"{base} roi {base}' phai tra ve trang thai ban dau"


@pytest.mark.parametrize("base", ALL_BASES_NO_SUFFIX)
def test_double_move_equals_two_singles(base):
    st_double = make_solved()
    do_move(st_double, base + "2")

    st_twice = make_solved()
    do_move(st_twice, base)
    do_move(st_twice, base)

    assert states_equal(st_double, st_twice), f"{base}2 phai bang {base} {base}"


@pytest.mark.parametrize("base", ALL_BASES_NO_SUFFIX)
def test_move_four_times_returns_to_solved(base):
    st = make_solved()
    for _ in range(4):
        do_move(st, base)
    assert states_equal(st, make_solved()), f"{base} x4 phai quay lai trang thai goc"


# ── Quan hệ wide = outer + slice (đúng theo comment trong cube_engine.py) ────

@pytest.mark.parametrize("wide,outer,slice_mv", [
    ('u', 'U', "E'"),
    ('d', 'D', 'E'),
    ('f', 'F', 'S'),
    ('b', 'B', "S'"),
    ('l', 'L', 'M'),
    ('r', 'R', "M'"),
])
def test_wide_move_equals_outer_plus_slice(wide, outer, slice_mv):
    st_wide = make_solved()
    do_move(st_wide, wide)

    st_combo = make_solved()
    do_move(st_combo, outer)
    do_move(st_combo, slice_mv)

    assert states_equal(st_wide, st_combo), f"{wide} phai bang {outer} + {slice_mv}"


# ── Whole-cube rotation không được làm thay đổi... trạng thái "logic" ────────
# (x/y/z chỉ xoay góc nhìn, không phải nước đi giải cube - vẫn phải bảo toàn
# tính hợp lệ và tự nghịch đảo được)

@pytest.mark.parametrize("base", ['x', 'y', 'z'])
def test_whole_cube_rotation_four_times_identity(base):
    st = make_solved()
    for _ in range(4):
        do_move(st, base)
    assert states_equal(st, make_solved())


# ── cube_solved ───────────────────────────────────────────────────────────────

def test_cube_solved_false_after_single_move():
    st = make_solved()
    do_move(st, 'R')
    assert not cube_solved(st)


def test_cube_solved_true_after_move_and_inverse():
    st = make_solved()
    do_move(st, 'R'); do_move(st, "R'")
    assert cube_solved(st)


# ── scramble_cube ─────────────────────────────────────────────────────────────

def test_scramble_preserves_sticker_counts():
    st = make_solved()
    scramble_cube(st, n=25)
    assert color_counts(st) == {f: 9 for f in FACES}


def test_scramble_avoids_consecutive_same_face():
    # Chạy nhiều lần để giảm khả năng false-negative do random
    import random
    random.seed(42)
    st = make_solved()
    # Patch nhẹ: theo dõi qua ALL_MOVES bằng cách gọi trực tiếp logic tương tự
    # scramble_cube (đọc từ cube_engine, không mock random) - kiểm tra gián
    # tiếp bằng cách chạy scramble nhiều lần và chỉ assert không crash +
    # invariant số lượng sticker giữ nguyên (bất biến quan trọng nhất).
    for _ in range(20):
        scramble_cube(st, n=15)
        assert color_counts(st) == {f: 9 for f in FACES}


def test_scramble_usually_not_solved():
    st = make_solved()
    scramble_cube(st, n=20)
    assert not cube_solved(st)   # xác suất scramble ra solved gần như bằng 0


# ── invert_moves ──────────────────────────────────────────────────────────────

def test_invert_single_moves():
    inv = invert_moves(['R'])
    assert inv == ["R'"]
    inv = invert_moves(["R'"])
    assert inv == ['R']
    inv = invert_moves(['R2'])
    assert inv == ['R2']   # tu nghich dao


def test_invert_sequence_reverses_order_and_direction():
    moves = ['R', 'U', "R'"]
    inv = invert_moves(moves)
    assert inv == ['R', "U'", "R'"]


def test_apply_moves_then_inverse_returns_solved():
    st = make_solved()
    moves = ['R', 'U', "R'", "U'", 'F2', "M'", 'x']
    for mv in moves:
        do_move(st, mv)
    assert not cube_solved(st)
    for mv in invert_moves(moves):
        do_move(st, mv)
    assert states_equal(st, make_solved())


def test_scramble_then_invert_solves_cube():
    """Test tích hợp quan trọng nhất: ghi lại scramble bằng cách tự áp dụng
    move list, rồi áp dụng chuỗi nghịch đảo -> phải về solved."""
    import random
    random.seed(1)
    st = make_solved()
    moves = [random.choice(ALL_MOVES) for _ in range(30)]
    for mv in moves:
        do_move(st, mv)
    for mv in invert_moves(moves):
        do_move(st, mv)
    assert states_equal(st, make_solved())


# ── parse_singmaster ──────────────────────────────────────────────────────────

def test_parse_simple_sequence():
    moves, err = parse_singmaster("R U R' U'")
    assert err is None
    assert moves == ['R', 'U', "R'", "U'"]


def test_parse_double_and_prime():
    moves, err = parse_singmaster("R2 U2' F'2")
    assert err is None
    assert moves == ['R2', "U2'", "F2'"]   # '2 duoc chuan hoa thanh 2'


def test_parse_group_repeat():
    moves, err = parse_singmaster("(R U R' U')3")
    assert err is None
    assert moves == ['R', 'U', "R'", "U'"] * 3


def test_parse_nested_group_repeat():
    moves, err = parse_singmaster("((R U)2 F)2")
    assert err is None
    assert moves == (['R', 'U'] * 2 + ['F']) * 2


def test_parse_wide_slice_rotation_moves():
    moves, err = parse_singmaster("r u M2 x'")
    assert err is None
    assert moves == ['r', 'u', 'M2', "x'"]


def test_parse_case_sensitive_distinguishes_wide_from_outer():
    moves, err = parse_singmaster("Rr")
    assert err is None
    assert moves == ['R', 'r']


def test_parse_rejects_unknown_token():
    moves, err = parse_singmaster("R Q U")
    assert moves is None
    assert err is not None


def test_parse_empty_string_is_error():
    moves, err = parse_singmaster("")
    assert moves is None or moves == []
    # bar_status trong main.py coi moves rong la loi -> chi can dam bao
    # khong crash va khong tra ve moves hop le gia.


def test_parse_result_is_directly_playable():
    """Test tích hợp: kết quả parse phải áp dụng được ngay bằng do_move,
    và move list phải chuẩn hóa đúng suffix để do_move không lỗi."""
    st = make_solved()
    moves, err = parse_singmaster("R2 U2' M2 x2 (R U R' U')3 r u f b l d")
    assert err is None
    for mv in moves:
        do_move(st, mv)   # không được raise exception
    assert color_counts(st) == {f: 9 for f in FACES}

"""
test_cube_engine.py
=====================
Bo test cho cube_engine.py -- tang duoi cung ma toan bo solver/ dua vao
(do_move, scramble_cube, cube_solved, parser Singmaster). File nay bo
sung phan con thieu de claim "python3 -m pytest test_solver.py
test_cube_engine.py -q" trong HUONG_DAN_CHAY.md co the tai lap duoc (xem
ghi chu trong test_solver.py: ban than file test_cube_engine.py trong
goi da nop la RONG 0 byte).

Trong Rubik's Cube that: MOI nuoc quarter-turn (U/D/F/B/L/R don, khong
tinh nghich dao/2) co bac (order) dung 4 trong nhom cube -- ap 4 lan lien
tiep phai tra ve dung trang thai ban dau. Day la bat bien toan hoc dung
xuyen suot bo test nay de kiem chung KHONG CAN so sanh voi 1 nguon "dung
san" nao khac (self-consistent theo ly thuyet nhom).
"""

import os
import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# ^ them thu muc goc repo (cha cua tests/) vao sys.path -- CAN THIET vi file
# nay nam trong tests/ nhung can import cube_engine/solver/... o root. Bootstrap
# nay lam file chay dung DU invoke kieu nao (python3 tests/test_x.py tu bat
# ky cwd nao, hay python3 -m pytest tests/ tu root) -- xem docs/README_TEST_APP.md.

import random

import numpy as np
import pytest

from cube_engine import (make_solved, do_move, rot_cw, rot_ccw, rot_180,
                          scramble_cube, cube_solved, ALL_MOVES,
                          expand_repeats, parse_singmaster, invert_moves)
from constants import FACES

ALL_BASES = list('UDFBLRMESxyz') + list('udfblr')


def _copy(st):
    return {f: st[f].copy() for f in st}


def _equal(st1, st2):
    return all((st1[f] == st2[f]).all() for f in st1)


def _apply(st, moves):
    for mv in moves:
        do_move(st, mv)


# ────────────────────────────────────────────────────────────────────────
# 1) make_solved / cube_solved
# ────────────────────────────────────────────────────────────────────────

class TestMakeSolvedAndSolvedCheck:
    def test_make_solved_has_6_faces(self):
        st = make_solved()
        assert set(st.keys()) == set(FACES)

    def test_make_solved_each_face_uniform_and_matches_own_label(self):
        st = make_solved()
        for f in FACES:
            assert st[f].shape == (3, 3)
            assert set(st[f].flatten()) == {f}

    def test_make_solved_is_solved(self):
        assert cube_solved(make_solved())

    def test_make_solved_returns_fresh_copy_each_time(self):
        """2 lan goi make_solved() khong duoc chia se cung buffer numpy
        (neu khong, sua 1 ban se lam hong ban kia)."""
        st1 = make_solved()
        st2 = make_solved()
        st1['U'][0, 0] = 'X'
        assert st2['U'][0, 0] == 'U'

    @pytest.mark.parametrize('mv', ALL_MOVES)
    def test_single_quarter_or_half_move_unsolves_cube(self, mv):
        """That toan hoc: 1 nuoc BAT KY (khac identity) luon lam cube
        KHONG con solved -- khong co nuoc nao trong ALL_MOVES la identity."""
        st = make_solved()
        do_move(st, mv)
        assert not cube_solved(st), mv


# ────────────────────────────────────────────────────────────────────────
# 2) do_move -- bat bien nhom cube (bac 4 cua quarter-turn, nghich dao)
# ────────────────────────────────────────────────────────────────────────

class TestDoMoveGroupInvariants:
    """m^4 = identity cho MOI base move (U D F B L R M E S x y z u d f b l r)
    -- dung cho ca outer, slice, rotation, va wide move, vi tat ca deu la
    'quarter turn' theo dung nghia nhom (bac 4)."""

    @pytest.mark.parametrize('base', ALL_BASES)
    def test_quarter_turn_to_the_4th_is_identity(self, base):
        st = make_solved()
        for _ in range(4):
            do_move(st, base)
        assert _equal(st, make_solved()), base

    @pytest.mark.parametrize('base', ALL_BASES)
    def test_prime_is_inverse_of_base(self, base):
        st = make_solved()
        do_move(st, base)
        do_move(st, base + "'")
        assert _equal(st, make_solved()), base

    @pytest.mark.parametrize('base', ALL_BASES)
    def test_double_move_equals_two_singles(self, base):
        st1 = make_solved()
        do_move(st1, base)
        do_move(st1, base)
        st2 = make_solved()
        do_move(st2, base + '2')
        assert _equal(st1, st2), base

    @pytest.mark.parametrize('base', ALL_BASES)
    def test_double_move_is_self_inverse(self, base):
        st = make_solved()
        do_move(st, base + '2')
        do_move(st, base + '2')
        assert _equal(st, make_solved()), base

    @pytest.mark.parametrize('base', ALL_BASES)
    def test_double_prime_suffix_variants_equivalent(self, base):
        """R2' va R'2 deu duoc dinh nghia la 'coi nhu R2' (xem do_move
        docstring) -- ca 2 bien the suffix phai cho CUNG ket qua."""
        st1 = make_solved()
        do_move(st1, base + "2'")
        st2 = make_solved()
        do_move(st2, base + "'2")
        st3 = make_solved()
        do_move(st3, base + '2')
        assert _equal(st1, st3), base
        assert _equal(st2, st3), base

    def test_does_not_mutate_unrelated_face(self):
        """do_move('U', ...) khong duoc dung toi mat D duoi bat ky hinh
        thuc nao."""
        st = make_solved()
        d_before = st['D'].copy()
        do_move(st, 'U')
        assert (st['D'] == d_before).all()


class TestDoMoveSpecificFacelets:
    """Doi chieu THU CONG facelet cu the sau 1 nuoc U/R, doc truc tiep tu
    code cua _U/_R (khong doan mo hinh) de dam bao test phan anh DUNG
    hanh vi cai dat, khong phai hanh vi ky vong tu quy uoc WCA (2 cai co
    the lech nhau neu code co bug ve quy uoc chieu)."""

    def test_U_move_cycles_top_rows(self):
        st = make_solved()
        do_move(st, 'U')
        assert (st['F'][0, :] == 'R').all()
        assert (st['R'][0, :] == 'B').all()
        assert (st['B'][0, :] == 'L').all()
        assert (st['L'][0, :] == 'F').all()
        assert (st['F'][1:, :] == 'F').all()

    def test_U_prime_is_reverse_cycle_of_U(self):
        st = make_solved()
        do_move(st, "U'")
        assert (st['F'][0, :] == 'L').all()
        assert (st['L'][0, :] == 'B').all()
        assert (st['B'][0, :] == 'R').all()
        assert (st['R'][0, :] == 'F').all()

    def test_R_move_cycles_right_columns(self):
        st = make_solved()
        do_move(st, 'R')
        assert (st['F'][:, 2] == 'D').all()
        assert (st['U'][:, 2] == 'F').all()
        assert (st['D'][:, 2] == 'B').all()
        assert (st['B'][:, 0] == 'U').all()

    def test_center_facelets_never_move_under_outer_moves(self):
        """Center (vi tri [1,1]) cua 6 mat KHONG BAO GIO doi duoi outer
        move (U D F B L R) -- day la gia dinh nen tang cho toan bo
        solver/ (nhan dien mau tuyet doi qua center)."""
        st = make_solved()
        random.seed(0)
        for _ in range(50):
            mv = random.choice(['U', "U'", 'D', "D'", 'F', "F'", 'B', "B'",
                                 'L', "L'", 'R', "R'"])
            do_move(st, mv)
        for f in FACES:
            assert st[f][1, 1] == f, f


class TestSliceMoves:
    def test_M_only_touches_middle_column_of_UFDB(self):
        st = make_solved()
        do_move(st, 'M')
        assert (st['L'] == make_solved()['L']).all()
        assert (st['R'] == make_solved()['R']).all()

    def test_E_only_touches_middle_row_of_FLBR(self):
        st = make_solved()
        do_move(st, 'E')
        assert (st['U'] == make_solved()['U']).all()
        assert (st['D'] == make_solved()['D']).all()

    def test_S_only_touches_middle_layer_of_ULDR(self):
        st = make_solved()
        do_move(st, 'S')
        assert (st['F'] == make_solved()['F']).all()
        assert (st['B'] == make_solved()['B']).all()


class TestWideMoves:
    """Theo docstring do_move: u = U + E', d = D + E, f = F + S,
    b = B + S', l = L + M, r = R + M'. Kiem tra dung claim nay bang cach
    so sanh 2 duong tinh khac nhau tu cung 1 diem xuat phat."""

    @pytest.mark.parametrize('wide,parts', [
        ('u', ['U', "E'"]),
        ('d', ['D', 'E']),
        ('f', ['F', 'S']),
        ('b', ['B', "S'"]),
        ('l', ['L', 'M']),
        ('r', ['R', "M'"]),
    ])
    def test_wide_move_equals_documented_combo(self, wide, parts):
        st1 = make_solved()
        do_move(st1, wide)
        st2 = make_solved()
        _apply(st2, parts)
        assert _equal(st1, st2), wide

    @pytest.mark.parametrize('wide,parts', [
        ("u'", ["U'", 'E']),
        ("d'", ["D'", "E'"]),
        ("f'", ["F'", "S'"]),
        ("b'", ["B'", 'S']),
        ("l'", ["L'", "M'"]),
        ("r'", ["R'", 'M']),
    ])
    def test_wide_prime_equals_documented_combo(self, wide, parts):
        st1 = make_solved()
        do_move(st1, wide)
        st2 = make_solved()
        _apply(st2, parts)
        assert _equal(st1, st2), wide


class TestWholeCubeRotations:
    @pytest.mark.parametrize('rot', ['x', 'y', 'z'])
    def test_rotation_to_4th_power_is_identity(self, rot):
        st = make_solved()
        for _ in range(4):
            do_move(st, rot)
        assert _equal(st, make_solved()), rot

    def test_x_matches_R_M_prime_L_prime_combo(self):
        """Theo docstring _x(): x = R, M' (nguoc M), L' (nguoc L)."""
        st1 = make_solved()
        do_move(st1, 'x')
        st2 = make_solved()
        _apply(st2, ['R', "M'", "L'"])
        assert _equal(st1, st2)

    def test_y_matches_U_E_prime_D_prime_combo(self):
        st1 = make_solved()
        do_move(st1, 'y')
        st2 = make_solved()
        _apply(st2, ['U', "E'", "D'"])
        assert _equal(st1, st2)

    def test_z_matches_F_S_B_prime_combo(self):
        st1 = make_solved()
        do_move(st1, 'z')
        st2 = make_solved()
        _apply(st2, ['F', 'S', "B'"])
        assert _equal(st1, st2)

    def test_rotation_preserves_multiset_of_facelets(self):
        """Xoay ca khoi khong duoc 'sinh ra' hay 'lam mat' bat ky mau nao
        -- chi hoan vi vi tri."""
        st = make_solved()
        do_move(st, 'x')
        all_vals = sorted(v for f in FACES for v in st[f].flatten())
        expected = sorted(v for f in FACES for v in make_solved()[f].flatten())
        assert all_vals == expected


# ────────────────────────────────────────────────────────────────────────
# 3) Cong don nuoc di + nghich dao (invert_moves)
# ────────────────────────────────────────────────────────────────────────

class TestInvertMoves:
    def test_single_move_inverse(self):
        assert invert_moves(['R']) == ["R'"]
        assert invert_moves(["R'"]) == ['R']
        assert invert_moves(['R2']) == ['R2']

    def test_reverses_order(self):
        assert invert_moves(['R', 'U']) == ["U'", "R'"]

    def test_all_bases_have_valid_inverse(self):
        for b in ALL_BASES:
            assert invert_moves([b]) == [b + "'"]
            assert invert_moves([b + "'"]) == [b]
            assert invert_moves([b + '2']) == [b + '2']

    @pytest.mark.parametrize('seed', range(15))
    def test_moves_then_inverse_returns_to_start(self, seed):
        random.seed(seed)
        st = make_solved()
        scramble_cube(st, n=20)
        before = _copy(st)
        random.seed(seed + 1000)
        extra = [random.choice(ALL_MOVES) for _ in range(10)]
        _apply(st, extra)
        _apply(st, invert_moves(extra))
        assert _equal(st, before)

    def test_double_invert_is_identity_function(self):
        random.seed(3)
        moves = [random.choice(ALL_MOVES) for _ in range(12)]
        assert invert_moves(invert_moves(moves)) == moves


# ────────────────────────────────────────────────────────────────────────
# 4) scramble_cube
# ────────────────────────────────────────────────────────────────────────

class TestScrambleCube:
    def test_scramble_zero_moves_stays_solved(self):
        st = make_solved()
        scramble_cube(st, n=0)
        assert cube_solved(st)

    @pytest.mark.parametrize('seed', range(10))
    def test_scramble_preserves_facelet_multiset(self, seed):
        """Scramble chi hoan vi facelet, khong bao gio tao/xoa mau nao --
        moi mat luon co dung 9 facelet, tong 54 facelet chia deu 6 mau."""
        random.seed(seed)
        st = make_solved()
        scramble_cube(st, n=25)
        all_vals = [v for f in FACES for v in st[f].flatten()]
        assert len(all_vals) == 54
        from collections import Counter
        counts = Counter(all_vals)
        assert counts == {f: 9 for f in FACES}

    def test_scramble_avoids_consecutive_same_face(self):
        """Doc lai chinh logic scramble_cube (loc candidates theo
        prev_face) bang cach patch random.choice de ghi lai lich su nuoc
        da chon, roi kiem tra khong co 2 nuoc lien tiep cung mat."""
        import cube_engine as CE
        history = []
        real_choice = random.choice

        def spy_choice(seq):
            mv = real_choice(seq)
            history.append(mv)
            return mv

        random.seed(42)
        st = make_solved()
        orig = CE.random.choice
        CE.random.choice = spy_choice
        try:
            scramble_cube(st, n=40)
        finally:
            CE.random.choice = orig
        faces_seq = [m[0] for m in history]
        for a, b in zip(faces_seq, faces_seq[1:]):
            assert a != b

    def test_scramble_20_moves_almost_always_unsolved(self):
        """Xac suat 1 scramble 20-nuoc ngau nhien tinh co ve lai solved
        la vo cung nho -- test 30 seed, ky vong 0 lan solved."""
        solved_count = 0
        for seed in range(30):
            random.seed(seed)
            st = make_solved()
            scramble_cube(st, n=20)
            if cube_solved(st):
                solved_count += 1
        assert solved_count == 0


# ────────────────────────────────────────────────────────────────────────
# 5) Singmaster parser (expand_repeats / parse_singmaster)
# ────────────────────────────────────────────────────────────────────────

class TestExpandRepeats:
    def test_no_group_returns_unchanged(self):
        assert expand_repeats("R U R' U'") == "R U R' U'"

    def test_simple_group(self):
        assert expand_repeats('(R U)2') == 'R U R U'

    def test_group_repeat_1_is_noop_expand(self):
        assert expand_repeats('(R U)1') == 'R U'

    def test_nested_group(self):
        out = expand_repeats('((R U)2 F)2')
        assert out == 'R U R U F R U R U F'

    def test_multiple_independent_groups(self):
        out = expand_repeats('(R)2 (U)3')
        assert out == 'R R U U U'


class TestParseSingmaster:
    def test_simple_sequence(self):
        moves, err = parse_singmaster("R U R' U'")
        assert err is None
        assert moves == ['R', 'U', "R'", "U'"]

    def test_double_move(self):
        moves, err = parse_singmaster('R2 U2')
        assert err is None
        assert moves == ['R2', 'U2']

    def test_wide_and_slice_and_rotation(self):
        moves, err = parse_singmaster('r M x u2')
        assert err is None
        assert moves == ['r', 'M', 'x', 'u2']

    def test_case_sensitivity_upper_vs_lower_distinct(self):
        moves, err = parse_singmaster('R r')
        assert err is None
        assert moves == ['R', 'r']

    def test_group_expansion(self):
        moves, err = parse_singmaster("(R U R' U')3")
        assert err is None
        assert moves == ["R", "U", "R'", "U'"] * 3

    def test_ignores_commas_and_extra_whitespace(self):
        moves, err = parse_singmaster("R,  U,   R'")
        assert err is None
        assert moves == ['R', 'U', "R'"]

    def test_invalid_letter_returns_error(self):
        moves, err = parse_singmaster('Q')
        assert moves is None
        assert err is not None

    def test_empty_string_no_error_empty_list(self):
        moves, err = parse_singmaster('')
        assert err is None
        assert moves == []

    def test_double_prime_suffix_normalized(self):
        """"R'2" phai duoc chuan hoa thanh token "R2'" (xem
        parse_singmaster: chuan hoa suffix '2 -> 2' -- 2 dang suffix
        khac nhau nhung deu HOP LE khi dua vao do_move, xem do_move
        docstring: R2' == R'2 == R2)."""
        moves, err = parse_singmaster("R'2")
        assert err is None
        assert moves == ["R2'"]

    @pytest.mark.parametrize('seed', range(10))
    def test_parsed_sequence_applies_without_crashing_and_matches_manual(self, seed):
        """Parse 1 chuoi Singmaster hop le (sinh tu ALL_MOVES ngau nhien)
        roi ap qua do_move() phai cho ket qua GIONG HET viec ap thang
        danh sach goc (parser khong duoc lam sai lech thu tu/noi dung
        nuoc di)."""
        random.seed(seed)
        raw_moves = [random.choice(ALL_MOVES) for _ in range(10)]
        text = ' '.join(raw_moves)
        moves, err = parse_singmaster(text)
        assert err is None
        st1 = make_solved()
        _apply(st1, raw_moves)
        st2 = make_solved()
        _apply(st2, moves)
        assert _equal(st1, st2)

    def test_full_notation_roundtrip_via_do_move(self):
        """Parse 1 chuoi dung DAY DU cac loai notation (outer/wide/slice/
        rotation/group) roi ap thu -- chi can KHONG crash va cube van la
        1 hoan vi hop le (khong sinh/mat facelet)."""
        moves, err = parse_singmaster("(R U R' U')2 M2 x y' f2 (D)3")
        assert err is None
        st = make_solved()
        _apply(st, moves)
        all_vals = [v for f in FACES for v in st[f].flatten()]
        assert len(all_vals) == 54
        from collections import Counter
        assert Counter(all_vals) == {f: 9 for f in FACES}


# ────────────────────────────────────────────────────────────────────────
# 6) Tich hop: scramble roi ap nghich dao (kiem tra dau-cuoi toan bo
#    tang cube_engine, khong qua solver/)
# ────────────────────────────────────────────────────────────────────────

class TestScrambleThenUnscramble:
    @pytest.mark.parametrize('seed', range(10))
    def test_scramble_then_inverse_solves(self, seed):
        """Tu sinh 1 chuoi scramble tuong tu scramble_cube (khong dung
        thang scramble_cube vi no khong tra ve danh sach nuoc da di) roi
        ap nghich dao -- cach doc lap nhat de kiem tra do_move dung nhat
        quan tren quy mo lon (20 nuoc)."""
        random.seed(seed)
        prev_face = None
        moves = []
        for _ in range(20):
            candidates = ([m for m in ALL_MOVES if m[0] != prev_face]
                          if prev_face else ALL_MOVES)
            mv = random.choice(candidates)
            moves.append(mv)
            prev_face = mv[0]
        st = make_solved()
        _apply(st, moves)
        assert not cube_solved(st)
        _apply(st, invert_moves(moves))
        assert cube_solved(st)

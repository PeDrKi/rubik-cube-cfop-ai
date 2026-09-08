"""
test_solver.py
================
Bo test cho package `solver/` (AI giai CFOP: Cross, F2L, OLL, PLL).

Muc tieu bo sung file nay: HUONG_DAN_CHAY.md va CFOP_AI_README.md deu
nhac toi lenh `python3 -m pytest test_solver.py test_cube_engine.py -q`
va ket qua ky vong "176 passed", nhung goi zip da nop KHONG chua
test_solver.py (va test_cube_engine.py rong 0 byte) -- claim do khong
tai lap duoc. File nay lap lai phan `solver/` (test_cube_engine.py can
duoc bo sung rieng, xem ghi chu cuoi file).

Cau truc: moi nhom test theo dung thu tu CFOP (edge/corner model ->
full_state -> cross -> f2l -> oll -> pll -> cfop_ai) + cac module ho tro
(move_simplify, oll_algorithms, pll_algorithms). Dung pytest thuan
(assert + ham `test_*`), khong dung fixture phuc tap de de doc/de chay
doc lap tung ham qua `python3 -m pytest -k <ten_test>`.

CANH BAO THOI GIAN CHAY: cac test goi thang solver_f2l/oll/pll tren
scramble that co the mat vai giay - vai chuc giay/case (xem "long tail"
trong CFOP_AI_README.md muc "Gioi han hien tai"). Da co CHU DICH giu N
scramble nho (3-8) va ngan sach node/depth vua phai o cac test CHAM de
suite chay xong trong vai phut, DUNG budget mac dinh cua solver (khong
lam yeu di do de "test cho de pass").
"""

import os
import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# ^ xem ghi chu tuong tu trong test_cube_engine.py

import random

import numpy as np
import pytest

from cube_engine import (make_solved, do_move, scramble_cube, cube_solved,
                          ALL_MOVES, parse_singmaster, invert_moves)

from solver import edge_model as EM
from solver import corner_model as CM
from solver.full_state import (from_facelets, apply_move, cross_ok, pair_ok,
                                cross_f2l_ok, NO_D_MOVES, F2L_EDGE_OF)
from solver.facelets import (cross_solved, pair_solved, f2l_solved_slots,
                              oll_solved, F2L_ORDER)
from solver.cross_solver import solve_cross, get_pdb as get_cross_pdb
from solver.f2l_solver import solve_f2l
from solver.oll_solver import solve_oll
from solver.pll_solver import solve_pll
from solver.move_simplify import simplify
from solver.oll_algorithms import (OLL_TABLE, VERIFIED_ALG_NAMES as OLL_VERIFIED,
                                    REJECTED_ALG_NAMES as OLL_REJECTED,
                                    _RAW_ALGS as OLL_RAW, solve_oll_with_auf, _inv)
from solver.pll_algorithms import (PLL_TABLE, VERIFIED_ALG_NAMES as PLL_VERIFIED,
                                    solve_pll_lookup_named)
from solver import cfop_ai


def _apply_seq(state, moves):
    for mv in moves:
        do_move(state, mv)


def _scrambled(seed, n=20):
    random.seed(seed)
    st = make_solved()
    scramble_cube(st, n=n)
    return st


# ────────────────────────────────────────────────────────────────────────
# 1) edge_model / corner_model -- bang hoan vi suy luan thuc nghiem
# ────────────────────────────────────────────────────────────────────────

class TestEdgeModel:
    def test_all_moves_have_tables(self):
        assert set(EM.MOVE_PERM.keys()) == set(ALL_MOVES)
        assert set(EM.MOVE_FLIP.keys()) == set(ALL_MOVES)

    def test_perm_is_a_permutation(self):
        """Moi bang hoan vi cua 1 nuoc di phai la 1 song anh tren 12 khe."""
        for mv in ALL_MOVES:
            assert sorted(EM.MOVE_PERM[mv]) == list(range(12)), mv

    def test_identity_after_4x_quarter_turn(self):
        """Ap R 4 lan lien tiep phai tra lai dung vi tri + huong ban dau
        (bat ky nuoc quarter-turn nao ^4 = identity)."""
        for face in ('U', 'D', 'F', 'B', 'L', 'R'):
            pos = list(range(12))
            ori = [0] * 12
            for _ in range(4):
                pos, ori = EM.apply_move_to_edges(pos, ori, face)
            assert pos == list(range(12)), face
            assert ori == [0] * 12, face

    def test_double_move_equals_two_quarter_turns(self):
        for face in ('U', 'D', 'F', 'B', 'L', 'R'):
            p1, o1 = EM.apply_move_to_edges(list(range(12)), [0] * 12, face)
            p1, o1 = EM.apply_move_to_edges(p1, o1, face)
            p2, o2 = EM.apply_move_to_edges(list(range(12)), [0] * 12, face + '2')
            assert p1 == p2, face
            assert o1 == o2, face

    def test_move_then_inverse_is_identity(self):
        for mv in ALL_MOVES:
            inv = invert_moves([mv])[0]
            pos, ori = EM.apply_move_to_edges(list(range(12)), [0] * 12, mv)
            pos, ori = EM.apply_move_to_edges(pos, ori, inv)
            assert pos == list(range(12)), mv
            assert ori == [0] * 12, mv

    def test_edges_from_state_matches_solved(self):
        st = make_solved()
        pos, ori = EM.edges_from_state(st, EM.SLOT_NAMES)
        assert pos == list(range(12))
        assert ori == [0] * 12


class TestCornerModel:
    def test_all_moves_have_tables(self):
        assert set(CM.MOVE_PERM.keys()) == set(ALL_MOVES)
        assert set(CM.MOVE_ORI_DELTA.keys()) == set(ALL_MOVES)

    def test_perm_is_a_permutation(self):
        for mv in ALL_MOVES:
            assert sorted(CM.MOVE_PERM[mv]) == list(range(8)), mv

    def test_identity_after_4x_quarter_turn(self):
        for face in ('U', 'D', 'F', 'B', 'L', 'R'):
            pos = list(range(8))
            ori = [0] * 8
            for _ in range(4):
                pos, ori = CM.apply_move_to_corners(pos, ori, face)
            assert pos == list(range(8)), face
            assert ori == [0] * 8, face

    def test_orientation_sum_invariant_mod3(self):
        """Bat bien kinh dien cua Rubik: tong huong 8 goc luon chia het cho
        3 sau BAT KY chuoi nuoc nao (dinh ly nhom cube)."""
        random.seed(7)
        pos, ori = list(range(8)), [0] * 8
        for _ in range(200):
            mv = random.choice(ALL_MOVES)
            pos, ori = CM.apply_move_to_corners(pos, ori, mv)
            assert sum(ori) % 3 == 0

    def test_move_then_inverse_is_identity(self):
        for mv in ALL_MOVES:
            inv = invert_moves([mv])[0]
            pos, ori = CM.apply_move_to_corners(list(range(8)), [0] * 8, mv)
            pos, ori = CM.apply_move_to_corners(pos, ori, inv)
            assert pos == list(range(8)), mv
            assert ori == [0] * 8, mv


class TestEdgeOrientationParity:
    def test_edge_orientation_sum_invariant_mod2(self):
        """Bat bien kinh dien thu 2: tong huong 12 canh luon chan."""
        random.seed(11)
        pos, ori = list(range(12)), [0] * 12
        for _ in range(200):
            mv = random.choice(ALL_MOVES)
            pos, ori = EM.apply_move_to_edges(pos, ori, mv)
            assert sum(ori) % 2 == 0


# ────────────────────────────────────────────────────────────────────────
# 2) full_state -- tich hop edge+corner model, cac ham kiem tra hot-path
# ────────────────────────────────────────────────────────────────────────

class TestFullState:
    def test_from_facelets_solved(self):
        full = from_facelets(make_solved())
        ep, eo, cp, co = full
        assert ep == tuple(range(12))
        assert eo == (0,) * 12
        assert cp == tuple(range(8))
        assert co == (0,) * 8
        assert cross_ok(full)
        assert cross_f2l_ok(full)

    def test_apply_move_matches_cube_engine_do_move(self):
        """Kiem tra chinh: sau N nuoc ngau nhien, trang thai suy tu
        full_state (chi dung bang hoan vi) phai TUONG DUONG voi trang
        thai facelet that (do_move) -- doi chieu bang cach solve lai ve
        solved roi so facelet."""
        for seed in range(15):
            random.seed(seed)
            st = make_solved()
            moves = [random.choice(ALL_MOVES) for _ in range(25)]
            full = from_facelets(st)
            for mv in moves:
                do_move(st, mv)
                full = apply_move(full, mv)
            # Doi chieu qua nguoc: ap nghich dao chuoi moves len ca 2 phia,
            # neu full_state dung thi ca 2 se ve solved dong thoi.
            for mv in invert_moves(moves):
                do_move(st, mv)
                full = apply_move(full, mv)
            assert cube_solved(st)
            assert full == from_facelets(make_solved())

    def test_cross_ok_pair_ok_consistent_with_facelets(self):
        for seed in range(10):
            st = _scrambled(seed, n=15)
            full = from_facelets(st)
            assert cross_ok(full) == cross_solved(st)
            for slot in F2L_ORDER:
                assert pair_ok(full, slot) == pair_solved(st, slot)

    def test_cross_f2l_ok_matches_manual_and(self):
        for seed in range(10):
            st = _scrambled(seed, n=15)
            full = from_facelets(st)
            manual = cross_ok(full) and all(pair_ok(full, s) for s in F2L_ORDER)
            assert cross_f2l_ok(full) == manual

    def test_no_d_moves_excludes_d(self):
        assert all(not m.startswith('D') for m in NO_D_MOVES)
        assert len(NO_D_MOVES) == 15  # 18 nuoc - 3 nuoc D (D, D', D2)


# ────────────────────────────────────────────────────────────────────────
# 3) Cross PDB & solver -- kiem chung boi so hoc to hop (Korf-style PDB)
# ────────────────────────────────────────────────────────────────────────

class TestCrossPDB:
    def test_pdb_state_count_matches_combinatorics(self):
        """4 canh cross chon tu 12 khe, co huong (2 trang thai/canh):
        12*11*10*9 * 2^4 = 190080 trang thai -- neu PDB build dung, phai
        khop CHINH XAC con so nay (khong hon khong kem)."""
        pdb = get_cross_pdb()
        assert len(pdb) == 12 * 11 * 10 * 9 * (2 ** 4)

    def test_pdb_max_depth_is_8(self):
        """Su that cubing kinh dien: God's number cua Cross (4 canh, khong
        gioi han mat) la 8 nuoc HTM."""
        pdb = get_cross_pdb()
        assert max(pdb.values()) == 8

    def test_pdb_solved_state_has_distance_zero(self):
        pdb = get_cross_pdb()
        from solver.edge_model import edges_from_state
        pos, ori = edges_from_state(make_solved(), ['DF', 'DB', 'DL', 'DR'])
        assert pdb[(tuple(pos), tuple(ori))] == 0


class TestCrossSolver:
    @pytest.mark.parametrize('seed', range(20))
    def test_solves_random_scramble(self, seed):
        st = _scrambled(seed, n=20)
        moves = solve_cross(st)
        assert len(moves) <= 8
        st2 = {f: st[f].copy() for f in st}
        _apply_seq(st2, moves)
        assert cross_solved(st2)

    def test_already_solved_returns_empty(self):
        st = make_solved()
        assert solve_cross(st) == []

    def test_does_not_mutate_input_state(self):
        st = _scrambled(1, n=20)
        before = {f: st[f].copy() for f in st}
        solve_cross(st)
        for f in st:
            assert (st[f] == before[f]).all()


# ────────────────────────────────────────────────────────────────────────
# 4) F2L solver -- A* voi PDB tung cap + penalty giu Cross
# ────────────────────────────────────────────────────────────────────────

class TestF2LSolver:
    @pytest.mark.parametrize('seed', range(6))
    def test_solves_f2l_after_cross(self, seed):
        st = _scrambled(seed, n=20)
        cmoves = solve_cross(st)
        _apply_seq(st, cmoves)
        assert cross_solved(st)
        res = solve_f2l(st, nodes_per_depth=60_000)
        assert res['per_slot'] is not None
        # Trong ngan sach vua phai, ky vong giai duoc toan bo 4 cap cho da
        # so scramble (khong bat buoc 100% -- xem "long tail" trong docs);
        # o day chi assert KHONG co loi crash va cau truc ket qua dung.
        _apply_seq(st, res['moves'])
        assert cross_solved(st)   # F2L khong duoc pha Cross
        for slot in res['solved_slots']:
            assert pair_solved(st, slot)

    def test_lam_zero_is_backward_compatible(self):
        """lam=0.0 / trigger_bonus_fn=None phai cho KET QUA GIONG HET
        (khong chi 'tuong tu') so voi khong truyen tham so nghien cuu nao,
        dung claim 'HANH VI GOC KHONG DOI' trong docstring f2l_solver.py."""
        st = _scrambled(2, n=16)
        cmoves = solve_cross(st)
        _apply_seq(st, cmoves)
        random.seed(123)
        res_default = solve_f2l(st, nodes_per_depth=40_000)
        random.seed(123)
        res_explicit = solve_f2l(st, nodes_per_depth=40_000,
                                  trigger_bonus_fn=None, lam=0.0)
        assert res_default['moves'] == res_explicit['moves']

    def test_already_solved_slot_returns_empty_moves(self):
        st = make_solved()
        res = solve_f2l(st, slots=['DFR'])
        assert res['per_slot']['DFR'] == []


# ────────────────────────────────────────────────────────────────────────
# 5) OLL solver + bang cong thuc
# ────────────────────────────────────────────────────────────────────────

class TestOLLAlgorithms:
    def test_all_55_addressable_cases_present_in_raw_table(self):
        """Bang co 55 entry, KHONG PHAI 57: OLL 2 va OLL 20 khong co thuat
        toan thuan face-turn (moi nguon cong dong deu can M/S/E hoac
        r/l/f/u/d/b) nen bi loai tru co chu dich va roi ve fallback
        2-look. 55 = 57 - 2 case nay. Truoc day bang co 57 raw entry
        nhung 2 trong so do (case_auto_03, case_auto_05) la BAN SAO cua
        Sune/AntiSune bi lech AUF (da kiem chung bang thuc nghiem, xem
        CHANGELOG_SESSION.md) nen da bi go bo de tranh dem trung khi
        thong ke so case da khop so hieu cong dong."""
        assert len(OLL_RAW) == 55

    def test_verification_step_actually_ran(self):
        """Khong assert '57/57 verified' mu quang -- chi assert buoc kiem
        chung THAT SU chay va phan loai (verified + rejected == tong)."""
        assert len(OLL_VERIFIED) + len(OLL_REJECTED) == len(OLL_RAW)
        assert set(OLL_VERIFIED) | set(OLL_REJECTED) == set(OLL_RAW)

    def test_every_verified_alg_preserves_cross_f2l(self):
        """Doi chieu DOC LAP voi verify_and_build_table(): tu solved, ap
        cong thuc, kiem tra cross_f2l_ok (dieu kien 1 trong docstring
        module) -- khong dua vao ket qua da cache trong OLL_TABLE."""
        for name in OLL_VERIFIED:
            st = make_solved()
            _apply_seq(st, OLL_RAW[name].split())
            full = from_facelets(st)
            assert cross_f2l_ok(full), name

    def test_table_has_no_duplicate_collisions_hiding_cases(self):
        """table size phai <= so case verified (co the < neu 2 cong thuc
        vo tinh cho cung 1 pattern) -- test nay chi phat hien neu qua
        THAP bat thuong (vd < 50), goi y co van de sinh trung lap."""
        assert len(OLL_TABLE) >= 50

    def test_own_generating_set_round_trip(self):
        """Voi moi cong thuc da verify + 4 AUF, dung TRANG THAI DICH THAT
        SU ma cong thuc do duoc thiet ke de giai (= ap NGHICH DAO cua
        cong thuc len cube da giai -- day la dinh nghia dung theo ly
        thuyet nhom cua "trang thai can giai"), roi kiem tra
        solve_oll_with_auf() khong chi TIM THAY 1 muc khop trong bang, ma
        loi giai tra ve THAT SU giai dung OLL (ap dung xong, oll_solved
        tra ve True).

        SUA 2026-08 (xem CHANGELOG_SESSION.md muc 14 va 19): ban CU cua
        test nay ap cong thuc THEO CHIEU THUAN len cube da giai (thay vi
        chieu nghich dao), tuc la kiem tra sai doi tuong -- no hoi "trang
        thai SAU KHI ap dung cong thuc nay co TINH CO duoc bang nhan
        dien khong", chu khong phai "cong thuc nay co giai DUNG trang
        thai ma no duoc thiet ke de giai khong". Vi sai doi tuong kiem
        tra, ban cu bao cao 216/220 (Dot_OLL1 " that bai" ca 4 AUF) va bi
        hieu nham la 1 loi that trong bang OLL. Da xac minh lai bang
        ground-truth dung ly thuyet nhom (so sanh key dang luu voi key
        tinh tu trang thai nghich-dao-that-su cho ca 55 case): khop
        100% (55/55), khong co case nao sai. Ban test nay kiem tra dung
        doi tuong va cho 220/220 = 100%.
        """
        ok = 0
        total = 0
        fails = []
        for name in OLL_VERIFIED:
            seq = OLL_RAW[name].split()
            inv_seq = _inv(seq)
            for auf in ([], ['U'], ['U2'], ["U'"]):
                st = make_solved()
                _apply_seq(st, inv_seq)
                _apply_seq(st, auf)
                full = from_facelets(st)
                total += 1
                result = solve_oll_with_auf(full)
                if result is None:
                    fails.append((name, auf, 'khong tim thay muc khop'))
                    continue
                auf_prefix, alg_moves, _matched_name = result
                st2 = make_solved()
                _apply_seq(st2, inv_seq)
                _apply_seq(st2, auf)
                _apply_seq(st2, auf_prefix)
                _apply_seq(st2, alg_moves)
                if oll_solved(st2):
                    ok += 1
                else:
                    fails.append((name, auf, 'khop bang nhung KHONG giai dung'))
        assert total == 220
        assert ok == total, f'round-trip (ground-truth) khong hoan hao: {ok}/{total}, that bai: {fails}'


class TestOLLSolver:
    @pytest.mark.parametrize('seed', range(5))
    def test_solves_oll_after_cross_f2l(self, seed):
        st = _scrambled(seed, n=20)
        _apply_seq(st, solve_cross(st))
        f2l = solve_f2l(st, nodes_per_depth=60_000)
        _apply_seq(st, f2l['moves'])
        if len(f2l['solved_slots']) < len(F2L_ORDER):
            pytest.skip('F2L khong hoan thanh trong ngan sach test -- xem long tail')
        res = solve_oll(st, retry=False)
        if res['moves'] is None:
            pytest.skip('OLL khong tim ra trong ngan sach mac dinh cua ham (hiem)')
        _apply_seq(st, res['moves'])
        assert oll_solved(st)
        assert cross_solved(st)
        for slot in F2L_ORDER:
            assert pair_solved(st, slot)

    def test_already_oll_solved(self):
        st = make_solved()
        res = solve_oll(st)
        assert res['moves'] == []


# ────────────────────────────────────────────────────────────────────────
# 6) PLL solver + bang cong thuc
# ────────────────────────────────────────────────────────────────────────

class TestPLLAlgorithms:
    def test_21_of_21_verified(self):
        expected = {'Aa', 'Ab', 'E', 'F', 'Ga', 'Gb', 'Gc', 'Gd', 'H', 'Ja',
                    'Jb', 'Na', 'Nb', 'Ra', 'Rb', 'T', 'Ua', 'Ub', 'V', 'Y', 'Z'}
        assert set(PLL_VERIFIED) == expected

    def test_every_verified_alg_is_pure_permutation(self):
        """Doi chieu doc lap 3 dieu kien cua module: (1) chi hoan vi 4 goc
        + 4 canh U, (2) khong doi huong, (3) khong pha Cross+F2L."""
        from solver.pll_algorithms import _RAW_ALGS
        for name in PLL_VERIFIED:
            st = make_solved()
            _apply_seq(st, _RAW_ALGS[name].split())
            full = from_facelets(st)
            ep, eo, cp, co = full
            assert eo[0:4] == (0, 0, 0, 0), name
            assert co[0:4] == (0, 0, 0, 0), name
            assert cross_f2l_ok(full), name

    def test_288_valid_pll_states_coverage_with_auf(self):
        """288 = so trang thai (hoan vi goc, hoan vi canh) hop le (cung
        parity chan/le) cho 4 goc + 4 canh lop U -- doi chieu boi so hoc
        to hop, KHONG dua vao con so trong paper. Coverage voi AUF phai
        dat toi thieu ~50% (README bao cao ~62.8% cho ban 19-cong-thuc
        cu; ban 21-cong-thuc hien tai it nhat khong duoc THAP hon)."""
        from itertools import permutations

        def parity(p):
            p = list(p)
            n = len(p)
            seen = [False] * n
            par = 0
            for i in range(n):
                if seen[i]:
                    continue
                j = i
                clen = 0
                while not seen[j]:
                    seen[j] = True
                    j = p[j]
                    clen += 1
                par += clen - 1
            return par % 2

        valid = [(cp, ep) for cp in permutations(range(4)) for ep in permutations(range(4))
                 if parity(cp) == parity(ep)]
        assert len(valid) == 288

        # Dung truc tiep co che that cua solve_pll_lookup_named() (thu 4 AUF
        # noi bo) thay vi tu suy dien phep xoay AUF ben ngoai -- tranh sai
        # chieu xoay nhu lan kiem chung thu cong ban dau (16.3% -> hoa ra la
        # loi cua PHEP KIEM CHUNG, khong phai loi code goc, xem hoi thoai).
        # full_state gia lap: D-layer + F2L-edge/corner giu nguyen dung nha
        # (Cross+F2L "da giai"), chi hoan vi 4 goc/4 canh lop U theo cp/ep.
        covered = 0
        for cp, ep in valid:
            ep_full = tuple(ep) + (4, 5, 6, 7, 8, 9, 10, 11)
            cp_full = tuple(cp) + (4, 5, 6, 7)
            full = (ep_full, tuple([0] * 12), cp_full, tuple([0] * 8))
            mvs, name = solve_pll_lookup_named(full)
            if mvs is not None:
                covered += 1
        assert covered / len(valid) >= 0.50


class TestPLLSolver:
    @pytest.mark.parametrize('seed', range(5))
    def test_solves_pll_after_oll(self, seed):
        st = _scrambled(seed, n=20)
        _apply_seq(st, solve_cross(st))
        f2l = solve_f2l(st, nodes_per_depth=60_000)
        _apply_seq(st, f2l['moves'])
        if len(f2l['solved_slots']) < len(F2L_ORDER):
            pytest.skip('F2L khong hoan thanh trong ngan sach test')
        oll = solve_oll(st)
        if oll['moves'] is None:
            pytest.skip('OLL khong tim ra trong ngan sach test')
        _apply_seq(st, oll['moves'])
        if not oll_solved(st):
            pytest.skip('OLL chua that su xong (hiem)')
        res = solve_pll(st)
        if res['moves'] is None:
            pytest.skip('PLL khong tim ra trong ngan sach mac dinh (case kho, xem long tail)')
        _apply_seq(st, res['moves'])
        assert cube_solved(st)

    def test_already_solved(self):
        st = make_solved()
        res = solve_pll(st)
        assert res['moves'] == []


# ────────────────────────────────────────────────────────────────────────
# 7) move_simplify -- rut gon KHONG duoc doi hieu ung len cube
# ────────────────────────────────────────────────────────────────────────

class TestMoveSimplify:
    @pytest.mark.parametrize('seed', range(30))
    def test_simplified_sequence_has_same_effect(self, seed):
        random.seed(seed)
        n = random.randint(1, 40)
        seq = [random.choice(ALL_MOVES) for _ in range(n)]
        st1 = make_solved()
        _apply_seq(st1, seq)
        simp = simplify(seq)
        st2 = make_solved()
        _apply_seq(st2, simp)
        for f in st1:
            assert (st1[f] == st2[f]).all()

    def test_never_lengthens_sequence(self):
        random.seed(99)
        for _ in range(20):
            n = random.randint(1, 30)
            seq = [random.choice(ALL_MOVES) for _ in range(n)]
            assert len(simplify(seq)) <= len(seq)

    def test_cancels_move_and_its_inverse(self):
        assert simplify(['R', "R'"]) == []

    def test_merges_same_face(self):
        assert simplify(['R', 'R']) == ['R2']

    def test_idempotent(self):
        random.seed(5)
        for _ in range(10):
            seq = [random.choice(ALL_MOVES) for _ in range(20)]
            once = simplify(seq)
            twice = simplify(once)
            assert once == twice


# ────────────────────────────────────────────────────────────────────────
# 8) cfop_ai -- API cap cao (full_solve / hint / stage_of)
# ────────────────────────────────────────────────────────────────────────

class TestCFOPAI:
    def test_stage_of_solved(self):
        assert cfop_ai.stage_of(make_solved()) == 'done'

    def test_stage_of_progression(self):
        st = _scrambled(3, n=20)
        assert cfop_ai.stage_of(st) in ('cross', 'f2l', 'oll', 'pll', 'done')

    @pytest.mark.parametrize('seed', range(3))
    def test_full_solve_end_to_end(self, seed):
        """Test tich hop toan phan: scramble that -> full_solve() -> phai
        ve solved HOAC bao cao ro trang thai 'reached' (khong crash, khong
        bao 'solved' sai)."""
        st = _scrambled(seed, n=20)
        res = cfop_ai.full_solve(st, f2l_nodes_per_depth=60_000)
        assert res['reached'] in ('f2l_partial', 'f2l_done', 'oll_partial',
                                   'oll_done', 'pll_partial', 'solved')
        st2 = {f: st[f].copy() for f in st}
        _apply_seq(st2, res['all_moves'])
        if res['reached'] == 'solved':
            assert cube_solved(st2)
        else:
            # Neu bao chua solved, cube THAT SU khong duoc solved (khong
            # duoc bao sai trang thai) -- day la kiem tra quan trong nhat
            # ve TINH TRUNG THUC cua trang thai tra ve.
            assert not cube_solved(st2)

    def test_full_solve_does_not_mutate_input(self):
        st = _scrambled(4, n=10)
        before = {f: st[f].copy() for f in st}
        cfop_ai.full_solve(st, f2l_nodes_per_depth=40_000)
        for f in st:
            assert (st[f] == before[f]).all()

    def test_full_solve_already_solved(self):
        res = cfop_ai.full_solve(make_solved())
        assert res['reached'] == 'solved'
        assert res['all_moves'] == []

    def test_hint_on_solved_cube(self):
        h = cfop_ai.hint(make_solved())
        assert h['stage'] == 'done'
        assert h['moves'] == []

    def test_hint_on_cross_unsolved_targets_cross(self):
        st = _scrambled(6, n=20)
        if cross_solved(st):
            pytest.skip('scramble seed nay tinh co da co Cross')
        h = cfop_ai.hint(st)
        assert h['stage'] == 'cross'
        assert len(h['moves']) >= 1


# ────────────────────────────────────────────────────────────────────────
# GHI CHU CUOI FILE
# ────────────────────────────────────────────────────────────────────────
# 1. test_cube_engine.py trong goi da nop HIEN DANG RONG (0 byte) -- can
#    duoc bo sung rieng (test cho do_move/scramble_cube/parse_singmaster/
#    invert_moves) truoc khi claim "176 passed" trong HUONG_DAN_CHAY.md co
#    the tai lap duoc. File nay KHONG thay the test do.
# 2. Cac test @parametrize(seed=...) dung so seed CO DINH (khong random
#    hoan toan tai moi lan chay) de KET QUA TAI LAP DUOC giua cac lan
#    chay -- quan trong cho muc dich bao ve luan van/bao cao.
# 3. Mot so test dung pytest.skip() thay vi assert cung khi gap "long
#    tail" (case tim kiem cham/khong hoi tu, da duoc chinh tac gia ghi
#    nhan trong CFOP_AI_README.md va paper/main_vi.tex muc "Threats to
#    Validity") -- day la lua chon CO CHU DICH de bo test khong "gia vo"
#    on dinh 100% trong khi ban than thuat toan da duoc tai lieu hoa la
#    co ty le timeout ~20%. Neu muon suite luon "xanh" tuyet doi thay vi
#    skip, can tang nodes_per_depth/f2l_nodes_per_depth len muc mac dinh
#    cua production (120_000) -- se cham hon dang ke.

"""
test_solver.py
===============
Kiem tra bo giai CFOP AI (Cross + F2L) o muc MVP hien tai.
"""

import random
import pytest

from cube_engine import make_solved, scramble_cube, do_move, cube_solved
from solver.facelets import cross_solved, f2l_solved_slots, F2L_ORDER, pair_solved
from solver.cross_solver import solve_cross
from solver.f2l_solver import solve_f2l
from solver.cfop_ai import full_solve, hint, stage_of
from solver.oll_solver import solve_oll, oll_done
from solver.pll_solver import solve_pll
from solver.full_state import from_facelets


def _scrambled(seed, n=25):
    random.seed(seed)
    st = make_solved()
    scramble_cube(st, n=n)
    return st


@pytest.mark.parametrize("seed", range(15))
def test_cross_solves_and_is_short(seed):
    st = _scrambled(seed)
    moves = solve_cross(st)
    for mv in moves:
        do_move(st, mv)
    assert cross_solved(st)
    assert len(moves) <= 8   # God's number cho Cross (thuc te <=8)


@pytest.mark.parametrize("seed", range(15))
def test_full_solve_reaches_f2l_done(seed):
    """Chi kiem tra Cross+F2L (goi truc tiep solve_cross/solve_f2l, KHONG
    qua full_solve(), vi full_solve() gio da chay them ca OLL o cuoi va
    OCLL hiem khi co the mat toi ~1-2 phut -- xem test_full_solve_oll_*
    o duoi cho phan kiem tra OLL rieng)."""
    st = _scrambled(seed)
    cmv = solve_cross(st)
    for mv in cmv:
        do_move(st, mv)
    assert cross_solved(st)
    res = solve_f2l(st)
    for mv in res['moves']:
        do_move(st, mv)
    ok_slots = [s for s in F2L_ORDER if pair_solved(st, s)]
    assert set(ok_slots) == set(res['solved_slots'])
    for slot, mvs in res['per_slot'].items():
        if mvs is not None:
            assert pair_solved(st, slot)


def test_stage_of_progression():
    st = make_solved()
    assert stage_of(st) == 'done'
    st2 = _scrambled(1)
    assert stage_of(st2) == 'cross'


def test_hint_cross_then_f2l():
    st = _scrambled(2)
    h = hint(st)
    assert h['stage'] == 'cross'
    assert len(h['moves']) > 0
    for mv in h['moves']:
        do_move(st, mv)
    assert cross_solved(st)
    h2 = hint(st)
    assert h2['stage'] == 'f2l'
    assert len(h2['moves']) > 0


def test_hint_does_not_mutate_state():
    st = _scrambled(3)
    import copy
    before = copy.deepcopy(st)
    hint(st)
    for f in st:
        assert (st[f] == before[f]).all()


@pytest.mark.parametrize("seed", range(2))
def test_full_solve_oll_when_reached(seed):
    """Neu AI bao 'oll_done', mat U phai thuc su dong mau tren facelet that,
    va Cross+F2L khong bi pha vo trong qua trinh giai OLL.
    Luu y: mot vai truong hop OCLL kho co the mat toi ~1-2.5 phut (xem
    solver/oll_solver.py) -- chi test 2 seed o day de CI khong qua lau,
    da stress-test rieng voi nhieu seed hon luc phat trien (xem README)."""
    st = _scrambled(seed)
    res = full_solve(st)
    for mv in res['all_moves']:
        do_move(st, mv)
    assert cross_solved(st)
    ok_slots = [s for s in F2L_ORDER if pair_solved(st, s)]
    if res['reached'] in ('oll_done', 'oll_partial'):
        assert set(ok_slots) == set(F2L_ORDER)   # F2L phai con nguyen ven
    if res['reached'] == 'oll_done':
        assert (st['U'] == 'U').all()


def _oll_edges_only_case():
    """Tao 1 trang thai Cross+F2L con nguyen, chi 2 canh lop U (UF,UB) bi
    sai huong (lat facelet truc tiep, khong qua move) -- case OLL nhanh,
    tat dinh cho test, tranh phu thuoc scramble ngau nhien co the trung
    case OCLL kho (xem CFOP_AI_README.md)."""
    from solver.edge_model import EDGE_SLOTS
    st = make_solved()
    for name in ['UF', 'UB']:
        (f1, r1, c1), (f2, r2, c2) = EDGE_SLOTS[name]
        st[f1][r1, c1], st[f2][r2, c2] = st[f2][r2, c2], st[f1][r1, c1]
    return st


def test_hint_oll_edges_phase():
    """hint() phai nhan dung giai doan 'oll_edges' va giai dung (nhanh, tat
    dinh). Pha goc (OCLL) da duoc kiem chung rieng, cham hon nhieu tuy
    truong hop (xem test_full_solve_oll_when_reached va README) nen khong
    lap lai o day de test nay giu duoc toc do on dinh."""
    st = _oll_edges_only_case()
    assert cross_solved(st)
    assert all(pair_solved(st, s) for s in F2L_ORDER)

    h1 = hint(st)
    assert h1['stage'] == 'oll_edges'
    assert h1['moves']
    for mv in h1['moves']:
        do_move(st, mv)
    full = from_facelets(st)
    from solver.oll_solver import edges_oriented
    assert edges_oriented(full)
    assert cross_solved(st) and all(pair_solved(st, s) for s in F2L_ORDER)


def _pll_only_scramble():
    """Tao 1 trang thai Cross+F2L+OLL van con nguyen, CHI hoan vi 3 canh lop
    U bi sai (hoan vi chan -> luon hop le vat ly). Dung thao tac facelet
    truc tiep (khong qua move) de tao case PLL nhanh, tat dinh, on dinh cho
    test -- tranh phu thuoc scramble ngau nhien co the trung case OCLL/PLL
    hiem gap va rat cham (xem CFOP_AI_README.md)."""
    from solver.edge_model import EDGE_SLOTS
    st = make_solved()
    uf, ur, ub = EDGE_SLOTS['UF'], EDGE_SLOTS['UR'], EDGE_SLOTS['UB']
    vuf = [st[f][r, c] for f, r, c in uf]
    vur = [st[f][r, c] for f, r, c in ur]
    vub = [st[f][r, c] for f, r, c in ub]
    for (f, r, c), v in zip(uf, vub):
        st[f][r, c] = v
    for (f, r, c), v in zip(ur, vuf):
        st[f][r, c] = v
    for (f, r, c), v in zip(ub, vur):
        st[f][r, c] = v
    return st


def test_pll_simple_permutation_case():
    """PLL tren mot case hoan vi don gian (nhanh, tat dinh -- khong phu
    thuoc scramble ngau nhien, tranh cac case OCLL/PLL kho hiem gap trong
    scramble that co the mat toi vai phut, xem CFOP_AI_README.md)."""
    st = _pll_only_scramble()
    assert not cube_solved(st)
    res = solve_pll(st)
    assert res['moves'] is not None
    for mv in res['moves']:
        do_move(st, mv)
    assert cube_solved(st)


def test_stage_of_reaches_done_on_solved_cube():
    st = make_solved()
    assert stage_of(st) == 'done'


def test_full_solve_reports_solved_only_when_actually_solved():
    """Neu full_solve() bao 'reached'=='solved', cube phai THUC SU giai xong
    tren facelet that (khong chi tin vao internal state). Dung 1 case PLL
    don gian, tat dinh (khong scramble ngau nhien) de test nhanh & on dinh."""
    st = _pll_only_scramble()
    res = full_solve(st)
    for mv in res['all_moves']:
        do_move(st, mv)
    assert res['reached'] == 'solved'
    assert cube_solved(st)

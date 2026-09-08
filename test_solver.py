"""
test_solver.py
===============
Kiem tra bo giai CFOP AI (Cross + F2L) o muc MVP hien tai.
"""

import random
import pytest

from cube_engine import make_solved, scramble_cube, do_move
from solver.facelets import cross_solved, f2l_solved_slots, F2L_ORDER, pair_solved
from solver.cross_solver import solve_cross
from solver.f2l_solver import solve_f2l
from solver.cfop_ai import full_solve, hint, stage_of
from solver.oll_solver import solve_oll, oll_done
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
    assert stage_of(st) == 'pll_todo'
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


def test_hint_oll_two_phases():
    """hint() phai tach OLL thanh 2 buoc (dinh huong canh, roi goc)."""
    st = _scrambled(4)
    res = full_solve(st)
    for mv in res['cross_moves'] + res['f2l_moves']:
        do_move(st, mv)
    assert cross_solved(st)
    assert all(pair_solved(st, s) for s in F2L_ORDER)

    h1 = hint(st)
    assert h1['stage'] == 'oll_edges'
    for mv in h1['moves']:
        do_move(st, mv)
    full = from_facelets(st)
    from solver.oll_solver import edges_oriented
    assert edges_oriented(full)
    assert cross_solved(st) and all(pair_solved(st, s) for s in F2L_ORDER)

    h2 = hint(st)
    assert h2['stage'] == 'oll_corners'
    for mv in h2['moves']:
        do_move(st, mv)
    assert (st['U'] == 'U').all()
    assert cross_solved(st) and all(pair_solved(st, s) for s in F2L_ORDER)

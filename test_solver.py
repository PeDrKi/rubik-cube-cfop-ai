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
    st = _scrambled(seed)
    res = full_solve(st)
    for mv in res['all_moves']:
        do_move(st, mv)
    assert cross_solved(st)
    ok_slots = [s for s in F2L_ORDER if pair_solved(st, s)]
    assert set(ok_slots) == set(res['f2l_per_slot'].keys()) & set(res['f2l_per_slot'].keys())
    # tat ca cac cap co moves (khong None) phai thuc su duoc giai tren facelet that
    for slot, mvs in res['f2l_per_slot'].items():
        if mvs is not None:
            assert pair_solved(st, slot)


def test_stage_of_progression():
    st = make_solved()
    assert stage_of(st) == 'oll_pll_todo'
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

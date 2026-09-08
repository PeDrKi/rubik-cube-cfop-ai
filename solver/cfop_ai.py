"""
solver/cfop_ai.py
==================
Bo dieu phoi CFOP cap cao nhat, dung cho ca 2 che do UI yeu cau:
  - Auto-solve: goi full_solve(state) -> danh sach nuoc di lien tuc.
  - Hint (goi y tung buoc): goi hint(state) -> buoc TIEP THEO can lam
    (Cross / 1 cap F2L / OLL-canh / OLL-goc), KHONG thuc thi, de nguoi
    dung tu xoay.

Pham vi hien tai (MVP): Cross + F2L + OLL. PLL se bo sung o giai doan sau
(interface da du de cam them ma khong doi kien truc: chi can them
solver/pll_solver.py roi noi vao day, tuong tu cach OLL da duoc noi vao).
"""

import copy

from .facelets import cross_solved, F2L_ORDER, pair_solved, f2l_solved_slots, oll_solved
from .cross_solver import solve_cross
from .f2l_solver import solve_f2l
from .oll_solver import solve_oll, edges_oriented, corners_oriented
from .full_state import from_facelets


def stage_of(state):
    """Tra ve ten giai doan CFOP hien tai: 'cross' | 'f2l' | 'oll' | 'pll_todo'."""
    if not cross_solved(state):
        return 'cross'
    if len(f2l_solved_slots(state)) < len(F2L_ORDER):
        return 'f2l'
    if not oll_solved(state):
        return 'oll'
    return 'pll_todo'   # PLL: phase ke tiep, chua trien khai trong MVP nay


def full_solve(state, f2l_depths=(8, 10, 12, 14), f2l_nodes_per_depth=120_000):
    """
    Giai toan bo nhung gi MVP ho tro (Cross + F2L + OLL) tu state hien tai.
    KHONG thay doi state truyen vao.
    Tra ve dict:
      {
        'cross_moves': [...],
        'f2l_moves':   [...],
        'f2l_per_slot': {...},
        'oll_moves':   [...],
        'all_moves':   [...]  (noi tiep, dung de animate/enqueue truc tiep),
        'reached':     'f2l_partial' | 'f2l_done' | 'oll_partial' | 'oll_done',
      }
    """
    from cube_engine import do_move
    st = copy.deepcopy(state)

    cross_moves = []
    if not cross_solved(st):
        cross_moves = solve_cross(st)
        for mv in cross_moves:
            do_move(st, mv)

    f2l_res = solve_f2l(st, depths=f2l_depths, nodes_per_depth=f2l_nodes_per_depth)
    for mv in f2l_res['moves']:
        do_move(st, mv)

    f2l_done = len(f2l_res['solved_slots']) == len(F2L_ORDER)

    oll_moves = []
    reached = 'f2l_partial'
    if f2l_done:
        reached = 'f2l_done'
        oll_res = solve_oll(st)
        if oll_res['moves']:
            oll_moves = oll_res['moves']
            for mv in oll_moves:
                do_move(st, mv)
        reached = 'oll_done' if oll_solved(st) else 'oll_partial'

    return {
        'cross_moves': cross_moves,
        'f2l_moves': f2l_res['moves'],
        'f2l_per_slot': f2l_res['per_slot'],
        'oll_moves': oll_moves,
        'all_moves': cross_moves + f2l_res['moves'] + oll_moves,
        'reached': reached,
    }


def hint(state):
    """
    Tra ve goi y CHO BUOC TIEP THEO duy nhat (khong thuc thi):
      {'stage': 'cross', 'label': 'Cross', 'moves': [...]}
      {'stage': 'f2l', 'label': 'F2L - cap DFR', 'moves': [...]}
      {'stage': 'oll_edges', 'label': 'OLL - Định hướng 4 cạnh', 'moves': [...]}
      {'stage': 'oll_corners', 'label': 'OLL - Định hướng 4 góc', 'moves': [...]}
      {'stage': 'pll_todo', 'label': '...', 'moves': []}  khi Cross+F2L+OLL
      da xong (PLL la phase ke tiep, chua co trong MVP).
    """
    stage = stage_of(state)

    if stage == 'cross':
        return {'stage': 'cross', 'label': 'Cross', 'moves': solve_cross(state)}

    if stage == 'f2l':
        done = f2l_solved_slots(state)
        remaining = [s for s in F2L_ORDER if s not in done]
        target = remaining[0]
        res = solve_f2l(state, slots=[target])
        mvs = res['per_slot'].get(target)
        if mvs is None:
            return {'stage': 'f2l', 'label': f'F2L - cặp {target} (chưa tìm được trong ngân sách)',
                    'moves': []}
        return {'stage': 'f2l', 'label': f'F2L - cặp {target}', 'moves': mvs}

    if stage == 'oll':
        full = from_facelets(state)
        if not edges_oriented(full):
            oll_res = solve_oll(state)
            mvs = oll_res['edge_moves']
            if mvs is None:
                return {'stage': 'oll_edges',
                        'label': 'OLL - Định hướng 4 cạnh (chưa tìm được trong ngân sách)',
                        'moves': []}
            return {'stage': 'oll_edges', 'label': 'OLL - Định hướng 4 cạnh', 'moves': mvs}
        else:
            oll_res = solve_oll(state)
            mvs = oll_res['corner_moves']
            if mvs is None:
                return {'stage': 'oll_corners',
                        'label': 'OLL - Định hướng 4 góc (chưa tìm được trong ngân sách, thử lại)',
                        'moves': []}
            return {'stage': 'oll_corners', 'label': 'OLL - Định hướng 4 góc', 'moves': mvs}

    return {'stage': 'pll_todo',
            'label': 'Cross + F2L + OLL đã xong! PLL sắp có (phase kế tiếp).',
            'moves': []}

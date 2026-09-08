"""
solver/cfop_ai.py
==================
Bo dieu phoi CFOP cap cao nhat, dung cho ca 2 che do UI yeu cau:
  - Auto-solve: goi full_solve(state) -> danh sach nuoc di lien tuc.
  - Hint (goi y tung buoc): goi hint(state) -> buoc TIEP THEO can lam
    (Cross hoac 1 cap F2L), KHONG thuc thi, de nguoi dung tu xoay.

Pham vi hien tai (MVP): Cross + F2L. OLL/PLL se bo sung o giai doan sau
(interface da du de cam them ma khong doi kien truc: chi can them
solver/oll_solver.py, solver/pll_solver.py roi noi vao day).
"""

from .facelets import cross_solved, F2L_ORDER, pair_solved, f2l_solved_slots
from .cross_solver import solve_cross
from .f2l_solver import solve_f2l


def stage_of(state):
    """Tra ve ten giai doan CFOP hien tai cua state: 'cross' | 'f2l' | 'oll_pll_todo'."""
    if not cross_solved(state):
        return 'cross'
    if len(f2l_solved_slots(state)) < len(F2L_ORDER):
        return 'f2l'
    return 'oll_pll_todo'   # OLL/PLL: phase 2, chua trien khai trong MVP nay


def full_solve(state, f2l_depths=(8, 10, 12, 14), f2l_nodes_per_depth=120_000):
    """
    Giai toan bo nhung gi MVP ho tro (Cross + F2L) tu state hien tai.
    KHONG thay doi state truyen vao.
    Tra ve dict:
      {
        'cross_moves': [...],
        'f2l_moves':   [...],
        'f2l_per_slot': {...},
        'all_moves':   [...]  (noi tiep, dung de animate/enqueue truc tiep),
        'reached':     'f2l_done' | 'f2l_partial',
      }
    """
    import copy
    st = copy.deepcopy(state)

    cross_moves = []
    if not cross_solved(st):
        cross_moves = solve_cross(st)
        from cube_engine import do_move
        for mv in cross_moves:
            do_move(st, mv)

    f2l_res = solve_f2l(st, depths=f2l_depths, nodes_per_depth=f2l_nodes_per_depth)

    reached = 'f2l_done' if len(f2l_res['solved_slots']) == len(F2L_ORDER) else 'f2l_partial'

    return {
        'cross_moves': cross_moves,
        'f2l_moves': f2l_res['moves'],
        'f2l_per_slot': f2l_res['per_slot'],
        'all_moves': cross_moves + f2l_res['moves'],
        'reached': reached,
    }


def hint(state):
    """
    Tra ve goi y CHO BUOC TIEP THEO duy nhat (khong thuc thi):
      {'stage': 'cross', 'label': 'Cross', 'moves': [...]}
    hoac
      {'stage': 'f2l', 'label': 'F2L - cap DFR', 'moves': [...]}
    hoac
      {'stage': 'oll_pll_todo', 'label': '...', 'moves': []}  khi Cross+F2L
      da xong (OLL/PLL la phase 2, chua co trong MVP).
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
            return {'stage': 'f2l', 'label': f'F2L - cap {target} (chua tim duoc trong ngan sach)',
                    'moves': []}
        return {'stage': 'f2l', 'label': f'F2L - cap {target}', 'moves': mvs}

    return {'stage': 'oll_pll_todo',
            'label': 'Cross + F2L da xong! OLL/PLL sap ra mat (phase 2).',
            'moves': []}

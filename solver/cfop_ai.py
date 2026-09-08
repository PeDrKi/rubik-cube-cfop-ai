"""
solver/cfop_ai.py
==================
Bo dieu phoi CFOP cap cao nhat, dung cho ca 2 che do UI yeu cau:
  - Auto-solve: goi full_solve(state) -> danh sach nuoc di lien tuc.
  - Hint (goi y tung buoc): goi hint(state) -> buoc TIEP THEO can lam
    (Cross / 1 cap F2L / OLL-canh / OLL-goc / PLL-goc / PLL-canh), KHONG
    thuc thi, de nguoi dung tu xoay.

Pham vi hien tai: Cross + F2L + OLL + PLL -- toan bo CFOP. PLL la buoc kho
nhat (khong gian hoan vi lon, uu tien "giu nguyen phan da giai" lam bai toan
tim kiem nang) nen thinh thoang tim khong ra trong ngan sach (xem
solver/pll_solver.py) -- khi do AI se bao ro chu khong bao loi/treo may.
"""

import copy

from cube_engine import cube_solved
from .facelets import cross_solved, F2L_ORDER, pair_solved, f2l_solved_slots, oll_solved
from .cross_solver import solve_cross
from .f2l_solver import solve_f2l
from .oll_solver import (solve_oll, edges_oriented, corners_oriented,
                          solve_oll_edges_only, solve_oll_corners_only)
from .pll_solver import (solve_pll, corners_home, edges_home,
                          solve_pll_corners_only, solve_pll_edges_only)
from .full_state import from_facelets
from .move_simplify import simplify


def stage_of(state):
    """Tra ve ten giai doan CFOP hien tai: 'cross'|'f2l'|'oll'|'pll'|'done'."""
    if not cross_solved(state):
        return 'cross'
    if len(f2l_solved_slots(state)) < len(F2L_ORDER):
        return 'f2l'
    if not oll_solved(state):
        return 'oll'
    if not cube_solved(state):
        return 'pll'
    return 'done'


def _full_solve_raw(state, f2l_depths=(8, 10, 12, 14), f2l_nodes_per_depth=120_000, retry=False):
    """
    Giai toan bo CFOP (Cross + F2L + OLL + PLL) tu state hien tai.
    KHONG thay doi state truyen vao.
    Tra ve dict:
      {
        'cross_moves', 'f2l_moves', 'f2l_per_slot', 'oll_moves', 'pll_moves',
        'all_moves':   [...]  (noi tiep, dung de animate/enqueue truc tiep),
        'reached':     'f2l_partial' | 'f2l_done' | 'oll_partial' | 'oll_done'
                       | 'pll_partial' | 'solved',
      }
    """
    from cube_engine import do_move
    st = copy.deepcopy(state)

    cross_moves = []
    if not cross_solved(st):
        cross_moves = solve_cross(st)
        for mv in cross_moves:
            do_move(st, mv)

    f2l_res = solve_f2l(st, depths=f2l_depths, nodes_per_depth=f2l_nodes_per_depth, retry=retry)
    for mv in f2l_res['moves']:
        do_move(st, mv)
    f2l_done = len(f2l_res['solved_slots']) == len(F2L_ORDER)

    oll_moves = []
    pll_moves = []
    reached = 'f2l_partial'
    oll_pll_stage_count = 0
    oll_pll_named_count = 0

    if f2l_done:
        reached = 'f2l_done'
        oll_res = solve_oll(st, retry=retry)
        if oll_res['moves']:
            oll_moves = oll_res['moves']
            for mv in oll_moves:
                do_move(st, mv)
        # Pha A (canh) luon la search trong cai dat nay -> khong tinh vao
        # pattern-conformity (khong co "case ten" nao de so sanh). Chi pha
        # B (goc, Sune/Anti-Sune) moi co khai niem named/search ro rang.
        if oll_res.get('corner_source') is not None:
            oll_pll_stage_count += 1
            if oll_res['corner_source'] == 'named':
                oll_pll_named_count += 1
        oll_done_flag = oll_solved(st)
        reached = 'oll_done' if oll_done_flag else 'oll_partial'

        if oll_done_flag:
            pll_res = solve_pll(st, retry=retry)
            if pll_res['moves']:
                pll_moves = pll_res['moves']
                for mv in pll_moves:
                    do_move(st, mv)
            if pll_res.get('source') is not None:
                oll_pll_stage_count += 1
                if pll_res['source'] == 'named':
                    oll_pll_named_count += 1
            reached = 'solved' if cube_solved(st) else 'pll_partial'

    return {
        'cross_moves': cross_moves,
        'f2l_moves': f2l_res['moves'],
        'f2l_per_slot': f2l_res['per_slot'],
        'oll_moves': oll_moves,
        'pll_moves': pll_moves,
        'all_moves': cross_moves + f2l_res['moves'] + oll_moves + pll_moves,
        'reached': reached,
        'oll_pll_stage_count': oll_pll_stage_count,
        'oll_pll_named_count': oll_pll_named_count,
    }


def _hint_raw(state, retry=False):
    """
    Tra ve goi y CHO BUOC TIEP THEO duy nhat (khong thuc thi):
      {'stage': 'cross', 'label': 'Cross', 'moves': [...]}
      {'stage': 'f2l', 'label': 'F2L - cap DFR', 'moves': [...]}
      {'stage': 'oll_edges'/'oll_corners', 'label': '...', 'moves': [...]}
      {'stage': 'pll_corners'/'pll_edges', 'label': '...', 'moves': [...]}
      {'stage': 'done', 'label': 'Cube da giai xong!', 'moves': []}

    retry=True: xao tron thu tu nuoc di trong search (cung ngan sach
    node/RAM, KHONG tang) de kham pha nhanh KHAC voi lan goi truoc -- dung
    khi nguoi dung bam Hint lai cho CUNG 1 giai doan vua that bai (tim
    kiem von tat dinh, goi lai voi cung state se cho ket qua GIONG HET
    lan truoc neu khong xao tron).
    """
    stage = stage_of(state)

    if stage == 'cross':
        return {'stage': 'cross', 'label': 'Cross', 'moves': solve_cross(state)}

    if stage == 'f2l':
        done = f2l_solved_slots(state)
        remaining = [s for s in F2L_ORDER if s not in done]
        target = remaining[0]
        res = solve_f2l(state, slots=[target], retry=retry)
        mvs = res['per_slot'].get(target)
        if mvs is None:
            return {'stage': 'f2l', 'label': f'F2L - cặp {target} (chưa tìm được trong ngân sách)',
                    'moves': []}
        return {'stage': 'f2l', 'label': f'F2L - cặp {target}', 'moves': mvs}

    if stage == 'oll':
        # Quan trong (toi uu toc do): chi goi PHA CAN THIET, khong goi
        # solve_oll() nguyen khoi -- vi solve_oll() se tinh CA 2 pha, va
        # pha khong can cung co the roi vao case kho (~1 phut), lang phi
        # thoi gian vo ich khi hint() chi can 1 pha.
        full = from_facelets(state)
        if not edges_oriented(full):
            mvs = solve_oll_edges_only(state, retry=retry)
            if mvs is None:
                return {'stage': 'oll_edges',
                        'label': 'OLL - Định hướng 4 cạnh (chưa tìm được, bấm H lại để thử hướng khác)',
                        'moves': []}
            return {'stage': 'oll_edges', 'label': 'OLL - Định hướng 4 cạnh', 'moves': mvs}
        else:
            mvs = solve_oll_corners_only(state, retry=retry)
            if mvs is None:
                return {'stage': 'oll_corners',
                        'label': 'OLL - Định hướng 4 góc (chưa tìm được, bấm H lại để thử hướng khác)',
                        'moves': []}
            return {'stage': 'oll_corners', 'label': 'OLL - Định hướng 4 góc', 'moves': mvs}

    if stage == 'pll':
        # T-perm/Y-perm giai CA PLL trong 1 buoc (khong tach 2-look nhu
        # OLL, vi cac thuat toan nay hoan vi goc+canh DONG THOI) -- nhanh
        # (<0.2s), nen hint tra ve toan bo PLL cung 1 luc thay vi 2 pha.
        res = solve_pll(state, retry=retry)
        mvs = res['moves']
        if not mvs:
            if mvs is None:
                return {'stage': 'pll',
                        'label': 'PLL (chưa tìm được, bấm H lại để thử hướng khác)',
                        'moves': []}
            # mvs == [] : da o dung PLL roi (hiem khi xay ra o day)
        return {'stage': 'pll', 'label': 'PLL - Hoán vị cuối cùng', 'moves': mvs}

    return {'stage': 'done', 'label': 'Cube đã giải xong! 🎉', 'moves': []}


def full_solve(state, f2l_depths=(8, 10, 12, 14), f2l_nodes_per_depth=120_000, retry=False):
    """Wrapper cua _full_solve_raw(): rut gon chuoi nuoc di cuoi cung
    (simplify.py) truoc khi tra ve, khong doi ket qua/tinh dung dan."""
    res = _full_solve_raw(state, f2l_depths=f2l_depths,
                           f2l_nodes_per_depth=f2l_nodes_per_depth, retry=retry)
    res['all_moves'] = simplify(res['all_moves'])
    return res


def hint(state, retry=False):
    """Wrapper cua _hint_raw(): rut gon chuoi nuoc di cua goi y truoc khi
    tra ve, khong doi ket qua/tinh dung dan."""
    res = _hint_raw(state, retry=retry)
    if res.get('moves'):
        res['moves'] = simplify(res['moves'])
    return res

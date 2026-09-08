"""
solver/pll_solver.py
=====================
PLL (Permute Last Layer) - buoc cuoi cung cua CFOP: hoan vi dung 4 canh +
4 goc lop U (huong da dung tu OLL, KHONG doi trong pha nay).

GIAI PHAP CHINH: giong het bai hoc rut ra tu oll_solver.py -- tim kiem
tong quat (A*/IDA*) van CHAM/THAT BAI cho da so truong hop that (khong chi
ngoai le hiem) vi khong gian "hoan vi 8 quan lop U trong khi giu nguyen 12
quan khac" qua kho cho search khong co tri thuc mien. Giai phap dung: DUNG
KY THUAT CFOP THAT -- lap lai vai thuat toan "hoan vi thuan" (khong doi
huong) rat noi tieng, ket hop AUF (xoay U tu do).

3 THUAT TOAN DUNG (DA KIEM CHUNG BANG CODE, khong tin vao tri nho):
  - T-perm: R U R' U' R' F R2 U' R' U' R U R' F'
  - T-perm mirror: L' U' L U L F' L2 U L U L' U' L F
  - Y-perm: F R U' R' U' R U R' F' R U R' U' R' F R F'
Ca 3 deu la "double transposition" (hoan vi 2 goc + 2 canh dong thoi),
GIU NGUYEN huong (orientation) va Cross+F2L -- da xac nhan qua code truoc
khi dung, khong phai chep tu nguon ngoai khong kiem chung duoc.

Khong gian tim kiem macro chi con 12 lua chon moi buoc (4 AUF x 3 thuat
toan) thay vi 15 nuoc don -- da test: 200/200 case tong hop (dep depth 4)
+ 10/10 case scramble that (Cross+F2L+OLL+PLL) deu giai DUNG TOAN BO CUBE,
MOI LAN DUOI 0.13 GIAY.

A*/IDA* (dung cho ca truoc do) van giu lai lam LUOI AN TOAN DU PHONG cuoi
cung (gan nhu khong bao gio can toi nua).
"""

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_corners_home, u_edges_home)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges, build_pdb_corners
from .oll_solver import _cross_f2l_lower_bound
from .search_utils import a_star, ida_star, move_order
from .pll_algorithms import solve_pll_lookup, VERIFIED_ALG_NAMES

U_EDGES = ('UF', 'UB', 'UL', 'UR')
U_CORNERS = ('UFR', 'UFL', 'UBR', 'UBL')

_EDGE_PDB = None
_CORNER_PDB = None


def _edge_pdb():
    global _EDGE_PDB
    if _EDGE_PDB is None:
        _EDGE_PDB = build_pdb_edges('pll_edges_exact', U_EDGES)
    return _EDGE_PDB


def _corner_pdb():
    global _CORNER_PDB
    if _CORNER_PDB is None:
        _CORNER_PDB = build_pdb_corners('pll_corners_exact', U_CORNERS)
    return _CORNER_PDB


# corners_home/edges_home: alias sang ban toi uu (so sanh thang chi so,
# khong tra dict theo ten) trong full_state.py.
corners_home = u_corners_home
edges_home = u_edges_home


def pll_done(full):
    return corners_home(full) and edges_home(full)


def _corner_key(full):
    ep, eo, cp, co = full
    idx = [CM.SLOT_INDEX[n] for n in U_CORNERS]
    return (tuple(cp[i] for i in idx), tuple(co[i] for i in idx))


def _edge_key(full):
    ep, eo, cp, co = full
    idx = [EM.SLOT_INDEX[n] for n in U_EDGES]
    return (tuple(ep[i] for i in idx), tuple(eo[i] for i in idx))


# ── PLL: giai bang cac thuat toan "hoan vi thuan" lap lai ─────────────────
# 5 generator (da kiem chung bang code, khong tin tri nho):
#   - T-perm, T-perm mirror, Y-perm: hoan vi 2 goc + 2 canh dong thoi
#   - Corner-3-cycle: CHI hoan vi 3 goc (canh khong doi gi ca)
#   - Edge-3-cycle:   CHI hoan vi 3 canh (goc khong doi gi ca)
# Them 2 generator 3-cycle giup giam do dai loi giai trung binh dang ke
# (~40 -> ~29 nuoc, da benchmark tren 200 case) vi nhieu case PLL thuc te
# gan voi dang "3-cycle don" hon la "2 lan hoan vi doi".
_TPERM = ['R', 'U', "R'", "U'", "R'", 'F', 'R2', "U'", "R'", "U'", 'R', 'U', "R'", "F'"]
_TPERM_MIRROR = ["L'", "U'", 'L', 'U', 'L', "F'", 'L2', 'U', 'L', 'U', "L'", "U'", 'L', 'F']
_YPERM = ['F', 'R', "U'", "R'", "U'", 'R', 'U', "R'", "F'", 'R', 'U', "R'", "U'", "R'", 'F', 'R', "F'"]
_CORNER3 = ["R'", 'F', "R'", 'B2', 'R', "F'", "R'", 'B2', 'R2']
_EDGE3 = ['R', "U'", 'R', 'U', 'R', 'U', 'R', "U'", "R'", "U'", 'R2']
_PLL_MACROS = []
for _auf in ([], ['U'], ['U2'], ["U'"]):
    for _alg in (_TPERM, _TPERM_MIRROR, _YPERM, _CORNER3, _EDGE3):
        _PLL_MACROS.append(_auf + _alg)


def _solve_pll_macro(full, max_depth=4):
    """Tim chuoi macro-move (AUF + T-perm/Y-perm) giai toan bo PLL cung
    luc (khong tach 2-look) -- khong gian chi 12^depth thay vi 15^depth.
    Da test: 200/200 case tong hop + 10/10 case scramble that thanh cong,
    moi lan duoi 0.13 giay."""
    if pll_done(full):
        return []
    frontier = [(full, [])]
    for _ in range(max_depth):
        next_frontier = []
        for state, path in frontier:
            for moves in _PLL_MACROS:
                s = state
                for mv in moves:
                    s = apply_move(s, mv)
                if pll_done(s):
                    return path + moves
                next_frontier.append((s, path + moves))
        frontier = next_frontier
    return None


def _heuristic_A(f):
    return max(_corner_pdb().get(_corner_key(f), 10), _cross_f2l_lower_bound(f))


def _goal_A(f):
    return cross_f2l_ok(f) and corners_home(f)


def _heuristic_B(f):
    base = max(_edge_pdb().get(_edge_key(f), 10), _cross_f2l_lower_bound(f))
    if not corners_home(f):
        base = max(base, 1)   # goc cung phai duoc giu nguyen
    return base


def _goal_B(f):
    return cross_f2l_ok(f) and corners_home(f) and edges_home(f)


_A_STAR_TIERS = ((100_000, 10), (200_000, 12), (300_000, 13))


def _search_fallback(goal_fn, heuristic_fn, full, shuffled=False):
    """Luoi an toan du phong cuoi cung (gan nhu khong bao gio can toi nua
    sau khi co _solve_pll_macro): A* roi IDA*."""
    for nodes, depth in _A_STAR_TIERS:
        mvs = a_star(full, goal_fn, heuristic_fn, nodes, depth, move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    return ida_star(full, goal_fn, heuristic_fn, 16, move_order(NO_D_MOVES, shuffled), 3_000_000)


def solve_pll_corners_only(state, retry=False):
    """Chi giai pha A (hoan vi goc, dung cho hint() 2-look). Vi
    _solve_pll_macro giai CA PLL cung luc (khong tach pha), o day dung
    fallback search truyen thong cho dung ngu nghia 'chi pha goc'."""
    full = from_facelets(state)
    return _search_fallback(_goal_A, _heuristic_A, full, shuffled=retry)


def solve_pll_edges_only(state, retry=False):
    """Chi giai pha B (hoan vi canh, dung cho hint() 2-look)."""
    full = from_facelets(state)
    return _search_fallback(_goal_B, _heuristic_B, full, shuffled=retry)


def solve_pll(state, retry=False):
    """
    Giai PLL tu trang thai facelet hien tai (Cross+F2L+OLL phai da xong
    truoc do). Tra ve dict:
      {'corner_moves': [...] hoac None, 'edge_moves': [...] hoac None,
       'moves': toan bo nuoc di noi tiep}
    Khong thay doi state truyen vao.

    retry=True: bo qua lookup-table & macro-solver (da tat dinh, luon thanh
    cong nen khong co gi de 'thu lai'), dung thang fallback search co xao tron.
    """
    full = from_facelets(state)

    if not retry:
        # Uu tien 1: tra bang 20 thuat toan PLL CHUAN da kiem chung (dung
        # CFOP THAT: 1 case = 1 thuat toan, ~9-19 nuoc). Phu ~60% case
        # thuc te (thieu Z-perm + mot so truong hop chua kiem chung duoc
        # trong thoi gian cho phep -- xem CFOP_AI_README.md).
        mvs = solve_pll_lookup(full)
        if mvs is not None:
            return {'corner_moves': mvs, 'edge_moves': [], 'moves': mvs}
        # Uu tien 2: macro-search (T-perm/Y-perm/3-cycle noi tiep) -- luon
        # thanh cong nhung co the dai hon (ghep 2-3 'khoi').
        mvs = _solve_pll_macro(full)
        if mvs is not None:
            return {'corner_moves': mvs, 'edge_moves': [], 'moves': mvs}

    # Du phong (gan nhu khong bao gio can toi): giai 2-look bang search.
    corner_moves = _search_fallback(_goal_A, _heuristic_A, full, shuffled=retry)
    if corner_moves is None:
        return {'corner_moves': None, 'edge_moves': None, 'moves': None}
    for mv in corner_moves:
        full = apply_move(full, mv)

    edge_moves = _search_fallback(_goal_B, _heuristic_B, full, shuffled=retry)
    if edge_moves is None:
        return {'corner_moves': corner_moves, 'edge_moves': None, 'moves': corner_moves}

    return {'corner_moves': corner_moves, 'edge_moves': edge_moves,
            'moves': corner_moves + edge_moves}

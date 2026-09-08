"""
solver/pll_solver.py
=====================
PLL (Permute Last Layer) - buoc cuoi cung cua CFOP: hoan vi dung 4 canh +
4 goc lop U (huong da dung tu OLL, KHONG doi trong pha nay).

Kien truc: giong OLL nhung cho HOAN VI thay vi HUONG, va DE HON OLL vi muc
tieu la MOT trang thai DICH CU THE (identity: moi quan ve dung nha), khong
phai "bat ky hoan vi nao" -- nen dung lai dung PDB CHINH XAC (build_pdb_edges/
build_pdb_corners, giong het ky thuat da dung cho Cross) thay vi PDB
"any-perm" cua OLL.

2-Look PLL (dung ky thuat CFOP that):
  Pha A - Permute Corners: hoan vi dung 4 goc U, CHUA quan tam canh.
  Pha B - Permute Edges: hoan vi dung 4 canh U, GIU NGUYEN goc vua xong.

TIM KIEM: IDA* (khong phai A*+dict) -- bo nho O(do sau) thay vi O(so node),
tranh OOM tren may han che RAM. Xem giai thich chi tiet trong oll_solver.py
va solver/search_utils.py.

HEURISTIC: h(state) = max(PDB hoan vi cua nhom dang giai, PDB Cross/F2L
neu bi vo -- tai su dung PDB co san, xem oll_solver._cross_f2l_lower_bound
cho cung ky thuat) -- can duoi THUC SU ADMISSIBLE, manh hon nhieu so voi
penalty 0/1 tuy y dung truoc day.
"""

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_corners_home, u_edges_home)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges, build_pdb_corners
from .oll_solver import _cross_f2l_lower_bound
from .search_utils import ida_star, move_order

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


def _solve_phase_A(full, max_threshold, node_budget, moves=NO_D_MOVES):
    """Permute Corners: 4 goc U ve dung nha, chua quan tam canh."""
    cpdb = _corner_pdb()

    def goal(f):
        return cross_f2l_ok(f) and corners_home(f)

    def heuristic(f):
        return max(cpdb.get(_corner_key(f), 10), _cross_f2l_lower_bound(f))

    return ida_star(full, goal, heuristic, max_threshold, moves, node_budget)


def _solve_phase_B(full, max_threshold, node_budget, moves=NO_D_MOVES):
    """Permute Edges: 4 canh U ve dung nha, GIU NGUYEN goc (da xong pha A)."""
    epdb = _edge_pdb()

    def goal(f):
        return cross_f2l_ok(f) and corners_home(f) and edges_home(f)

    def heuristic(f):
        base = max(epdb.get(_edge_key(f), 10), _cross_f2l_lower_bound(f))
        if not corners_home(f):
            base = max(base, 1)   # goc cung phai duoc giu nguyen
        return base

    return ida_star(full, goal, heuristic, max_threshold, moves, node_budget)


# Ngan sach IDA*: KHONG anh huong RAM -- co the dat lon (xem oll_solver.py).
_TIERS_A = ((10, 300_000), (14, 2_000_000))
_TIERS_B = ((10, 300_000), (14, 2_000_000))


def _ladder(solve_fn, full, tiers, shuffled=False):
    for threshold, budget in tiers:
        mvs = solve_fn(full, threshold, budget, moves=move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    return None


def solve_pll_corners_only(state, retry=False):
    """Chi giai pha A (hoan vi goc). Dung cho hint() de tranh tinh thua
    pha B khi chua can toi. retry=True: xao tron thu tu nuoc di."""
    full = from_facelets(state)
    return _ladder(_solve_phase_A, full, _TIERS_A, shuffled=retry)


def solve_pll_edges_only(state, retry=False):
    """Chi giai pha B (hoan vi canh), GIA SU goc da dung nha san. Dung cho
    hint() de tranh tinh thua pha A. retry=True: xao tron thu tu nuoc di."""
    full = from_facelets(state)
    return _ladder(_solve_phase_B, full, _TIERS_B, shuffled=retry)


def solve_pll(state, retry=False):
    """
    Giai PLL (2-look) tu trang thai facelet hien tai (Cross+F2L+OLL phai da
    xong truoc do: 8 quan lop U da dung huong, chi con sai hoan vi).
    Tra ve dict:
      {'corner_moves': [...] hoac None, 'edge_moves': [...] hoac None,
       'moves': corner_moves + edge_moves}
    Khong thay doi state truyen vao. retry=True: xao tron thu tu nuoc di.
    """
    full = from_facelets(state)

    corner_moves = _ladder(_solve_phase_A, full, _TIERS_A, shuffled=retry)
    if corner_moves is None:
        return {'corner_moves': None, 'edge_moves': None, 'moves': None}
    for mv in corner_moves:
        full = apply_move(full, mv)

    edge_moves = _ladder(_solve_phase_B, full, _TIERS_B, shuffled=retry)
    if edge_moves is None:
        return {'corner_moves': corner_moves, 'edge_moves': None, 'moves': corner_moves}

    return {'corner_moves': corner_moves, 'edge_moves': edge_moves,
            'moves': corner_moves + edge_moves}

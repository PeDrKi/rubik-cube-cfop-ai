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

TIM KIEM: KET HOP A* + IDA* -- GIONG HET oll_solver.py (bai hoc rut ra tu
qua trinh phat trien OLL, ap dung lai o day):
  - A* (heapq + dict nho) la LUA CHON CHINH: co memoization nen KHONG
    DUYET LAI cung 1 nhanh nhieu lan -> nhanh, du doan duoc thoi gian.
    Ngan sach gioi han o muc an toan RAM (~300k node ~3GB, may chi ~4GB).
  - IDA* CHI LA DU PHONG CUOI CUNG khi A* het ngan sach ma van chua ra --
    bo nho O(do sau) nen khong bao gio OOM, du co the cham hon (khong nho
    trang thai da tham nen phai duyet lai mot so nhanh -- nhuoc diem ly
    thuyet kinh dien cua IDA* "re-expansion").
  (Ban dau PLL chi dung IDA* thuan, gay treo lau bat ngo cho vai case --
   xem CFOP_AI_README.md muc "IDA* thuan gay treo".)

HEURISTIC: h(state) = max(PDB hoan vi cua nhom dang giai, PDB Cross/F2L
neu bi vo -- tai su dung PDB co san, xem oll_solver._cross_f2l_lower_bound)
-- can duoi THUC SU ADMISSIBLE.
"""

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_corners_home, u_edges_home)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges, build_pdb_corners
from .oll_solver import _cross_f2l_lower_bound
from .search_utils import a_star, ida_star, move_order

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


def _ladder(goal_fn, heuristic_fn, full, a_star_tiers, ida_threshold, ida_budget, shuffled=False):
    # A* (co nho) truoc -- nhanh, du doan duoc, an toan RAM.
    for nodes, depth in a_star_tiers:
        mvs = a_star(full, goal_fn, heuristic_fn, nodes, depth, move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    # A* het ngan sach ma van chua ra -> IDA* du phong (khong bao gio OOM).
    return ida_star(full, goal_fn, heuristic_fn, ida_threshold,
                     move_order(NO_D_MOVES, shuffled), ida_budget)


_A_STAR_TIERS = ((100_000, 10), (200_000, 12), (300_000, 13))


def solve_pll_corners_only(state, retry=False):
    """Chi giai pha A (hoan vi goc). Dung cho hint() de tranh tinh thua
    pha B khi chua can toi. retry=True: xao tron thu tu nuoc di."""
    full = from_facelets(state)
    return _ladder(_goal_A, _heuristic_A, full, _A_STAR_TIERS, 14, 3_000_000, shuffled=retry)


def solve_pll_edges_only(state, retry=False):
    """Chi giai pha B (hoan vi canh), GIA SU goc da dung nha san. Dung cho
    hint() de tranh tinh thua pha A. retry=True: xao tron thu tu nuoc di."""
    full = from_facelets(state)
    return _ladder(_goal_B, _heuristic_B, full, _A_STAR_TIERS, 16, 4_000_000, shuffled=retry)


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

    corner_moves = _ladder(_goal_A, _heuristic_A, full, _A_STAR_TIERS, 14, 3_000_000, shuffled=retry)
    if corner_moves is None:
        return {'corner_moves': None, 'edge_moves': None, 'moves': None}
    for mv in corner_moves:
        full = apply_move(full, mv)

    edge_moves = _ladder(_goal_B, _heuristic_B, full, _A_STAR_TIERS, 16, 4_000_000, shuffled=retry)
    if edge_moves is None:
        return {'corner_moves': corner_moves, 'edge_moves': None, 'moves': corner_moves}

    return {'corner_moves': corner_moves, 'edge_moves': edge_moves,
            'moves': corner_moves + edge_moves}

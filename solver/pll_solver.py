"""
solver/pll_solver.py
=====================
PLL (Permute Last Layer) - buoc cuoi cung cua CFOP: hoan vi dung 4 canh +
4 goc lop U (huong da dung tu OLL, KHONG doi trong pha nay).

Kien truc: giong OLL nhung cho HOAN VI thay vi HUONG, va DE HON OLL vi muc
tieu la MOT trang thai DICH CU THE (identity: moi quan ve dung nha), khong
phai "bat ky hoan vi nao" -- nen dung lai dung PDB CHINH XAC (build_pdb_edges/
build_pdb_corners, giong het ky thuat da dung cho Cross) thay vi PDB
"any-perm" cua OLL. Heuristic vi vay manh hon nhieu -> nhanh hon OLL nhieu.

2-Look PLL (dung ky thuat CFOP that):
  Pha A - Permute Corners: hoan vi dung 4 goc U, CHUA quan tam canh.
  Pha B - Permute Edges: hoan vi dung 4 canh U, GIU NGUYEN goc vua xong.

Ghi chu: sau OLL, AUF (xoay U tu do) khong lam thay doi hoan vi tuong doi
giua cac quan -- search van tu do dung U khi can, khong can xu ly rieng.
"""

import heapq

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_corners_home, u_edges_home)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges, build_pdb_corners

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
# khong tra dict theo ten) trong full_state.py -- xem CFOP_AI_README.md.
corners_home = u_corners_home
edges_home = u_edges_home


def pll_done(full):
    return corners_home(full) and edges_home(full)


def _cross_f2l_penalty(full):
    return 0 if cross_f2l_ok(full) else 1


def _corner_key(full):
    ep, eo, cp, co = full
    idx = [CM.SLOT_INDEX[n] for n in U_CORNERS]
    return (tuple(cp[i] for i in idx), tuple(co[i] for i in idx))


def _edge_key(full):
    ep, eo, cp, co = full
    idx = [EM.SLOT_INDEX[n] for n in U_EDGES]
    return (tuple(ep[i] for i in idx), tuple(eo[i] for i in idx))


def _search_generic(full_start, goal_fn, heuristic_fn, max_nodes, max_depth):
    if goal_fn(full_start):
        return []

    h0 = heuristic_fn(full_start)
    counter = 0
    heap = [(h0, 0, counter, full_start, (), None)]
    best_g = {full_start: 0}
    nodes = 0

    while heap:
        f, g, _, cur, path, last_face = heapq.heappop(heap)
        if g > best_g.get(cur, 1 << 30):
            continue
        if g >= max_depth:
            continue
        nodes += 1
        if nodes > max_nodes:
            return None
        for mv in NO_D_MOVES:
            face = mv[0]
            if face == last_face:
                continue
            nxt = apply_move(cur, mv)
            ng = g + 1
            if ng < best_g.get(nxt, 1 << 30):
                best_g[nxt] = ng
                npath = path + (mv,)
                if goal_fn(nxt):
                    return list(npath)
                counter += 1
                h = heuristic_fn(nxt)
                heapq.heappush(heap, (ng + h, ng, counter, nxt, npath, face))
    return None


def _solve_phase_A(full, max_nodes, max_depth):
    """Permute Corners: 4 goc U ve dung nha, chua quan tam canh."""
    cpdb = _corner_pdb()

    def goal(f):
        return cross_f2l_ok(f) and corners_home(f)

    def heuristic(f):
        return cpdb.get(_corner_key(f), 10) + 3 * _cross_f2l_penalty(f)

    return _search_generic(full, goal, heuristic, max_nodes, max_depth)


def _solve_phase_B(full, max_nodes, max_depth):
    """Permute Edges: 4 canh U ve dung nha, GIU NGUYEN goc (da xong pha A)."""
    epdb = _edge_pdb()

    def corner_penalty(f):
        return 0 if corners_home(f) else 1

    def goal(f):
        return cross_f2l_ok(f) and corners_home(f) and edges_home(f)

    def heuristic(f):
        return epdb.get(_edge_key(f), 10) + 3 * _cross_f2l_penalty(f) + 4 * corner_penalty(f)

    return _search_generic(full, goal, heuristic, max_nodes, max_depth)


def _ladder(solve_fn, full, tiers):
    for nodes, depth in tiers:
        mvs = solve_fn(full, max_nodes=nodes, max_depth=depth)
        if mvs is not None:
            return mvs
    return None


_TIERS_A = ((60_000, 8), (150_000, 10), (300_000, 12))
_TIERS_B = ((60_000, 8), (150_000, 10), (300_000, 12))


def solve_pll_corners_only(state):
    """Chi giai pha A (hoan vi goc). Dung cho hint() de tranh tinh thua
    pha B khi chua can toi."""
    full = from_facelets(state)
    return _ladder(_solve_phase_A, full, _TIERS_A)


def solve_pll_edges_only(state):
    """Chi giai pha B (hoan vi canh), GIA SU goc da dung nha san. Dung cho
    hint() de tranh tinh thua pha A."""
    full = from_facelets(state)
    return _ladder(_solve_phase_B, full, _TIERS_B)


def solve_pll(state):
    """
    Giai PLL (2-look) tu trang thai facelet hien tai (Cross+F2L+OLL phai da
    xong truoc do: 8 quan lop U da dung huong, chi con sai hoan vi).
    Tra ve dict:
      {'corner_moves': [...] hoac None, 'edge_moves': [...] hoac None,
       'moves': corner_moves + edge_moves}
    Khong thay doi state truyen vao.
    """
    full = from_facelets(state)

    corner_moves = _ladder(_solve_phase_A, full, _TIERS_A)
    if corner_moves is None:
        return {'corner_moves': None, 'edge_moves': None, 'moves': None}
    for mv in corner_moves:
        full = apply_move(full, mv)

    edge_moves = _ladder(_solve_phase_B, full, _TIERS_B)
    if edge_moves is None:
        return {'corner_moves': corner_moves, 'edge_moves': None, 'moves': corner_moves}

    return {'corner_moves': corner_moves, 'edge_moves': edge_moves,
            'moves': corner_moves + edge_moves}

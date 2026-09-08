"""
solver/oll_solver.py
=====================
OLL (Orient Last Layer) - buoc thu 3 cua CFOP: xoay huong toan bo 4 canh +
4 goc lop U sao cho mat U dong mau, KHONG quan tam hoan vi (PLL lo sau).

Nhan xet quan trong (suy ra tu Cross+F2L da xong):
  Cross xong (4 canh D) + F2L xong (4 goc D + 4 canh giua) = 12/20 quan da
  ve dung nha. Theo nguyen ly Dirichlet (pigeonhole), 8 quan con lai (4 canh
  U + 4 goc U) CHAC CHAN dang nam o 8 khe U (co the sai huong/sai hoan vi,
  nhung chac chan o lop U) -- vi khong con khe nao khac trong.

Kien truc: "2-Look OLL" (dung ky thuat CFOP that: tach OLL thanh 2 pha rieng,
thay vi tim ca 8 quan cung luc -- ca hai deu dung PDB + A* CUA CHINH TA,
KHONG phai bang tra 57 cong thuc hoc thuoc long tu nguon ngoai):

  Pha A - Orient Edges (EOLL): chi xoay huong 4 canh U, dung PDB "gop" cho
    canh (da xay o F2L/Cross-tuong-tu, 190,080 trang thai, CHINH XAC tuyet
    doi) -- giong tinh than solve_cross (PDB chinh xac -> tim nhanh).

  Pha B - Orient Corners (OCLL): sau khi canh da huong dung, xoay huong 4
    goc U ma KHONG lam sai huong canh vua xong, dung PDB "gop" cho goc
    (136,080 trang thai, chinh xac tuyet doi) + penalty giu canh/Cross/F2L.

Tach lam 2 pha (thay vi 1 pha tim ca 8 quan) giup khong gian tim kiem nho
lai rat nhieu (heuristic khop sat voi tung pha), nhanh hon nhieu so voi
tim dong thoi ca edge+corner (da thu nghiem: tim chung mat >60s, tach pha
mat <2s/pha o da so truong hop).
"""

import heapq

from .full_state import from_facelets, apply_move, cross_ok, pair_ok, NO_D_MOVES
from . import edge_model as EM
from . import corner_model as CM
from .facelets import F2L_ORDER
from .pdb_builder import build_pdb_edges_group_anyperm, build_pdb_corners_group_anyperm

U_EDGES = ('UF', 'UB', 'UL', 'UR')
U_CORNERS = ('UFR', 'UFL', 'UBR', 'UBL')

_EDGE_PDB = None
_CORNER_PDB = None


def _edge_pdb():
    global _EDGE_PDB
    if _EDGE_PDB is None:
        _EDGE_PDB = build_pdb_edges_group_anyperm('oll_edges_group', U_EDGES)
    return _EDGE_PDB


def _corner_pdb():
    global _CORNER_PDB
    if _CORNER_PDB is None:
        _CORNER_PDB = build_pdb_corners_group_anyperm('oll_corners_group', U_CORNERS)
    return _CORNER_PDB


def edges_oriented(full):
    ep, eo, cp, co = full
    return all(eo[EM.SLOT_INDEX[n]] == 0 for n in U_EDGES)


def corners_oriented(full):
    ep, eo, cp, co = full
    return all(co[CM.SLOT_INDEX[n]] == 0 for n in U_CORNERS)


def oll_done(full):
    return edges_oriented(full) and corners_oriented(full)


def _cross_f2l_penalty(full):
    bad = 0 if cross_ok(full) else 1
    bad += sum(1 for s in F2L_ORDER if not pair_ok(full, s))
    return bad


def _edge_key(full):
    ep, eo, cp, co = full
    idx = [EM.SLOT_INDEX[n] for n in U_EDGES]
    return (tuple(ep[i] for i in idx), tuple(eo[i] for i in idx))


def _corner_key(full):
    ep, eo, cp, co = full
    idx = [CM.SLOT_INDEX[n] for n in U_CORNERS]
    return (tuple(cp[i] for i in idx), tuple(co[i] for i in idx))


def _search_generic(full_start, goal_fn, heuristic_fn, max_nodes, max_depth):
    """A* tong quat (dung chung cho ca 2 pha), giong f2l_solver.py."""
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


def _solve_phase_A(full, max_nodes=200_000, max_depth=10):
    """Orient Edges: chi can 4 canh U huong dung, khong quan tam goc."""
    epdb = _edge_pdb()

    def goal(f):
        return cross_ok(f) and all(pair_ok(f, s) for s in F2L_ORDER) and edges_oriented(f)

    def heuristic(f):
        base = epdb.get(_edge_key(f), 8)
        return base + 3 * _cross_f2l_penalty(f)

    return _search_generic(full, goal, heuristic, max_nodes, max_depth)


def _solve_phase_B(full, max_nodes=200_000, max_depth=13):
    """Orient Corners: 4 goc U huong dung, GIU NGUYEN canh (da xong o pha A)."""
    cpdb = _corner_pdb()

    def edge_penalty(f):
        ep, eo, cp, co = f
        return sum(1 for n in U_EDGES if eo[EM.SLOT_INDEX[n]] != 0)

    def goal(f):
        return (cross_ok(f) and all(pair_ok(f, s) for s in F2L_ORDER)
                and edges_oriented(f) and corners_oriented(f))

    def heuristic(f):
        base = cpdb.get(_corner_key(f), 8)
        return base + 3 * _cross_f2l_penalty(f) + 4 * edge_penalty(f)

    return _search_generic(full, goal, heuristic, max_nodes, max_depth)


def _solve_phase_A_ladder(full):
    for nodes, depth in ((80_000, 8), (200_000, 10), (400_000, 12)):
        mvs = _solve_phase_A(full, max_nodes=nodes, max_depth=depth)
        if mvs is not None:
            return mvs
    return None


def _solve_phase_B_ladder(full):
    # da so truong hop (Sune/Anti-Sune/T/U ...) giai nhanh voi ngan sach nho;
    # mot so truong hop hiem (VD H-case) can toi da sau hon -> ngan sach lon
    # hon nhieu nhung chi tra gia khi thuc su can (chay nen, khong dong UI).
    # Da gioi han tang cuoi de tranh cho qua lau (~1 phut toi da thuc te).
    for nodes, depth in ((80_000, 9), (150_000, 11), (300_000, 13), (500_000, 14)):
        mvs = _solve_phase_B(full, max_nodes=nodes, max_depth=depth)
        if mvs is not None:
            return mvs
    return None


def solve_oll(state):
    """
    Giai OLL (2-look) tu trang thai facelet hien tai (Cross+F2L phai da xong).
    Tra ve dict:
      {'edge_moves': [...] hoac None, 'corner_moves': [...] hoac None,
       'moves': edge_moves + corner_moves (noi tiep, de animate truc tiep)}
    Khong thay doi state truyen vao.

    Ghi chu: da so truong hop giai <2s. Mot vai truong hop OCLL kho (VD
    H-case) co the mat toi ~1 phut do phai leo thang ngan sach tim kiem --
    chay nen (thread) nen khong lam dong UI.
    """
    full = from_facelets(state)

    edge_moves = _solve_phase_A_ladder(full)
    if edge_moves is None:
        return {'edge_moves': None, 'corner_moves': None, 'moves': None}
    for mv in edge_moves:
        full = apply_move(full, mv)

    corner_moves = _solve_phase_B_ladder(full)
    if corner_moves is None:
        return {'edge_moves': edge_moves, 'corner_moves': None, 'moves': edge_moves}

    return {'edge_moves': edge_moves, 'corner_moves': corner_moves,
            'moves': edge_moves + corner_moves}

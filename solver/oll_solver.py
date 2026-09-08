"""
solver/oll_solver.py
=====================
OLL (Orient Last Layer) - buoc thu 3 cua CFOP: xoay huong toan bo 4 canh +
4 goc lop U sao cho mat U dong mau, KHONG quan tam hoan vi (PLL lo sau).

Nhan xet quan trong (suy ra tu Cross+F2L da xong):
  Cross xong (4 canh D) + F2L xong (4 goc D + 4 canh giua) = 12/20 quan da
  ve dung nha. Theo nguyen ly Dirichlet (pigeonhole), 8 quan con lai (4 canh
  U + 4 goc U) CHAC CHAN dang nam o 8 khe U -- vi khong con khe nao khac trong.

Kien truc: "2-Look OLL" (tach OLL thanh 2 pha rieng: dinh huong canh truoc,
dinh huong goc sau -- da thu tim ca 8 quan cung luc va qua cham vi khong
gian tuong tac giua canh+goc qua lon de lam PDB dung nghia).

TIM KIEM: KET HOP A* + IDA* (bai hoc rut ra tu qua trinh phat trien):
  - A* (heapq + dict nho best_g) la LUA CHON CHINH: memoization giup
    KHONG DUYET LAI cung 1 nhanh nhieu lan, nen nhanh va DU DOAN DUOC thoi
    gian -- nhung dict phinh to theo so node (~300k node ~ 3GB tren may
    chi ~4GB RAM, da kiem tra qua /proc/meminfo) nen phai gioi han ngan
    sach o muc an toan.
  - IDA* la PHUONG AN DU PHONG CUOI CUNG (chi dung khi A* het ngan sach ma
    van chua tim ra): bo nho O(do sau) thay vi O(so node) nen KHONG BAO GIO
    OOM, du co the cham hon do khong memo hoa (duyet lai mot so nhanh).

  Ly do dung ca hai (khong chi IDA* thuan): da phat hien thuc te mot case
  h0=4 (nhin qua don gian) khien IDA* thuan bi "re-expansion" (nhuoc diem
  ly thuyet kinh dien cua IDA*: khong nho trang thai da tham nen phai
  duyet lai toan bo cay tim kiem o MOI nguong do sau) treo rat lau, trong
  khi A* co nho giai no gan nhu ngay lap tuc. Ket hop ca hai vua nhanh cho
  da so case (A* uu tien) vua an toan RAM cho case kho nhat (IDA* du phong).

HEURISTIC: h(state) = max(
    PDB "gop" huong cua 4 goc U (hoac 4 canh U o pha A),
    PDB Cross (dung lai tu cross_solver.py) neu Cross dang bi vo,
    PDB tung cap F2L (dung lai tu f2l_solver.py) neu cap do bi vo,
  )
  Day la CAN DUOI THUC SU ADMISSIBLE (khong phai penalty tuy y): neu Cross
  dang lech d buoc so voi da giai, chac chan can it nhat d buoc nua de sua
  no -- nen max(...) khong bao gio danh gia cao hon thuc te. Manh hon
  nhieu so voi co/khong-bi-vo (0/1) dung truoc do.
"""

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_edges_oriented, u_corners_oriented)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges_group_anyperm, build_pdb_corners_group_anyperm
from .cross_solver import get_pdb as _get_cross_pdb
from .f2l_solver import _pair_pdb, F2L_EDGE_OF
from .facelets import F2L_ORDER
from .search_utils import a_star, ida_star, move_order

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


# edges_oriented/corners_oriented: alias sang ban toi uu (so sanh thang
# chi so, khong tra dict theo ten) trong full_state.py.
edges_oriented = u_edges_oriented
corners_oriented = u_corners_oriented


def oll_done(full):
    return edges_oriented(full) and corners_oriented(full)


def _cross_f2l_lower_bound(full):
    """Can duoi THUC SU (khong phai co/khong) cho so buoc can de phuc hoi
    Cross+F2L neu dang bi vo, tai su dung nguyen PDB cua cross_solver.py va
    f2l_solver.py (khong xay PDB moi)."""
    ep, eo, cp, co = full
    best = _get_cross_pdb().get((ep[4:8], eo[4:8]), 8)
    for slot in F2L_ORDER:
        ci = CM.SLOT_INDEX[slot]
        ei = EM.SLOT_INDEX[F2L_EDGE_OF[slot]]
        key = ((ep[ei],), (eo[ei],), (cp[ci],), (co[ci],))
        d = _pair_pdb(slot).get(key, 8)
        if d > best:
            best = d
    return best


def _edge_key(full):
    ep, eo, cp, co = full
    idx = [EM.SLOT_INDEX[n] for n in U_EDGES]
    return (tuple(ep[i] for i in idx), tuple(eo[i] for i in idx))


def _corner_key(full):
    ep, eo, cp, co = full
    idx = [CM.SLOT_INDEX[n] for n in U_CORNERS]
    return (tuple(cp[i] for i in idx), tuple(co[i] for i in idx))


def _heuristic_A(f):
    return max(_edge_pdb().get(_edge_key(f), 8), _cross_f2l_lower_bound(f))


def _goal_A(f):
    return cross_f2l_ok(f) and edges_oriented(f)


def _heuristic_B(f):
    base = max(_corner_pdb().get(_corner_key(f), 8), _cross_f2l_lower_bound(f))
    if not edges_oriented(f):
        base = max(base, 1)   # canh cung phai duoc giu nguyen
    return base


def _goal_B(f):
    return cross_f2l_ok(f) and edges_oriented(f) and corners_oriented(f)


def _solve_phase_A_ladder(full, shuffled=False):
    # A* (co nho) truoc -- nhanh, du doan duoc thoi gian, an toan RAM voi
    # ngan sach da kiem chung (~300k node ~ 3GB tren may ~4GB RAM).
    for nodes, depth in ((80_000, 8), (200_000, 10), (300_000, 12)):
        mvs = a_star(full, _goal_A, _heuristic_A, nodes, depth, move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    # A* het ngan sach ma van chua ra -> IDA* du phong (khong bao gio OOM,
    # co the cham hon nhung se khong crash).
    return ida_star(full, _goal_A, _heuristic_A, 14, move_order(NO_D_MOVES, shuffled), 3_000_000)


def _solve_phase_B_ladder(full, shuffled=False):
    for nodes, depth in ((80_000, 9), (150_000, 11), (300_000, 13)):
        mvs = a_star(full, _goal_B, _heuristic_B, nodes, depth, move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    return ida_star(full, _goal_B, _heuristic_B, 16, move_order(NO_D_MOVES, shuffled), 4_000_000)


def solve_oll_edges_only(state, retry=False):
    """Chi giai pha A (dinh huong canh). Dung cho hint() de tranh tinh
    thua pha B khi chua can toi. retry=True: xao tron thu tu nuoc di."""
    full = from_facelets(state)
    return _solve_phase_A_ladder(full, shuffled=retry)


def solve_oll_corners_only(state, retry=False):
    """Chi giai pha B (dinh huong goc), GIA SU canh da huong dung san.
    retry=True: xem solve_oll_edges_only()."""
    full = from_facelets(state)
    return _solve_phase_B_ladder(full, shuffled=retry)


def solve_oll(state, retry=False):
    """
    Giai OLL (2-look) tu trang thai facelet hien tai (Cross+F2L phai da xong).
    Tra ve dict:
      {'edge_moves': [...] hoac None, 'corner_moves': [...] hoac None,
       'moves': edge_moves + corner_moves}
    Khong thay doi state truyen vao. retry=True: xao tron thu tu nuoc di.
    """
    full = from_facelets(state)

    edge_moves = _solve_phase_A_ladder(full, shuffled=retry)
    if edge_moves is None:
        return {'edge_moves': None, 'corner_moves': None, 'moves': None}
    for mv in edge_moves:
        full = apply_move(full, mv)

    corner_moves = _solve_phase_B_ladder(full, shuffled=retry)
    if corner_moves is None:
        return {'edge_moves': edge_moves, 'corner_moves': None, 'moves': edge_moves}

    return {'edge_moves': edge_moves, 'corner_moves': corner_moves,
            'moves': edge_moves + corner_moves}

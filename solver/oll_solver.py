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
thay vi tim ca 8 quan cung luc -- da thu va qua cham vi khong gian tuong tac
giua canh+goc qua lon de lam PDB dung nghia):

  Pha A - Orient Edges (EOLL): chi xoay huong 4 canh U, dung PDB "gop" cho
    canh (190,080 trang thai, CHINH XAC tuyet doi).

  Pha B - Orient Corners (OCLL): sau khi canh da huong dung, xoay huong 4
    goc U ma KHONG lam sai huong canh vua xong.

TIM KIEM: dung IDA* (khong phai A*+dict) -- ly do:
  A* voi dict luu tat ca trang thai da tham chiem RAM O(so node), va may
  chay chi co ~4GB RAM (da kiem tra qua /proc/meminfo) -- ~300k node A* da
  gan cham RAM, hon nua co the bi kernel OOM-kill CA TIEN TRINH. IDA* (DFS
  lap sau tang dan) chi ton RAM O(do sau tim kiem) (vai chuc phan tu), cho
  phep duyet nhieu trieu node ma khong lo het RAM -- doi lai co the duyet
  lai 1 nhanh nhieu lan (khong memo hoa). Day la ly do Korf (1997) dung
  IDA* cho solver Rubik toi uu thay vi A* thuan.

HEURISTIC: h(state) = max(
    PDB "gop" huong cua 4 goc U (hoac 4 canh U o pha A),
    PDB Cross (dung lai nguyen ban tu cross_solver.py) neu Cross dang bi vo,
    PDB tung cap F2L (dung lai nguyen ban tu f2l_solver.py) neu cap do bi vo,
  )
  Day la CAN DUOI THUC SU ADMISSIBLE (khong phai penalty tuy y nhu ban truoc):
  neu Cross dang bi lech d buoc so voi trang thai da giai, thi CHAC CHAN can
  it nhat d buoc nua de sua no (bat ke sua gi khac) -- nen max(...) khong
  bao gio danh gia CAO HON so thuc te. Manh hon nhieu so voi "co/khong bi
  vo" (0/1) dung truoc day -- chinh nho heuristic nay ma IDA* giai duoc ca
  cac case OCLL kho nhat da gap trong qua trinh phat trien (VD 2 goc xoay
  nguoc chieu -- truoc day A* that bai hoan toan du dung toi RAM toi da).
"""

from .full_state import (from_facelets, apply_move, NO_D_MOVES, cross_f2l_ok,
                          u_edges_oriented, u_corners_oriented)
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges_group_anyperm, build_pdb_corners_group_anyperm
from .cross_solver import get_pdb as _get_cross_pdb
from .f2l_solver import _pair_pdb, F2L_EDGE_OF
from .facelets import F2L_ORDER
from .search_utils import ida_star, move_order

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


# edges_oriented/corners_oriented: alias sang ban toi uu (khong tra dict
# theo ten, so sanh thang chi so) trong full_state.py.
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


def _solve_phase_A(full, max_threshold, node_budget, moves=NO_D_MOVES):
    """Orient Edges: chi can 4 canh U huong dung, khong quan tam goc."""
    epdb = _edge_pdb()

    def goal(f):
        return cross_f2l_ok(f) and edges_oriented(f)

    def heuristic(f):
        return max(epdb.get(_edge_key(f), 8), _cross_f2l_lower_bound(f))

    return ida_star(full, goal, heuristic, max_threshold, moves, node_budget)


def _solve_phase_B(full, max_threshold, node_budget, moves=NO_D_MOVES):
    """Orient Corners: 4 goc U huong dung, GIU NGUYEN canh (da xong o pha A)."""
    cpdb = _corner_pdb()

    def goal(f):
        return cross_f2l_ok(f) and edges_oriented(f) and corners_oriented(f)

    def heuristic(f):
        base = max(cpdb.get(_corner_key(f), 8), _cross_f2l_lower_bound(f))
        if not edges_oriented(f):
            base = max(base, 1)   # canh cung phai duoc giu nguyen
        return base

    return ida_star(full, goal, heuristic, max_threshold, moves, node_budget)


# Ngan sach IDA*: KHONG anh huong RAM (chi anh huong thoi gian cho phep) vi
# IDA* khong luu visited-state -- co the dat lon hon nhieu so voi A* truoc
# day ma van an toan (da benchmark: ~65-120MB kha ca voi 3 trieu node).
_TIERS_A = ((10, 300_000), (14, 2_000_000))
_TIERS_B = ((10, 300_000), (13, 1_000_000), (16, 4_000_000))


def _solve_phase_A_ladder(full, shuffled=False):
    for threshold, budget in _TIERS_A:
        mvs = _solve_phase_A(full, threshold, budget, moves=move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    return None


def _solve_phase_B_ladder(full, shuffled=False):
    for threshold, budget in _TIERS_B:
        mvs = _solve_phase_B(full, threshold, budget, moves=move_order(NO_D_MOVES, shuffled))
        if mvs is not None:
            return mvs
    return None


def solve_oll_edges_only(state, retry=False):
    """Chi giai pha A (dinh huong canh). Dung cho hint() de tranh tinh
    thua pha B khi chua can toi (xem ghi chu trong cfop_ai.hint()).
    retry=True: xao tron thu tu nuoc di cho lan 'thu lai'."""
    full = from_facelets(state)
    return _solve_phase_A_ladder(full, shuffled=retry)


def solve_oll_corners_only(state, retry=False):
    """Chi giai pha B (dinh huong goc), GIA SU canh da huong dung san.
    Dung cho hint() de tranh tinh thua pha A. retry=True: xem
    solve_oll_edges_only()."""
    full = from_facelets(state)
    return _solve_phase_B_ladder(full, shuffled=retry)


def solve_oll(state, retry=False):
    """
    Giai OLL (2-look) tu trang thai facelet hien tai (Cross+F2L phai da xong).
    Tra ve dict:
      {'edge_moves': [...] hoac None, 'corner_moves': [...] hoac None,
       'moves': edge_moves + corner_moves (noi tiep, de animate truc tiep)}
    Khong thay doi state truyen vao.

    Ghi chu: da so truong hop giai <2s. Mot vai truong hop OCLL kho co the
    mat toi ~1-2 phut do phai leo thang ngan sach tim kiem -- chay nen
    (thread) nen khong lam dong UI, va KHONG BAO GIO lam het RAM (IDA*).
    retry=True: xao tron thu tu nuoc di cho lan thu lai kham pha khac.
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

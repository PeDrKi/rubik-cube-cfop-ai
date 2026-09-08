"""
solver/cross_solver.py
=======================
Giai Cross (4 canh cua mat D) toi uu (it nuoc nhat) bang Pattern Database
o muc do "cubie" (vi tri + huong cua tung canh trong 12 khe), theo dung ky
thuat PDB kinh dien (Korf). Xem giai thich chi tiet trong edge_model.py.

Vi PDB cho khoang cach CHINH XAC trong khong gian rut gon nay, tu bat ky
trang thai nao (khac 0) luon ton tai it nhat 1 nuoc lam khoang cach giam
dung 1 -> chi can leo doc tham lam (greedy) la ra loi giai toi uu, khong
can A*/IDA*.
"""

from cube_engine import ALL_MOVES
from .edge_model import apply_move_to_edges, edges_from_state
from .pdb_builder import build_pdb_edges

TRACKED = ['DF', 'DB', 'DL', 'DR']   # 4 canh cross
_PDB = None


def get_pdb(verbose=False):
    global _PDB
    if _PDB is None:
        _PDB = build_pdb_edges('cross_edges', TRACKED, verbose=verbose)
    return _PDB


def solve_cross(state, max_moves=20):
    """Tra ve list nuoc di (Singmaster) giai Cross toi uu tu state hien tai.
    Khong thay doi state truyen vao."""
    pdb = get_pdb()
    pos, ori = edges_from_state(state, TRACKED)
    key = (tuple(pos), tuple(ori))
    d = pdb.get(key)
    if d is None:
        raise ValueError('Trang thai cross khong hop le / khong nam trong PDB')

    moves = []
    guard = 0
    while d > 0 and guard < max_moves:
        guard += 1
        for mv in ALL_MOVES:
            new_pos, new_ori = apply_move_to_edges(pos, ori, mv)
            k = (tuple(new_pos), tuple(new_ori))
            nd = pdb.get(k)
            if nd is not None and nd == d - 1:
                pos, ori = new_pos, new_ori
                moves.append(mv)
                d = nd
                break
        else:
            raise RuntimeError('Khong tim duoc nuoc giam khoang cach (bug PDB?)')
    return moves

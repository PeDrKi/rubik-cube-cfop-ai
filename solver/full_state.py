"""
solver/full_state.py
=====================
Trang thai cube day du o muc "cubie": (vi tri+huong cua 12 canh, vi tri+huong
cua 8 goc). Day la mo hinh chuan de lam AI search (A*/IDA*) chinh xac, khong
con van de "thong tin an" nhu khi doc truc tiep facelet cuc bo (xem
edge_model.py). Chuyen doi tu facelet (cube_engine state) sang cubie state
chi can lam MOT LAN khi bat dau tim loi giai; sau do toan bo qua trinh search
chay thuan tren tuple so nguyen (nhanh, khong dung numpy).
"""

from cube_engine import ALL_MOVES
from . import edge_model as EM
from . import corner_model as CM

# Tap 12 nuoc khong dung D (bao toan lop D trong khi ghep F2L)
NO_D_MOVES = [m for m in ALL_MOVES if not m.startswith('D')]


def from_facelets(state):
    ep, eo = EM.edges_from_state(state, EM.SLOT_NAMES)
    cp, co = CM.corners_from_state(state, CM.SLOT_NAMES)
    return (tuple(ep), tuple(eo), tuple(cp), tuple(co))


def apply_move(full, mv):
    ep, eo, cp, co = full
    nep, neo = EM.apply_move_to_edges(list(ep), list(eo), mv)
    ncp, nco = CM.apply_move_to_corners(list(cp), list(co), mv)
    return (tuple(nep), tuple(neo), tuple(ncp), tuple(nco))


def cross_ok(full):
    ep, eo, cp, co = full
    for name in ('DF', 'DB', 'DL', 'DR'):
        i = EM.SLOT_INDEX[name]
        if ep[i] != i or eo[i] != 0:
            return False
    return True


F2L_EDGE_OF = {'DFR': 'FR', 'DFL': 'FL', 'DBR': 'BR', 'DBL': 'BL'}


def pair_ok(full, slot):
    ep, eo, cp, co = full
    ci = CM.SLOT_INDEX[slot]
    if cp[ci] != ci or co[ci] != 0:
        return False
    ei = EM.SLOT_INDEX[F2L_EDGE_OF[slot]]
    if ep[ei] != ei or eo[ei] != 0:
        return False
    return True

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
    ep, eo = full[0], full[1]
    return (ep[4] == 4 and ep[5] == 5 and ep[6] == 6 and ep[7] == 7
            and eo[4] == 0 and eo[5] == 0 and eo[6] == 0 and eo[7] == 0)


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


# ── Kiem tra nhanh (hot path cho A*) ──────────────────────────────────────
# Nho thu tu SLOT_NAMES co dinh (xem edge_model.py / corner_model.py):
#   edges:   0-3 = U* (UF,UB,UL,UR)   4-7 = D* (cross)   8-11 = F2L-edge
#   corners: 0-3 = U* (UFR..UBL)      4-7 = D* (F2L-corner)
# => cac ham duoi day so sanh THANG chi so nguyen (khong tra dict theo ten)
# nen nhanh hon dang ke so voi vong lap qua cross_ok()/pair_ok() (da benchmark
# ~1.5x nhanh hon) -- quan trong vi day la ham goi NHIEU NHAT trong A*.


def cross_f2l_ok(full):
    """True neu Cross + toan bo 4 cap F2L deu con nguyen (dung vi tri +
    huong). Tuong duong cross_ok(full) and all(pair_ok(full,s) for s in
    F2L_ORDER) nhung nhanh hon (~1.5x, da do benchmark) vi khong tra dict
    theo ten (EM.SLOT_INDEX[...], F2L_EDGE_OF[...]) ma so sanh thang chi
    so nguyen da biet truoc (dua vao thu tu SLOT_NAMES co dinh o tren)."""
    ep, eo, cp, co = full
    return (ep[4] == 4 and ep[5] == 5 and ep[6] == 6 and ep[7] == 7
            and ep[8] == 8 and ep[9] == 9 and ep[10] == 10 and ep[11] == 11
            and eo[4] == 0 and eo[5] == 0 and eo[6] == 0 and eo[7] == 0
            and eo[8] == 0 and eo[9] == 0 and eo[10] == 0 and eo[11] == 0
            and cp[4] == 4 and cp[5] == 5 and cp[6] == 6 and cp[7] == 7
            and co[4] == 0 and co[5] == 0 and co[6] == 0 and co[7] == 0)


def u_edges_oriented(full):
    """4 canh lop U (UF,UB,UL,UR) da dung huong (khong quan tam vi tri)."""
    eo = full[1]
    return eo[0] == 0 and eo[1] == 0 and eo[2] == 0 and eo[3] == 0


def u_corners_oriented(full):
    """4 goc lop U (UFR,UFL,UBR,UBL) da dung huong (khong quan tam vi tri)."""
    co = full[3]
    return co[0] == 0 and co[1] == 0 and co[2] == 0 and co[3] == 0


def u_corners_home(full):
    """4 goc lop U dung ca vi tri lan huong (PLL: pha goc da xong)."""
    cp, co = full[2], full[3]
    return (cp[0] == 0 and cp[1] == 1 and cp[2] == 2 and cp[3] == 3
            and co[0] == 0 and co[1] == 0 and co[2] == 0 and co[3] == 0)


def u_edges_home(full):
    """4 canh lop U dung ca vi tri lan huong (PLL: pha canh da xong)."""
    ep, eo = full[0], full[1]
    return (ep[0] == 0 and ep[1] == 1 and ep[2] == 2 and ep[3] == 3
            and eo[0] == 0 and eo[1] == 0 and eo[2] == 0 and eo[3] == 0)

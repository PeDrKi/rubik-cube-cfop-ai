"""
solver/facelets.py
===================
Định nghĩa các "khe" (slot) facelet dùng để trích xuất trạng thái rút gọn
(abstracted state) từ state đầy đủ của cube_engine, phục vụ xây dựng
pattern database (PDB) và kiểm tra điều kiện đã giải cho Cross / F2L.

Quy ước: state là dict {face: np.ndarray(3,3)} giống cube_engine.make_solved().
Tâm mỗi mặt luôn bất biến dưới 18 nước outer-move (U D F B L R + ' + 2),
nên state[f][1,1] == f luôn đúng miễn scramble/solve chỉ dùng các nước này.

Mỗi "cubie" tham chiếu bằng danh sách toạ độ (face, row, col) của các facelet
thuộc cubie đó, theo đúng thứ tự cố định để so khớp giá trị đã-giải.
"""

# ── Cross edges (giả định cross ở mặt D) ───────────────────────────────────────
# Mỗi entry: tên khe -> ((face,row,col) trên D, (face,row,col) mặt bên)
CROSS_EDGES = {
    'DF': (('D', 0, 1), ('F', 2, 1)),
    'DB': (('D', 2, 1), ('B', 2, 1)),
    'DL': (('D', 1, 0), ('L', 2, 1)),
    'DR': (('D', 1, 2), ('R', 2, 1)),
}
CROSS_ORDER = ['DF', 'DB', 'DL', 'DR']   # thứ tự cố định khi tạo key

# ── F2L slots: mỗi slot có 1 corner (3 facelet) + 1 edge (2 facelet) ──────────
F2L_SLOTS = {
    'DFR': {
        'corner': (('D', 0, 2), ('F', 2, 2), ('R', 2, 0)),
        'edge':   (('F', 1, 2), ('R', 1, 0)),
    },
    'DFL': {
        'corner': (('D', 0, 0), ('F', 2, 0), ('L', 2, 2)),
        'edge':   (('F', 1, 0), ('L', 1, 2)),
    },
    'DBR': {
        'corner': (('D', 2, 2), ('B', 2, 0), ('R', 2, 2)),
        'edge':   (('R', 1, 2), ('B', 1, 0)),
    },
    'DBL': {
        'corner': (('D', 2, 0), ('B', 2, 2), ('L', 2, 0)),
        'edge':   (('L', 1, 0), ('B', 1, 2)),
    },
}
F2L_ORDER = ['DFR', 'DFL', 'DBR', 'DBL']   # thứ tự giải mặc định


def _read(state, coords):
    return tuple(str(state[f][r, c]) for f, r, c in coords)


def cross_key(state):
    """Tuple 8 ký tự đại diện toàn bộ trạng thái 4 cạnh cross (vị trí+hướng)."""
    out = []
    for name in CROSS_ORDER:
        d_coord, side_coord = CROSS_EDGES[name]
        out.extend(_read(state, (d_coord, side_coord)))
    return tuple(out)


def cross_solved(state):
    for name in CROSS_ORDER:
        (df, dr, dc), (sf, sr, sc) = CROSS_EDGES[name]
        if state[df][dr, dc] != 'D' or state[sf][sr, sc] != sf:
            return False
    return True


def corner_key(state, slot):
    return _read(state, F2L_SLOTS[slot]['corner'])


def edge_key(state, slot):
    return _read(state, F2L_SLOTS[slot]['edge'])


def pair_solved(state, slot):
    coords_c = F2L_SLOTS[slot]['corner']
    coords_e = F2L_SLOTS[slot]['edge']
    for f, r, c in coords_c:
        if state[f][r, c] != f:
            return False
    for f, r, c in coords_e:
        if state[f][r, c] != f:
            return False
    return True


def f2l_solved_slots(state):
    """Danh sách các slot F2L đã hoàn thành (theo F2L_ORDER)."""
    return [s for s in F2L_ORDER if pair_solved(state, s)]

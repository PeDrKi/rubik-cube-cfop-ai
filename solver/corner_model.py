"""
solver/corner_model.py
=======================
Tuong tu edge_model.py nhung cho 8 goc (corner). Huong (orientation) cua
goc nhan 3 gia tri (0,1,2) thay vi 2 nhu canh.

Quy uoc: toa do dau tien trong moi slot LUON la facelet huong ve U/D cua
goc do khi o dung vi tri goc (home). Huong hien tai = vi tri (0,1,2) cua
facelet mang gia tri 'U' hoac 'D' trong bo 3 facelet doc duoc tai slot.
"""

from cube_engine import make_solved, do_move, ALL_MOVES

CORNER_SLOTS = {
    'UFR': (('U', 2, 2), ('F', 0, 2), ('R', 0, 0)),
    'UFL': (('U', 2, 0), ('L', 0, 2), ('F', 0, 0)),
    'UBR': (('U', 0, 2), ('R', 0, 2), ('B', 0, 0)),
    'UBL': (('U', 0, 0), ('B', 0, 2), ('L', 0, 0)),
    'DFR': (('D', 0, 2), ('R', 2, 0), ('F', 2, 2)),
    'DFL': (('D', 0, 0), ('F', 2, 0), ('L', 2, 2)),
    'DBR': (('D', 2, 2), ('B', 2, 0), ('R', 2, 2)),
    'DBL': (('D', 2, 0), ('L', 2, 0), ('B', 2, 2)),
}
SLOT_NAMES = list(CORNER_SLOTS.keys())
SLOT_INDEX = {name: i for i, name in enumerate(SLOT_NAMES)}

_SOLVED_TRIPLE = {}
for name, coords in CORNER_SLOTS.items():
    _SOLVED_TRIPLE[name] = frozenset(f for f, r, c in coords)


def _read(state, slot_name):
    coords = CORNER_SLOTS[slot_name]
    return tuple(str(state[f][r, c]) for f, r, c in coords)


def _identify(state, slot_name):
    vals = _read(state, slot_name)
    pair = frozenset(vals)
    for orig, triple in _SOLVED_TRIPLE.items():
        if triple == pair:
            ori = next(i for i, v in enumerate(vals) if v in ('U', 'D'))
            return orig, ori
    raise ValueError(f'Khong nhan dien duoc goc tai {slot_name}: {vals}')


def build_move_tables():
    perm = {}
    ori_delta = {}
    for mv in ALL_MOVES:
        p = [None] * 8
        o = [0] * 8
        after = make_solved()
        do_move(after, mv)
        for slot_name in SLOT_NAMES:
            i = SLOT_INDEX[slot_name]
            for cand in SLOT_NAMES:
                orig, ori = _identify(after, cand)
                if orig == slot_name:
                    p[i] = SLOT_INDEX[cand]
                    o[i] = ori
                    break
            if p[i] is None:
                raise RuntimeError(f'Loi suy luan bang hoan vi goc move={mv} slot={slot_name}')
        perm[mv] = tuple(p)
        ori_delta[mv] = tuple(o)
    return perm, ori_delta


MOVE_PERM, MOVE_ORI_DELTA = build_move_tables()


def apply_move_to_corners(positions, orients, mv):
    perm = MOVE_PERM[mv]
    delta = MOVE_ORI_DELTA[mv]
    new_pos = [perm[p] for p in positions]
    new_ori = [(o + delta[p]) % 3 for p, o in zip(positions, orients)]
    return new_pos, new_ori


def corners_from_state(state, tracked_slot_names):
    loc_of = {}
    for cand in SLOT_NAMES:
        orig, ori = _identify(state, cand)
        loc_of[orig] = (cand, ori)
    positions = []
    orients = []
    for name in tracked_slot_names:
        cand, ori = loc_of[name]
        positions.append(SLOT_INDEX[cand])
        orients.append(ori)
    return positions, orients

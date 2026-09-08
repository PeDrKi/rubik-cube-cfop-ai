"""
solver/f2l_solver.py
=====================
Giai F2L (4 cap corner+edge) sau khi Cross da xong, tung cap mot, theo dung
thu tu CFOP. Dung A* graph-search tren khong gian cubie day du (khong con
van de facelet-hidden-state), voi heuristic tu Pattern Database don-quan
(single-piece PDB) cho tung corner/edge -- admissible vi la khoang cach toi
thieu de rieng quan do ve nha, bo qua tat ca quan khac.

A* (thay vi IDA*) vi co the memo-hoa (visited dict) tren tuple trang thai
day du -> tranh lap lai cong viec, phu hop khong gian tim kiem F2L (nho, vai
nghin - vai chuc nghin node cho moi cap trong thuc te).

Khong dung nuoc D trong pha nay (giu nguyen Cross da giai o lop D).
"""

import heapq
import random

from .full_state import from_facelets, apply_move, cross_ok, pair_ok, NO_D_MOVES, F2L_EDGE_OF
from . import edge_model as EM
from . import corner_model as CM
from .pdb_builder import build_pdb_edges, build_pdb_corners, build_pdb_pair
from .search_utils import check_cancel

_CANCEL_CHECK_EVERY = 512   # dong bo voi search_utils.py -- xem ghi chu do dac o do

F2L_ORDER = ['DFR', 'DFL', 'DBR', 'DBL']

_PAIR_PDB_CACHE = {}


def _pair_pdb(slot):
    if slot not in _PAIR_PDB_CACHE:
        edge_name = F2L_EDGE_OF[slot]
        _PAIR_PDB_CACHE[slot] = build_pdb_pair(f'pair_{slot}', slot, edge_name)
    return _PAIR_PDB_CACHE[slot]


CROSS_EDGE_NAMES = ('DF', 'DB', 'DL', 'DR')


def _mismatch_penalty(full, done_slots):
    """Dem so quan (trong Cross + cac slot da xong) hien khong o dung nha.
    KHONG phai heuristic admissible (chi dung de dan huong tim kiem uu tien
    tranh pha vo phan da giai -- chap nhan mat tinh toi uu tuyet doi de doi
    lay toc do, phu hop MVP)."""
    ep, eo, cp, co = full
    # Cross = canh 4-7 co dinh (xem edge_model.SLOT_NAMES) -> so sanh thang
    # chi so (nhanh hon vong lap tra dict theo ten, xem full_state.py).
    bad = 0 if (ep[4] == 4 and ep[5] == 5 and ep[6] == 6 and ep[7] == 7
                and eo[4] == 0 and eo[5] == 0 and eo[6] == 0 and eo[7] == 0) else 1
    for s in done_slots:
        ci = CM.SLOT_INDEX[s]
        if cp[ci] != ci or co[ci] != 0:
            bad += 1
        ei = EM.SLOT_INDEX[F2L_EDGE_OF[s]]
        if ep[ei] != ei or eo[ei] != 0:
            bad += 1
    return bad


def _heuristic(full, slot, done_slots, path=(), trigger_bonus_fn=None, lam=0.0):
    ep, eo, cp, co = full
    ci = CM.SLOT_INDEX[slot]
    ei = EM.SLOT_INDEX[F2L_EDGE_OF[slot]]
    key = ((ep[ei],), (eo[ei],), (cp[ci],), (co[ci],))
    base = _pair_pdb(slot).get(key, 8)
    # trong so lon cho vi pham phan da giai -- uu tien manh tranh cac
    # nhanh lam hong Cross / cac cap truoc do (search khong con toi uu
    # tuyet doi nhung nhanh & thuc te hon nhieu cho MVP)
    penalty = 2 * _mismatch_penalty(full, done_slots)
    h = base + penalty
    # ── [NGHIEN CUU] Trigger-biased search (xem research/trigger_biased_search.py) ──
    # Neu duoc cung cap, tru bot mot luong (lam * so nuoc thuoc trigger da
    # biet trong `path`) de A* uu tien cac nhanh "giong nguoi" hon, DOI LAY
    # co the mat mot phan toi uu ve so nuoc. Mac dinh trigger_bonus_fn=None,
    # lam=0.0 -> h khong doi, HANH VI GOC KHONG ANH HUONG.
    if trigger_bonus_fn is not None and lam:
        h = h - lam * trigger_bonus_fn(path)
    return h


def _solve_pair(full_start, slot, done_slots, max_nodes=120_000, max_depth=13, moves=NO_D_MOVES,
                 trigger_bonus_fn=None, lam=0.0):
    """A*/greedy-best-first tim chuoi nuoc (khong dung D) dua slot ve dung
    vi tri, uu tien manh giu nguyen Cross va cac slot trong done_slots.
    Tra ve list moves hoac None neu khong tim thay trong ngan sach cho phep.
    `moves`: thu tu duyet nuoc di -- co the xao tron cho lan 'thu lai'.

    `trigger_bonus_fn`, `lam`: xem _heuristic() -- [NGHIEN CUU], mac dinh
    khong anh huong hanh vi goc."""

    def goal(full):
        if not cross_ok(full):
            return False
        for s in done_slots:
            if not pair_ok(full, s):
                return False
        return pair_ok(full, slot)

    if goal(full_start):
        return []

    h0 = _heuristic(full_start, slot, done_slots, (), trigger_bonus_fn, lam)
    counter = 0
    # (f, g, counter, state, path, last_face)
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
        if nodes % _CANCEL_CHECK_EVERY == 0:
            check_cancel()   # xem solver/search_utils.py -- huy job AI giua chung
        for mv in moves:
            face = mv[0]
            if face == last_face:
                continue   # tranh lap lai cung 1 mat lien tiep (khong bao gio toi uu)
            nxt = apply_move(cur, mv)
            ng = g + 1
            if ng < best_g.get(nxt, 1 << 30):
                best_g[nxt] = ng
                npath = path + (mv,)
                if goal(nxt):
                    return list(npath)
                counter += 1
                h = _heuristic(nxt, slot, done_slots, npath, trigger_bonus_fn, lam)
                heapq.heappush(heap, (ng + h, ng, counter, nxt, npath, face))
    return None


def _move_order(shuffled):
    """Xao tron ngau nhien NO_D_MOVES cho lan 'thu lai' (retry) -- cung
    ngan sach node/RAM nhung kham pha nhanh khac lan truoc, vi tim kiem
    von tat dinh nen thu lai voi thu tu cu se that bai y het lan truoc."""
    if not shuffled:
        return NO_D_MOVES
    mv = list(NO_D_MOVES)
    random.shuffle(mv)
    return mv


def _solve_pair_ladder(full, slot, done, depths=(8, 10, 12, 14), nodes_per_depth=120_000,
                        shuffled=False, trigger_bonus_fn=None, lam=0.0):
    for depth in depths:
        mvs = _solve_pair(full, slot, done, max_nodes=nodes_per_depth, max_depth=depth,
                           moves=_move_order(shuffled), trigger_bonus_fn=trigger_bonus_fn, lam=lam)
        if mvs is not None:
            return mvs
    return None


def solve_f2l(state, slots=None, depths=(8, 10, 12, 14), nodes_per_depth=120_000, retry=False,
              trigger_bonus_fn=None, lam=0.0):
    """
    Giai F2L tu trang thai facelet hien tai (Cross phai da xong truoc do).
    Tra ve dict: {'moves': [...tat ca nuoc noi tiep...],
                  'per_slot': {slot: [moves] hoac None neu that bai},
                  'solved_slots': [...]}
    Khong thay doi state truyen vao. retry=True: xem _move_order().

    trigger_bonus_fn, lam: [NGHIEN CUU] xem research/trigger_biased_search.py.
    Mac dinh None/0.0 -> HANH VI Y HET BAN GOC (khong anh huong test cu).
    """
    slots = slots or F2L_ORDER
    full = from_facelets(state)
    all_moves = []
    per_slot = {}
    # quan trong: neu chi giai 1 slot cu the (VD tu hint()), cac slot KHAC
    # da xong truoc do (khong nam trong `slots`) van phai duoc coi la "done"
    # de search khong vo tinh pha vo chung.
    done = [s for s in F2L_ORDER if s not in slots and pair_ok(full, s)]
    for slot in slots:
        if pair_ok(full, slot):
            per_slot[slot] = []
            done.append(slot)
            continue
        mvs = _solve_pair_ladder(full, slot, done, depths=depths, nodes_per_depth=nodes_per_depth,
                                  shuffled=retry, trigger_bonus_fn=trigger_bonus_fn, lam=lam)
        per_slot[slot] = mvs
        if mvs is None:
            break   # giu MVP: dung lai o cap dau tien khong giai duoc
        for mv in mvs:
            full = apply_move(full, mv)
        all_moves.extend(mvs)
        done.append(slot)
    return {'moves': all_moves, 'per_slot': per_slot, 'solved_slots': done}

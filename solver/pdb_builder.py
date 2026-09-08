"""
solver/pdb_builder.py
======================
Xây dựng Pattern Database (PDB) tổng quát bằng BFS ngược từ trạng thái đã giải.

Ý tưởng (kỹ thuật PDB kinh điển trong AI search — Culberson & Schaeffer 1998,
dùng lại trong solver Rubik tối ưu của Korf):
  - Ta không quan tâm toàn bộ 4.3*10^19 trạng thái của cube, chỉ quan tâm
    một tập con nhỏ các "quân" (cross edges, hoặc 1 corner+edge của F2L).
  - Hàm extract_fn(state) -> abstracted_key rút gọn state đầy đủ thành trạng
    thái của riêng tập quân đó (bỏ qua phần còn lại).
  - BFS từ trạng thái đã giải, nhưng chỉ lưu/mở rộng lần đầu tiên mỗi
    abstracted_key xuất hiện. Vì phép quay tác động lên các quân một cách
    tất định (không phụ thuộc quân khác), đây chính là BFS đúng trên
    "đồ thị thương" (quotient graph) — cho khoảng cách chính xác, dùng làm
    heuristic admissible cho IDA*/A*.
  - Kết quả: dict abstracted_key -> số bước tối thiểu để giải riêng tập quân đó.

PDB được cache ra đĩa (pickle) để không phải build lại mỗi lần chạy app.
"""

import os
import pickle
from collections import deque

from cube_engine import make_solved, do_move, ALL_MOVES

CACHE_DIR = os.path.join(os.path.dirname(__file__), 'cache')


def _cache_path(name):
    os.makedirs(CACHE_DIR, exist_ok=True)
    return os.path.join(CACHE_DIR, f'{name}.pkl')


def build_pdb(name, extract_fn, moves=None, max_states=None, verbose=False):
    """
    Trả về dict abstracted_key -> distance.
    Kết quả được cache theo `name` trong solver/cache/<name>.pkl.
    """
    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    moves = moves or ALL_MOVES
    start = make_solved()
    start_key = extract_fn(start)

    dist = {start_key: 0}
    queue = deque([start])
    processed = 0

    while queue:
        st = queue.popleft()
        cur_key = extract_fn(st)
        d = dist[cur_key]
        processed += 1
        if verbose and processed % 20000 == 0:
            print(f'[pdb:{name}] processed={processed} keys={len(dist)}')
        if max_states and len(dist) >= max_states:
            break
        for mv in moves:
            nxt = {f: st[f].copy() for f in st}
            do_move(nxt, mv)
            k = extract_fn(nxt)
            if k not in dist:
                dist[k] = d + 1
                queue.append(nxt)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_edges_group_anyperm(name, tracked_slots, verbose=False):
    """
    PDB cho MOT NHOM canh cung luc (VD 4 canh lop U), muc tieu la "TAT CA
    huong = 0, BAT KY hoan vi nao giua chung" (khong quan tam quan nao o
    khe nao). Manh hon nhieu so voi PDB tung quan rieng le vi nam duoc
    tuong tac giua cac quan cung nhom (1 nuoc anh huong ca 4).

    Xay bang multi-source BFS: hat giong = TAT CA hoan vi cua tracked_slots
    trong 12 khe, voi vector huong = (0,0,...,0).
    """
    from itertools import permutations
    from .edge_model import apply_move_to_edges, SLOT_INDEX, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    n = len(tracked_slots)
    all_slot_idx = list(range(12))
    zero_ori = tuple([0] * n)

    dist = {}
    queue = deque()
    for perm in permutations(all_slot_idx, n):
        g = (perm, zero_ori)
        if g not in dist:
            dist[g] = 0
            queue.append(g)
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        for mv in moves:
            new_pos, new_ori = apply_move_to_edges(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_corners_group_anyperm(name, tracked_slots, verbose=False):
    """Tuong tu build_pdb_edges_group_anyperm nhung cho goc (mod 3)."""
    from itertools import permutations
    from .corner_model import apply_move_to_corners, SLOT_INDEX, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    n = len(tracked_slots)
    all_slot_idx = list(range(8))
    zero_ori = tuple([0] * n)

    dist = {}
    queue = deque()
    for perm in permutations(all_slot_idx, n):
        g = (perm, zero_ori)
        if g not in dist:
            dist[g] = 0
            queue.append(g)
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        for mv in moves:
            new_pos, new_ori = apply_move_to_corners(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_edges_multigoal(name, goal_states, verbose=False):
    """
    Nhu build_pdb_edges nhung BFS xuat phat tu NHIEU trang thai dich cung luc
    (multi-source BFS) -- dung cho OLL: muc tieu la "canh nay huong dung,
    O BAT KY khe U nao" (khong quan tam vi tri cu the, PLL se sap xep sau).
    goal_states: list cac (positions_tuple, orients_tuple) - moi tuple do dai 1
    (theo doi 1 quan duy nhat).
    """
    from .edge_model import apply_move_to_edges, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    dist = {}
    queue = deque()
    for g in goal_states:
        if g not in dist:
            dist[g] = 0
            queue.append(g)
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        for mv in moves:
            new_pos, new_ori = apply_move_to_edges(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_corners_multigoal(name, goal_states, verbose=False):
    """Tuong tu build_pdb_edges_multigoal nhung cho goc."""
    from .corner_model import apply_move_to_corners, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    dist = {}
    queue = deque()
    for g in goal_states:
        if g not in dist:
            dist[g] = 0
            queue.append(g)
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        for mv in moves:
            new_pos, new_ori = apply_move_to_corners(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_pair(name, corner_slot, edge_slot, verbose=False):
    """PDB cho DONG THOI 1 goc + 1 canh (khong gian nho: 24*24=576 trang thai),
    heuristic manh hon nhieu so voi max(rieng le) vi tinh ca 2 quan cung luc."""
    from .edge_model import apply_move_to_edges, SLOT_INDEX as ESI, MOVE_PERM as EMP
    from .corner_model import apply_move_to_corners, SLOT_INDEX as CSI

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    start_key = ((ESI[edge_slot],), (0,), (CSI[corner_slot],), (0,))
    dist = {start_key: 0}
    queue = deque([start_key])
    moves = list(EMP.keys())

    while queue:
        key = queue.popleft()
        ep, eo, cp, co = key
        d = dist[key]
        for mv in moves:
            nep, neo = apply_move_to_edges(list(ep), list(eo), mv)
            ncp, nco = apply_move_to_corners(list(cp), list(co), mv)
            k = (tuple(nep), tuple(neo), tuple(ncp), tuple(nco))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_corners(name, tracked_slots, verbose=False):
    """Nhu build_pdb_edges nhung cho goc (huong mod 3)."""
    from .corner_model import apply_move_to_corners, SLOT_INDEX, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    n = len(tracked_slots)
    start_pos = tuple(SLOT_INDEX[s] for s in tracked_slots)
    start_ori = tuple([0] * n)
    start_key = (start_pos, start_ori)

    dist = {start_key: 0}
    queue = deque([start_key])
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        for mv in moves:
            new_pos, new_ori = apply_move_to_corners(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist


def build_pdb_edges(name, tracked_slots, verbose=False):
    """
    Xay PDB cho mot tap canh (edge) theo doi, thuan combinatorial tren
    (vi tri trong 12 khe, huong) - dung edge_model thay vi mo phong facelet.
    Nhanh hon nhieu lan so voi build_pdb vi khong can numpy simulation.
    Tra ve dict: key=(tuple(positions), tuple(orients)) -> distance.
    """
    from .edge_model import apply_move_to_edges, SLOT_INDEX, MOVE_PERM

    path = _cache_path(name)
    if os.path.exists(path):
        with open(path, 'rb') as fh:
            return pickle.load(fh)

    n = len(tracked_slots)
    start_pos = tuple(SLOT_INDEX[s] for s in tracked_slots)
    start_ori = tuple([0] * n)
    start_key = (start_pos, start_ori)

    dist = {start_key: 0}
    queue = deque([start_key])
    processed = 0
    moves = list(MOVE_PERM.keys())

    while queue:
        key = queue.popleft()
        pos, ori = key
        d = dist[key]
        processed += 1
        if verbose and processed % 50000 == 0:
            print(f'[pdb:{name}] processed={processed} keys={len(dist)}')
        for mv in moves:
            new_pos, new_ori = apply_move_to_edges(list(pos), list(ori), mv)
            k = (tuple(new_pos), tuple(new_ori))
            if k not in dist:
                dist[k] = d + 1
                queue.append(k)

    with open(path, 'wb') as fh:
        pickle.dump(dist, fh, protocol=pickle.HIGHEST_PROTOCOL)
    if verbose:
        print(f'[pdb:{name}] DONE. total keys={len(dist)}')
    return dist

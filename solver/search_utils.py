"""
solver/search_utils.py
========================
IDA* (Iterative Deepening A*) dung chung cho cac pha kho (OLL pha goc, PLL
pha canh) -- thay the A* (heapq + dict best_g) da dung truoc do.

TAI SAO CAN IDA* THAY VI A*:
  A* voi dict `best_g` luu MOI trang thai da tham -> bo nho tang TUYEN TINH
  theo so node duyet (O(n)). Tren may chi ~4GB RAM (da kiem tra thuc te
  qua /proc/meminfo), ~300k node A* da chiem ~3GB -- vuot qua se bi kernel
  OOM-kill CA TIEN TRINH (crash toan bo app, khong chi that bai giai duoc).

  IDA* la DFS lap sau tang dan, KHONG luu visited-state -> bo nho chi
  O(do sau tim kiem) (vai chuc phan tu tren call stack), khong phu thuoc
  so node da duyet. Danh doi: co the duyet lai mot nhanh nhieu lan (khong
  memo hoa) nen can nhieu "node-visit" hon A* de dat cung do sau -- nhung
  vi khong con bi gioi han boi RAM, co the cho phep ngan sach node-visit
  LON HON NHIEU (hang trieu thay vi hang tram nghin), thuc te tim ra loi
  giai cho nhieu case ma A*-voi-RAM-han-che truoc day khong tim duoc.

  Day chinh la ky thuat IDA* kinh dien dung trong solver Rubik toi uu cua
  Korf (1997) -- ly do goc IDA* duoc chon thay A* cho bai toan Rubik chinh
  la vi khong gian trang thai qua lon de luu toan bo visited-set.
"""

from .full_state import apply_move

# Sentinel tra ve tu dfs() khi tim thay loi giai hoac het ngan sach.
_FOUND = -1
_BUDGET_EXCEEDED = -2


def move_order(moves, shuffled):
    """Thu tu duyet nuoc di cho 1 lan goi search. Neu shuffled=True, tra ve
    1 hoan vi NGAU NHIEN cua `moves` (dung cho lan 'thu lai' -- cung ngan
    sach node/RAM nhung kham pha nhanh khac, vi tim kiem von tat dinh nen
    thu lai voi thu tu cu se that bai/ra ket qua y het lan truoc)."""
    if not shuffled:
        return moves
    import random
    mv = list(moves)
    random.shuffle(mv)
    return mv


def ida_star(full_start, goal_fn, heuristic_fn, max_threshold, moves, node_budget):
    """
    Tim chuoi nuoc di ngan nhat (theo threshold tang dan) tu full_start den
    trang thai thoa goal_fn, dung heuristic_fn lam chan duoi admissible.

    - max_threshold: nguong f=g+h toi da cho phep (chan tren do sau tim kiem
      thuc te, vi h>=0 nen g<=threshold).
    - moves: danh sach nuoc di duyet qua moi node (co the xao tron cho retry).
    - node_budget: so node toi da duoc "tham" (KHONG anh huong RAM, chi
      anh huong thoi gian cho phep).

    Tra ve list nuoc di neu tim thay, None neu het ngan sach hoac vuot
    max_threshold ma khong tim thay.
    """
    threshold = heuristic_fn(full_start)
    if threshold > max_threshold:
        return None

    path = []
    nodes = [0]

    def dfs(node, g, last_face):
        h = heuristic_fn(node)
        f = g + h
        if f > threshold:
            return f
        if goal_fn(node):
            return _FOUND
        nodes[0] += 1
        if nodes[0] > node_budget:
            return _BUDGET_EXCEEDED

        min_next = None
        for mv in moves:
            face = mv[0]
            if face == last_face:
                continue
            nxt = apply_move(node, mv)
            path.append(mv)
            t = dfs(nxt, g + 1, face)
            if t == _FOUND or t == _BUDGET_EXCEEDED:
                return t
            path.pop()
            if min_next is None or t < min_next:
                min_next = t
        return min_next if min_next is not None else float('inf')

    while threshold <= max_threshold:
        result = dfs(full_start, 0, None)
        if result == _FOUND:
            return list(path)
        if result == _BUDGET_EXCEEDED:
            return None
        if result == float('inf'):
            return None
        threshold = result
    return None

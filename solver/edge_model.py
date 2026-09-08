"""
solver/edge_model.py
=====================
Mô hình hoá 12 cạnh (edge) của Rubik ở mức "cubie" (vị trí + hướng), thay vì
đọc trực tiếp màu facelet. Đây là cách làm chuẩn để xây Pattern Database
(Korf 1997): với mỗi nước đi, có một PHÉP HOÁN VỊ CỐ ĐỊNH trên 12 vị trí
cạnh (kèm cờ lật hướng) — hoàn toàn không phụ thuộc trạng thái cube hiện tại.

Lý do cần thiết: khi 1 mặt bên (VD L) tự xoay, cạnh đang nằm ở vị trí không-
liên-quan tới cross (VD FL) sẽ được đưa vào vị trí cross (VD DL). Nếu chỉ đọc
2 facelet của mỗi khe DF/DB/DL/DR (bỏ qua thông tin "ẩn" ở FL/FR/BL/BR/UF...),
trạng thái rút gọn sẽ KHÔNG đủ để xác định trạng thái kế tiếp một cách nhất
quán (đã kiểm chứng bằng thực nghiệm). Do đó ta phải suy ra phép hoán vị đủ
trên toàn bộ 12 khe cạnh, rồi mới rút gọn (chỉ theo dõi 4 khe mong muốn) —
lúc này việc rút gọn mới hợp lệ, vì phép hoán vị của mỗi nước đi độc lập với
nội dung các khe khác.

Ta suy ra bảng hoán vị bằng thực nghiệm: áp nước đi lên cube đã giải, rồi so
khớp facelet để biết cạnh nào đã chạy tới đâu.
"""

from cube_engine import make_solved, do_move, ALL_MOVES

# 12 khe cạnh, mỗi khe = 2 toạ độ facelet (face,row,col)
EDGE_SLOTS = {
    'UF': (('U', 2, 1), ('F', 0, 1)),
    'UB': (('U', 0, 1), ('B', 0, 1)),
    'UL': (('U', 1, 0), ('L', 0, 1)),
    'UR': (('U', 1, 2), ('R', 0, 1)),
    'DF': (('D', 0, 1), ('F', 2, 1)),
    'DB': (('D', 2, 1), ('B', 2, 1)),
    'DL': (('D', 1, 0), ('L', 2, 1)),
    'DR': (('D', 1, 2), ('R', 2, 1)),
    'FR': (('F', 1, 2), ('R', 1, 0)),
    'FL': (('F', 1, 0), ('L', 1, 2)),
    'BR': (('R', 1, 2), ('B', 1, 0)),
    'BL': (('L', 1, 0), ('B', 1, 2)),
}
SLOT_NAMES = list(EDGE_SLOTS.keys())
SLOT_INDEX = {name: i for i, name in enumerate(SLOT_NAMES)}

# Cặp màu "gốc" (đã giải) của mỗi khe -> dùng để nhận diện cạnh nào đang ở đâu
_SOLVED = make_solved()
_SOLVED_PAIR = {}
for name, (c1, c2) in EDGE_SLOTS.items():
    v1 = str(_SOLVED[c1[0]][c1[1], c1[2]])
    v2 = str(_SOLVED[c2[0]][c2[1], c2[2]])
    _SOLVED_PAIR[name] = frozenset((v1, v2))


def _identify(state, slot_name):
    """Trả về (edge_gốc, flipped) đang chiếm slot_name trong state hiện tại."""
    c1, c2 = EDGE_SLOTS[slot_name]
    v1 = str(state[c1[0]][c1[1], c1[2]])
    v2 = str(state[c2[0]][c2[1], c2[2]])
    pair = frozenset((v1, v2))
    for orig, opair in _SOLVED_PAIR.items():
        if opair == pair:
            # flipped nếu v1 (màu tại vị trí facelet thứ nhất của slot hiện tại)
            # khác với màu mà edge gốc đó có tại facelet thứ nhất lúc solved
            orig_c1 = EDGE_SLOTS[orig][0]
            orig_v1 = str(_SOLVED[orig_c1[0]][orig_c1[1], orig_c1[2]])
            flipped = (v1 != orig_v1)
            return orig, flipped
    raise ValueError(f'Không nhận diện được cạnh tại {slot_name}: {pair}')


def build_move_tables():
    """
    Với mỗi move trong ALL_MOVES, trả về dict:
      perm[move][slot_index_before] = slot_index_after
      flip[move][slot_index_before] = 0/1 (có bị lật hướng thêm không)
    Suy ra bằng thực nghiệm: đặt 1 cạnh "đánh dấu" tại từng slot của cube đã
    giải, áp move, xem cạnh đó chạy tới slot nào & có lật hướng hay không.
    """
    perm = {}
    flip = {}
    for mv in ALL_MOVES:
        p = [None] * 12
        f = [0] * 12
        # Với từng slot gốc, edge gốc mang tên = tên slot đó (vì bắt đầu từ solved)
        after = make_solved()
        do_move(after, mv)
        for slot_name in SLOT_NAMES:
            i = SLOT_INDEX[slot_name]
            # tìm slot nào trong `after` hiện đang chứa edge gốc `slot_name`
            for cand in SLOT_NAMES:
                orig, flipped = _identify(after, cand)
                if orig == slot_name:
                    p[i] = SLOT_INDEX[cand]
                    f[i] = 1 if flipped else 0
                    break
            if p[i] is None:
                raise RuntimeError(f'Lỗi suy luận bảng hoán vị move={mv} slot={slot_name}')
        perm[mv] = tuple(p)
        flip[mv] = tuple(f)
    return perm, flip


MOVE_PERM, MOVE_FLIP = build_move_tables()


def apply_move_to_edges(positions, orients, mv):
    """
    positions: list các slot_index hiện tại của N cạnh đang theo dõi (thứ tự cố định)
    orients:   list 0/1 hướng tương ứng
    Trả về (positions_mới, orients_mới) sau khi áp mv.
    """
    perm = MOVE_PERM[mv]
    flip = MOVE_FLIP[mv]
    new_pos = [perm[p] for p in positions]
    new_ori = [(o + flip[p]) % 2 for p, o in zip(positions, orients)]
    return new_pos, new_ori


def edges_from_state(state, tracked_slot_names):
    """Đọc trực tiếp từ facelet state hiện tại: vị trí + hướng của các cạnh
    NGUYÊN GỐC (đã giải) có tên trong tracked_slot_names."""
    # với mỗi slot hiện tại, nhận diện cạnh gốc đang ở đó
    loc_of = {}   # edge gốc -> (slot hiện tại, flipped)
    for cand in SLOT_NAMES:
        orig, flipped = _identify(state, cand)
        loc_of[orig] = (cand, flipped)
    positions = []
    orients = []
    for name in tracked_slot_names:
        cand, flipped = loc_of[name]
        positions.append(SLOT_INDEX[cand])
        orients.append(1 if flipped else 0)
    return positions, orients

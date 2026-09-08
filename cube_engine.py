"""
cube_engine.py
==============
Tất cả logic liên quan đến trạng thái Rubik's Cube:
  - Khởi tạo / reset trạng thái
  - Thực hiện nước đi (do_move)
  - Scramble ngẫu nhiên
  - Kiểm tra đã solved
  - Parser ký hiệu Singmaster (đầy đủ WCA notation)

Notation được hỗ trợ:
  Outer (1 tầng) : U D F B L R  và '  (ví dụ R U' F2)
  Wide (2 tầng)  : u d f b l r  và '  (ví dụ u = U + E')
  Slice (tầng giữa): M E S        và '
  Rotation (cả khối): x y z       và '
  Số 2           : lặp 2 lần      (ví dụ R2 = R R)
  Nhóm           : (R U R' U')3

Không phụ thuộc pygame.  Import numpy và stdlib chỉ.
"""

import numpy as np
import random
import re

from constants import FACES

# ── Khởi tạo ──────────────────────────────────────────────────────────────────

def make_solved():
    return {f: np.full((3, 3), f, dtype='U1') for f in FACES}

# ── Phép quay ma trận mặt ─────────────────────────────────────────────────────

def rot_cw(m):  return np.rot90(m, k=-1)
def rot_ccw(m): return np.rot90(m, k=1)
def rot_180(m): return np.rot90(m, k=2)

# ── Thực hiện một nước đi ──────────────────────────────────────────────────────

def do_move(st, mv):
    """
    Áp dụng nước đi mv lên trạng thái st (in-place).
    Hỗ trợ đầy đủ: U D F B L R, M E S, x y z, u d f b l r (wide)
    và các biến thể ', 2, 2'.
    """
    # Xử lý double và prime trước khi dispatch
    if mv.endswith("2'") or mv.endswith("'2"):
        # R2' = R2 (vô nghĩa, coi như 2)
        _do_single(st, mv[:-2])
        _do_single(st, mv[:-2])
    elif mv.endswith('2'):
        _do_single(st, mv[:-1])
        _do_single(st, mv[:-1])
    else:
        _do_single(st, mv)

def _do_single(st, mv):
    """Thực hiện đúng 1 move (không có suffix 2)."""
    prime = mv.endswith("'")
    base  = mv[:-1] if prime else mv

    # Dispatch theo base move
    if   base == 'U': _U(st, prime)
    elif base == 'D': _D(st, prime)
    elif base == 'F': _F(st, prime)
    elif base == 'B': _B(st, prime)
    elif base == 'L': _L(st, prime)
    elif base == 'R': _R(st, prime)
    elif base == 'M': _M(st, prime)
    elif base == 'E': _E(st, prime)
    elif base == 'S': _S(st, prime)
    elif base == 'x': _x(st, prime)
    elif base == 'y': _y(st, prime)
    elif base == 'z': _z(st, prime)
    # wide moves (lowercase → uppercase + slice)
    elif base == 'u': _U(st, prime); _E(st, not prime)  # u = U + E'
    elif base == 'd': _D(st, prime); _E(st, prime)       # d = D + E
    elif base == 'f': _F(st, prime); _S(st, prime)       # f = F + S
    elif base == 'b': _B(st, prime); _S(st, not prime)   # b = B + S'
    elif base == 'l': _L(st, prime); _M(st, prime)       # l = L + M
    elif base == 'r': _R(st, prime); _M(st, not prime)   # r = R + M'

# ── Outer layer moves ─────────────────────────────────────────────────────────

def _snap(st):
    """Snapshot mỗi face (để read cũ trong khi ghi mới)."""
    return {f: st[f].copy() for f in FACES}

def _U(st, prime):
    s = _snap(st)
    st['U'] = rot_ccw(s['U']) if prime else rot_cw(s['U'])
    if not prime:
        st['F'][0,:] = s['R'][0,:]
        st['R'][0,:] = s['B'][0,:]
        st['B'][0,:] = s['L'][0,:]
        st['L'][0,:] = s['F'][0,:].copy()   # s['F'] original
    else:
        st['F'][0,:] = s['L'][0,:]
        st['L'][0,:] = s['B'][0,:]
        st['B'][0,:] = s['R'][0,:]
        st['R'][0,:] = s['F'][0,:].copy()

def _D(st, prime):
    s = _snap(st)
    st['D'] = rot_ccw(s['D']) if prime else rot_cw(s['D'])
    if not prime:
        st['F'][2,:] = s['L'][2,:]
        st['L'][2,:] = s['B'][2,:]
        st['B'][2,:] = s['R'][2,:]
        st['R'][2,:] = s['F'][2,:].copy()
    else:
        st['F'][2,:] = s['R'][2,:]
        st['R'][2,:] = s['B'][2,:]
        st['B'][2,:] = s['L'][2,:]
        st['L'][2,:] = s['F'][2,:].copy()

def _F(st, prime):
    s = _snap(st)
    st['F'] = rot_ccw(s['F']) if prime else rot_cw(s['F'])
    if not prime:
        st['U'][2,:] = s['L'][:,2][::-1]
        st['L'][:,2] = s['D'][0,:]
        st['D'][0,:] = s['R'][:,0][::-1]
        st['R'][:,0] = s['U'][2,:]
    else:
        st['U'][2,:] = s['R'][:,0]
        st['R'][:,0] = s['D'][0,:][::-1]
        st['D'][0,:] = s['L'][:,2]
        st['L'][:,2] = s['U'][2,:][::-1]

def _B(st, prime):
    s = _snap(st)
    st['B'] = rot_ccw(s['B']) if prime else rot_cw(s['B'])
    if not prime:
        st['U'][0,:] = s['R'][:,2]
        st['R'][:,2] = s['D'][2,:][::-1]
        st['D'][2,:] = s['L'][:,0]
        st['L'][:,0] = s['U'][0,:][::-1]
    else:
        st['U'][0,:] = s['L'][:,0][::-1]
        st['L'][:,0] = s['D'][2,:]
        st['D'][2,:] = s['R'][:,2][::-1]
        st['R'][:,2] = s['U'][0,:]

def _L(st, prime):
    s = _snap(st)
    st['L'] = rot_ccw(s['L']) if prime else rot_cw(s['L'])
    if not prime:
        st['F'][:,0] = s['U'][:,0]
        st['U'][:,0] = s['B'][:,2][::-1]
        st['B'][:,2] = s['D'][:,0][::-1]
        st['D'][:,0] = s['F'][:,0].copy()
    else:
        st['F'][:,0] = s['D'][:,0]
        st['D'][:,0] = s['B'][:,2][::-1]
        st['B'][:,2] = s['U'][:,0][::-1]
        st['U'][:,0] = s['F'][:,0].copy()

def _R(st, prime):
    s = _snap(st)
    st['R'] = rot_ccw(s['R']) if prime else rot_cw(s['R'])
    if not prime:
        st['F'][:,2] = s['D'][:,2]
        st['D'][:,2] = s['B'][:,0][::-1]
        st['B'][:,0] = s['U'][:,2][::-1]
        st['U'][:,2] = s['F'][:,2].copy()
    else:
        st['F'][:,2] = s['U'][:,2]
        st['U'][:,2] = s['B'][:,0][::-1]
        st['B'][:,0] = s['D'][:,2][::-1]
        st['D'][:,2] = s['F'][:,2].copy()

# ── Slice moves ───────────────────────────────────────────────────────────────

def _M(st, prime):
    """M: tầng giữa song song với L/R, hướng của L (CW nhìn từ trái)."""
    s = _snap(st)
    if not prime:  # M CW (same direction as L)
        st['F'][:,1] = s['U'][:,1]
        st['U'][:,1] = s['B'][:,1][::-1]
        st['B'][:,1] = s['D'][:,1][::-1]
        st['D'][:,1] = s['F'][:,1].copy()
    else:          # M' (same direction as L')
        st['F'][:,1] = s['D'][:,1]
        st['D'][:,1] = s['B'][:,1][::-1]
        st['B'][:,1] = s['U'][:,1][::-1]
        st['U'][:,1] = s['F'][:,1].copy()

def _E(st, prime):
    """E: tầng giữa song song với U/D, hướng của D."""
    s = _snap(st)
    if not prime:  # E CW (same direction as D)
        st['F'][1,:] = s['L'][1,:]
        st['L'][1,:] = s['B'][1,:]
        st['B'][1,:] = s['R'][1,:]
        st['R'][1,:] = s['F'][1,:].copy()
    else:          # E' (same direction as U)
        st['F'][1,:] = s['R'][1,:]
        st['R'][1,:] = s['B'][1,:]
        st['B'][1,:] = s['L'][1,:]
        st['L'][1,:] = s['F'][1,:].copy()

def _S(st, prime):
    """S: tầng giữa song song với F/B, hướng của F."""
    s = _snap(st)
    if not prime:  # S CW (same direction as F)
        st['U'][1,:] = s['L'][:,1][::-1]
        st['L'][:,1] = s['D'][1,:]
        st['D'][1,:] = s['R'][:,1][::-1]
        st['R'][:,1] = s['U'][1,:]
    else:          # S'
        st['U'][1,:] = s['R'][:,1]
        st['R'][:,1] = s['D'][1,:][::-1]
        st['D'][1,:] = s['L'][:,1]
        st['L'][:,1] = s['U'][1,:][::-1]

# ── Whole-cube rotations ──────────────────────────────────────────────────────

def _x(st, prime):
    """x: toàn bộ khối xoay như R (theo trục R/L)."""
    _R(st, prime)
    _M(st, not prime)  # middle follows R direction (opposite of M)
    _L(st, not prime)  # L goes opposite to R

def _y(st, prime):
    """y: toàn bộ khối xoay như U (theo trục U/D)."""
    _U(st, prime)
    _E(st, not prime)  # E goes opposite to U
    _D(st, not prime)  # D goes opposite to U

def _z(st, prime):
    """z: toàn bộ khối xoay như F (theo trục F/B)."""
    _F(st, prime)
    _S(st, prime)       # S follows F direction
    _B(st, not prime)   # B goes opposite to F

# ── Danh sách moves dùng để scramble (18 moves, chuẩn WCA) ────────────────────
ALL_MOVES = [
    'U', "U'", 'U2',
    'D', "D'", 'D2',
    'F', "F'", 'F2',
    'B', "B'", 'B2',
    'L', "L'", 'L2',
    'R', "R'", 'R2',
]

# ── Scramble & solved check ───────────────────────────────────────────────────

def scramble_cube(st, n=20):
    """
    Scramble n nước ngẫu nhiên, tránh 2 nước cùng mặt liên tiếp
    (vì R rồi R' = lãng phí 1 nước).
    """
    prev_face = None
    for _ in range(n):
        # Lọc bỏ các move cùng mặt với nước trước
        candidates = [m for m in ALL_MOVES if m[0] != prev_face] if prev_face else ALL_MOVES
        mv = random.choice(candidates)
        do_move(st, mv)
        prev_face = mv[0]

def cube_solved(st):
    return all(len(set(st[f].flatten())) == 1 for f in FACES)

# ── Singmaster parser ──────────────────────────────────────────────────────────

def expand_repeats(text):
    """
    Mở rộng nhóm lặp lại, hỗ trợ lồng nhau.
    Ví dụ: ((R U)2 F)3 → mở từ trong ra ngoài.
    """
    pat = re.compile(r'\(([^()]+)\)(\d+)')
    # Lặp cho đến khi không còn group nào (xử lý nested từ trong ra ngoài)
    for _ in range(10):   # giới hạn 10 lần lồng, tránh vòng lặp vô hạn
        m = pat.search(text)
        if not m:
            break
        text = (text[:m.start()]
                + (' '.join([m.group(1).strip()] * int(m.group(2))))
                + text[m.end():])
    return text

# Tất cả base moves được hỗ trợ (case-sensitive)
_VALID_BASES = set('UDFBLRMESxyzudfblr')

def parse_singmaster(text):
    """
    Parse chuỗi Singmaster đầy đủ (WCA notation, case-sensitive).

    Hỗ trợ:
      Outer (1 tầng) : U D F B L R  [+ ' hoặc 2]
      Wide  (2 tầng) : u d f b l r  [+ ' hoặc 2]
      Slice          : M E S         [+ ' hoặc 2]
      Rotation       : x y z         [+ ' hoặc 2]
      Nhóm           : (R U)3

    Phân biệt hoa/thường: U ≠ u, F ≠ f, v.v.

    Trả về (list_of_moves, None) nếu OK,
    hoặc (None, error_string) nếu có lỗi.
    """
    expanded = expand_repeats(text)

    # Token pattern (case-sensitive): chữ đơn + suffix tùy chọn
    # Thứ tự: 2' trước ', 2 để tránh nhầm
    TOKEN = re.compile(r"[UDFBLRMESxyzudfblr](?:2'|'2|2|')?")
    tokens = TOKEN.findall(expanded)

    # Phần còn lại sau khi bóc hết token và khoảng trắng/dấu phẩy
    remainder = TOKEN.sub('', expanded)
    remainder = re.sub(r'[\s,()]', '', remainder)
    if remainder:
        return None, f"Unknown: '{remainder}'"

    # Validate: mỗi token có base hợp lệ không?
    # (regex đã đảm bảo, nhưng double-check)
    moves = []
    for tok in tokens:
        base = tok.rstrip("2'")
        if base not in _VALID_BASES:
            return None, f"Unknown move: '{tok}'"
        # Chuẩn hóa suffix '2 → 2 (thứ tự không quan trọng)
        if tok.endswith("'2"):
            tok = base + "2'"
        moves.append(tok)

    return moves, None


def invert_moves(moves):
    """Đảo ngược chuỗi moves (tất cả notation đều được hỗ trợ)."""
    inv = {}
    for b in _VALID_BASES:
        inv[b]       = b + "'"
        inv[b + "'"] = b
        inv[b + '2'] = b + '2'   # self-inverse
    return [inv.get(m, m) for m in reversed(moves)]

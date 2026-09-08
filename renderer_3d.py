"""
renderer_3d.py
==============
Renderer 3D cho Rubik's Cube bằng phép chiếu phối cảnh (không dùng OpenGL).

Bao gồm:
  - Dữ liệu hình học cubie (FACE_INFO, CUBIE_H, STICKER_H)
  - Ma trận xoay Rx, Ry
  - Shading theo normal vector
  - Vẽ cube lên pygame Surface (draw_cube_3d)
  - Hit-test mặt bằng chuột (hit_test_3d)
  - Cache geometry tĩnh (chỉ re-project khi camera thay đổi)
"""

import math
import numpy as np
import pygame

from constants import FACES, COL3D, GLOW

# ── Kích thước cubie / sticker ─────────────────────────────────────────────────
CUBIE_H   = 0.96
STICKER_H = 0.76

# ── Dữ liệu hình học 6 mặt ────────────────────────────────────────────────────
FACE_INFO = {
    'U': (np.array([ 0,  1,  0.]), np.array([1, 0,  0.]), np.array([ 0, 0, -1.]), 1,  1),
    'D': (np.array([ 0, -1,  0.]), np.array([1, 0,  0.]), np.array([ 0, 0,  1.]), 1, -1),
    'F': (np.array([ 0,  0,  1.]), np.array([1, 0,  0.]), np.array([ 0, 1,  0.]), 2,  1),
    'B': (np.array([ 0,  0, -1.]), np.array([-1,0, 0.]), np.array([ 0, 1,  0.]), 2, -1),
    'L': (np.array([-1,  0,  0.]), np.array([0, 0,  1.]), np.array([ 0, 1,  0.]), 0, -1),
    'R': (np.array([ 1,  0,  0.]), np.array([0, 0, -1.]), np.array([ 0, 1,  0.]), 0,  1),
}

LIGHT = np.array([0.45, 0.85, 0.65])
LIGHT /= np.linalg.norm(LIGHT)

# ── Đặc tả animation cho TẤT CẢ 18 base move (outer, slice, wide, rotation) ─────
# Mỗi entry: (axis_vector, axis_index, {các giá trị layer bị ảnh hưởng})
#   axis_index: 0=x (L/R), 1=y (U/D), 2=z (F/B)
#   layer values: -1 / 0 / 1 ứng với ix/iy/iz của từng cubie (xem cubies_on_face)
# Trục & chiều dương (không-prime) LUÔN lấy theo mặt "gốc" của move đó, khớp
# đúng quy ước CW/CCW trong cube_engine.py (vd M không-prime = cùng chiều L
# không-prime, x không-prime = cùng chiều R không-prime, ...).
MOVE_SPEC = {
    # Outer — quay đúng 1 tầng ngoài
    'U': (FACE_INFO['U'][0], 1, {1}),
    'D': (FACE_INFO['D'][0], 1, {-1}),
    'F': (FACE_INFO['F'][0], 2, {1}),
    'B': (FACE_INFO['B'][0], 2, {-1}),
    'L': (FACE_INFO['L'][0], 0, {-1}),
    'R': (FACE_INFO['R'][0], 0, {1}),
    # Slice — quay đúng tầng giữa
    'M': (FACE_INFO['L'][0], 0, {0}),   # M cùng chiều L
    'E': (FACE_INFO['D'][0], 1, {0}),   # E cùng chiều D
    'S': (FACE_INFO['F'][0], 2, {0}),   # S cùng chiều F
    # Wide — quay tầng ngoài + tầng giữa cùng lúc (khối 2 tầng)
    'u': (FACE_INFO['U'][0], 1, {0, 1}),
    'd': (FACE_INFO['D'][0], 1, {-1, 0}),
    'f': (FACE_INFO['F'][0], 2, {0, 1}),
    'b': (FACE_INFO['B'][0], 2, {-1, 0}),
    'l': (FACE_INFO['L'][0], 0, {-1, 0}),
    'r': (FACE_INFO['R'][0], 0, {0, 1}),
    # Rotation — quay cả khối (cả 3 tầng)
    'x': (FACE_INFO['R'][0], 0, {-1, 0, 1}),
    'y': (FACE_INFO['U'][0], 1, {-1, 0, 1}),
    'z': (FACE_INFO['F'][0], 2, {-1, 0, 1}),
}
ANIMATABLE_BASES = set(MOVE_SPEC.keys())

# ── Ma trận xoay ──────────────────────────────────────────────────────────────
def Rx(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[1,0,0],[0,c,-s],[0,s,c]], dtype=float)

def Ry(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c,0,s],[0,1,0],[-s,0,c]], dtype=float)

# ── Helpers hình học ───────────────────────────────────────────────────────────
def cubie_face_quads(ix, iy, iz, face):
    n, rv, uv, _, _ = FACE_INFO[face]
    ctr = np.array([ix, iy, iz], float) + n * (CUBIE_H / 2 + 0.001)
    hp, hs = CUBIE_H / 2, STICKER_H / 2
    corners = [(-1,-1),(1,-1),(1,1),(-1,1)]
    pl = [ctr + rv*dx*hp + uv*dy*hp           for dx,dy in corners]
    st = [ctr + rv*dx*hs + uv*dy*hs + n*0.015 for dx,dy in corners]
    return pl, st

def cubie_to_rc(ix, iy, iz, face):
    _, rv, uv, _, _ = FACE_INFO[face]
    pos = np.array([ix, iy, iz], float)
    col = int(round(float(np.dot(pos, rv)))) + 1
    row = 1 - int(round(float(np.dot(pos, uv))))
    return max(0, min(2, row)), max(0, min(2, col))

def axis_rot_mat(axis, ang):
    """Ma trận xoay quanh 1 trục bất kỳ (không chỉ trục của 6 mặt ngoài)."""
    x, y, z = axis
    c, s, t  = math.cos(ang), math.sin(ang), 1 - math.cos(ang)
    return np.array([
        [t*x*x+c,   t*x*y-s*z, t*x*z+s*y],
        [t*x*y+s*z, t*y*y+c,   t*y*z-s*x],
        [t*x*z-s*y, t*y*z+s*x, t*z*z+c  ],
    ])

def shade(col, nw):
    f = 0.80 + 0.20 * max(0., float(np.dot(nw, LIGHT)))
    return tuple(min(255, int(c * f)) for c in col)

# ── Phép chiếu ─────────────────────────────────────────────────────────────────
def proj3(pt, Rm, zoom, cx, cy):
    q = Rm @ pt
    return (cx + q[0]*zoom, cy - q[1]*zoom, q[2])

def proj_quad3(quad, Rm, zoom, cx, cy):
    return [proj3(p, Rm, zoom, cx, cy) for p in quad]

def avg_z(pts):
    return sum(p[2] for p in pts) / len(pts)

def pt_in_poly(mx, my, pts4):
    poly   = [(p[0], p[1]) for p in pts4]
    inside = False
    j      = 3
    for i in range(4):
        xi, yi = poly[i]; xj, yj = poly[j]
        if (yi > my) != (yj > my) and mx < (xj-xi)*(my-yi)/(yj-yi)+xi:
            inside = not inside
        j = i
    return inside

# ── Z-buffer rasterizer (thay painter's-algorithm) ──────────────────────────────
# Painter's algorithm (sắp theo 1 số z trung bình mỗi đa giác) sai khi 2 đa giác
# đè chéo lên nhau trên màn hình (xảy ra khi cho sticker mặt lân cận cùng quay
# theo mặt chính — vd hàng trên của F/R/B/L quay theo U). Z-buffer so độ sâu
# TỪNG PIXEL nên luôn đúng bất kể các đa giác đè chéo thế nào.
def _poly_mask_and_z(gx, gy, pts):
    """gx: shape (1,W), gy: shape (H,1) — dùng broadcasting thay vì lưới đầy
    đủ (H,W) để giảm cấp phát bộ nhớ + số lệnh gọi numpy.
    pts: 4 điểm (X,Y,Z) đã chiếu màn hình (Z = camera-space).
    Trả về (mask bool shape (H,W), zvals nội suy shape (H,W))."""
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3), (x4, y4, z4) = pts

    # Đa giác lồi: điểm nằm trong nếu cross-product cùng dấu ở mọi cạnh
    # (chấp nhận cả 2 chiều winding CW/CCW tùy góc xoay). Tính trực tiếp
    # bằng phép toán mảng (không np.stack) để tránh overhead gọi hàm.
    c1 = (x2 - x1) * (gy - y1) - (y2 - y1) * (gx - x1)
    c2 = (x3 - x2) * (gy - y2) - (y3 - y2) * (gx - x2)
    c3 = (x4 - x3) * (gy - y3) - (y4 - y3) * (gx - x3)
    c4 = (x1 - x4) * (gy - y4) - (y1 - y4) * (gx - x4)
    mask = ((c1 >= -1e-6) & (c2 >= -1e-6) & (c3 >= -1e-6) & (c4 >= -1e-6)) | \
           ((c1 <=  1e-6) & (c2 <=  1e-6) & (c3 <=  1e-6) & (c4 <=  1e-6))

    # Đa giác phẳng trong 3D + phép chiếu trực giao (không chia phối cảnh theo
    # z) -> z là hàm TUYẾN TÍNH của (X,Y) màn hình. Giải mặt phẳng z=aX+bY+c
    # từ 3 điểm đầu bằng công thức Cramer tường minh (số thực Python, không
    # gọi np.linalg.solve) — rẻ hơn nhiều cho hệ 3x3 nhỏ gọi lặp lại.
    det = (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3)
    if abs(det) < 1e-9:
        zvals = (z1 + z2 + z3) / 3.0   # scalar, numpy tự broadcast khi dùng
    else:
        a = ((z1 - z3) * (y2 - y3) - (z2 - z3) * (y1 - y3)) / det
        b = ((x1 - x3) * (z2 - z3) - (x2 - x3) * (z1 - z3)) / det
        c = z3 - a * x3 - b * y3
        zvals = a * gx + b * gy + c
    return mask, zvals

_RASTER_SENTINEL = (1, 2, 3)   # màu "trong suốt" nội bộ, không trùng màu thật nào

def _rasterize(quads, screen_w, screen_h):
    """quads: [(pts[4 điểm XYZ], color, border_color_hoặc_None, border_width), ...]
    Vẽ bằng z-buffer thật (so độ sâu từng pixel), trả về (surface, ox, oy)
    đã cắt gọn theo đúng vùng khối chiếm trên màn hình, nền trong suốt.
    """
    all_x = [p[0] for q in quads for p in q[0]]
    all_y = [p[1] for q in quads for p in q[0]]
    if not all_x:
        return None
    margin = 3
    ox = max(0, int(math.floor(min(all_x))) - margin)
    oy = max(0, int(math.floor(min(all_y))) - margin)
    ex = min(screen_w, int(math.ceil(max(all_x))) + margin)
    ey = min(screen_h, int(math.ceil(max(all_y))) + margin)
    w, h = max(1, ex - ox), max(1, ey - oy)

    zbuf   = np.full((h, w), -np.inf)
    colbuf = np.empty((h, w, 3), dtype=np.uint8)
    colbuf[:, :] = _RASTER_SENTINEL

    for pts, color, border, bw in quads:
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        qx0 = max(int(math.floor(min(xs))), ox)
        qx1 = min(int(math.ceil(max(xs))),  ex - 1)
        qy0 = max(int(math.floor(min(ys))), oy)
        qy1 = min(int(math.ceil(max(ys))),  ey - 1)
        if qx0 > qx1 or qy0 > qy1:
            continue
        gx = np.arange(qx0, qx1 + 1, dtype=float)[None, :]   # (1, W)
        gy = np.arange(qy0, qy1 + 1, dtype=float)[:, None]   # (H, 1)
        mask, zvals = _poly_mask_and_z(gx, gy, pts)
        lx0, ly0 = qx0 - ox, qy0 - oy
        lx1, ly1 = lx0 + (qx1 - qx0) + 1, ly0 + (qy1 - qy0) + 1
        zslice = zbuf[ly0:ly1, lx0:lx1]
        better = mask & (zvals > zslice)
        if not np.any(better):
            continue
        zslice[better] = zvals if np.isscalar(zvals) else zvals[better]
        colbuf[ly0:ly1, lx0:lx1][better] = color

    surf3d = pygame.surfarray.make_surface(colbuf.swapaxes(0, 1))
    surf3d.set_colorkey(_RASTER_SENTINEL)

    # Viền chọn (sel_face glow): vẽ đè lên sau cùng, không cần z-test vì chỉ
    # là chỉ báo UI, không phải hình khối thật.
    for pts, color, border, bw in quads:
        if border:
            ipts = [(int(p[0]) - ox, int(p[1]) - oy) for p in pts]
            pygame.draw.polygon(surf3d, border, ipts, bw)

    return surf3d, ox, oy

# ── Geometry cache ─────────────────────────────────────────────────────────────
# Mô hình CUBIE-CENTRIC: mỗi trong 26 cubie (bỏ cubie tâm 0,0,0 không bao giờ
# thấy được) là 1 khối THẬT có đủ 6 mặt — mặt nào trùng lớp ngoài thì là
# sticker (có màu), mặt còn lại là "thân nhựa" màu tối (is_sticker=False).
# Nhờ vậy khi 1 lớp tách ra quay, cubie đứng yên bên cạnh LUÔN có sẵn mặt
# nhựa của chính nó lấp đúng chỗ hở — không cần "backing" giả lập nữa.
_CUBIES = [
    (ix, iy, iz)
    for ix in (-1, 0, 1) for iy in (-1, 0, 1) for iz in (-1, 0, 1)
    if not (ix == 0 and iy == 0 and iz == 0)
]

_CUBIE_GEOM = {}   # (ix,iy,iz) → [(face, is_sticker, panel_pts, sticker_pts, row, col), ...]

def _build_cubie_geom():
    global _CUBIE_GEOM
    _CUBIE_GEOM = {}
    for (ix, iy, iz) in _CUBIES:
        entries = []
        for face in FACES:
            axis, val   = FACE_INFO[face][3], FACE_INFO[face][4]
            is_sticker  = [ix, iy, iz][axis] == val
            pl, st      = cubie_face_quads(ix, iy, iz, face)
            r, c        = cubie_to_rc(ix, iy, iz, face) if is_sticker else (None, None)
            entries.append((face, is_sticker, pl, st, r, c))
        _CUBIE_GEOM[(ix, iy, iz)] = entries

_build_cubie_geom()

# Index phụ dùng cho hit-test bằng chuột (chỉ cần các mặt sticker).
_STICKERS_BY_FACE = {f: [] for f in FACES}
for _pos, _entries in _CUBIE_GEOM.items():
    for _face, _is_sticker, _pl, _st, _r, _c in _entries:
        if _is_sticker:
            _STICKERS_BY_FACE[_face].append(_st)

# Projection cache: keyed by (Rm_hash, zoom, cx, cy, anim_face, anim_ang)
# Stores projected polys ready to draw (sorted by depth)
_PROJ_CACHE = None
_PROJ_CACHE_KEY = None

def _make_cache_key(Rm, zoom, cx, cy, sel_face, anim_face, anim_ang, state_version):
    """Tạo cache key từ tất cả tham số ảnh hưởng đến projection.

    state_version PHẢI đổi mỗi khi state cube bị mutate (do_move, scramble,
    reset, undo...) — nếu không, các nước đi không animate (M E S x y z,
    wide moves, scramble, reset, undo) sẽ không kích hoạt rebuild và 3D
    view sẽ hiển thị hình ảnh CŨ dù state đã thay đổi.
    """
    rm_key = tuple(Rm.flatten().round(5))
    ang_key = round(anim_ang, 5) if anim_ang else 0
    return (rm_key, round(zoom, 3), cx, cy, sel_face, anim_face, ang_key, state_version)

# ── Hit-test ───────────────────────────────────────────────────────────────────
def hit_test_3d(mx, my, Rm, zoom, lo):
    cx, cy   = lo.CX3, lo.CY3
    best_z, best = -1e9, None
    for face in FACES:
        nw = Rm @ FACE_INFO[face][0]
        if nw[2] < 0.05:
            continue
        for st in _STICKERS_BY_FACE[face]:
            pts = proj_quad3(st, Rm, zoom, cx, cy)
            z   = avg_z(pts)
            if z > best_z and pt_in_poly(mx, my, pts):
                best_z, best = z, face
    return best

# ── Draw cube 3D ───────────────────────────────────────────────────────────────
def draw_cube_3d(surf, state, Rm, zoom, sel_face, anim_face, anim_ang, lo, state_version=0):
    """
    Vẽ toàn bộ cube 3D lên surf.
    Dùng geometry cache: chỉ re-project khi Rm/zoom/camera/state thay đổi.

    state_version: bộ đếm do caller (main.py) tăng mỗi khi state bị mutate.
    Bắt buộc phải truyền đúng, nếu không cache sẽ không biết state đã đổi
    khi camera/animation params không đổi (case: wide/slice/rotation move,
    scramble, reset, undo — những thao tác không đi qua animation).
    """
    cx, cy = lo.CX3, lo.CY3
    cache_key = _make_cache_key(Rm, zoom, cx, cy, sel_face, anim_face, anim_ang, state_version)

    # Rebuild ảnh nếu camera/animation/state thay đổi
    global _PROJ_CACHE, _PROJ_CACHE_KEY
    if cache_key != _PROJ_CACHE_KEY:
        quads = []

        spec = MOVE_SPEC.get(anim_face) if anim_face else None
        if spec and abs(anim_ang) > 1e-4:
            axis, axis_idx, allowed = spec
            anim_Ra = Rm @ axis_rot_mat(axis, anim_ang)
        else:
            anim_Ra, axis_idx, allowed = None, None, None

        for (ix, iy, iz) in _CUBIES:
            included = anim_Ra is not None and (ix, iy, iz)[axis_idx] in allowed
            Ra = anim_Ra if included else Rm

            for face, is_sticker, pl, st, r, c in _CUBIE_GEOM[(ix, iy, iz)]:
                # Mặt không phải sticker (thân nhựa nội bộ) bình thường luôn
                # bị mặt ngoài của chính cubie đó hoặc cubie kề che khuất,
                # z-buffer tự loại — nhưng khi KHÔNG animate thì chắc chắn
                # không bao giờ lộ ra (khối lắp khít), nên bỏ qua hẳn để đỡ
                # tốn CPU rasterize; chỉ cần tính khi có animation (lúc đó
                # mặt nhựa của cubie đứng yên có thể lộ ra qua khe hở).
                if not is_sticker and anim_Ra is None:
                    continue

                n  = FACE_INFO[face][0]
                nw = Ra @ n
                if nw[2] < 0.:
                    continue   # mặt quay lưng camera -> backface culling

                ppts = proj_quad3(pl, Ra, zoom, cx, cy)
                quads.append((ppts, (20, 20, 20), None, 0))

                if is_sticker:
                    ck     = state[face][r, c]
                    color  = shade(COL3D[ck], nw)
                    spts   = proj_quad3(st, Ra, zoom, cx, cy)
                    border = GLOW if face == sel_face else None
                    bw     = 2   if face == sel_face else 0
                    quads.append((spts, color, border, bw))

        sw, sh          = surf.get_size()
        _PROJ_CACHE     = _rasterize(quads, sw, sh)
        _PROJ_CACHE_KEY = cache_key

    # Blit ảnh đã dựng sẵn từ cache
    if _PROJ_CACHE is not None:
        img, ox, oy = _PROJ_CACHE
        surf.blit(img, (ox, oy))

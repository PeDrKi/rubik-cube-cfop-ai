"""
prepare_data.py — dựng bộ dữ liệu huấn luyện "dò 1 mặt cube" từ bộ ảnh
Bielefeld (3 mặt/ảnh, có toạ độ 7 góc thật).

Ý tưởng: từ 7 điểm góc, suy ra hộp bao (bounding box) của TỪNG mặt trong
3 mặt nhìn thấy (top/right/left). Với mỗi mặt, cắt 1 vùng vuông NGẪU
NHIÊN quanh mặt đó (tâm + độ phóng to lệch ngẫu nhiên) để mô phỏng đúng
tình huống thực tế: webcam nhìn thấy 1 mặt cube không hoàn toàn canh
giữa, không hoàn toàn đúng tỉ lệ khung -- rồi lưu lại (ảnh crop, bbox
mặt đó quy về toạ độ 0..1 trong ảnh crop) làm 1 mẫu huấn luyện.

Input:  /home/claude/bielefeld/dataset4/<setup>/<n>.png (+ .pos, config.npy)
Output: /home/claude/train_detect/data/{images/*.jpg, labels.csv}
        labels.csv: filename,cx,cy,half_size   (toạ độ chuẩn hoá 0..1 trong ảnh crop 96x96)
"""
import pickle
import random
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

random.seed(0)

SRC = Path("/home/claude/bielefeld/dataset4")
OUT_IMG = Path("/home/claude/train_detect/data/images")
OUT_IMG.mkdir(parents=True, exist_ok=True)
LABELS_PATH = Path("/home/claude/train_detect/data/labels.csv")

CROP_SIZE = 96  # kích thước ảnh đưa vào CNN
CROPS_PER_FACE = 7  # số lần crop ngẫu nhiên khác nhau cho MỖI mặt -> tăng cường dữ liệu
VAL_SETUP_FRACTION = 0.15  # tỉ lệ THƯ MỤC (không phải ảnh/crop lẻ) dành cho validation


def face_polygons(pos: dict):
    """4 đỉnh (thứ tự liền kề, để vẽ mask đa giác) của từng mặt trong 3
    mặt, ĐÚNG thứ tự top/right/left khớp config.npy."""
    top, tr, br, bot, bl, tl, center = (
        pos["top"], pos["tr"], pos["br"], pos["bot"], pos["bl"], pos["tl"], pos["center"]
    )
    return [
        [top, tr, center, tl],
        [center, tr, br, bot],
        [tl, center, bot, bl],
    ]


def face_corners(pos: dict):
    """Hộp bao (xmin,ymin,xmax,ymax) của từng mặt — dùng làm nhãn huấn
    luyện (regression target), khác với `face_polygons` (dùng để vẽ
    mask cắt-dán nền)."""
    def bbox(pts):
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        return min(xs), min(ys), max(xs), max(ys)
    return [bbox(poly) for poly in face_polygons(pos)]


COMPOSITE_PROB = 0.6  # tỉ lệ mẫu được cắt-dán sang nền khác (còn lại giữ nền thật gốc)


def random_background(size):
    """Sinh 1 nền thay thế ngẫu nhiên: màu phẳng, gradient, hoặc nhiễu
    -- KHÔNG dùng ảnh cube khác làm nền (tránh vô tình dán đè 1 mặt cube
    thật lên nền của mặt khác, gây nhãn sai)."""
    w, h = size
    mode = random.choice(["solid", "gradient", "noise"])
    if mode == "solid":
        c = tuple(random.randint(0, 255) for _ in range(3))
        return Image.new("RGB", (w, h), c)
    if mode == "gradient":
        c1 = np.array([random.randint(0, 255) for _ in range(3)], dtype=np.float32)
        c2 = np.array([random.randint(0, 255) for _ in range(3)], dtype=np.float32)
        t = np.linspace(0, 1, w)[None, :, None]
        row = c1[None, None, :] * (1 - t) + c2[None, None, :] * t
        arr = np.repeat(row, h, axis=0).astype(np.uint8)
        return Image.fromarray(arr)
    arr = np.random.randint(0, 255, (h, w, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    return img.filter(ImageFilter.GaussianBlur(radius=random.uniform(2, 10)))


def composite_onto_random_background(crop: Image.Image, mask: Image.Image) -> Image.Image:
    """Giữ nguyên pixel THẬT bên trong `mask` (đúng hình mặt cube, kể cả
    viền đen quanh sticker), thay TOÀN BỘ phần còn lại bằng 1 nền ngẫu
    nhiên mới -- buộc model học đặc trưng THẬT của cube (lưới 3x3 +
    viền đen), không học "nhớ" nền phòng thí nghiệm cố định lặp lại
    trong bộ dữ liệu gốc."""
    bg = random_background(crop.size)
    return Image.composite(crop, bg, mask)


def random_crop_around(img: Image.Image, box, polygon=None, out_size=CROP_SIZE):
    """Crop 1 vùng vuông NGẪU NHIÊN quanh `box` (xmin,ymin,xmax,ymax):
    tâm lệch ngẫu nhiên (jitter) + kích thước phóng to ngẫu nhiên (zoom
    khác nhau) -- mô phỏng người dùng cầm cube không canh giữa tuyệt
    đối. Nếu có `polygon` (4 đỉnh thật của mặt), có xác suất cắt-dán mặt
    đó sang 1 nền ngẫu nhiên khác (xem `composite_onto_random_background`).
    Trả về (ảnh đã resize CROP_SIZE, bbox mặt quy về 0..1 trong ảnh crop
    đó) hoặc None nếu crop ra ngoài ảnh quá nhiều.
    """
    W, H = img.size
    xmin, ymin, xmax, ymax = box
    fw, fh = xmax - xmin, ymax - ymin
    fcx, fcy = (xmin + xmax) / 2, (ymin + ymax) / 2
    face_size = max(fw, fh)

    # Vùng crop rộng hơn mặt 1.1x .. 7x (mô phỏng cả zoom gần SÁT lẫn
    # trường hợp cube chỉ chiếm 1 góc nhỏ giữa khung hình webcam rộng
    # -- nếu chỉ train với zoom gần, model sẽ không học được cách "tìm"
    # cube khi nó nhỏ/lệch xa tâm khung hình, chỉ học được cách tinh
    # chỉnh nhẹ quanh vị trí đã gần đúng sẵn).
    crop_size = face_size * random.uniform(1.1, 7.0)
    # Lệch tâm tối đa ~45% kích thước crop
    jitter = crop_size * 0.45
    ccx = fcx + random.uniform(-jitter, jitter)
    ccy = fcy + random.uniform(-jitter, jitter)

    cxmin, cymin = ccx - crop_size / 2, ccy - crop_size / 2
    cxmax, cymax = ccx + crop_size / 2, ccy + crop_size / 2

    # Bỏ qua nếu crop thò ra ngoài ảnh gốc quá nhiều (không đủ ngữ cảnh nền thật)
    if cxmin < -crop_size * 0.4 or cymin < -crop_size * 0.4:
        return None
    if cxmax > W + crop_size * 0.4 or cymax > H + crop_size * 0.4:
        return None

    crop = img.crop((cxmin, cymin, cxmax, cymax)).resize((out_size, out_size), Image.BILINEAR)
    # Vùng ngoài ảnh gốc (do crop âm/thò ra) sẽ được PIL tự độn đen -- ổn,
    # giống hệt cách sample.rs xử lý vùng ngoài ảnh trong app thật.

    if polygon is not None and random.random() < COMPOSITE_PROB:
        # Vẽ mask đa giác đúng hình mặt cube, quy đổi thẳng sang toạ độ
        # ảnh CUỐI (out_size x out_size, rẻ hơn nhiều so với vẽ+giãn nở
        # trên ảnh crop gốc có thể rất lớn trước khi resize), nới rộng
        # vài pixel để giữ viền đen quanh sticker, làm mờ nhẹ biên để
        # không tạo cạnh cắt giả tạo.
        scale = out_size / crop_size
        rel_poly = [((px - cxmin) * scale, (py - cymin) * scale) for px, py in polygon]
        mask = Image.new("L", (out_size, out_size), 0)
        ImageDraw.Draw(mask).polygon(rel_poly, fill=255)
        mask = mask.filter(ImageFilter.MaxFilter(5))
        mask = mask.filter(ImageFilter.GaussianBlur(radius=1.2))
        crop = composite_onto_random_background(crop, mask)

    # Quy đổi bbox mặt sang toạ độ 0..1 trong ảnh crop
    rel_xmin = (xmin - cxmin) / crop_size
    rel_ymin = (ymin - cymin) / crop_size
    rel_xmax = (xmax - cxmin) / crop_size
    rel_ymax = (ymax - cymin) / crop_size
    cx = (rel_xmin + rel_xmax) / 2
    cy = (rel_ymin + rel_ymax) / 2
    half = max(rel_xmax - rel_xmin, rel_ymax - rel_ymin) / 2
    return crop, (cx, cy, half)


def main():
    rows = []
    n_images = 0

    setup_dirs = sorted(d for d in SRC.iterdir() if d.is_dir())
    random.shuffle(setup_dirs)
    n_val_setups = max(1, int(len(setup_dirs) * VAL_SETUP_FRACTION))
    val_setup_names = {d.name for d in setup_dirs[:n_val_setups]}
    print(f"Thư mục setup dùng cho VAL (ảnh hoàn toàn chưa thấy lúc train): {sorted(val_setup_names)}")

    for setup_dir in sorted(SRC.iterdir()):
        if not setup_dir.is_dir():
            continue
        is_val = setup_dir.name in val_setup_names
        for pos_file in sorted(setup_dir.glob("*.png.pos")):
            img_file = setup_dir / pos_file.name[: -len(".pos")]
            if not img_file.exists():
                continue
            with open(pos_file, "rb") as f:
                pos = pickle.load(f)
            img = Image.open(img_file).convert("RGB")
            n_images += 1

            for face, polygon in zip(face_corners(pos), face_polygons(pos)):
                for _ in range(CROPS_PER_FACE):
                    result = random_crop_around(img, face, polygon=polygon)
                    if result is None:
                        continue
                    crop, (cx, cy, half) = result
                    name = f"{len(rows):06d}.jpg"
                    crop.save(OUT_IMG / name, quality=88)
                    rows.append((name, cx, cy, half, is_val))

    with open(LABELS_PATH, "w") as f:
        f.write("filename,cx,cy,half_size,is_val\n")
        for name, cx, cy, half, is_val in rows:
            f.write(f"{name},{cx:.5f},{cy:.5f},{half:.5f},{int(is_val)}\n")

    n_val = sum(1 for r in rows if r[4])
    print(f"Ảnh gốc dùng: {n_images}")
    print(f"Mẫu huấn luyện tạo ra: {len(rows)} (train={len(rows)-n_val}, val={n_val})")


if __name__ == "__main__":
    main()

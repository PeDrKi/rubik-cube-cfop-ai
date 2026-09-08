"""
train.py — huấn luyện 1 CNN nhỏ dự đoán vị trí mặt cube (cx, cy,
half_size, đều chuẩn hoá 0..1) từ ảnh crop 96x96.

Kiến trúc cố tình rất nhỏ (4 lớp conv, ~150K tham số) để: (1) huấn
luyện nhanh trên CPU 1 nhân trong sandbox, (2) suy luận đủ nhẹ để chạy
mượt trong app Rust ở ~10fps trên máy yếu -- đúng ràng buộc thực tế của
dự án, không phải chọn kiến trúc "tốt nhất lý thuyết".
"""
import csv
import random
import time
from pathlib import Path

import torch
import torch.nn as nn
import numpy as np
from PIL import Image
from torch.utils.data import Dataset, DataLoader

DATA_DIR = Path("/home/claude/train_detect/data")
CKPT_PATH = Path("/home/claude/train_detect/face_detector.pt")
ONNX_PATH = Path("/home/claude/train_detect/face_detector.onnx")

torch.manual_seed(0)
random.seed(0)


class FaceBoxDataset(Dataset):
    """`augment=True` (chỉ dùng cho tập train): lật ngang ngẫu nhiên +
    rung màu/độ sáng nhẹ + xoay nhẹ -- mô phỏng thêm biến thể mà 338 ảnh
    gốc không có sẵn (mọi ảnh gốc đều chụp từ 1-2 khối cube, vài kiểu
    ánh sáng cố định), giúp đỡ overfit vào đúng những gì đã thấy.
    """

    def __init__(self, rows, augment: bool):
        self.rows = rows
        self.augment = augment

    def __len__(self):
        return len(self.rows)

    def __getitem__(self, idx):
        name, cx, cy, half = self.rows[idx]
        img = Image.open(DATA_DIR / "images" / name).convert("RGB")

        if self.augment:
            if random.random() < 0.5:
                img = img.transpose(Image.FLIP_LEFT_RIGHT)
                cx = 1.0 - cx
            if random.random() < 0.7:
                arr = np.array(img, dtype=np.float32)
                arr *= random.uniform(0.7, 1.3)  # độ sáng
                arr = (arr - 127.5) * random.uniform(0.8, 1.25) + 127.5  # tương phản
                # rung nhẹ từng kênh màu riêng (mô phỏng cân bằng trắng webcam khác nhau)
                for c in range(3):
                    arr[:, :, c] *= random.uniform(0.85, 1.15)
                img = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            if random.random() < 0.4:
                angle = random.uniform(-12, 12)
                img = img.rotate(angle, resample=Image.BILINEAR, fillcolor=(30, 30, 30))

        t = torch.from_numpy(np.array(img, dtype="float32") / 255.0).permute(2, 0, 1)
        target = torch.tensor([cx, cy, half], dtype=torch.float32)
        return t, target


def load_rows():
    train_rows, val_rows = [], []
    with open(DATA_DIR / "labels.csv") as f:
        for r in csv.DictReader(f):
            row = (r["filename"], float(r["cx"]), float(r["cy"]), float(r["half_size"]))
            (val_rows if r["is_val"] == "1" else train_rows).append(row)
    return train_rows, val_rows


class TinyFaceNet(nn.Module):
    """Input: (B,3,96,96) -> Output: (B,3) = (cx, cy, half_size) trong 0..1."""

    def __init__(self):
        super().__init__()
        self.features = nn.Sequential(
            nn.Conv2d(3, 24, 3, stride=2, padding=1), nn.BatchNorm2d(24), nn.ReLU(inplace=True),   # 96->48
            nn.Conv2d(24, 48, 3, stride=2, padding=1), nn.BatchNorm2d(48), nn.ReLU(inplace=True),  # 48->24
            nn.Conv2d(48, 96, 3, stride=2, padding=1), nn.BatchNorm2d(96), nn.ReLU(inplace=True),  # 24->12
            nn.Conv2d(96, 96, 3, stride=2, padding=1), nn.BatchNorm2d(96), nn.ReLU(inplace=True),  # 12->6
        )
        self.pool = nn.AdaptiveAvgPool2d(1)
        self.head = nn.Sequential(
            nn.Flatten(),
            nn.Linear(96, 48), nn.ReLU(inplace=True),
            nn.Linear(48, 3), nn.Sigmoid(),
        )

    def forward(self, x):
        x = self.features(x)
        x = self.pool(x)
        return self.head(x)


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--epochs", type=int, default=8, help="số epoch chạy TRONG LƯỢT NÀY")
    ap.add_argument("--resume", action="store_true", help="tiếp tục từ checkpoint đã lưu")
    args = ap.parse_args()

    train_rows, val_rows = load_rows()
    print(f"train={len(train_rows)} val={len(val_rows)} (val = ảnh gốc KHÁC hoàn toàn với train)")

    train_ds = FaceBoxDataset(train_rows, augment=True)
    val_ds = FaceBoxDataset(val_rows, augment=False)
    train_dl = DataLoader(train_ds, batch_size=64, shuffle=True, num_workers=0)
    val_dl = DataLoader(val_ds, batch_size=128, shuffle=False, num_workers=0)

    model = TinyFaceNet()
    n_params = sum(p.numel() for p in model.parameters())
    print(f"params: {n_params:,}")

    opt = torch.optim.Adam(model.parameters(), lr=1e-3)
    sched = torch.optim.lr_scheduler.StepLR(opt, step_size=15, gamma=0.5)
    loss_fn = nn.MSELoss()

    state_path = Path("/home/claude/train_detect/train_state.pt")
    start_epoch = 1
    best_val = float("inf")
    if args.resume and state_path.exists():
        state = torch.load(state_path)
        model.load_state_dict(state["model"])
        opt.load_state_dict(state["opt"])
        sched.load_state_dict(state["sched"])
        start_epoch = state["epoch"] + 1
        best_val = state["best_val"]
        print(f"Tiếp tục từ epoch {start_epoch}, best_val hiện tại = {best_val:.5f}")

    end_epoch = start_epoch + args.epochs - 1
    for epoch in range(start_epoch, end_epoch + 1):
        t0 = time.time()
        model.train()
        train_loss = 0.0
        for x, y in train_dl:
            opt.zero_grad()
            pred = model(x)
            loss = loss_fn(pred, y)
            loss.backward()
            opt.step()
            train_loss += loss.item() * x.size(0)
        train_loss /= len(train_ds)
        sched.step()

        model.eval()
        val_loss = 0.0
        with torch.no_grad():
            for x, y in val_dl:
                pred = model(x)
                val_loss += loss_fn(pred, y).item() * x.size(0)
        val_loss /= len(val_ds)

        dt = time.time() - t0
        print(f"epoch {epoch:2d}  train_mse={train_loss:.5f}  val_mse={val_loss:.5f}  ({dt:.1f}s)")

        if val_loss < best_val:
            best_val = val_loss
            torch.save(model.state_dict(), CKPT_PATH)

        torch.save(
            {"model": model.state_dict(), "opt": opt.state_dict(), "sched": sched.state_dict(),
             "epoch": epoch, "best_val": best_val},
            state_path,
        )

    print(f"Best val MSE so far: {best_val:.5f} (đã lưu {CKPT_PATH})")

    # ── Export ONNX ─────────────────────────────────────────────────
    model.load_state_dict(torch.load(CKPT_PATH))
    model.eval()
    dummy = torch.randn(1, 3, 96, 96)
    torch.onnx.export(
        model, dummy, str(ONNX_PATH),
        input_names=["image"], output_names=["bbox"],
        opset_version=13,
        dynamic_axes=None,
        dynamo=False,
    )
    print(f"Đã export ONNX: {ONNX_PATH}")


if __name__ == "__main__":
    main()

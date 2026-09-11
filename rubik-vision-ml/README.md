# rubik-vision-ml — dò vị trí mặt cube bằng CNN (THỬ NGHIỆM)

## Trạng thái thật, nói thẳng

- Model (`models/face_detector.onnx`, 253KB) đã huấn luyện xong, xuất ONNX,
  và **đã kiểm chứng khớp 100% với PyTorch gốc** (sai số ~6×10⁻⁸).
- Code Rust (`src/lib.rs`) **CHƯA build-test được** — môi trường tôi dùng để
  viết code này có cargo quá cũ (1.75), còn crate `ort` (ONNX Runtime cho
  Rust) bản duy nhất còn trên crates.io (2.0 release-candidate — toàn bộ
  dòng 1.x đã bị gỡ) cần Rust ~1.88+. Bạn cần build thử trên máy mình để
  biết chắc code có đúng cú pháp/API không.
- Crate này **CỐ TÌNH tách khỏi workspace chính** (`rubik-core`/`rubik-app`/
  `rubik-vision`) — không đụng gì đến phần đã test kỹ, dù build ở đây có
  lỗi gì cũng không ảnh hưởng app chính.

## Bước 1 — Thử build độc lập

```bash
cd rubik-vision-ml
cargo build
```

Nếu lỗi vì **"requires rustc 1.88"** hoặc tương tự: chạy `rustup update`
rồi thử lại.

Nếu lỗi vì **thiếu `onnxruntime.dll`/`.so` lúc chạy** (không phải lúc
build): `ort` mặc định tự tải sẵn thư viện onnxruntime phù hợp lúc build
(feature `download-binaries`, đã bật theo mặc định) — nếu vẫn thiếu, xem
tài liệu `ort` (https://ort.pyke.io) phần "Cargo features" / "Linking".

Nếu lỗi vì **API không khớp** (ví dụ `try_extract_array` đổi tên/chữ ký):
crate `ort` 2.0 còn ở dạng release-candidate, API có thể đổi giữa các bản
rc — báo lỗi cụ thể lại, tôi sẽ sửa theo đúng bản bạn có.

## Bước 2 — Test nhanh không cần UI

Đã viết sẵn `examples/try.rs`. Chạy:
```bash
cargo run --example try -- duong/dan/toi/1_anh_cube.png
```
In ra toạ độ dò được + lưu `ket_qua.png` có vẽ khung đỏ tại vị trí đó —
mở lên xem bằng mắt có đúng quanh mặt cube không. Thử với vài ảnh khác
nhau (kể cả ảnh không phải cube, ảnh mờ...) trước khi nối vào UI.

## Bước 3 — Nối vào rubik-app (nếu Bước 1+2 ổn)

Thêm vào `rubik-app/Cargo.toml`:
```toml
rubik-vision-ml = { path = "../rubik-vision-ml" }
```
Rồi trong `scan_ui.rs`, thêm 1 lựa chọn chế độ dò thứ 3 ("ML — thử nghiệm")
cạnh khung xanh/vàng hiện có, gọi `rubik_vision_ml::MlDetector::load()` một
lần lúc mở webcam, và `detector.detect(&img)` mỗi khi có frame mới — thay
cho (hoặc cùng hiển thị song song để so sánh với) `detect_cube_face`.
Báo tôi kết quả Bước 1+2, tôi sẽ viết phần nối UI này cụ thể theo đúng
API đã xác nhận chạy được trên máy bạn.

## Huấn luyện lại / cải thiện thêm

Toàn bộ script huấn luyện (`prepare_data.py`, `train.py`) nằm ở
`/train_detect/` gửi kèm riêng (không đóng gói vào đây vì cần PyTorch +
bộ ảnh gốc, không cần thiết nếu chỉ muốn dùng model đã có sẵn).

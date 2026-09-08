# rubik-vision-ml — dò vị trí mặt cube bằng CNN (THỬ NGHIỆM)

## Trạng thái

- Model (`models/face_detector.onnx`, ~551 KB / 564.450 byte) đã huấn
  luyện xong, xuất ONNX, và **đã kiểm chứng khớp 100% với PyTorch gốc**
  (sai số ~6×10⁻⁸). Model được nhúng thẳng vào binary qua `include_bytes!`.
- Crate này **đã là thành viên của workspace chính** (`members` trong
  `Cargo.toml` gốc) và **đã được nối vào `rubik-app`** qua feature
  `ml_detect` — không còn ở dạng "viết xong nhưng chưa nối" như trước.
- Tuy vậy feature **TẮT mặc định**: `cargo build`/`cargo run -p rubik-app`
  bình thường KHÔNG kéo `ort`/ONNX Runtime vào. Chỉ khi bật `--features
  ml_detect` thì crate này mới được biên dịch.
- Lý do vẫn tách feature: `ort` 2.0 (release-candidate — bản duy nhất còn
  trên crates.io, toàn bộ dòng 1.x đã bị gỡ) cần Rust khá mới (~1.88+).
  Giữ nó sau feature-gate để phần lõi đã test kỹ
  (`rubik-core`/`rubik-vision`/`rubik-app`) luôn build được kể cả khi
  toolchain chưa đủ mới cho `ort`.

## Dùng trong app

```bash
cargo run --release -p rubik-app --features ml_detect
```

Rồi bấm `Ctrl+K` mở cửa sổ quét, chọn chế độ **"ML (thử nghiệm)"** cạnh
chế độ dò hình học. Model chạy **song song** để so sánh, không thay thế
`detect::detect_cube_face`. Nếu build KHÔNG bật feature mà vẫn chọn chế
độ này, app chỉ hiện cảnh báo "chưa bật tính năng ML" chứ không crash.

## Thử độc lập, không cần UI

`examples/try.rs` chạy model trên 1 ảnh và vẽ khung đỏ tại vị trí dò được:

```bash
cargo run -p rubik-vision-ml --example try -- duong/dan/toi/1_anh_cube.png
```

In ra toạ độ vùng vuông + lưu `ket_qua.png` (cùng thư mục chạy lệnh) —
mở lên xem bằng mắt có đúng quanh mặt cube không. Nên thử với vài ảnh
khác nhau (kể cả ảnh không phải cube, ảnh mờ) để nắm giới hạn của model.
Lưu ý: model luôn trả về đúng 1 dự đoán; `None` nghĩa là có lỗi kỹ thuật
khi chạy suy luận, không phải "không tìm thấy".

## Nếu build lỗi

- **"requires rustc 1.88"** hoặc tương tự → `rustup update` rồi thử lại.
- **Thiếu `onnxruntime.dll`/`.so` lúc chạy** (không phải lúc build) →
  `ort` mặc định bật feature `download-binaries` để tự tải thư viện
  onnxruntime phù hợp lúc build; nếu vẫn thiếu, xem
  https://ort.pyke.io phần "Cargo features" / "Linking".
- **API `ort` không khớp** (ví dụ hàm đổi tên/chữ ký giữa các bản rc) →
  báo lỗi cụ thể kèm phiên bản `ort` đang dùng để sửa cho đúng.

Cách lùi nhanh nếu `ort` gây lỗi khi build TOÀN BỘ workspace trên
máy/CI khác: bỏ `rubik-vision-ml` khỏi `members` ở `Cargo.toml` gốc và
thêm lại dòng `[workspace]` rỗng trong `rubik-vision-ml/Cargo.toml` để
cô lập hoàn toàn (khi đó chạy `cd rubik-vision-ml && cargo build` riêng).

## Huấn luyện lại / cải thiện

Toàn bộ script và dữ liệu nằm ở thư mục **`../train_detect/`** đi kèm dự
án. Cần PyTorch + bộ ảnh gốc (Bielefeld) để chạy lại; không cần gì trong
đây nếu chỉ dùng model có sẵn.

| File | Vai trò |
|---|---|
| `prepare_data.py` | Sinh dữ liệu huấn luyện (tăng cường cắt-dán nền) |
| `train.py` | Huấn luyện CNN, xuất `face_detector.onnx` |
| `train_state.pt` | Checkpoint PyTorch (train dở có thể tiếp tục) |
| `face_detector.pt` | Model PyTorch gốc — dùng đối chiếu với bản ONNX |
| `face_detector.onnx` | Bản ONNX xuất ra (giống hệt file trong `models/` của crate này) |
| `train_log.txt` | Nhật ký huấn luyện |
| `*_sanity.png`, `full_frame_test*.png`, `sanity_grid.png` | Ảnh kiểm tra kết quả bằng mắt |

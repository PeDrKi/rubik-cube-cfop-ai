# Cách chạy trên máy bạn

## Cài Rust + Visual C++ Build Tools (Windows)
Xem hướng dẫn đã gửi trước đó nếu chưa cài. Tóm tắt:
1. https://rustup.rs -- cài Rust
2. https://visualstudio.microsoft.com/visual-cpp-build-tools/ -- cài Build
   Tools, chọn workload "Desktop development with C++"
3. Khởi động lại máy

## Chạy
```powershell
cd rubik-render-windowed
cargo run --release
```

## Điều khiển
- **Kéo chuột trái**: xoay góc nhìn.
- **Cuộn chuột**: zoom.
- **U D F B L R**: xoay lớp tương ứng, đúng chiều Singmaster (không-prime).
- **Shift + U/D/F/B/L/R**: nước nghịch đảo (U', D', F', B', L', R').

## Có gì mới so với bản trước
State cube giờ là THẬT (dùng `cube.rs` -- cùng engine đã kiểm chứng khớp
Python trong `rubik-rs`), không còn chỉ animate hình học suông. Mỗi lần
bấm phím và animation chạy xong, `CubeState::do_move()` thật sự được gọi
-- màu các mặt sau khi xoay phản ánh đúng trạng thái cube, xoay nhiều
nước liên tiếp sẽ cộng dồn đúng.

## Việc còn thiếu (biết trước, không phải bug)
- Chưa có phím cho M/E/S, wide (u/d/f/b/l/r), rotation (x/y/z), hay nước
  đúp (2) -- chỉ 12 nước cơ bản (6 mặt x 2 chiều).
- Chưa có nút Scramble / nút gọi solver (`rubik-rs`) để AI tự giải.
- Chưa có bảng công thức / HUD như bản Python (`formula_panel.py`).

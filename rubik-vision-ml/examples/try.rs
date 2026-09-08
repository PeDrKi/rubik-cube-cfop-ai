//! Chạy thử MlDetector trên 1 ảnh thật, KHÔNG đụng gì đến UI/webcam --
//! cô lập việc kiểm tra model chạy đúng khỏi việc nối giao diện, để dễ
//! biết lỗi (nếu có) nằm ở đâu.
//!
//! Cách chạy:
//!   cargo run --example try -- duong/dan/anh.png
//!
//! Kết quả: in ra toạ độ vùng vuông dò được, và LƯU 1 ảnh mới
//! "ket_qua.png" (cùng thư mục chạy lệnh) có vẽ khung đỏ tại vị trí dò
//! được, để bạn xem trực quan bằng mắt.

use image::{Rgb, RgbImage};
use rubik_vision_ml::MlDetector;

fn draw_rect(img: &mut RgbImage, x: u32, y: u32, size: u32, color: Rgb<u8>) {
    let (w, h) = (img.width(), img.height());
    let x1 = (x + size).min(w.saturating_sub(1));
    let y1 = (y + size).min(h.saturating_sub(1));
    for px in x..=x1 {
        if y < h { img.put_pixel(px, y, color); }
        if y1 < h { img.put_pixel(px, y1, color); }
    }
    for py in y..=y1 {
        if x < w { img.put_pixel(x, py, color); }
        if x1 < w { img.put_pixel(x1, py, color); }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("Dùng: cargo run --example try -- duong/dan/anh.png");

    println!("Đang nạp model...");
    let mut detector = MlDetector::load().expect("Không nạp được model ONNX");
    println!("Đã nạp model xong.");

    let img = image::open(&path).expect("Không đọc được ảnh").to_rgb8();
    println!("Ảnh: {}x{}", img.width(), img.height());

    match detector.detect(&img) {
        Some(region) => {
            println!("Dò được vùng: x={}, y={}, size={}", region.x, region.y, region.size);
            let mut out = img.clone();
            draw_rect(&mut out, region.x, region.y, region.size, Rgb([255, 0, 0]));
            out.save("ket_qua.png").expect("Không lưu được ảnh kết quả");
            println!("Đã lưu ket_qua.png -- mở lên xem khung đỏ có đúng quanh mặt cube không.");
        }
        None => println!("Suy luận LỖI (không phải 'không tìm thấy' -- model này luôn ra 1 dự đoán, None nghĩa là có lỗi kỹ thuật khi chạy)."),
    }
}

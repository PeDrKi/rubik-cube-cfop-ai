//! ml_detect.rs — dò vị trí mặt cube bằng 1 CNN nhỏ đã huấn luyện sẵn
//! (xem `/train_detect/` đi kèm dự án để biết cách huấn luyện lại),
//! chạy suy luận qua ONNX Runtime (crate `ort`).
//!
//! Đây là lựa chọn THỬ NGHIỆM, chạy SONG SONG với
//! `detect::detect_cube_face` (heuristic thuần hình học, không cần ML)
//! — không thay thế, để so sánh trực tiếp trong app xem cách nào thực
//! tế tốt hơn trên máy/webcam cụ thể của bạn.
//!
//! ⚠️ YÊU CẦU MÔI TRƯỜNG BUILD: crate `ort` (bản 2.0 release-candidate,
//! bản DUY NHẤT còn trên crates.io tại thời điểm viết — toàn bộ dòng
//! 1.x đã bị gỡ) cần Rust ~1.88 trở lên. Nếu lỗi build ở đây, chạy
//! `rustup update` trước. Phần này KHÔNG được build-test được trong môi
//! trường phát triển ban đầu (cargo quá cũ) — cần bạn tự xác nhận build
//! được trên máy bạn.

use image::RgbImage;
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;

use rubik_vision::sample::SquareRegion;

/// Model nhúng thẳng vào binary lúc compile (~560KB) -- không cần phát
/// hành kèm file rời. Huấn luyện từ bộ ảnh Bielefeld (xem
/// train_detect/README) + tăng cường cắt-dán nền.
const MODEL_BYTES: &[u8] = include_bytes!("../models/face_detector.onnx");
const INPUT_SIZE: u32 = 96;

pub struct MlDetector {
    session: Session,
}

impl MlDetector {
    /// Nạp model 1 LẦN lúc khởi động app (hoặc lần đầu người dùng chọn
    /// chế độ ML) -- việc này có chi phí (dựng session ONNX Runtime),
    /// KHÔNG gọi lại mỗi khung hình.
    pub fn load() -> ort::Result<Self> {
        let session = Session::builder()?.commit_from_memory(MODEL_BYTES)?;
        Ok(Self { session })
    }

    /// Dò vị trí mặt cube trong `img`. Trả về `None` nếu suy luận lỗi
    /// kỹ thuật -- LƯU Ý khác với `detect::detect_cube_face`: model này
    /// LUÔN ra 1 toạ độ dự đoán (không có khái niệm "chắc chắn không
    /// phải cube"), nên `Some` không tự động nghĩa là "chắc chắn đúng"
    /// -- vẫn nên kết hợp với `face_looks_valid` phía UI như đang làm
    /// với `detect_cube_face`.
    pub fn detect(&mut self, img: &RgbImage) -> Option<SquareRegion> {
        let (w, h) = (img.width(), img.height());
        if w == 0 || h == 0 {
            return None;
        }

        // Cắt vuông ở GIỮA ảnh trước khi resize -- khớp với cách dữ
        // liệu huấn luyện được tạo (luôn là 1 vùng VUÔNG cắt thẳng từ
        // ảnh gốc, resize sau). Nếu đưa thẳng ảnh 640x480 (không vuông)
        // vào resize ép về 96x96, ảnh sẽ bị bóp méo tỉ lệ khác hẳn lúc
        // huấn luyện, làm sai lệch không cần thiết.
        let side = w.min(h);
        let off_x = (w - side) / 2;
        let off_y = (h - side) / 2;
        let square = image::imageops::crop_imm(img, off_x, off_y, side, side).to_image();
        let resized = image::imageops::resize(&square, INPUT_SIZE, INPUT_SIZE, image::imageops::FilterType::Triangle);

        let mut input = Array4::<f32>::zeros((1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize));
        for y in 0..INPUT_SIZE {
            for x in 0..INPUT_SIZE {
                let p = resized.get_pixel(x, y);
                for c in 0..3 {
                    input[[0, c, y as usize, x as usize]] = p.0[c] as f32 / 255.0;
                }
            }
        }

        let outputs = self.session.run(ort::inputs!["image" => TensorRef::from_array_view(&input).ok()?]).ok()?;
        let arr = outputs["bbox"].try_extract_array::<f32>().ok()?;
        let (cx, cy, half) = (arr[[0, 0]], arr[[0, 1]], arr[[0, 2]]);

        // cx,cy,half chuẩn hoá 0..1 THEO VÙNG VUÔNG đã cắt -- quy đổi
        // ngược về toạ độ pixel của ẢNH GỐC (cộng lại phần lệch off_x/
        // off_y đã cắt bớt lúc đầu).
        let size_px = ((half * 2.0 * side as f32).max(1.0)) as u32;
        let region_x = off_x + ((cx * side as f32) - size_px as f32 / 2.0).max(0.0) as u32;
        let region_y = off_y + ((cy * side as f32) - size_px as f32 / 2.0).max(0.0) as u32;

        Some(SquareRegion {
            x: region_x.min(w.saturating_sub(1)),
            y: region_y.min(h.saturating_sub(1)),
            size: size_px.min(side).max(1),
        })
    }
}

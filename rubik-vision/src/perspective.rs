//! perspective.rs — hiệu chỉnh PHỐI CẢNH (perspective/homography): khi
//! người dùng chụp mặt cube hơi lệch góc (không vuông góc hoàn toàn với
//! camera), mặt đó xuất hiện trong khung hình như 1 TỨ GIÁC (hình thang)
//! chứ không phải hình vuông thẳng trục -- `sample::sample_grid` giả định
//! sẵn 1 `SquareRegion` thẳng trục nên sẽ lấy mẫu sai lưới 3x3 nếu dùng
//! trực tiếp trên ảnh gốc trong trường hợp này.
//!
//! Cách xử lý: từ 4 góc TỨ GIÁC đã dò được (`detect::Detection::quad`),
//! giải 1 phép biến đổi PHỐI CẢNH (homography 3x3, KHÔNG PHẢI affine —
//! affine không biểu diễn được hiệu ứng "xa thì nhỏ, gần thì to" của phối
//! cảnh thật) ánh xạ hình vuông chuẩn [0,1]x[0,1] sang đúng tứ giác đó,
//! rồi "duỗi thẳng" (`warp_to_square`) tứ giác đó thành 1 ảnh vuông mới
//! bằng lấy mẫu NGƯỢC (inverse mapping) + nội suy song tuyến. Ảnh vuông
//! kết quả dùng lại NGUYÊN VẸN toàn bộ `sample::sample_grid_with_confidence`
//! hiện có (coi như `SquareRegion` phủ trọn ảnh) -- không cần viết lại
//! logic chống loá/lấy mẫu robust đã có, chỉ thêm 1 bước hình học TRƯỚC nó.
//!
//! THUẦN RUST, không dùng thư viện CV nặng — cùng triết lý với `detect.rs`:
//! homography giải bằng khử Gauss thủ công trên hệ 8 ẩn (bài toán "4-point
//! DLT" kinh điển), không cần đại số tuyến tính tổng quát.
//!
//! GIỚI HẠN: 4 góc tứ giác đến từ 1 HEURISTIC (cực trị theo 2 hướng chéo
//! của mặt nạ dò được, xem `detect.rs`) chứ không phải dò góc chính xác
//! từng pixel -- sai số ở bước dò góc sẽ truyền thẳng vào phép duỗi này,
//! không có cơ chế tự sửa ở tầng này. Nếu 4 góc suy biến (gần thẳng
//! hàng/trùng nhau -- ví dụ dò lỗi), `warp_to_square` trả `None` và bên
//! gọi PHẢI lùi về lấy mẫu trực tiếp trên `SquareRegion` (hành vi trước
//! khi có module này), không được coi là lỗi nghiêm trọng.

use image::{Rgb, RgbImage};

/// Ma trận homography 3x3 (hàng-chính), luôn chuẩn hoá `h[2][2] = 1`.
pub type Homography = [[f64; 3]; 3];

/// Giải phép biến đổi phối cảnh (projective) ánh xạ 4 điểm `src` sang 4
/// điểm `dst` tương ứng (ĐÚNG thứ tự, ví dụ TL,TR,BR,BL ở cả 2 phía).
///
/// Đây là bài toán "4-point DLT" kinh điển: 4 cặp điểm cho ĐÚNG 8 phương
/// trình tuyến tính (2 phương trình/điểm) để giải 8 ẩn số (`h11..h32`, cố
/// định `h33=1`) -- không dư, không thiếu, luôn có nghiệm DUY NHẤT trừ
/// khi 4 điểm suy biến (thẳng hàng/trùng nhau, hoặc 3 điểm trở lên thẳng
/// hàng).
///
/// Trả `None` nếu hệ suy biến -- bên gọi PHẢI có phương án dự phòng (xem
/// doc đầu file), không được coi là lỗi nghiêm trọng: dữ liệu góc đến từ
/// 1 heuristic dò ảnh, suy biến hoàn toàn có thể xảy ra với ảnh nhiễu.
pub fn homography_from_points(src: [(f32, f32); 4], dst: [(f32, f32); 4]) -> Option<Homography> {
    // Hệ 8x8 (dạng mở rộng 8x9: 8 cột hệ số + 1 cột vế phải), mỗi điểm
    // góp 2 hàng, suy ra từ:
    //   u = (h11*x + h12*y + h13) / (h31*x + h32*y + 1)
    //   v = (h21*x + h22*y + h23) / (h31*x + h32*y + 1)
    // nhân chéo mẫu số ra để tuyến tính hoá:
    //   h11*x + h12*y + h13 - h31*x*u - h32*y*u = u
    //   h21*x + h22*y + h23 - h31*x*v - h32*y*v = v
    let mut a = [[0f64; 9]; 8];
    for i in 0..4 {
        let (x, y) = (src[i].0 as f64, src[i].1 as f64);
        let (u, v) = (dst[i].0 as f64, dst[i].1 as f64);
        a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u, u];
        a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v, v];
    }
    let h = solve_8x8(a)?;
    Some([[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]])
}

/// Áp dụng homography lên 1 điểm: nhân ma trận rồi chia phối cảnh (chia
/// cho toạ độ đồng nhất thứ 3) -- phép chia này chính là thứ tạo ra hiệu
/// ứng "xa thì nhỏ" mà 1 phép biến đổi affine (không chia) không có được.
pub fn apply_homography(h: &Homography, p: (f32, f32)) -> (f32, f32) {
    let (x, y) = (p.0 as f64, p.1 as f64);
    let w = h[2][0] * x + h[2][1] * y + h[2][2];
    let u = (h[0][0] * x + h[0][1] * y + h[0][2]) / w;
    let v = (h[1][0] * x + h[1][1] * y + h[1][2]) / w;
    (u as f32, v as f32)
}

/// Giải hệ tuyến tính 8 ẩn bằng khử Gauss có CHỌN TRỤC (partial pivoting,
/// chọn hàng có trị tuyệt đối lớn nhất ở mỗi cột trước khi khử) để giảm
/// sai số số học -- `a` là ma trận mở rộng 8x9, bị sửa TẠI CHỖ. Trả
/// `None` nếu không tìm được trục đủ lớn (hệ suy biến).
fn solve_8x8(mut a: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    const N: usize = 8;
    for col in 0..N {
        let mut pivot_row = col;
        let mut pivot_val = a[col][col].abs();
        for r in (col + 1)..N {
            if a[r][col].abs() > pivot_val {
                pivot_val = a[r][col].abs();
                pivot_row = r;
            }
        }
        if pivot_val < 1e-9 {
            return None; // suy biến -- 4 điểm không tạo thành 1 tứ giác hợp lệ
        }
        a.swap(col, pivot_row);

        let inv_pivot = 1.0 / a[col][col];
        for c in col..=N {
            a[col][c] *= inv_pivot;
        }
        for r in 0..N {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            if factor != 0.0 {
                for c in col..=N {
                    a[r][c] -= factor * a[col][c];
                }
            }
        }
    }
    let mut out = [0.0; N];
    for (i, row) in out.iter_mut().enumerate() {
        *row = a[i][N];
    }
    Some(out)
}

/// "Duỗi thẳng" tứ giác `quad` (4 góc, toạ độ pixel ẢNH GỐC, thứ tự BẮT
/// BUỘC TL,TR,BR,BL -- khớp thứ tự `detect::Detection::quad`) trong `img`
/// thành 1 ảnh vuông mới `out_size x out_size` bằng phép biến đổi phối
/// cảnh, thay vì chỉ cắt 1 vùng hình vuông thẳng trục như trước đây.
///
/// Lấy mẫu NGƯỢC (inverse mapping: với MỖI pixel ĐÍCH, tính điểm tương
/// ứng bên ảnh NGUỒN rồi nội suy song tuyến) -- đây là cách chuẩn để
/// tránh lỗ hổng/răng cưa mà lấy mẫu THUẬN (rải từng pixel nguồn sang
/// đích) hay gặp, vì đảm bảo MỌI pixel đích đều được gán đúng 1 giá trị.
///
/// Trả `None` nếu 4 góc suy biến (`homography_from_points` thất bại) --
/// nơi gọi nên lùi về lấy mẫu trực tiếp trên `SquareRegion` thẳng trục
/// như hành vi trước khi có module này, KHÔNG coi là lỗi nghiêm trọng.
pub fn warp_to_square(img: &RgbImage, quad: [(f32, f32); 4], out_size: u32) -> Option<RgbImage> {
    let unit_square = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    // Ánh xạ CẦN DÙNG là "hình vuông chuẩn -> tứ giác" (không phải chiều
    // ngược lại) vì với MỖI điểm trong ảnh ĐÍCH (hình vuông chuẩn hoá),
    // ta cần tìm điểm tương ứng bên ảnh NGUỒN (tứ giác) để lấy mẫu.
    let h = homography_from_points(unit_square, quad)?;
    if out_size == 0 || img.width() == 0 || img.height() == 0 {
        return None;
    }

    let mut out = RgbImage::new(out_size, out_size);
    let (w, h_img) = (img.width() as f32, img.height() as f32);
    for oy in 0..out_size {
        let v = (oy as f32 + 0.5) / out_size as f32;
        for ox in 0..out_size {
            let u = (ox as f32 + 0.5) / out_size as f32;
            let (sx, sy) = apply_homography(&h, (u, v));
            out.put_pixel(ox, oy, bilinear_sample(img, sx, sy, w, h_img));
        }
    }
    Some(out)
}

/// Nội suy song tuyến (bilinear) màu tại toạ độ pixel THỰC (`x`, `y`, có
/// thể lẻ) trong `img` -- toạ độ ngoài biên ảnh được GHIM (clamp) vào mép
/// gần nhất thay vì trả lỗi, vì phép phối cảnh có thể tính ra điểm hơi
/// vượt biên ảnh gốc 1-2 pixel do sai số làm tròn ở chính các góc `quad`.
fn bilinear_sample(img: &RgbImage, x: f32, y: f32, w: f32, h: f32) -> Rgb<u8> {
    let x = x.clamp(0.0, (w - 1.0).max(0.0));
    let y = y.clamp(0.0, (h - 1.0).max(0.0));
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w as u32 - 1);
    let y1 = (y0 + 1).min(h as u32 - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;
    let mut out = [0u8; 3];
    for k in 0..3 {
        let top = p00[k] as f32 * (1.0 - fx) + p10[k] as f32 * fx;
        let bottom = p01[k] as f32 * (1.0 - fx) + p11[k] as f32 * fx;
        out[k] = (top * (1.0 - fy) + bottom * fy).round() as u8;
    }
    Rgb(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT_SQUARE: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

    #[test]
    fn homography_from_points_returns_none_for_degenerate_quad() {
        let dst = [(10.0, 10.0); 4]; // 4 điểm trùng nhau -- suy biến hoàn toàn
        assert!(homography_from_points(UNIT_SQUARE, dst).is_none());
    }

    /// Với 1 tứ giác hình thang THẬT (không phải hình chữ nhật -- cạnh
    /// trên hẹp hơn cạnh dưới, mô phỏng nhìn chếch), homography giải ra
    /// phải ánh xạ ĐÚNG cả 4 góc chuẩn về đúng 4 góc tứ giác đã cho --
    /// đây là ràng buộc TOÁN HỌC bắt buộc của DLT (4 cặp điểm, 8 ẩn số),
    /// không phụ thuộc affine hay không.
    #[test]
    fn homography_from_points_maps_all_four_correspondences_exactly() {
        let quad = [(120.0, 40.0), (280.0, 40.0), (340.0, 260.0), (60.0, 260.0)];
        let h = homography_from_points(UNIT_SQUARE, quad).unwrap();
        for i in 0..4 {
            let (x, y) = apply_homography(&h, UNIT_SQUARE[i]);
            assert!((x - quad[i].0).abs() < 0.01, "góc {i}: x lệch {x} vs {}", quad[i].0);
            assert!((y - quad[i].1).abs() < 0.01, "góc {i}: y lệch {y} vs {}", quad[i].1);
        }
    }

    /// Khi tứ giác truyền vào CHÍNH LÀ 1 hình chữ nhật thẳng trục (không
    /// lệch góc thật), `warp_to_square` phải cho kết quả tương đương crop
    /// + resize thông thường -- bài test "chốt hạ" thứ tự góc TL/TR/BR/BL
    /// không bị đảo lộn, trước khi thử case lệch góc thật sự bên dưới.
    #[test]
    fn warp_to_square_recovers_axis_aligned_crop_when_quad_is_a_rectangle() {
        let mut img = RgbImage::from_pixel(200, 200, Rgb([0, 0, 0]));
        for y in 40..160u32 {
            for x in 40..160u32 {
                let color = match (x >= 100, y >= 100) {
                    (false, false) => Rgb([255, 0, 0]),   // trên-trái
                    (true, false) => Rgb([0, 255, 0]),    // trên-phải
                    (false, true) => Rgb([0, 0, 255]),    // dưới-trái
                    (true, true) => Rgb([255, 255, 0]),   // dưới-phải
                };
                img.put_pixel(x, y, color);
            }
        }
        let quad = [(40.0, 40.0), (160.0, 40.0), (160.0, 160.0), (40.0, 160.0)];
        let out = warp_to_square(&img, quad, 100).expect("quad hợp lệ phải duỗi được");
        let px = |x: u32, y: u32| out.get_pixel(x, y).0;
        assert_eq!(px(25, 25), [255, 0, 0]);
        assert_eq!(px(75, 25), [0, 255, 0]);
        assert_eq!(px(25, 75), [0, 0, 255]);
        assert_eq!(px(75, 75), [255, 255, 0]);
    }

    /// Tứ giác hình thang THẬT (cạnh trên hẹp hơn cạnh dưới) -- mô phỏng
    /// camera nhìn hơi CHẾCH xuống 1 mặt phẳng vuông thật ngoài đời. Khác
    /// test trên: đây là phép chiếu KHÔNG AFFINE (h31/h32 khác 0), bài
    /// test then chốt phân biệt "chỉ xoay/co giãn" với "hiệu chỉnh phối
    /// cảnh thật sự".
    ///
    /// Cách kiểm chứng: "chụp" 1 mặt phẳng có gradient màu đỏ tăng dần
    /// theo trục ngang chuẩn hoá `u` bằng cách RẢI (scatter) nó vào ảnh
    /// nguồn qua ĐÚNG phép chiếu đang muốn kiểm tra (mật độ rải cao hơn
    /// hẳn `out_size` để hạn chế lỗ hổng), rồi gọi `warp_to_square` và so
    /// kênh đỏ đọc lại với giá trị gradient kỳ vọng theo toạ độ `u` của
    /// từng cột — sai lệch phải nhỏ (không đòi khớp tuyệt đối vì có nội
    /// suy song tuyến + lỗ hổng rải).
    #[test]
    fn warp_to_square_undoes_genuine_perspective_skew() {
        let quad = [(120.0, 40.0), (280.0, 40.0), (340.0, 260.0), (60.0, 260.0)];
        let h = homography_from_points(UNIT_SQUARE, quad).unwrap();

        let mut img = RgbImage::from_pixel(400, 320, Rgb([0, 0, 0]));
        let render_res = 240u32;
        for oy in 0..render_res {
            let v = (oy as f32 + 0.5) / render_res as f32;
            for ox in 0..render_res {
                let u = (ox as f32 + 0.5) / render_res as f32;
                let (sx, sy) = apply_homography(&h, (u, v));
                let (sx, sy) = (sx.round(), sy.round());
                if sx >= 0.0 && sy >= 0.0 && (sx as u32) < img.width() && (sy as u32) < img.height() {
                    let r = (u * 255.0).round() as u8;
                    img.put_pixel(sx as u32, sy as u32, Rgb([r, 128, 128]));
                }
            }
        }

        let out = warp_to_square(&img, quad, 60).expect("quad hợp lệ phải duỗi được");
        for ox in [5u32, 30, 55] {
            let u = (ox as f32 + 0.5) / 60.0;
            let expected_r = (u * 255.0).round() as i32;
            let got_r = out.get_pixel(ox, 30).0[0] as i32;
            assert!(
                (got_r - expected_r).abs() < 40,
                "cột {ox}: kênh đỏ sau khi duỗi ({got_r}) lệch quá xa giá trị kỳ vọng theo gradient gốc ({expected_r})"
            );
        }
    }
}

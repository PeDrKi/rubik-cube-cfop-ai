//! detect.rs — tự động dò vị trí mặt cube trong khung hình, thay vì bắt
//! người dùng tự di chuyển cube vào 1 ô vuông cố định.
//!
//! KHÔNG dùng thư viện CV nặng (opencv...) — thuần Rust, thuật toán đơn
//! giản có chủ đích:
//! 1. Tạo mặt nạ "không phải nền tối" (sticker sáng hơn khe/viền đen).
//! 2. GIÃN NỞ (dilate) nhẹ mặt nạ để nối liền khe hở mảnh giữa 9 ô
//!    sticker — nếu không làm bước này, 9 ô sẽ rời rạc thành 9 vùng
//!    liên thông nhỏ riêng biệt thay vì 1 khối, vì bản thân cube có khe
//!    tối phân cách các ô.
//! 3. Gán nhãn vùng liên thông (flood fill), chọn vùng "to vừa phải,
//!    đủ vuông, đủ đặc, gần giữa khung hình" nhất — đó là mặt cube.
//!
//! Đây KHÔNG phải nhận diện chính xác từng viên sticker (không dò lưới
//! 3x3 theo góc) — chỉ định vị 1 hình vuông bao quanh khối màu đó, đủ
//! để thay thế việc canh tay.

use crate::sample::SquareRegion;
use image::RgbImage;

pub struct Detection {
    pub region: SquareRegion,
    /// Tỉ lệ pixel "có màu" (mặt nạ gốc, chưa giãn nở) trong vùng hình
    /// vuông được chọn — càng cao càng đáng tin là 1 mặt cube đặc, thay
    /// vì nhiễu/nền lộn xộn ghép lại tình cờ đủ vuông.
    pub fill_ratio: f32,
    /// Tứ giác ƯỚC LƯỢNG của mặt cube (TL, TR, BR, BL — toạ độ pixel ẢNH
    /// GỐC), dùng để hiệu chỉnh phối cảnh (`perspective::warp_to_square`)
    /// khi mặt cube bị chụp lệch góc, thay vì giả định nó đã vuông góc
    /// hoàn toàn với camera như `region` (hình vuông thẳng trục).
    ///
    /// Suy ra từ 4 điểm CỰC TRỊ của mặt nạ theo 2 hướng chéo (x+y nhỏ
    /// nhất/lớn nhất, x-y nhỏ nhất/lớn nhất) trong đúng hộp bao quanh đã
    /// chọn -- 1 heuristic RẺ, đúng cho tứ giác LỒI bất kỳ (kể cả hình
    /// vuông xoay, hình thang do phối cảnh; với hình chữ nhật thẳng trục
    /// nó cho ra CHÍNH XÁC 4 góc thật, không chỉ gần đúng), nhưng có thể
    /// lệch nếu mặt nạ bị nhiễu/khuyết đúng ngay tại 1 góc thật.
    pub quad: [(f32, f32); 4],
}

/// Hạ ảnh xuống độ phân giải này để dò cho nhanh — không cần chính xác
/// từng pixel, chỉ cần đủ để định vị hình vuông.
const WORK_SIZE: u32 = 220;
/// Ngưỡng sáng tối thiểu để tính là "không phải nền tối / khe cube".
/// Hạ so với bản đầu (55) — webcam thật + ánh sáng phòng thường tối hơn
/// điều kiện lý tưởng, ngưỡng cao dễ khiến cube bị coi là "nền tối".
const BRIGHT_THRESHOLD: u8 = 40;
/// Bán kính giãn nở (đơn vị pixel ở độ phân giải WORK_SIZE) để nối liền
/// khe hở giữa các ô sticker.
const DILATE_RADIUS: i32 = 2;

struct RectF {
    x: f32,
    y: f32,
    size: f32,
}

pub fn detect_cube_face(img: &RgbImage) -> Option<Detection> {
    let (orig_w, orig_h) = (img.width(), img.height());
    if orig_w == 0 || orig_h == 0 {
        return None;
    }
    let scale = WORK_SIZE as f32 / orig_w.max(orig_h) as f32;
    let ww = ((orig_w as f32 * scale).round().max(1.0)) as u32;
    let wh = ((orig_h as f32 * scale).round().max(1.0)) as u32;
    let small = image::imageops::resize(img, ww, wh, image::imageops::FilterType::Triangle);

    let mask: Vec<bool> = small
        .pixels()
        .map(|p| p.0[0].max(p.0[1]).max(p.0[2]) > BRIGHT_THRESHOLD)
        .collect();
    let dilated = dilate(&mask, ww, wh, DILATE_RADIUS);

    let components = label_components(&dilated, ww, wh);
    let total_area = (ww * wh) as f32;
    let center = (ww as f32 / 2.0, wh as f32 / 2.0);
    let diag = ((ww * ww + wh * wh) as f32).sqrt();

    let mut best: Option<(f32, RectF)> = None;
    for c in &components {
        let bw = (c.maxx - c.minx + 1) as f32;
        let bh = (c.maxy - c.miny + 1) as f32;
        let area_frac = (bw * bh) / total_area;
        // Quá nhỏ -> nhiễu. Quá to (~gần hết khung) -> nhiều khả năng là
        // nền sáng đồng nhất (tường trắng...), không phải mặt cube được
        // cầm ở khoảng cách chụp thông thường.
        if !(0.03..=0.85).contains(&area_frac) {
            continue;
        }
        let aspect = bw / bh;
        if !(0.5..=2.0).contains(&aspect) {
            continue; // không đủ vuông
        }

        let cx = (c.minx as f32 + c.maxx as f32) / 2.0;
        let cy = (c.miny as f32 + c.maxy as f32) / 2.0;
        let dist_from_center = ((cx - center.0).powi(2) + (cy - center.1).powi(2)).sqrt() / diag;

        // Ưu tiên: to + đặc (nhiều pixel liên thông so với diện tích hộp
        // bao quanh) + gần giữa khung hình.
        let fill_dilated = c.count as f32 / (bw * bh);
        let score = area_frac * fill_dilated * (1.0 - dist_from_center.min(1.0));

        let side = bw.max(bh);
        let rect = RectF { x: cx - side / 2.0, y: cy - side / 2.0, size: side };
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, rect));
        }
    }

    let (_, rect) = best?;

    // Đo lại độ đặc bằng mặt nạ GỐC (chưa giãn nở) trong đúng vùng vuông
    // đã chọn -- mặt nạ giãn nở dùng để NỐI LIỀN khe hở lúc gán nhãn,
    // nhưng nếu dùng nó để đo độ đặc sẽ luôn ra số ảo cao (vì đã tô lấp
    // khe). Mặt cube thật có khe đen chiếm 1 phần diện tích nên độ đặc
    // thật luôn thấp hơn 1 (~0.5-0.85 tuỳ độ dày khe).
    let x0 = rect.x.max(0.0) as i32;
    let y0 = rect.y.max(0.0) as i32;
    let x1 = ((rect.x + rect.size).min(ww as f32)) as i32;
    let y1 = ((rect.y + rect.size).min(wh as f32)) as i32;
    let mut on = 0u32;
    let mut total = 0u32;
    // Cực trị theo 2 hướng chéo (x+y, x-y) của MẶT NẠ GỐC (chưa giãn nở,
    // giống fill_ratio ở trên) trong đúng hộp bao quanh -- dùng để ước
    // lượng 4 góc TỨ GIÁC thật của mặt cube (xem doc `Detection::quad`),
    // thay vì chỉ trả về hộp bao thẳng trục như trước đây.
    let (mut min_s, mut max_s, mut min_d, mut max_d) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let (mut p_min_s, mut p_max_s, mut p_min_d, mut p_max_d) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32), (0.0f32, 0.0f32), (0.0f32, 0.0f32));
    for y in y0.max(0)..y1.max(0) {
        for x in x0.max(0)..x1.max(0) {
            if x < 0 || y < 0 || x as u32 >= ww || y as u32 >= wh {
                continue;
            }
            total += 1;
            if mask[(y as u32 * ww + x as u32) as usize] {
                on += 1;
                let (xf, yf) = (x as f32, y as f32);
                let (s, d) = (xf + yf, xf - yf);
                if s < min_s {
                    min_s = s;
                    p_min_s = (xf, yf);
                }
                if s > max_s {
                    max_s = s;
                    p_max_s = (xf, yf);
                }
                if d < min_d {
                    min_d = d;
                    p_min_d = (xf, yf);
                }
                if d > max_d {
                    max_d = d;
                    p_max_d = (xf, yf);
                }
            }
        }
    }
    let fill_ratio = if total > 0 { on as f32 / total as f32 } else { 0.0 };
    if fill_ratio < 0.28 {
        return None; // quá rỗng/lởm chởm để tin là 1 mặt cube thật
    }

    let inv = 1.0 / scale;
    let size_px = ((rect.size * inv) as u32).min(orig_w).min(orig_h).max(1);
    let to_orig = |p: (f32, f32)| (p.0 * inv, p.1 * inv);
    Some(Detection {
        region: SquareRegion {
            x: ((rect.x * inv).max(0.0) as u32).min(orig_w.saturating_sub(size_px)),
            y: ((rect.y * inv).max(0.0) as u32).min(orig_h.saturating_sub(size_px)),
            size: size_px,
        },
        fill_ratio,
        // Thứ tự BẮT BUỘC TL,TR,BR,BL (khớp `perspective::warp_to_square`):
        // x+y nhỏ nhất -> trên-trái; x-y lớn nhất -> trên-phải; x+y lớn
        // nhất -> dưới-phải; x-y nhỏ nhất -> dưới-trái.
        quad: [to_orig(p_min_s), to_orig(p_max_d), to_orig(p_max_s), to_orig(p_min_d)],
    })
}

fn dilate(mask: &[bool], w: u32, h: u32, radius: i32) -> Vec<bool> {
    let mut out = vec![false; mask.len()];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let idx = (y as u32 * w + x as u32) as usize;
            if mask[idx] {
                out[idx] = true;
                continue;
            }
            'search: for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    if mask[(ny as u32 * w + nx as u32) as usize] {
                        out[idx] = true;
                        break 'search;
                    }
                }
            }
        }
    }
    out
}

struct Component {
    minx: u32,
    miny: u32,
    maxx: u32,
    maxy: u32,
    count: u32,
}

/// Gán nhãn vùng liên thông 4-hướng bằng flood fill (DFS lặp bằng
/// stack). Đơn giản, đủ nhanh ở độ phân giải WORK_SIZE.
fn label_components(mask: &[bool], w: u32, h: u32) -> Vec<Component> {
    let mut visited = vec![false; mask.len()];
    let mut out = Vec::new();
    let mut stack = Vec::new();

    for start in 0..mask.len() {
        if !mask[start] || visited[start] {
            continue;
        }
        let (mut minx, mut miny) = (u32::MAX, u32::MAX);
        let (mut maxx, mut maxy) = (0u32, 0u32);
        let mut count = 0u32;
        stack.push(start);
        visited[start] = true;
        while let Some(idx) = stack.pop() {
            let x = idx as u32 % w;
            let y = idx as u32 / w;
            minx = minx.min(x);
            miny = miny.min(y);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
            count += 1;
            let neighbors = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
            for (nx, ny) in neighbors {
                if nx >= w || ny >= h {
                    continue;
                }
                let nidx = (ny * w + nx) as usize;
                if mask[nidx] && !visited[nidx] {
                    visited[nidx] = true;
                    stack.push(nidx);
                }
            }
        }
        out.push(Component { minx, miny, maxx, maxy, count });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    /// Vẽ 1 ảnh mô phỏng: nền tối lộn xộn + 1 mặt cube (khối 3x3 ô màu,
    /// có khe đen mảnh giữa các ô) đặt lệch tâm — kiểm tra detect có tìm
    /// đúng vị trí/kích thước khối đó không, KHÔNG phải khoanh nhầm nền.
    fn synth_scene(face_x: u32, face_y: u32, face_size: u32) -> RgbImage {
        let mut img = RgbImage::from_pixel(500, 400, Rgb([25, 25, 25])); // nền tối
        // Vài đốm sáng nhỏ rải rác trên nền (nhiễu, không phải cube).
        for (nx, ny) in [(30, 30), (450, 40), (20, 350), (470, 370)] {
            for y in ny..(ny + 12).min(400) {
                for x in nx..(nx + 12).min(500) {
                    img.put_pixel(x, y, Rgb([90, 90, 90]));
                }
            }
        }

        let cell = face_size / 3;
        let colors: [[u8; 3]; 9] = [
            [230, 30, 30], [30, 180, 60], [230, 220, 20],
            [30, 90, 220], [230, 230, 230], [230, 140, 20],
            [230, 30, 30], [30, 180, 60], [230, 220, 20],
        ];
        let gap = (cell / 12).max(1); // khe đen mảnh giữa các ô
        for row in 0..3u32 {
            for col in 0..3u32 {
                let c = colors[(row * 3 + col) as usize];
                let x0 = face_x + col * cell + gap;
                let y0 = face_y + row * cell + gap;
                let x1 = face_x + (col + 1) * cell - gap;
                let y1 = face_y + (row + 1) * cell - gap;
                for y in y0..y1.min(400) {
                    for x in x0..x1.min(500) {
                        img.put_pixel(x, y, Rgb(c));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn detects_off_center_face_close_to_true_bounds() {
        let (fx, fy, fsize) = (140, 90, 220);
        let img = synth_scene(fx, fy, fsize);
        let d = detect_cube_face(&img).expect("phải phát hiện được mặt cube");

        let cx = d.region.x as f32 + d.region.size as f32 / 2.0;
        let cy = d.region.y as f32 + d.region.size as f32 / 2.0;
        let true_cx = fx as f32 + fsize as f32 / 2.0;
        let true_cy = fy as f32 + fsize as f32 / 2.0;

        // Cho phép sai số vì đã hạ độ phân giải khi dò -- chỉ cần tâm và
        // kích thước gần đúng, không cần khớp pixel-chính-xác.
        assert!((cx - true_cx).abs() < 25.0, "lệch tâm X quá nhiều: {cx} vs {true_cx}");
        assert!((cy - true_cy).abs() < 25.0, "lệch tâm Y quá nhiều: {cy} vs {true_cy}");
        assert!(
            (d.region.size as f32 - fsize as f32).abs() < 40.0,
            "kích thước lệch quá nhiều: {} vs {fsize}",
            d.region.size
        );
        assert!(d.fill_ratio > 0.4, "fill_ratio thấp bất thường: {}", d.fill_ratio);
    }

    /// Với 1 mặt cube chụp THẲNG (không xoay/lệch góc), 4 góc `quad` ước
    /// lượng phải gần đúng CHÍNH XÁC 4 góc thật của hình vuông đó -- bài
    /// test đối chứng cho case dễ, trước khi tin heuristic này dùng được
    /// cho case xoay/phối cảnh thật (đã kiểm ở `perspective.rs`, module
    /// đó chỉ kiểm chứng phần TOÁN của việc duỗi tứ giác, không kiểm việc
    /// DÒ ra tứ giác đúng từ ảnh — đó là việc của test này).
    #[test]
    fn detected_quad_corners_are_close_to_true_axis_aligned_corners_for_a_straight_face() {
        let (fx, fy, fsize) = (140, 90, 220);
        let img = synth_scene(fx, fy, fsize);
        let d = detect_cube_face(&img).expect("phải phát hiện được mặt cube");
        let expected = [
            (fx as f32, fy as f32),
            ((fx + fsize) as f32, fy as f32),
            ((fx + fsize) as f32, (fy + fsize) as f32),
            (fx as f32, (fy + fsize) as f32),
        ];
        for (i, (ex, ey)) in expected.iter().enumerate() {
            let (gx, gy) = d.quad[i];
            assert!((gx - ex).abs() < 40.0, "góc {i}: lệch X quá nhiều: {gx} vs {ex}");
            assert!((gy - ey).abs() < 40.0, "góc {i}: lệch Y quá nhiều: {gy} vs {ey}");
        }
    }

    #[test]
    fn returns_none_on_uniform_dark_scene() {
        let img = RgbImage::from_pixel(300, 300, Rgb([10, 10, 10]));
        assert!(detect_cube_face(&img).is_none());
    }
}

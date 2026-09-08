//! sample.rs — lấy màu trung bình của 9 ô lưới 3x3 từ 1 vùng vuông trên
//! ảnh. Dùng chung cho cả luồng webcam (frame trực tiếp) và luồng nạp
//! ảnh file — cả hai quy về `image::RgbImage` trước khi tới đây.
//!
//! Người dùng khoanh 1 vùng vuông (qua overlay lưới trên UI) áp khít vào
//! mặt cube trong khung hình; hàm này KHÔNG tự dò cube trong ảnh (không
//! có transform phối cảnh) — đơn giản, đủ dùng khi có overlay canh sẵn,
//! và không cần thêm thư viện CV nặng.
//!
//! CHỐNG LOÁ/PHẢN CHIẾU: sticker nhựa bóng rất hay dính 1 đốm phản chiếu
//! đèn/cửa sổ ngay giữa ô (đúng chỗ hay lấy mẫu nhất). 1 đốm cháy sáng
//! nhỏ nhưng gần trắng tuyệt đối (255,255,255) đủ sức kéo TRUNG BÌNH
//! cộng đơn giản lệch hẳn về phía trắng — đây là nguyên nhân chính gây
//! nhầm trắng/vàng và đỏ/cam (kênh đỏ thường cháy trước các kênh khác
//! trên cảm biến, làm đỏ ngả sang cam hoặc hồng nhạt). Hai lớp phòng thủ
//! dưới đây độc lập với nhau:
//! 1. `trimmed_mean_patch`: loại pixel "cháy sáng" (chạm trần) trước khi
//!    lấy trung bình 1 patch.
//! 2. `sample_grid`: lấy nhiều patch con rải rác quanh tâm ô rồi lấy
//!    TRUNG VỊ giữa chúng — loá thường chỉ phủ 1 phần cục bộ của ô, nên
//!    trung vị của nhiều patch con sẽ tự động bỏ qua patch nào bị lóa
//!    trọn vẹn mà (1) không cần dò lóa tường minh ở bước này và (2) vẫn
//!    đúng khi CẢ ô thực sự là màu rất sáng (trắng thật).
//!
//! ĐỘ TIN CẬY: 2 lớp phòng thủ trên chỉ CHỊU ĐỰNG được nhiễu, không loại
//! bỏ hoàn toàn -- 1 ô bị loá nặng (nhiều patch con cùng dính, hoặc dính
//! trên diện rộng) vẫn có thể ra màu hơi lệch dù đã qua 2 lớp lọc.
//! `sample_grid_with_confidence` lượng hoá mức "hơi lệch" này thành 1 số
//! (0,1] dựa trên (a) tỉ lệ pixel cháy sáng trung bình và (b) mức 5 patch
//! con ĐỒNG THUẬN với nhau tới đâu (patch con nào bị loá cục bộ sẽ lệch
//! khỏi trung vị của cả nhóm, dù trung vị vẫn đúng). Bên gọi (thuật toán
//! phân công màu ở `color.rs`) dùng số này làm TRỌNG SỐ: ô càng ít tin
//! cậy càng "rẻ" để đổi nhãn khi cần dung hoà ràng buộc 6 màu x 9 ô, ưu
//! tiên giữ nguyên (những) ô đo chắc chắn.

use image::RgbImage;

/// Vùng vuông tính bằng pixel trong ảnh gốc.
#[derive(Clone, Copy, Debug)]
pub struct SquareRegion {
    pub x: u32,
    pub y: u32,
    pub size: u32,
}

/// Số patch con lấy mẫu trong MỖI ô: tâm + 4 điểm lệch nhẹ theo 4 hướng
/// chéo, vẫn nằm trong vùng 30% giữa ô (không chạm rãnh đen/mép sticker).
const SUB_PATCH_OFFSETS: [(i64, i64); 5] = [(0, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)];

/// Lấy 9 màu trung bình (row-major, giống thứ tự facelet 0..9 của
/// `cube.rs`) từ vùng `region` trong `img`. Wrapper mỏng quanh
/// `sample_grid_with_confidence` cho nơi gọi không cần tới độ tin cậy
/// (vd test, hoặc nơi chưa muốn đổi API).
pub fn sample_grid(img: &RgbImage, region: SquareRegion) -> [[u8; 3]; 9] {
    sample_grid_with_confidence(img, region).0
}

/// Như `sample_grid`, nhưng trả kèm ĐỘ TIN CẬY mỗi ô (0,1] -- xem doc
/// đầu file. Đây là hàm nên dùng khi lấy mẫu từ webcam/ảnh thật (nơi
/// loá/nhiễu có thể xảy ra); `RawFace::from_samples` (tin cậy tuyệt đối
/// mặc định) chỉ phù hợp cho dữ liệu đã biết chắc chắn đúng.
pub fn sample_grid_with_confidence(img: &RgbImage, region: SquareRegion) -> ([[u8; 3]; 9], [f32; 9]) {
    let cell = region.size as f32 / 3.0;
    // patch lấy mẫu = 30% kích thước ô, canh giữa ô.
    let patch = (cell * 0.3).max(1.0) as i64;
    // lệch patch con = 1/2 bán kính patch -- đủ để mỗi patch con nhìn
    // vào 1 vùng hơi khác nhau (tăng cơ hội né đốm loá cục bộ) nhưng vẫn
    // nằm trọn trong vùng 30% an toàn giữa ô.
    let sub_offset = ((patch as f32) * 0.5).round() as i64;

    let mut colors = [[0u8; 3]; 9];
    let mut confidence = [1.0f32; 9];
    for row in 0..3 {
        for col in 0..3 {
            let cx = region.x as f32 + cell * (col as f32 + 0.5);
            let cy = region.y as f32 + cell * (row as f32 + 0.5);
            let (color, conf) = robust_cell_color(img, cx as i64, cy as i64, patch, sub_offset);
            colors[row * 3 + col] = color;
            confidence[row * 3 + col] = conf;
        }
    }
    (colors, confidence)
}

/// Trọng số trộn 2 tín hiệu mất-tin-cậy (tỉ lệ cháy sáng, độ bất đồng
/// giữa các patch con) thành 1 điểm số duy nhất -- lựa chọn thủ công,
/// không có "đúng tuyệt đối"; chỉ cần đơn điệu và không để 1 tín hiệu
/// lấn át hoàn toàn tín hiệu kia.
const CLIP_WEIGHT: f32 = 0.6;
const SPREAD_WEIGHT: f32 = 0.4;
/// Khoảng cách kênh màu (0..255) coi là "bất đồng đáng kể" giữa các
/// patch con, dùng để chuẩn hoá độ lệch (spread) về thang 0..1.
const SPREAD_SCALE: f32 = 80.0;
/// Độ tin cậy không bao giờ chạm 0 tuyệt đối -- 1 ô dù đo rất nhiễu vẫn
/// giữ lại 1 phần ảnh hưởng nhỏ trong bài toán phân công màu, tránh
/// trường hợp "gán màu nào cũng coi như miễn phí" gây bất ổn.
const MIN_CONFIDENCE: f32 = 0.05;

/// Màu đại diện của 1 ô + độ tin cậy: lấy trung bình chống-cháy-sáng ở 5
/// patch con rải quanh tâm ô, lấy TRUNG VỊ từng kênh giữa 5 kết quả đó
/// làm màu cuối (chịu được tới 2/5 patch bị hỏng), đồng thời tính độ tin
/// cậy từ tỉ lệ cháy sáng trung bình và độ lệch của 5 patch con so với
/// trung vị đó (patch nào bị loá cục bộ dù đã lọc vẫn có xu hướng lệch
/// khỏi số đông).
fn robust_cell_color(img: &RgbImage, cx: i64, cy: i64, half: i64, sub_offset: i64) -> ([u8; 3], f32) {
    let half = half.max(1);
    let mut rs = [0u8; 5];
    let mut gs = [0u8; 5];
    let mut bs = [0u8; 5];
    let mut clip_fracs = [0f32; 5];
    for (i, (dx, dy)) in SUB_PATCH_OFFSETS.iter().enumerate() {
        let ([r, g, b], clip_frac) = trimmed_mean_patch(img, cx + dx * sub_offset, cy + dy * sub_offset, half);
        rs[i] = r;
        gs[i] = g;
        bs[i] = b;
        clip_fracs[i] = clip_frac;
    }
    let color = [median5(rs), median5(gs), median5(bs)];

    let avg_clip = clip_fracs.iter().sum::<f32>() / clip_fracs.len() as f32;
    let spread = (0..5)
        .map(|i| {
            let d = [rs[i] as f32 - color[0] as f32, gs[i] as f32 - color[1] as f32, bs[i] as f32 - color[2] as f32];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        })
        .fold(0f32, f32::max);

    let confidence = 1.0 - CLIP_WEIGHT * avg_clip.clamp(0.0, 1.0) - SPREAD_WEIGHT * (spread / SPREAD_SCALE).clamp(0.0, 1.0);
    (color, confidence.clamp(MIN_CONFIDENCE, 1.0))
}

fn median5(mut v: [u8; 5]) -> u8 {
    v.sort_unstable();
    v[2]
}

/// Ngưỡng "cháy sáng": kênh nào đó chạm gần trần 255 nghĩa là cảm biến
/// đã bão hoà tại điểm đó, giá trị không còn phản ánh màu thật của bề
/// mặt (thường do phản chiếu specular của đèn/cửa sổ trên nhựa bóng).
const CLIP_THRESHOLD: u8 = 248;

/// Trung bình 1 patch NHƯNG loại các pixel cháy sáng trước khi lấy trung
/// bình, trả kèm TỈ LỆ pixel đã bị loại (0..1, dùng để tính độ tin cậy ở
/// `robust_cell_color`). Nếu TOÀN BỘ (hoặc gần hết) patch đều cháy sáng
/// thì không loại gì cả — trường hợp đó nhiều khả năng ô thật sự là màu
/// rất sáng (trắng) dưới đèn mạnh, loại hết sẽ mất luôn dữ liệu hữu ích
/// (tỉ lệ trả về vẫn phản ánh đúng mức cháy sáng thật, chỉ không loại bỏ
/// pixel khỏi phép tính trung bình).
fn trimmed_mean_patch(img: &RgbImage, cx: i64, cy: i64, half: i64) -> ([u8; 3], f32) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut pixels: Vec<[u8; 3]> = Vec::new();
    for dy in -half..=half {
        for dx in -half..=half {
            let (px, py) = (cx + dx, cy + dy);
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            let p = img.get_pixel(px as u32, py as u32);
            pixels.push([p[0], p[1], p[2]]);
        }
    }
    if pixels.is_empty() {
        return ([128, 128, 128], 1.0); // vùng ngoài ảnh (region khoanh lệch hoặc patch con lệch ra ngoài) -> xám trung tính, tin cậy thấp nhất có thể
    }

    let clipped = pixels.iter().filter(|p| p[0] >= CLIP_THRESHOLD || p[1] >= CLIP_THRESHOLD || p[2] >= CLIP_THRESHOLD).count();
    let clip_frac = clipped as f32 / pixels.len() as f32;
    // >= 2/3 pixel cháy sáng -> coi như cả patch thật sự sáng (trắng
    // thật), giữ nguyên toàn bộ thay vì loại hết còn lại quá ít mẫu.
    let keep_all = clipped * 3 >= pixels.len() * 2;

    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for p in &pixels {
        let is_clipped = p[0] >= CLIP_THRESHOLD || p[1] >= CLIP_THRESHOLD || p[2] >= CLIP_THRESHOLD;
        if is_clipped && !keep_all {
            continue; // bỏ pixel cháy sáng cục bộ (loá/phản chiếu), không tính vào trung bình
        }
        r += p[0] as u64;
        g += p[1] as u64;
        b += p[2] as u64;
        n += 1;
    }
    if n == 0 {
        return ([128, 128, 128], 1.0);
    }
    ([(r / n) as u8, (g / n) as u8, (b / n) as u8], clip_frac)
}

/// Trọng số cho ĐỘ LỆCH GIỮA CÁC KHUNG HÌNH khi gộp nhiều khung hình
/// (khác với `SPREAD_WEIGHT` ở trên -- đó là lệch giữa các patch con
/// TRONG CÙNG 1 khung). Dùng ở `aggregate_frames`.
const TEMPORAL_SPREAD_WEIGHT: f32 = 0.5;
/// Thang chuẩn hoá độ lệch giữa các khung hình -- hẹp hơn `SPREAD_SCALE`
/// vì phần lệch còn lại ở tầng này (sau khi mỗi khung đã tự chống-loá
/// riêng) chủ yếu chỉ là nhiễu cảm biến/rung sáng nhỏ giữa các khung liên
/// tiếp, không phải do framing/góc chụp khác nhau như giữa các patch con.
const TEMPORAL_SPREAD_SCALE: f32 = 40.0;

/// Gộp NHIỀU khung hình đã lấy mẫu (`sample_grid_with_confidence`) của
/// CÙNG 1 mặt, chụp liên tiếp trong lúc người dùng giữ cube yên (xem
/// `scan_ui.rs::frame_history`), thành 1 kết quả duy nhất mỗi ô sticker:
///
/// - Màu: trung bình có TRỌNG SỐ theo độ tin cậy từng khung -- khung đang
///   bị loá/nhiễu đóng góp ít hơn vào kết quả cuối so với khung sạch, thay
///   vì trước đây chỉ dùng đúng 1 khung hình cuối cùng lúc bấm chụp. Giảm
///   ảnh hưởng của nhiễu cảm biến/rung sáng NGẪU NHIÊN đổi theo từng khung
///   (trung bình N khung độc lập giảm nhiễu ngẫu nhiên, dù không đổi được
///   sai lệch CÓ HỆ THỐNG lặp lại ở mọi khung, ví dụ lệch màu do đèn).
/// - Độ tin cậy: bằng tin cậy trung bình của các khung, GIẢM THÊM nếu màu
///   đọc được giữa các khung tự nó không đồng thuận (khung nào out-lier so
///   với trung bình chung là dấu hiệu có nhiễu thoáng qua giữa chừng, dù
///   từng khung riêng lẻ có thể không tự phát hiện ra). Cố tình KHÔNG bao
///   giờ tăng tin cậy vượt quá mức trung bình của các khung đơn lẻ --
///   nhiều khung đồng thuận chỉ được coi là "không có bằng chứng thêm về
///   sai sót", không phải bằng chứng dương tính về độ chính xác tuyệt đối.
///
/// Gọi với slice RỖNG là lỗi logic ở nơi gọi (`debug_assert`); ở bản
/// release trả về màu xám trung tính + tin cậy thấp nhất thay vì panic.
pub fn aggregate_frames(frames: &[([[u8; 3]; 9], [f32; 9])]) -> ([[u8; 3]; 9], [f32; 9]) {
    debug_assert!(!frames.is_empty(), "aggregate_frames không được gọi với slice rỗng");
    if frames.is_empty() {
        return ([[128, 128, 128]; 9], [MIN_CONFIDENCE; 9]);
    }
    if frames.len() == 1 {
        return frames[0];
    }

    let mut colors = [[0u8; 3]; 9];
    let mut confidence = [0f32; 9];
    for cell in 0..9 {
        let weight_sum: f32 = frames.iter().map(|(_, c)| c[cell]).sum();
        let mean = if weight_sum > 1e-6 {
            let mut acc = [0f32; 3];
            for (samples, conf) in frames {
                let w = conf[cell];
                for k in 0..3 {
                    acc[k] += samples[cell][k] as f32 * w;
                }
            }
            [acc[0] / weight_sum, acc[1] / weight_sum, acc[2] / weight_sum]
        } else {
            // Mọi khung đều gần như 0 tin cậy -- trọng số hoá vô nghĩa,
            // lùi về trung bình cộng thô thay vì chia cho ~0.
            let mut acc = [0f32; 3];
            for (samples, _) in frames {
                for k in 0..3 {
                    acc[k] += samples[cell][k] as f32;
                }
            }
            let n = frames.len() as f32;
            [acc[0] / n, acc[1] / n, acc[2] / n]
        };
        colors[cell] = [mean[0].round() as u8, mean[1].round() as u8, mean[2].round() as u8];

        let spread = frames
            .iter()
            .map(|(samples, _)| {
                let d = [
                    samples[cell][0] as f32 - mean[0],
                    samples[cell][1] as f32 - mean[1],
                    samples[cell][2] as f32 - mean[2],
                ];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            })
            .fold(0f32, f32::max);

        let avg_frame_conf = frames.iter().map(|(_, c)| c[cell]).sum::<f32>() / frames.len() as f32;
        let temporal_factor = 1.0 - TEMPORAL_SPREAD_WEIGHT * (spread / TEMPORAL_SPREAD_SCALE).clamp(0.0, 1.0);
        confidence[cell] = (avg_frame_conf * temporal_factor).clamp(MIN_CONFIDENCE, 1.0);
    }
    (colors, confidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn samples_center_of_each_cell() {
        let mut img = RgbImage::new(90, 90);
        // Tô 9 ô 30x30 với 9 màu khác nhau.
        for row in 0..3u32 {
            for col in 0..3u32 {
                let v = (row * 3 + col) as u8 * 25;
                for y in (row * 30)..(row * 30 + 30) {
                    for x in (col * 30)..(col * 30 + 30) {
                        img.put_pixel(x, y, image::Rgb([v, v, v]));
                    }
                }
            }
        }
        let out = sample_grid(&img, SquareRegion { x: 0, y: 0, size: 90 });
        for row in 0..3usize {
            for col in 0..3usize {
                let expected = (row * 3 + col) as u8 * 25;
                assert_eq!(out[row * 3 + col], [expected; 3]);
            }
        }
    }

    /// Sticker màu đỏ thật, nhưng dính 1 đốm phản chiếu trắng cháy sáng
    /// (255,255,255) ngay tại tâm ô (đúng chỗ patch trung tâm lấy mẫu).
    /// Nếu chỉ lấy trung bình cộng thô, đốm này đủ sức kéo màu đọc được
    /// lệch hẳn về phía trắng/hồng nhạt (dễ nhầm với cam) — sample_grid
    /// phải trả về gần đúng màu đỏ thật nhờ 4 patch con còn lại (và cơ
    /// chế loại pixel cháy sáng bên trong patch trung tâm) không bị ảnh
    /// hưởng bởi đốm loá cục bộ đó.
    #[test]
    fn ignores_localized_glare_spot_at_cell_center() {
        let mut img = RgbImage::from_pixel(90, 90, Rgb([230, 20, 20])); // đỏ thật phủ toàn bộ ảnh
        // Đốm loá nhỏ ở giữa ẢNH (x,y: 40..50) -- với lưới 3x3 trên vùng
        // 90x90 (mỗi ô 30x30), toạ độ này rơi đúng vào Ô GIỮA (hàng/cột 1
        // = [30,60)) tức index 4, KHÔNG PHẢI index 0 (đã sửa lỗi test cũ:
        // trước đây kiểm tra nhầm out[0] -- ô đó chưa từng dính loá nên
        // luôn đúng dù không thực sự kiểm chứng được cơ chế chống loá).
        for y in 40..50u32 {
            for x in 40..50u32 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let out = sample_grid(&img, SquareRegion { x: 0, y: 0, size: 90 });
        let [r, g, b] = out[4];
        assert!(r > 200 && g < 60 && b < 60, "màu đọc được lệch quá xa màu đỏ thật do loá: {:?}", out[4]);
    }

    /// Ô dính loá (dù màu đọc được đã được sửa gần đúng nhờ 2 lớp phòng
    /// thủ) vẫn phải bị đánh giá tin cậy THẤP HƠN 1 ô hoàn toàn sạch --
    /// đây là tín hiệu mà `color::balanced_assign` dùng để biết nên "hy
    /// sinh" ô nào trước khi cần đổi nhãn để thoả ràng buộc 6 màu x 9 ô.
    #[test]
    fn glare_affected_cell_gets_lower_confidence_than_clean_cell() {
        let mut img = RgbImage::from_pixel(90, 90, Rgb([230, 20, 20]));
        for y in 40..50u32 {
            for x in 40..50u32 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let (_, conf) = sample_grid_with_confidence(&img, SquareRegion { x: 0, y: 0, size: 90 });
        assert_eq!(conf[0], 1.0, "ô hoàn toàn sạch (không dính loá) phải tin cậy tuyệt đối, được: {}", conf[0]);
        assert!(
            conf[4] < conf[0],
            "ô dính loá (idx 4, conf={}) phải tin cậy THẤP HƠN ô sạch (idx 0, conf={})",
            conf[4],
            conf[0]
        );
    }

    /// Sticker trắng THẬT dưới đèn mạnh (toàn bộ ô cháy sáng, không chỉ
    /// 1 đốm cục bộ) vẫn phải đọc ra gần trắng, KHÔNG bị hàm chống-loá xử
    /// lý nhầm thành "mất dữ liệu" rồi trả về màu rác.
    #[test]
    fn keeps_genuinely_white_sticker_under_bright_light() {
        let img = RgbImage::from_pixel(90, 90, Rgb([255, 255, 255]));
        let out = sample_grid(&img, SquareRegion { x: 0, y: 0, size: 90 });
        assert_eq!(out[0], [255, 255, 255]);
    }

    #[test]
    fn aggregate_frames_with_a_single_frame_returns_it_unchanged() {
        let frame = ([[100u8, 100, 100]; 9], [0.7f32; 9]);
        let out = aggregate_frames(&[frame]);
        assert_eq!(out, frame);
    }

    /// 3 khung hình đọc cùng 1 ô nhưng lệch nhẹ ngẫu nhiên quanh màu thật
    /// (nhiễu cảm biến/rung sáng điển hình) -- trung bình có trọng số phải
    /// kéo về rất gần màu thật, gần hơn hẳn so với việc chỉ dùng 1 khung
    /// đơn lẻ bất kỳ trong 3 khung đó.
    #[test]
    fn aggregate_frames_averages_out_random_per_frame_noise() {
        let frames = [
            ([[100u8, 100, 100]; 9], [1.0f32; 9]),
            ([[112u8, 90, 102]; 9], [1.0f32; 9]),
            ([[88u8, 110, 98]; 9], [1.0f32; 9]),
        ];
        let (colors, _) = aggregate_frames(&frames);
        for ch in 0..3 {
            let v = colors[0][ch] as i32;
            assert!((v - 100).abs() <= 2, "kênh {ch} lệch quá xa 100 sau khi gộp: {v}");
        }
    }

    /// Khung hình có tin cậy THẤP (ví dụ đang dính loá thoáng qua) phải
    /// đóng góp ÍT hơn vào màu trung bình so với khung có tin cậy cao --
    /// nếu không trọng số hoá, kết quả sẽ lệch đều 50/50 về phía khung xấu.
    #[test]
    fn aggregate_frames_weighs_low_confidence_frame_less() {
        let good = ([[100u8, 100, 100]; 9], [1.0f32; 9]);
        let bad = ([[200u8, 100, 100]; 9], [0.05f32; 9]); // gần như bỏ qua
        let (colors, _) = aggregate_frames(&[good, bad]);
        let unweighted_mean = 150i32;
        assert!(
            (colors[0][0] as i32 - 100).abs() < (colors[0][0] as i32 - unweighted_mean).abs(),
            "màu gộp phải lệch về phía khung tin cậy cao hơn, được: {:?}",
            colors[0]
        );
        assert!(colors[0][0] < 120, "khung tin cậy thấp phải gần như không kéo được màu, được: {:?}", colors[0]);
    }

    /// 2 khung hình ĐỒNG THUẬN (cùng đọc ra gần như 1 màu) phải cho ra kết
    /// quả tin cậy CAO HƠN so với 2 khung BẤT ĐỒNG (đọc ra 2 màu khác hẳn
    /// nhau dù tin cậy riêng từng khung như nhau) -- bất đồng giữa các
    /// khung là dấu hiệu có nhiễu thoáng qua mà bản thân từng khung không
    /// tự phát hiện được.
    #[test]
    fn aggregate_frames_penalizes_disagreement_between_frames() {
        let agreeing = [([[100u8, 100, 100]; 9], [0.9f32; 9]), ([[102u8, 99, 101]; 9], [0.9f32; 9])];
        let disagreeing = [([[80u8, 100, 100]; 9], [0.9f32; 9]), ([[160u8, 100, 100]; 9], [0.9f32; 9])];
        let (_, conf_agree) = aggregate_frames(&agreeing);
        let (_, conf_disagree) = aggregate_frames(&disagreeing);
        assert!(
            conf_disagree[0] < conf_agree[0],
            "tin cậy khi bất đồng ({}) phải THẤP HƠN khi đồng thuận ({})",
            conf_disagree[0],
            conf_agree[0]
        );
    }
}

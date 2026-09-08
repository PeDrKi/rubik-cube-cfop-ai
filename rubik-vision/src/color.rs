//! color.rs — phân loại 54 ô sticker thành 6 nhóm màu, KHÔNG giả định
//! tên màu cụ thể (trắng/vàng/...). Ô tâm (index 4) của mỗi mặt trong
//! `RawScan` được coi là "màu chuẩn" của mặt đó vì tâm không bao giờ đổi
//! vị trí khi xoay cube — đây là điểm neo duy nhất cần thiết.
//!
//! VẤN ĐỀ ÁNH SÁNG KHÁC NHAU GIỮA CÁC MẶT: 6 mặt được chụp thành 6 ảnh
//! RIÊNG BIỆT (người dùng xoay/lật cube giữa các lần chụp), nên rất dễ
//! mỗi ảnh có góc chiếu sáng hơi khác nhau (đổ bóng, ám màu đèn...). Nếu
//! chỉ tin tuyệt đối 1 ô tâm của MỘT ảnh làm "màu chuẩn" cho cả mặt đó
//! rồi so trực tiếp với ô tâm của ảnh KHÁC (chụp dưới ánh sáng khác),
//! chênh lệch ánh sáng dễ khiến 2 màu vốn đã gần nhau trong không gian
//! Lab (trắng/vàng, đỏ/cam) đổi chỗ "gần nhất". `refine_centers` khắc
//! phục bằng cách coi 6 ô tâm chỉ là điểm khởi tạo, rồi dùng chính 54 ô
//! đã lấy mẫu (trải trên cả 6 ảnh) để "biểu quyết" lại vị trí thật của
//! mỗi nhóm màu trong Lab — nếu 1 ô tâm bị lệch do ánh sáng cục bộ của
//! riêng ảnh đó, các ô cùng màu xuất hiện rải rác ở NHỮNG MẶT KHÁC (chụp
//! dưới ánh sáng khác) sẽ kéo tâm màu đó về đúng vị trí trung bình thật.
//!
//! RÀNG BUỘC "6 MÀU, MỖI MÀU ĐÚNG 9 Ô": 1 khối Rubik 3x3 CHỈ có 6 màu,
//! mỗi màu phủ đúng 1 mặt = đúng 9 ô trên toàn bộ 54 ô — đây là ràng
//! buộc CỨNG mà mọi bức ảnh chụp đúng đều phải thoả, mạnh hơn nhiều so
//! với "chọn tâm gần nhất cho từng ô riêng lẻ" (cách cũ): xét riêng lẻ,
//! 1-2 ô ở ranh giới 2 màu (do loá/ánh sáng) có thể bị chọn nhầm, kéo
//! theo 1 màu bị đếm dư (vd 10 ô "trắng") và màu kia bị thiếu (vd 8 ô
//! "vàng") — điều KHÔNG THỂ xảy ra trên khối thật, và chắc chắn làm bước
//! ghép cubie ở `scan.rs` thất bại. `balanced_assign` giải bài toán phân
//! công cân bằng (balanced assignment) bằng thuật toán Hungarian, ép
//! ĐÚNG 9 ô cho mỗi màu trong khi vẫn tối thiểu hoá tổng khoảng cách Lab
//! trên toàn cục — (những) ô mơ hồ nằm ở ranh giới sẽ tự động được xếp
//! vào màu nào giúp tổng chi phí nhỏ nhất mà vẫn giữ đúng số lượng.

use crate::scan::RawScan;

/// Kết quả phân loại: với mỗi mặt đã chụp (0..6, đúng thứ tự chụp), 9 ô
/// -> chỉ số mặt (0..6) của màu gần nhất. Chưa áp dụng hướng xoay đúng.
pub type ClassifiedFaces = [[u8; 9]; 6];

/// sRGB (0..255) -> CIE Lab. Dùng Lab thay vì RGB thô vì khoảng cách
/// trong Lab gần với cảm nhận mắt người hơn, ít bị lệch bởi cường độ
/// sáng chung của cả bức ảnh (vd 1 mặt chụp dưới đèn vàng hơn mặt khác).
fn srgb_to_lab([r, g, b]: [u8; 3]) -> [f32; 3] {
    fn to_linear(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    let (r, g, b) = (to_linear(r), to_linear(g), to_linear(b));

    // sRGB -> XYZ (D65)
    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

    // Chuẩn hoá theo white point D65
    let (xn, yn, zn) = (0.95047, 1.0, 1.08883);
    fn f(t: f32) -> f32 {
        const DELTA: f32 = 6.0 / 29.0;
        if t > DELTA.powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
        }
    }
    let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let bb = 200.0 * (fy - fz);
    [l, a, bb]
}

fn lab_dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    d0 * d0 + d1 * d1 + d2 * d2
}

/// Số vòng lặp tinh chỉnh tâm màu kiểu k-means nhẹ (xem doc đầu file).
const REFINE_ITERS: usize = 3;
/// Trọng số kéo tâm về phía trung bình các ô được gán ở mỗi vòng lặp.
/// Không dùng 1.0 (thay thế hoàn toàn) để tránh 1 vài ô gán nhầm ở vòng
/// đầu kéo tâm lệch quá xa chỉ sau 1 bước — neo lại 1 phần tâm cũ giúp
/// quá trình hội tụ êm hơn qua nhiều vòng lặp thay vì nhảy dựng.
const REFINE_PULL: f32 = 0.5;

/// Chuyển toàn bộ 54 ô của `scan` sang Lab 1 lần, dùng chung cho cả
/// `classify_scan` lẫn `low_confidence_stickers`/`ambiguous_candidates`
/// để tránh tính `srgb_to_lab` lặp lại và đảm bảo cả 2 luôn nhất quán.
fn scan_to_labs(scan: &RawScan) -> [[[f32; 3]; 9]; 6] {
    std::array::from_fn(|f| std::array::from_fn(|s| srgb_to_lab(scan.faces[f].samples[s])))
}

/// Gom độ tin cậy mỗi ô (`RawFace::confidence`, xem `sample.rs`) thành
/// cùng hình dạng với `scan_to_labs` để 2 mảng luôn khớp chỉ số.
fn scan_to_confidence(scan: &RawScan) -> [[f32; 9]; 6] {
    std::array::from_fn(|f| scan.faces[f].confidence)
}

/// Số màu và số ô mỗi màu trên 1 khối Rubik 3x3 tiêu chuẩn — ràng buộc
/// CỨNG dùng xuyên suốt file này (xem doc đầu file).
const N_COLORS: usize = 6;
const STICKERS_PER_COLOR: usize = 9;
const TOTAL_STICKERS: usize = N_COLORS * STICKERS_PER_COLOR; // 54

/// Gán CHÍNH XÁC `STICKERS_PER_COLOR` ô cho mỗi màu trong `N_COLORS` màu
/// sao cho TỔNG khoảng cách Lab (bình phương, đã NHÂN TRỌNG SỐ tin cậy)
/// từ mỗi ô tới tâm màu được gán là NHỎ NHẤT trên toàn bộ
/// `TOTAL_STICKERS` ô — bài toán phân công cân bằng (balanced
/// assignment), giải CHÍNH XÁC bằng thuật toán Hungarian: mã hoá thành
/// ma trận vuông 54x54 (mỗi màu chiếm 9 "cột" giống hệt nhau về chi phí
/// = 9 chỗ trống của màu đó), rồi tìm phép khớp trọng số nhỏ nhất. O(n^3)
/// với n=54 nên chạy tức thời.
///
/// `confidence[f][s]` (0,1] nhân trực tiếp vào chi phí: ô càng ÍT tin
/// cậy (đo nhiễu, nghi loá) thì khoảng cách tới MỌI tâm màu càng bị co
/// lại gần 0 như nhau -- tức "gán màu nào cũng gần như miễn phí" -- nên
/// khi ràng buộc 6x9 buộc phải hy sinh 1 ô để cân đối số lượng, thuật
/// toán sẽ ưu tiên "hy sinh" đúng ô kém tin cậy đó thay vì đổi nhãn 1 ô
/// đo chắc chắn ở nơi khác (vốn có trọng số ~1.0, đổi nhãn tốn kém hơn
/// nhiều). Xem `sample::sample_grid_with_confidence` để biết cách đo.
fn balanced_assign(labs: &[[[f32; 3]; 9]; 6], confidence: &[[f32; 9]; 6], centers: &[[f32; 3]; 6]) -> ClassifiedFaces {
    // Làm phẳng 54 ô theo thứ tự (face, sticker) cố định: index i ứng
    // với face = i/9, sticker = i%9.
    let points: Vec<[f32; 3]> = labs.iter().flat_map(|f| f.iter().copied()).collect();
    let weights: Vec<f32> = confidence.iter().flat_map(|f| f.iter().copied()).collect();
    debug_assert_eq!(points.len(), TOTAL_STICKERS);

    // Ma trận chi phí 1-indexed (thuật toán Hungarian cổ điển dùng chỉ
    // số 1..=n, chỉ số 0 bỏ trống làm lính canh).
    let mut cost = vec![vec![0f64; TOTAL_STICKERS + 1]; TOTAL_STICKERS + 1];
    for i in 0..TOTAL_STICKERS {
        for c in 0..N_COLORS {
            let d = (lab_dist2(points[i], centers[c]) * weights[i]) as f64;
            for slot in 0..STICKERS_PER_COLOR {
                let j = c * STICKERS_PER_COLOR + slot;
                cost[i + 1][j + 1] = d;
            }
        }
    }

    let row_to_col = hungarian_min_cost(&cost, TOTAL_STICKERS);

    let mut out = [[0u8; 9]; 6];
    for i in 0..TOTAL_STICKERS {
        let j = row_to_col[i + 1] - 1; // cột 0-indexed được gán cho ô i
        let label = (j / STICKERS_PER_COLOR) as u8;
        out[i / 9][i % 9] = label;
    }
    out
}

/// Thuật toán Hungarian (Kuhn-Munkres) cổ điển O(n^3) cho ma trận chi
/// phí vuông n x n, 1-indexed (chỉ số 0 không dùng). Trả về mảng
/// `row_to_col` với `row_to_col[i]` = cột (1-indexed) được gán cho hàng
/// i -- đây là cách dùng thuận tiện hơn cho bên gọi so với biểu diễn gốc
/// của thuật toán (vốn trả "hàng được gán cho từng cột").
fn hungarian_min_cost(a: &[Vec<f64>], n: usize) -> Vec<usize> {
    let inf = f64::INFINITY;
    let mut u = vec![0f64; n + 1];
    let mut v = vec![0f64; n + 1];
    let mut p = vec![0usize; n + 1]; // p[j] = hàng đang được gán cho cột j
    let mut way = vec![0usize; n + 1];

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![inf; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = inf;
            let mut j1 = 0usize;
            for j in 1..=n {
                if !used[j] {
                    let cur = a[i0][j] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    let mut row_to_col = vec![0usize; n + 1];
    for j in 1..=n {
        row_to_col[p[j]] = j;
    }
    row_to_col
}

/// Tinh chỉnh 6 tâm màu (khởi tạo từ 6 ô tâm mặt) bằng chính 54 ô đã lấy
/// mẫu trải trên cả 6 ảnh — xem giải thích đầy đủ ở doc đầu file. Mỗi
/// vòng lặp dùng CHÍNH `balanced_assign` (không phải "gần nhất" tự do)
/// để tính lại trung bình mỗi cụm, nên luôn có đúng 9 ô/cụm -- không bao
/// giờ có cụm "rỗng" hay "quá tải" trong lúc tinh chỉnh. Trung bình mỗi
/// cụm là trung bình CÓ TRỌNG SỐ theo độ tin cậy -- 1 ô đo nhiễu (nghi
/// loá) đóng góp ít hơn vào vị trí tâm màu so với 1 ô đo sạch.
fn refine_centers(labs: &[[[f32; 3]; 9]; 6], confidence: &[[f32; 9]; 6]) -> [[f32; 3]; 6] {
    let mut centers: [[f32; 3]; 6] = std::array::from_fn(|f| labs[f][4]);
    for _ in 0..REFINE_ITERS {
        let assign = balanced_assign(labs, confidence, &centers);
        let mut sum = [[0f32; 3]; 6];
        let mut weight_sum = [0f32; 6];
        for f in 0..6 {
            for s in 0..9 {
                let label = assign[f][s] as usize;
                let w = confidence[f][s];
                for k in 0..3 {
                    sum[label][k] += labs[f][s][k] * w;
                }
                weight_sum[label] += w;
            }
        }
        for c in 0..6 {
            // weight_sum[c] > 0 luôn đúng: balanced_assign đảm bảo đúng
            // 9 ô/cụm, và confidence luôn > 0 (kẹp ở MIN_CONFIDENCE của
            // sample.rs) nên tổng trọng số không bao giờ bằng 0.
            let mean = [sum[c][0] / weight_sum[c], sum[c][1] / weight_sum[c], sum[c][2] / weight_sum[c]];
            for k in 0..3 {
                centers[c][k] = centers[c][k] * (1.0 - REFINE_PULL) + mean[k] * REFINE_PULL;
            }
        }
    }
    centers
}

/// Phân loại toàn bộ 54 ô của `scan` (thứ tự chụp cố định U,D,F,B,L,R
/// tương ứng index 0..6 trong `scan.faces`). Trả về, với mỗi ô, CHỈ SỐ
/// (0..6) của mặt màu được gán — chỉ số này chính là giá trị facelet mà
/// `CubeState` dùng (facelet = chỉ số mặt màu ở trạng thái đã giải), nên
/// không cần bảng tên màu trung gian. Dùng `balanced_assign` (không
/// phải "gần nhất" độc lập từng ô) để đảm bảo đúng 9 ô/màu trên toàn cục
/// — xem doc đầu file.
///
/// Ghi chú: đây MỚI chỉ là phân loại màu theo đúng vị trí đã chụp, CHƯA
/// xử lý việc ảnh có thể bị xoay lệch so với hướng chuẩn — bước đó nằm ở
/// `scan::resolve_orientation`.
pub fn classify_scan(scan: &RawScan) -> ClassifiedFaces {
    let labs = scan_to_labs(scan);
    let confidence = scan_to_confidence(scan);
    let centers = refine_centers(&labs, &confidence);
    balanced_assign(&labs, &confidence, &centers)
}

/// (chỉ số tâm gần nhất, khoảng cách gần nhì - gần nhất) -- hiệu số dùng
/// làm thước đo "độ chắc chắn": hiệu số càng nhỏ càng mơ hồ.
fn nearest_center(lab: [f32; 3], centers: &[[f32; 3]; 6]) -> (usize, f32) {
    let mut dists: Vec<(usize, f32)> = centers.iter().enumerate().map(|(i, &c)| (i, lab_dist2(lab, c).sqrt())).collect();
    dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let gap = if dists.len() >= 2 { dists[1].1 - dists[0].1 } else { f32::MAX };
    (dists[0].0, gap)
}

/// Cảnh báo chất lượng phân loại: nếu 2 tâm quá gần nhau trong Lab (vd
/// ánh sáng làm 2 mặt trông giống hệt nhau), hoặc 1 ô nằm gần như giữa
/// 2 màu (không tách bạch) -> đáng ngờ, nên yêu cầu người dùng xác nhận
/// thủ công thay vì tin tuyệt đối. Trả về danh sách (face, sticker) đáng
/// ngờ để UI tô đỏ cho người dùng sửa tay.
pub fn low_confidence_stickers(scan: &RawScan, threshold: f32) -> Vec<(usize, usize)> {
    let labs = scan_to_labs(scan);
    let centers = refine_centers(&labs, &scan_to_confidence(scan));
    let mut flagged = Vec::new();
    for f in 0..6 {
        for s in 0..9 {
            if s == 4 {
                continue; // ô tâm luôn là chính nó
            }
            let (_, gap) = nearest_center(labs[f][s], &centers);
            if gap < threshold {
                flagged.push((f, s));
            }
        }
    }
    flagged
}

/// 1 ô mơ hồ kèm PHƯƠNG ÁN NHÌ (nhãn màu gần nhì) -- dùng để thử "sửa
/// tay tự động": nếu ghép cubie thất bại hoàn toàn với phương án gần
/// nhất, rất có thể (những) ô mơ hồ nhất trong số này thực ra phải là
/// phương án nhì. `gap` = hiệu khoảng cách nhì-nhất trong Lab, CÀNG NHỎ
/// càng mơ hồ -- danh sách trả về đã sắp theo `gap` tăng dần (mơ hồ nhất
/// trước) để bên gọi ưu tiên thử đúng ô nghi ngờ nhất trước.
pub struct AmbiguousCandidate {
    pub face: usize,
    pub sticker: usize,
    pub alt_label: u8,
    pub gap: f32,
}

/// Lấy tối đa `limit` ô mơ hồ nhất (không tính ô tâm) kèm nhãn phương án
/// nhì, sắp theo độ mơ hồ tăng dần.
pub fn ambiguous_candidates(scan: &RawScan, limit: usize) -> Vec<AmbiguousCandidate> {
    let labs = scan_to_labs(scan);
    let centers = refine_centers(&labs, &scan_to_confidence(scan));
    let mut out = Vec::new();
    for f in 0..6 {
        for s in 0..9 {
            if s == 4 {
                continue;
            }
            let mut dists: Vec<(usize, f32)> =
                centers.iter().enumerate().map(|(i, &c)| (i, lab_dist2(labs[f][s], c).sqrt())).collect();
            dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let gap = if dists.len() >= 2 { dists[1].1 - dists[0].1 } else { f32::MAX };
            out.push(AmbiguousCandidate { face: f, sticker: s, alt_label: dists[1].0 as u8, gap });
        }
    }
    out.sort_by(|a, b| a.gap.partial_cmp(&b.gap).unwrap());
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::RawFace;

    fn face(center: [u8; 3]) -> RawFace {
        RawFace::from_samples([center; 9])
    }

    #[test]
    fn classifies_uniform_faces_to_six_distinct_groups() {
        // 6 màu tách bạch rõ ràng, mỗi mặt đồng màu tuyệt đối.
        let refs: [[u8; 3]; 6] = [
            [255, 255, 255], // trắng
            [255, 255, 0],   // vàng
            [0, 160, 0],     // xanh lá
            [0, 0, 200],     // xanh dương
            [255, 130, 0],   // cam
            [200, 0, 0],     // đỏ
        ];
        let scan = RawScan { faces: std::array::from_fn(|i| face(refs[i])) };
        let out = classify_scan(&scan);
        for f in 0..6 {
            assert!(out[f].iter().all(|&v| v == f as u8), "face {f} không đồng nhất: {:?}", out[f]);
        }
    }

    #[test]
    fn flags_ambiguous_sticker_between_two_close_centers() {
        let refs: [[u8; 3]; 6] = [
            [255, 255, 255],
            [255, 250, 245], // rất gần trắng -> mơ hồ khi có ô lưng chừng
            [0, 160, 0],
            [0, 0, 200],
            [255, 130, 0],
            [200, 0, 0],
        ];
        let mut scan = RawScan { faces: std::array::from_fn(|i| face(refs[i])) };
        // ô số 0 của mặt 0 nằm giữa trắng và "gần trắng"
        scan.faces[0].samples[0] = [255, 252, 250];
        let flagged = low_confidence_stickers(&scan, 8.0);
        assert!(flagged.contains(&(0, 0)));
    }

    /// Mô phỏng đúng vấn đề "ánh sáng khác nhau giữa các mặt": ô TÂM của
    /// mặt trắng bị ám vàng nhẹ do ảnh đó chụp dưới đèn ấm hơn các mặt
    /// khác, TRONG KHI các ô trắng thật khác (xuất hiện tự nhiên ở nhiều
    /// mặt khác nhau vì cube đang xáo trộn) vẫn giữ màu trắng chuẩn.
    ///
    /// Dùng 1 `CubeState` đã xáo trộn thật (không bịa mảng màu tay) để
    /// đảm bảo dữ liệu đầu vào tôn trọng đúng ràng buộc "6 màu, mỗi màu
    /// 9 ô" mà `balanced_assign` áp đặt -- nếu không, test sẽ tự vi phạm
    /// ràng buộc đó (vd tạo ra 13 ô "trắng" ảo) và không còn phản ánh
    /// đúng tình huống thực tế nữa.
    #[test]
    fn cross_face_lighting_bias_on_one_center_does_not_break_other_faces() {
        use rubik_core::cube::CubeState;

        let mut state = CubeState::solved();
        state.do_move("R");
        state.do_move("U");
        state.do_move("F");
        state.do_move("L2");
        state.do_move("D'");

        const COLORS: [[u8; 3]; 6] = [
            [255, 255, 255],
            [255, 235, 40],
            [0, 160, 60],
            [0, 70, 200],
            [255, 120, 0],
            [200, 20, 20],
        ];
        let mut scan = RawScan {
            faces: std::array::from_fn(|f| {
                RawFace::from_samples(std::array::from_fn(|s| COLORS[state.faces[f][s] as usize]))
            }),
        };
        // Ô tâm luôn giữ đúng chỉ số mặt của nó (bất biến của cube 3x3:
        // tâm không bao giờ đổi mặt) nên mặt 0 (trắng) luôn có
        // `state.faces[0][4] == 0`. Ám vàng nhẹ riêng ô tâm mặt trắng --
        // lệch về phía vàng nhưng chưa lệch đến mức "là vàng".
        scan.faces[0].samples[4] = [255, 250, 190];

        let out = classify_scan(&scan);
        // Mọi ô trắng thật khác (không phải chính ô tâm bị ám màu cố
        // tình) ở BẤT KỲ mặt nào cũng phải vẫn được phân loại đúng thành
        // trắng (0), dù mặt trắng có tâm bị ám màu.
        for f in 0..6 {
            for s in 0..9 {
                if f == 0 && s == 4 {
                    continue; // bỏ qua chính ô tâm bị ám màu cố tình
                }
                if state.faces[f][s] == 0 {
                    assert_eq!(out[f][s], 0, "ô trắng thật ở mặt {f} ô {s} bị phân loại nhầm: {}", out[f][s]);
                }
            }
        }
    }
}

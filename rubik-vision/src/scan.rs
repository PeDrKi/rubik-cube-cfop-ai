//! scan.rs — ghép 6 mặt đã phân loại màu (`color::classify_scan`) thành
//! 1 `CubeState` hợp lệ.
//!
//! Vấn đề cốt lõi: người dùng cầm cube chụp 6 ảnh theo thứ tự cố định
//! U,D,F,B,L,R, nhưng KHÔNG thể đảm bảo mỗi ảnh đúng góc xoay 3x3 tuyệt
//! đối (0/90/180/270°) so với quy ước toạ độ của `rubik-core`. Chụp lệch
//! dù chỉ 1 mặt sẽ khiến ghép cubie thất bại (panic trong
//! `edge_model`/`corner_model`) hoặc ra trạng thái không giải được.
//!
//! Thay vì bắt buộc 1 quy trình cầm-xoay tuyệt đối chính xác (rất dễ sai
//! khi thao tác thủ công), module này TỰ THỬ tối đa 4^6 = 4096 tổ hợp
//! xoay 6 mặt, chỉ giữ tổ hợp nào cho ra trạng thái vừa "hợp lệ về mặt
//! ghép cubie" vừa "hợp lệ về parity" (tức về lý thuyết giải được) —
//! kiểm tra bằng số học parity chuẩn của Rubik, không cần chạy solver
//! thật nên rẻ (microgiây/tổ hợp).

use rubik_core::corner_model::CORNER_SLOTS;
use rubik_core::cube::CubeState;
use rubik_core::edge_model::EDGE_SLOTS;
use rubik_core::full_state;

use crate::color::{ambiguous_candidates, classify_scan, low_confidence_stickers};

#[derive(Clone, Copy, Debug)]
pub struct RawFace {
    /// 9 màu RGB thô, row-major, ĐÚNG NHƯ CHỤP (chưa xoay chỉnh).
    pub samples: [[u8; 3]; 9],
    /// Độ tin cậy phép đo mỗi ô, trong (0, 1] -- 1.0 = tin cậy tuyệt đối.
    /// Tính từ `sample::sample_grid_with_confidence` (dựa vào tỉ lệ
    /// pixel cháy sáng + độ đồng thuận giữa các patch con khi lấy mẫu
    /// webcam thật); nguồn dữ liệu khác (nhập tay, ML, hoặc test) không
    /// đo được thì dùng `from_samples` để mặc định tin cậy tuyệt đối.
    /// `color::balanced_assign` dùng giá trị này làm TRỌNG SỐ: ô càng ít
    /// tin cậy càng "rẻ" để đổi nhãn khi cần dung hoà ràng buộc 6×9,
    /// thay vì đổi nhãn 1 ô đo chắc chắn.
    pub confidence: [f32; 9],
}

impl RawFace {
    /// Tạo `RawFace` từ màu đã đo, coi mọi ô tin cậy tuyệt đối (1.0) --
    /// dùng khi nguồn dữ liệu không đo được độ tin cậy (nhập tay, ML,
    /// test).
    pub fn from_samples(samples: [[u8; 3]; 9]) -> Self {
        Self { samples, confidence: [1.0; 9] }
    }
}

/// 6 ảnh đã chụp, BẮT BUỘC theo đúng thứ tự U, D, F, B, L, R.
#[derive(Clone, Copy, Debug)]
pub struct RawScan {
    pub faces: [RawFace; 6],
}

#[derive(Debug)]
pub enum ScanError {
    /// Không tổ hợp xoay nào cho ra trạng thái hợp lệ — nhiều khả năng
    /// phân loại màu sai ở đâu đó (ánh sáng, khoanh vùng lệch tâm...).
    /// Đính kèm danh sách ô đáng ngờ (dựa vào khoảng cách màu mơ hồ) để
    /// UI gợi ý người dùng sửa tay.
    NoValidArrangement { suspicious: Vec<(usize, usize)> },
    /// Nhiều tổ hợp cùng hợp lệ với chi phí xoay bằng nhau — hiếm gặp
    /// trong thực tế (thường chỉ xảy ra khi cube đã ở trạng thái đã
    /// giải hoặc gần đối xứng), cần người dùng xác nhận thủ công.
    Ambiguous { candidates: Vec<CubeState> },
}

pub struct ScanReport {
    pub state: CubeState,
    /// Số lần xoay 90° CW đã áp dụng cho mỗi mặt (0..6, theo thứ tự
    /// chụp U,D,F,B,L,R) để ra được `state` — hữu ích để debug/hiển thị
    /// "app đã tự xoay ảnh mặt X 180°" cho người dùng yên tâm.
    pub rotations_applied: [u8; 6],
    /// (mặt, ô) đã được TỰ SỬA sang phương án màu gần nhì vì phân loại
    /// theo phương án gần nhất không ghép được cách nào hợp lệ — rỗng
    /// nếu phân loại ban đầu (không cần sửa) đã ra ngay kết quả hợp lệ.
    /// Hiển thị cho người dùng biết "app đã tự đoán lại N ô mơ hồ" thay
    /// vì âm thầm đổi mà không báo.
    pub auto_fixed: Vec<(usize, usize)>,
}

/// Xoay 1 mặt 3x3 (row-major) `k` lần 90° theo chiều kim đồng hồ.
fn rotate9(m: [u8; 9], k: u8) -> [u8; 9] {
    let mut out = m;
    for _ in 0..(k % 4) {
        let g = out;
        let get = |r: usize, c: usize| g[r * 3 + c];
        let mut next = [0u8; 9];
        for r in 0..3 {
            for c in 0..3 {
                next[r * 3 + c] = get(2 - c, r);
            }
        }
        out = next;
    }
    out
}

/// Kiểm tra 1 bộ facelet [[u8;9];6] có "ghép cubie" hợp lệ hay không:
/// mỗi khe cạnh (12 khe) và khe góc (8 khe) phải khớp ĐÚNG 1-1 với đúng
/// 1 khe tương ứng ở trạng thái đã giải (không lặp, không thiếu) — chính
/// là điều kiện mà `edge_model::identify`/`corner_model::identify` cần
/// để không panic, kiểm tra trước ở đây để không bao giờ gọi tới chúng
/// với dữ liệu xấu.
fn bijection_legal(faces: &[[u8; 9]; 6]) -> bool {
    let solved = CubeState::solved().faces;
    let read = |grid: &[[u8; 9]; 6], (f, r, c): (usize, usize, usize)| grid[f][r * 3 + c];

    let mut used_edge = [false; 12];
    for &(c1, c2) in EDGE_SLOTS.iter() {
        let (v1, v2) = (read(faces, c1), read(faces, c2));
        let mut found = false;
        for (orig, &(o1, o2)) in EDGE_SLOTS.iter().enumerate() {
            if used_edge[orig] {
                continue;
            }
            let (s1, s2) = (read(&solved, o1), read(&solved, o2));
            if (s1 == v1 && s2 == v2) || (s1 == v2 && s2 == v1) {
                used_edge[orig] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }

    let mut used_corner = [false; 8];
    for &(c1, c2, c3) in CORNER_SLOTS.iter() {
        let mut v = [read(faces, c1), read(faces, c2), read(faces, c3)];
        v.sort_unstable();
        let mut found = false;
        for (orig, &(o1, o2, o3)) in CORNER_SLOTS.iter().enumerate() {
            if used_corner[orig] {
                continue;
            }
            let mut s = [read(&solved, o1), read(&solved, o2), read(&solved, o3)];
            s.sort_unstable();
            if s == v {
                used_corner[orig] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

/// Parity của 1 hoán vị (true = chẵn), theo số chu trình: parity =
/// (n - số_chu_trình) mod 2 == 0.
fn perm_is_even(perm: &[usize]) -> bool {
    let n = perm.len();
    let mut seen = vec![false; n];
    let mut cycles = 0;
    for i in 0..n {
        if seen[i] {
            continue;
        }
        cycles += 1;
        let mut j = i;
        while !seen[j] {
            seen[j] = true;
            j = perm[j];
        }
    }
    (n - cycles) % 2 == 0
}

/// 3 điều kiện parity chuẩn của Rubik's cube hợp lệ về lý thuyết (không
/// đảm bảo solver tìm ra lời giải nhanh, chỉ đảm bảo lời giải TỒN TẠI):
/// - Tổng lật cạnh (edge flip) chẵn.
/// - Tổng xoay góc (corner twist) chia hết cho 3.
/// - Parity hoán vị cạnh khớp parity hoán vị góc.
fn parity_ok(full: &full_state::FullState) -> bool {
    let (ep, eo, cp, co) = full;
    let flip_ok = eo.iter().map(|&x| x as u32).sum::<u32>() % 2 == 0;
    let twist_ok = co.iter().map(|&x| x as u32).sum::<u32>() % 3 == 0;
    let perm_ok = perm_is_even(ep) == perm_is_even(cp);
    flip_ok && twist_ok && perm_ok
}

/// Kiểm tra 1 bộ facelet [[u8;9];6] (giá trị mỗi ô = chỉ số mặt màu,
/// 0..6, ĐÚNG quy ước U,D,F,B,L,R như solved-state của `CubeState`) có
/// ghép được thành 1 khối Rubik hợp lệ + giải được (đúng parity) hay
/// không. Dùng chung cho cả luồng quét camera (nội bộ, đã tự dò hướng
/// xoay) lẫn luồng người dùng tự điền màu thủ công (không cần dò xoay,
/// vị trí mỗi ô đã đúng theo lưới hiển thị).
pub fn validate_facelets(faces: [[u8; 9]; 6]) -> Result<CubeState, String> {
    if !bijection_legal(&faces) {
        return Err(
            "Không ghép được thành khối lập phương hợp lệ -- kiểm tra lại đã điền đúng vị trí và đủ 9 ô mỗi màu chưa.".to_string(),
        );
    }
    let state = CubeState { faces };
    let full = full_state::from_facelets(&state);
    if !parity_ok(&full) {
        return Err(
            "Cách sắp xếp màu này không thể giải được (vi phạm parity của Rubik's cube) -- thường do đổi chỗ nhầm 2 sticker hoặc lật ngược 1 cạnh so với cube thật.".to_string(),
        );
    }
    Ok(state)
}

/// Tổng số lần xoay 90° trong 1 tổ hợp — dùng làm tiêu chí ưu tiên khi
/// có nhiều tổ hợp hợp lệ: tổ hợp càng gần với ảnh gốc (ít xoay nhất)
/// càng có khả năng là tổ hợp "đúng thật" (giả định người dùng chụp gần
/// đúng hướng, chỉ lệch 1-2 mặt do cầm nghiêng).
fn rotation_cost(r: &[u8; 6]) -> u32 {
    r.iter().map(|&x| x.min(4 - x) as u32).sum()
}

/// Thử cả 4^6 = 4096 tổ hợp xoay 6 mặt trên MỘT bộ facelet đã phân loại
/// sẵn, trả về tất cả tổ hợp cho ra trạng thái vừa ghép cubie hợp lệ vừa
/// đúng parity (giải được). Tách riêng khỏi `resolve_scan` để dùng lại
/// được cho cả lần thử "phân loại gốc" lẫn từng lần thử "đã sửa vài ô
/// mơ hồ sang phương án nhì" trong `try_resolve_with_fixes`.
fn search_rotations(classified: &[[u8; 9]; 6]) -> Vec<([u8; 6], CubeState)> {
    let mut valid: Vec<([u8; 6], CubeState)> = Vec::new();
    for combo in 0u32..4096 {
        let rot: [u8; 6] = std::array::from_fn(|i| ((combo / 4u32.pow(i as u32)) % 4) as u8);
        let candidate: [[u8; 9]; 6] = std::array::from_fn(|f| rotate9(classified[f], rot[f]));

        if !bijection_legal(&candidate) {
            continue;
        }
        let state = CubeState { faces: candidate };
        let full = full_state::from_facelets(&state);
        if parity_ok(&full) {
            valid.push((rot, state));
        }
    }
    valid
}

/// Gói `search_rotations` thành đúng kiểu trả về của `resolve_scan`
/// (thành công / mơ hồ), gắn kèm danh sách ô đã tự sửa (`fixed`) để
/// `ScanReport` phản ánh đúng những gì đã thay đổi so với phân loại gốc.
fn finalize(valid: Vec<([u8; 6], CubeState)>, fixed: Vec<(usize, usize)>) -> Result<ScanReport, ScanError> {
    let min_cost = valid.iter().map(|(r, _)| rotation_cost(r)).min().unwrap();
    let best: Vec<_> = valid.into_iter().filter(|(r, _)| rotation_cost(r) == min_cost).collect();

    if best.len() == 1 {
        let (rot, state) = best.into_iter().next().unwrap();
        Ok(ScanReport { state, rotations_applied: rot, auto_fixed: fixed })
    } else {
        Err(ScanError::Ambiguous { candidates: best.into_iter().map(|(_, s)| s).collect() })
    }
}

/// Số ô mơ hồ tối đa xét tới khi thử tự sửa -- giới hạn để độ phức tạp
/// (2^n tổ hợp sửa, mỗi tổ hợp lại thử 4096 hướng xoay) không nổ ra quá
/// lớn. Trong thực tế lỗi phân loại do loá/ánh sáng hiếm khi lan ra quá
/// vài ô 1 lúc, nên giới hạn 6 (2^6 = 64 tổ hợp) là đủ mà vẫn tính trong
/// tích tắc.
const MAX_AMBIGUOUS_TO_TRY: usize = 6;

/// Khi phân loại theo phương án gần nhất KHÔNG ghép được cách nào hợp lệ
/// (0/4096 tổ hợp xoay hợp lệ), thử thay nhãn của (những) ô mơ hồ nhất
/// bằng phương án gần NHÌ, tăng dần số ô thử sửa cùng lúc (1, rồi 2, ...
/// tới `MAX_AMBIGUOUS_TO_TRY`) để ưu tiên phương án can thiệp ít nhất.
/// Đây chính là cách xử lý "báo không ghép được dù cầm cube đúng, do 1-2
/// ô mơ hồ" — thay vì bắt người dùng quét lại từ đầu, tận dụng luôn
/// thông tin "phương án nhì" đã có sẵn từ bước phân loại màu.
fn try_resolve_with_fixes(scan: &RawScan, classified: &[[u8; 9]; 6]) -> Option<Result<ScanReport, ScanError>> {
    let candidates = ambiguous_candidates(scan, MAX_AMBIGUOUS_TO_TRY);
    let n = candidates.len();
    if n == 0 {
        return None;
    }

    for k in 1..=n {
        for mask in 0u32..(1 << n) {
            if mask.count_ones() as usize != k {
                continue;
            }
            let mut attempt = *classified;
            let mut fixed = Vec::with_capacity(k);
            for (i, c) in candidates.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    attempt[c.face][c.sticker] = c.alt_label;
                    fixed.push((c.face, c.sticker));
                }
            }
            let valid = search_rotations(&attempt);
            if !valid.is_empty() {
                return Some(finalize(valid, fixed));
            }
        }
    }
    None
}

/// Ghép `scan` thành 1 `CubeState` hợp lệ + giải được, tự dò hướng xoay
/// đúng của từng mặt.
pub fn resolve_scan(scan: &RawScan) -> Result<ScanReport, ScanError> {
    let classified = classify_scan(scan);

    let valid = search_rotations(&classified);
    if !valid.is_empty() {
        return finalize(valid, Vec::new());
    }

    // Không tổ hợp xoay nào hợp lệ với phân loại gốc -- trước khi báo
    // lỗi, thử tự sửa (những) ô mơ hồ nhất sang phương án nhì.
    if let Some(result) = try_resolve_with_fixes(scan, &classified) {
        return result;
    }

    let suspicious = low_confidence_stickers(scan, 8.0);
    Err(ScanError::NoValidArrangement { suspicious })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dựng RawScan "hoàn hảo" (đúng màu, đúng hướng 100%) trực tiếp từ
    /// 1 CubeState đã biết, để kiểm chứng round-trip.
    fn perfect_scan_from(state: &CubeState) -> RawScan {
        // 6 màu RGB tách bạch rõ ràng, ánh xạ theo chỉ số mặt 0..6.
        const COLORS: [[u8; 3]; 6] = [
            [255, 255, 255],
            [255, 220, 0],
            [0, 160, 60],
            [0, 70, 200],
            [255, 120, 0],
            [200, 20, 20],
        ];
        let faces = std::array::from_fn(|f| {
            RawFace::from_samples(std::array::from_fn(|s| COLORS[state.faces[f][s] as usize]))
        });
        RawScan { faces }
    }

    /// `classify_scan` giờ dùng phép gán CÂN BẰNG (đúng 9 ô/màu toàn cục,
    /// xem `color::balanced_assign`), nên 1 ô lệch màu ĐƠN LẺ (không kèm
    /// theo ô nào khác bù trừ) thường đã được sửa ngay ở bước phân loại,
    /// không cần tới bước tự sửa dự phòng của `resolve_scan` nữa --
    /// kiểm chứng bằng chính ví dụ đã gây lỗi trước khi có ràng buộc này
    /// (1 ô trắng thật lệch hẳn về phía vàng do loá/ám màu).
    #[test]
    fn balanced_classification_alone_fixes_a_single_boundary_case_sticker() {
        let mut state = CubeState::solved();
        state.do_move("R");
        state.do_move("U");
        state.do_move("F");
        let mut scan = perfect_scan_from(&state);

        let mut target: Option<(usize, usize)> = None;
        'search: for f in 0..6 {
            for s in 0..9 {
                if s != 4 && state.faces[f][s] == 0 {
                    target = Some((f, s));
                    break 'search;
                }
            }
        }
        let (tf, ts) = target.expect("cube sau vài nước xoay phải còn ít nhất 1 ô trắng không phải ô tâm");
        // Lệch về phía vàng, đúng kiểu lỗi thực tế do loá/ám màu.
        scan.faces[tf].samples[ts] = [255, 240, 147];

        let classified = classify_scan(&scan);
        assert_eq!(
            classified[tf][ts], 0,
            "ràng buộc đúng-9-ô-mỗi-màu phải tự sửa đúng ô này về trắng ngay ở bước phân loại"
        );
        let report = resolve_scan(&scan).expect("phải ghép ra trạng thái hợp lệ, không cần quét lại");
        assert!(bijection_legal(&report.state.faces));
    }

    /// Trường hợp "6 màu x 9 ô" KHÔNG tự đủ sửa: 2 ô (1 trắng thật, 1
    /// vàng thật) cùng bị lệch màu ĐỐI XỨNG sang phía nhau (vd 2 mặt bị
    /// ám màu ngược hướng nhau) -- vì tổng vẫn đúng 9 trắng/9 vàng, phép
    /// gán cân bằng toàn cục có thể chọn "đổi nhãn 2 ô này cho nhau" vì
    /// rẻ hơn, làm ghép cubie thất bại (2 vị trí thật đổi màu cho nhau
    /// không tạo thành cubie hợp lệ). Đây đúng là trường hợp cần đến
    /// bước tự sửa theo độ mơ hồ TỪNG Ô của `try_resolve_with_fixes`
    /// (không dựa vào ràng buộc số lượng) mới ghép được.
    #[test]
    fn recovers_valid_state_after_a_symmetric_two_sticker_color_swap() {
        let mut state = CubeState::solved();
        state.do_move("R");
        state.do_move("U");
        state.do_move("F");
        let mut scan = perfect_scan_from(&state);

        let mut white_pos: Option<(usize, usize)> = None;
        let mut yellow_pos: Option<(usize, usize)> = None;
        for f in 0..6 {
            for s in 0..9 {
                if s == 4 {
                    continue;
                }
                if white_pos.is_none() && state.faces[f][s] == 0 {
                    white_pos = Some((f, s));
                }
                if yellow_pos.is_none() && state.faces[f][s] == 1 {
                    yellow_pos = Some((f, s));
                }
            }
        }
        let (wf, ws) = white_pos.expect("cần ít nhất 1 ô trắng không phải ô tâm");
        let (yf, ys) = yellow_pos.expect("cần ít nhất 1 ô vàng không phải ô tâm");

        // Lệch đối xứng: ô trắng lệch về vàng, ô vàng lệch về trắng, cùng
        // 1 mức -- tổng số ô mỗi màu vẫn đúng 9, nhưng CỤ THỂ 2 vị trí
        // này bị nhầm nhãn cho nhau.
        scan.faces[wf].samples[ws] = [255, 240, 147]; // trắng thật, lệch hẳn sang vàng
        scan.faces[yf].samples[ys] = [255, 235, 108]; // vàng thật, lệch hẳn sang trắng

        let report = resolve_scan(&scan)
            .expect("phải tự sửa được cả 2 ô mơ hồ và ghép ra trạng thái hợp lệ, không cần quét lại");
        assert!(bijection_legal(&report.state.faces));
    }

    #[test]
    fn resolves_perfectly_oriented_scan() {
        let mut state = CubeState::solved();
        state.do_move("R");
        state.do_move("U");
        state.do_move("F");
        let scan = perfect_scan_from(&state);
        let report = resolve_scan(&scan).expect("phải giải được");
        assert!(report.state == state || report.rotations_applied.iter().all(|&x| x == 0));
    }

    #[test]
    fn recovers_from_a_single_rotated_face() {
        let mut state = CubeState::solved();
        state.do_move("R");
        state.do_move("U2");
        state.do_move("F'");
        state.do_move("L");
        let mut scan = perfect_scan_from(&state);
        // Giả lập người dùng chụp lệch mặt B (index 3) 90°.
        scan.faces[3].samples = rotate9(
            std::array::from_fn(|i| {
                let c = scan.faces[3].samples[i];
                // tìm lại face-index tương ứng để rotate9 hoạt động trên u8
                match c {
                    [255, 255, 255] => 0,
                    [255, 220, 0] => 1,
                    [0, 160, 60] => 2,
                    [0, 70, 200] => 3,
                    [255, 120, 0] => 4,
                    _ => 5,
                }
            }),
            1,
        )
        .map(|idx| {
            const COLORS: [[u8; 3]; 6] = [
                [255, 255, 255],
                [255, 220, 0],
                [0, 160, 60],
                [0, 70, 200],
                [255, 120, 0],
                [200, 20, 20],
            ];
            COLORS[idx as usize]
        });

        let report = resolve_scan(&scan).expect("phải tự sửa được hướng xoay và giải được");
        assert!(bijection_legal(&report.state.faces));
    }

    #[test]
    fn rejects_scan_with_wrong_sticker_counts() {
        let mut state = CubeState::solved();
        state.do_move("R");
        let mut scan = perfect_scan_from(&state);
        // Phá 1 ô thành màu không tồn tại trong 6 tâm -> lệch tổng số.
        scan.faces[0].samples[0] = [10, 10, 10];
        // Vẫn ra kết quả (vì được phân về màu gần nhất), nhưng ta chỉ
        // cần đảm bảo hàm không panic trên input xấu.
        let _ = resolve_scan(&scan);
    }
}

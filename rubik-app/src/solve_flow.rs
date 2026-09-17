//! solve_flow.rs — các kiểu dữ liệu và hàm THUẦN điều phối 1 nước/1 job
//! nền, tách riêng khỏi main.rs (vòng lặp sự kiện) để dễ đọc và test độc
//! lập. Không có gì ở đây đụng tới cửa sổ/GPU/webcam.

use rubik_core::cube::CubeState;
use std::collections::VecDeque;

/// 1 nước đang animate.
pub(crate) struct PendingMove {
    pub mv: char,
    pub prime: bool,
    /// Nước đúp: xoay liền 180° trong 1 lần, áp 2 lần khi xong.
    pub double: bool,
    pub target: f32,
    pub current: f32,
    pub speed: f32,
}

/// Kết quả 1 job nền (chạy trên thread riêng, gửi về qua channel).
pub(crate) enum JobResult {
    Hint(rubik_core::hint::Hint),
    /// Kèm theo CHÍNH trạng thái đã dùng để tính, để biết bảng còn khớp
    /// với khối hiện tại hay không (xem `formula_table`).
    Breakdown(CubeState, rubik_core::breakdown::Breakdown),
    /// Y HỆT Breakdown ở trên (dùng lại đúng `full_solve_breakdown`, tính
    /// nguyên lời giải 1 lần duy nhất TRƯỚC khi chạy nước nào) nhưng dành
    /// riêng cho "Giải tự động" (Enter) -- tách khỏi Breakdown để không
    /// đụng vào state của bảng công thức (formula_table/formula_open).
    /// Xem giải thích đầy đủ ở khối xử lý `AutoPlan` trong main.rs.
    AutoPlan(CubeState, rubik_core::breakdown::Breakdown),
}

/// Gom các chặng CFOP (Cross, 4 cặp F2L theo đúng thứ tự, OLL, PLL) của 1
/// `Breakdown` thành 1 kế hoạch PHẲNG theo thứ tự thực thi, bỏ qua chặng
/// "Done" (đã xong sẵn, không có nước) và "Pending". Nếu có chặng bị
/// "Failed", trả về tên chặng ĐẦU TIÊN bị vậy (để báo cho người dùng)
/// thay vì âm thầm bỏ qua -- các chặng SAU chặng Failed đầu tiên (nếu
/// Breakdown vẫn tính được, hiếm) vẫn được giữ trong kế hoạch, chỉ tên
/// chặng lỗi đầu tiên được báo ra ngoài.
///
/// Hàm THUẦN (không đụng tới `queue`/`state`/UI gì cả) nên test được độc
/// lập -- xem `tests::build_auto_plan_*` bên dưới.
pub(crate) fn build_auto_plan(
    b: &rubik_core::breakdown::Breakdown,
) -> (Vec<(String, Vec<String>)>, Option<&'static str>) {
    use rubik_core::breakdown::StageStatus;
    let mut plan: Vec<(String, Vec<String>)> = Vec::new();
    let mut failed_at: Option<&'static str> = None;
    let mut consider = |label: String, st: &StageStatus, name: &'static str| match st {
        StageStatus::Moves { moves, .. } if !moves.is_empty() => plan.push((label, moves.clone())),
        StageStatus::Failed => {
            if failed_at.is_none() {
                failed_at = Some(name);
            }
        }
        _ => {}
    };
    consider("Cross".to_string(), &b.cross, "Cross");
    for (slot, st) in &b.f2l {
        consider(format!("F2L - cặp {slot}"), st, "F2L");
    }
    consider("OLL".to_string(), &b.oll, "OLL");
    consider("PLL".to_string(), &b.pll, "PLL");
    (plan, failed_at)
}

/// Từ 1 kế hoạch phẳng (nhãn + nước mỗi chặng), tính mốc "đã chạy xong
/// bao nhiêu nước kể từ lúc bắt đầu thì tới lượt chặng này" cho TỪNG
/// chặng, theo đúng thứ tự trong `plan`. Hàm thuần, xem
/// `tests::build_boundaries_*`.
pub(crate) fn build_boundaries(plan: &[(String, Vec<String>)]) -> VecDeque<(usize, String, Vec<String>)> {
    let mut boundaries = VecDeque::new();
    let mut acc = 0usize;
    for (label, moves) in plan {
        boundaries.push_back((acc, label.clone(), moves.clone()));
        acc += moves.len();
    }
    boundaries
}

/// Bấm 1 hoặc nhiều mốc trong `boundaries` đã đạt tới (mốc <= `progress`,
/// tức số nước đã chạy xong kể từ lúc kế hoạch bắt đầu), trả về nhãn +
/// nước của chặng MỚI NHẤT đã bấm tới (nếu có ít nhất 1 mốc đạt), hoặc
/// `None` nếu chưa mốc nào đạt (chưa cần đổi nhãn hiển thị). Nếu nhiều
/// mốc đạt cùng lúc (chặng nào đó không có nước, dù `build_auto_plan` đã
/// lọc hết chặng rỗng nên bình thường không xảy ra) thì chỉ nhãn CUỐI
/// CÙNG được trả về -- các mốc ở giữa bị bỏ qua vì hiển thị lên rồi tắt
/// ngay trong cùng 1 khung hình không ai thấy được.
pub(crate) fn advance_auto_plan(
    boundaries: &mut VecDeque<(usize, String, Vec<String>)>,
    progress: usize,
) -> Option<(String, Vec<String>)> {
    let mut latest = None;
    while let Some((start, _, _)) = boundaries.front() {
        if progress >= *start {
            latest = boundaries.pop_front().map(|(_, label, moves)| (label, moves));
        } else {
            break;
        }
    }
    latest
}

/// Parse "R", "R'", "R2" thành 1 mục hàng đợi (mặt, có-nghịch-đảo,
/// có-180°). R2 giữ NGUYÊN thành 1 mục xoay 180° (không tách thành 2
/// vòng quay 90°) -- xem lý do trong test bên dưới.
pub(crate) fn queue_move(queue: &mut VecDeque<(char, bool, bool)>, mv_str: &str) {
    let base = mv_str.chars().next().unwrap();
    // Nước đúp (R2) đưa vào hàng đợi thành MỘT mục xoay 180°, không tách
    // thành 2 lần 90° -- nếu tách, số mục trong hàng đợi sẽ nhiều hơn số
    // nước của lời giải, khiến thanh tiến trình đếm lệch (VD lời giải 67
    // nước nhưng hiện "31/78"). Xoay liền 180° cũng đúng cách người chơi
    // thật thực hiện.
    if mv_str.ends_with('2') {
        queue.push_back((base, false, true));
    } else if mv_str.ends_with('\'') {
        queue.push_back((base, true, false));
    } else {
        queue.push_back((base, false, false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubik_core::breakdown::{Breakdown, StageStatus};

    // ---- queue_move: chuyển 1 chuỗi Singmaster ("R", "R'", "R2", "u"...)
    // thành 1 mục trong hàng đợi animate (mặt, có-nghịch-đảo, có-180°). ----

    #[test]
    fn queue_move_nuoc_thuong_khong_dau() {
        let mut q = VecDeque::new();
        queue_move(&mut q, "R");
        assert_eq!(q, VecDeque::from([('R', false, false)]));
    }

    #[test]
    fn queue_move_nuoc_nghich_dao() {
        let mut q = VecDeque::new();
        queue_move(&mut q, "R'");
        assert_eq!(q, VecDeque::from([('R', true, false)]));
    }

    #[test]
    fn queue_move_nuoc_180_do_la_mot_muc_khong_tach_thanh_hai() {
        // Quan trọng: R2 phải là 1 mục xoay 180°, KHÔNG tách thành 2 mục
        // 90° -- nếu tách, tổng số mục trong hàng đợi sẽ khác tổng số
        // nước của lời giải, làm lệch thanh tiến trình VÀ lệch `move_count`
        // dùng để đổi nhãn chặng trong "Giải tự động" (xem
        // advance_auto_plan bên dưới).
        let mut q = VecDeque::new();
        queue_move(&mut q, "R2");
        assert_eq!(q.len(), 1);
        assert_eq!(q, VecDeque::from([('R', false, true)]));
    }

    #[test]
    fn queue_move_nuoc_wide_chu_thuong() {
        // Chữ thường (nước wide, VD "u") giữ nguyên ký tự -- move_spec
        // (mesh.rs) phân biệt hoa/thường, queue_move không được tự viết hoa.
        let mut q = VecDeque::new();
        queue_move(&mut q, "u'");
        assert_eq!(q, VecDeque::from([('u', true, false)]));
    }

    #[test]
    fn queue_move_nhieu_nuoc_giu_dung_thu_tu() {
        let mut q = VecDeque::new();
        for m in ["R", "U", "R'", "U'"] {
            queue_move(&mut q, m);
        }
        assert_eq!(
            q,
            VecDeque::from([
                ('R', false, false),
                ('U', false, false),
                ('R', true, false),
                ('U', true, false),
            ])
        );
    }

    // ---- build_auto_plan / build_boundaries / advance_auto_plan: lõi
    // của "Giải tự động" (Enter) -- gom kết quả full_solve_breakdown
    // thành 1 kế hoạch phẳng rồi phát hiện đúng lúc chuyển chặng để đổi
    // nhãn hiển thị. Đây là phần LOGIC MỚI VÀ TINH VI NHẤT vừa thêm,
    // trước đây chỉ được kiểm bằng cách tự chạy app xem có khựng/lệch
    // nhãn không. ----

    fn moves(v: &[&str]) -> StageStatus {
        StageStatus::Moves { moves: v.iter().map(|s| s.to_string()).collect(), case_name: None }
    }

    fn breakdown_giai_binh_thuong() -> Breakdown {
        Breakdown {
            cross: moves(&["F", "R", "U'", "R'"]),
            f2l: [
                ("DFR", moves(&["U", "R", "U'", "R'"])),
                ("DFL", StageStatus::Done),
                ("DBR", moves(&["R", "U", "R'"])),
                ("DBL", moves(&["U'", "L'", "U", "L"])),
            ],
            oll: moves(&["R", "U2", "R2", "U'", "R2", "U'", "R2"]),
            pll: moves(&["R", "U'", "R", "U", "R", "U", "R", "U'", "R'", "U'", "R2"]),
        }
    }

    #[test]
    fn build_auto_plan_bo_qua_chang_da_xong_san_giu_dung_thu_tu() {
        let (plan, failed) = build_auto_plan(&breakdown_giai_binh_thuong());
        assert_eq!(failed, None);
        let labels: Vec<&str> = plan.iter().map(|(l, _)| l.as_str()).collect();
        // DFL đã "Done" (xong sẵn) nên KHÔNG được xuất hiện trong kế hoạch.
        assert_eq!(labels, vec!["Cross", "F2L - cặp DFR", "F2L - cặp DBR", "F2L - cặp DBL", "OLL", "PLL"]);
    }

    #[test]
    fn build_auto_plan_cube_da_giai_xong_ra_ke_hoach_rong() {
        let b = Breakdown {
            cross: StageStatus::Done,
            f2l: [
                ("DFR", StageStatus::Done),
                ("DFL", StageStatus::Done),
                ("DBR", StageStatus::Done),
                ("DBL", StageStatus::Done),
            ],
            oll: StageStatus::Done,
            pll: StageStatus::Done,
        };
        let (plan, failed) = build_auto_plan(&b);
        assert!(plan.is_empty());
        assert_eq!(failed, None);
    }

    #[test]
    fn build_auto_plan_bao_dung_ten_chang_that_bai_dau_tien() {
        let mut b = breakdown_giai_binh_thuong();
        b.oll = StageStatus::Failed;
        b.pll = StageStatus::Failed; // 2 chặng cùng fail
        let (plan, failed) = build_auto_plan(&b);
        // Vẫn giữ các chặng giải được (Cross, 3 cặp F2L) trong kế hoạch.
        assert_eq!(plan.len(), 4);
        // Chỉ báo tên chặng fail ĐẦU TIÊN (OLL), không phải PLL.
        assert_eq!(failed, Some("OLL"));
    }

    #[test]
    fn build_boundaries_moc_dau_luon_la_0_va_cong_don_dung() {
        let plan = vec![
            ("Cross".to_string(), vec!["F".to_string(), "R".to_string()]), // 2 nước
            ("OLL".to_string(), vec!["R".to_string(), "U".to_string(), "R'".to_string()]), // 3 nước
        ];
        let b = build_boundaries(&plan);
        let starts: Vec<usize> = b.iter().map(|(s, _, _)| *s).collect();
        assert_eq!(starts, vec![0, 2]);
    }

    #[test]
    fn advance_auto_plan_chua_toi_moc_thi_khong_doi_nhan() {
        let plan = vec![("Cross".to_string(), vec!["F".to_string(), "R".to_string()])];
        let mut b = build_boundaries(&plan);
        // Bỏ luôn mốc "Cross" (progress=0, mốc 0 <= 0 => sẽ bấm ngay) để
        // còn lại 1 mốc test tiếp theo -- ở đây chỉ có 1 chặng nên test
        // thẳng: progress=0 vẫn bấm được mốc 0 (chặng đầu hiện ngay từ
        // đầu, không đợi gì).
        assert!(advance_auto_plan(&mut b, 0).is_some());
        // Gọi lại lần 2 (không còn mốc nào) -- phải trả None, không panic.
        assert_eq!(advance_auto_plan(&mut b, 5), None);
    }

    #[test]
    fn advance_auto_plan_chuyen_chang_dung_luc_va_bo_qua_chang_giua_neu_nhay_coc() {
        // Cross: 2 nước (mốc 0), F2L: 3 nước (mốc 2), OLL: 1 nước (mốc 5).
        let plan = vec![
            ("Cross".to_string(), vec!["A".to_string(), "B".to_string()]),
            ("F2L".to_string(), vec!["C".to_string(), "D".to_string(), "E".to_string()]),
            ("OLL".to_string(), vec!["F".to_string()]),
        ];
        let mut b = build_boundaries(&plan);
        // Chặng đầu (Cross, mốc 0) coi như đã hiện thủ công lúc khởi tạo
        // kế hoạch (xem JobResult::AutoPlan trong main.rs) -- ở đây bỏ nó
        // ra trước cho giống thực tế, chỉ còn F2L/OLL trong boundaries.
        b.pop_front();

        // Mới chạy xong 1 nước (progress=1) -- vẫn chưa tới mốc F2L (2).
        assert_eq!(advance_auto_plan(&mut b, 1), None);

        // Chạy xong nước thứ 2 (progress=2) -- vừa chạm mốc F2L.
        let (label, moves) = advance_auto_plan(&mut b, 2).unwrap();
        assert_eq!(label, "F2L");
        assert_eq!(moves, vec!["C", "D", "E"]);

        // Nhảy thẳng progress=5 (giả sử khung hình bị trễ, bỏ lỡ vài lần
        // kiểm tra) -- phải nhảy thẳng tới OLL (mốc 5), không kẹt ở giữa.
        let (label, moves) = advance_auto_plan(&mut b, 5).unwrap();
        assert_eq!(label, "OLL");
        assert_eq!(moves, vec!["F"]);

        // Hết mốc -- gọi thêm không panic, trả None.
        assert_eq!(advance_auto_plan(&mut b, 99), None);
    }
}

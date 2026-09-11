//! hint.rs — port của cfop_ai.hint() (Python): chỉ tính BƯỚC TIẾP THEO
//! duy nhất (Cross / 1 cặp F2L / OLL-cạnh / OLL-góc / PLL), KHÔNG thực
//! thi -- trả về (nhãn, chuỗi nước) để UI hiển thị, người dùng tự quyết
//! định có áp dụng hay không.
//!
//! retry_seed: Some(seed) xáo thứ tự duyệt nước đi trong các pha search
//! (F2L, OLL phase A/B) -- dùng khi should_retry_hint() (main.rs) phát
//! hiện người dùng bấm gợi ý lại cho CÙNG giai đoạn vừa thất bại mà cube
//! chưa đổi gì (tìm kiếm vốn tất định nên gọi lại y hệt sẽ ra y hệt).

use crate::cross_solver;
use crate::cube::CubeState;
use crate::f2l_solver;
use crate::full_state::{self, F2L_ORDER};
use crate::macro_solver;
use crate::oll_algorithms;
use crate::oll_solver;
use crate::pll_algorithms;

pub struct Hint {
    pub label: String,
    pub moves: Vec<String>,
    /// Giai đoạn CFOP hiện tại ("cross"/"f2l"/"oll"/"pll"/"done") -- dùng
    /// bởi should_retry_hint() để so sánh giữa 2 lần bấm gợi ý liên tiếp.
    pub stage: &'static str,
    /// Tên ca OLL/PLL nếu gợi ý này nhận ra được một ca CỤ THỂ.
    ///
    /// `None` cho Cross, F2L, và cho hai bước 2-look ("định hướng 4 cạnh",
    /// "định hướng 4 góc") — đó là bước lẻ chứ không phải một ca, nên
    /// không có gì để ghi vào nhật ký.
    ///
    /// VÌ SAO LÀ TRƯỜNG RIÊNG: tên vốn đã nằm trong `label` dưới dạng
    /// "OLL - OLL27 (1-look)", nhưng bóc tên ra từ chuỗi hiển thị là kiểu
    /// code gãy ngay khi ai đó sửa câu chữ. Để riêng thì không gãy được.
    pub case_name: Option<String>,
}

pub fn stage_of(state: &CubeState) -> &'static str {
    let full = full_state::from_facelets(state);
    if !full_state::cross_ok(&full) {
        return "cross";
    }
    let done = F2L_ORDER.iter().filter(|&&s| full_state::pair_ok(&full, s)).count();
    if done < F2L_ORDER.len() {
        return "f2l";
    }
    if !(full_state::u_edges_oriented(&full) && full_state::u_corners_oriented(&full)) {
        return "oll";
    }
    if !state.is_solved() {
        return "pll";
    }
    "done"
}

/// Tính gợi ý cho bước tiếp theo từ `state` hiện tại. Không thay đổi
/// `state`. Giống `_hint_raw()` bản Python.
pub fn compute_hint(state: &CubeState, retry_seed: Option<u64>) -> Hint {
    let stage = stage_of(state);
    match stage {
        "cross" => {
            let mvs = cross_solver::solve_cross(state, 20);
            Hint { label: "Cross".to_string(), moves: crate::move_simplify::simplify(&mvs), stage, case_name: None }
        }
        "f2l" => {
            let cm = cross_solver::solve_cross(state, 20);
            let mut s = *state;
            s.apply_sequence(&cm);
            let full = full_state::from_facelets(&s);
            let done: Vec<&'static str> =
                F2L_ORDER.iter().copied().filter(|&sl| full_state::pair_ok(&full, sl)).collect();
            let target = F2L_ORDER.iter().find(|&&sl| !done.contains(&sl)).copied().unwrap();
            let res = f2l_solver::solve_f2l_seeded(full, &[8, 10, 12, 14], 400_000, retry_seed);
            match res.per_slot.get(target).cloned().flatten() {
                Some(mvs) => Hint { label: format!("F2L - cap {target}"), moves: crate::move_simplify::simplify(&mvs), stage, case_name: None },
                None => Hint {
                    label: format!("F2L - cap {target} (chua tim duoc, thu lai)"),
                    moves: vec![],
                    stage,
                    case_name: None,
                },
            }
        }
        "oll" => {
            let cm = cross_solver::solve_cross(state, 20);
            let mut s = *state;
            s.apply_sequence(&cm);
            let full_f2l = full_state::from_facelets(&s);
            let f2l_res = f2l_solver::solve_f2l_seeded(full_f2l, &[8, 10, 12, 14], 400_000, None);
            s.apply_sequence(&f2l_res.moves);

            let full = full_state::from_facelets(&s);
            if retry_seed.is_none() {
                if let Some((prefix, mvs, name)) = oll_algorithms::solve_oll_with_auf(full) {
                    let mut all = prefix;
                    all.extend(mvs);
                    return Hint {
                        label: format!("OLL - {name} (1-look)"),
                        moves: crate::move_simplify::simplify(&all),
                        stage,
                        case_name: Some(name.to_string()),
                    };
                }
            }
            if !full_state::u_edges_oriented(&full) {
                match oll_solver::solve_phase_a(full, retry_seed) {
                    Some(mvs) => Hint { label: "OLL - Dinh huong 4 canh".to_string(), moves: crate::move_simplify::simplify(&mvs), stage, case_name: None },
                    None => Hint {
                        label: "OLL - Dinh huong 4 canh (chua tim duoc, thu lai)".to_string(),
                        moves: vec![],
                        stage,
                        case_name: None,
                    },
                }
            } else {
                let mvs = if retry_seed.is_none() {
                    macro_solver::solve_ocll_macro(full, 4).or_else(|| oll_solver::solve_phase_b(full, retry_seed))
                } else {
                    oll_solver::solve_phase_b(full, retry_seed)
                };
                match mvs {
                    Some(mvs) => Hint { label: "OLL - Dinh huong 4 goc".to_string(), moves: crate::move_simplify::simplify(&mvs), stage, case_name: None },
                    None => Hint {
                        label: "OLL - Dinh huong 4 goc (chua tim duoc, thu lai)".to_string(),
                        moves: vec![],
                        stage,
                        case_name: None,
                    },
                }
            }
        }
        "pll" => {
            let cm = cross_solver::solve_cross(state, 20);
            let mut s = *state;
            s.apply_sequence(&cm);
            let full_f2l = full_state::from_facelets(&s);
            let f2l_res = f2l_solver::solve_f2l_seeded(full_f2l, &[8, 10, 12, 14], 400_000, None);
            s.apply_sequence(&f2l_res.moves);
            let full_oll = full_state::from_facelets(&s);
            if let Some((prefix, mvs, _)) = oll_algorithms::solve_oll_with_auf(full_oll) {
                s.apply_sequence(&prefix);
                s.apply_sequence(&mvs);
            }

            let full = full_state::from_facelets(&s);
            if retry_seed.is_none() {
                let (mvs, name) = pll_algorithms::solve_pll_lookup_named(full);
                if let Some(m) = mvs {
                    let cname = name.unwrap();
                    return Hint {
                        label: format!("PLL - {cname}"),
                        moves: crate::move_simplify::simplify(&m),
                        stage,
                        case_name: Some(cname.to_string()),
                    };
                }
            }
            match macro_solver::solve_pll_macro(full, 4) {
                Some(m) => Hint { label: "PLL - Hoan vi cuoi cung".to_string(), moves: crate::move_simplify::simplify(&m), stage, case_name: None },
                None => Hint {
                    label: "PLL (chua tim duoc, thu lai)".to_string(),
                    moves: vec![],
                    stage,
                    case_name: None,
                },
            }
        }
        _ => Hint { label: "Cube da giai xong!".to_string(), moves: vec![], stage, case_name: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::CubeState;

    /// Ap chuoi nuoc len khoi da giai.
    fn after(seq: &[&str]) -> CubeState {
        let mut st = CubeState::solved();
        st.apply_sequence(seq);
        st
    }

    #[test]
    fn goi_y_oll_kem_ten_ca() {
        // Ap Sune len khoi da giai -> dang o giai doan OLL.
        let st = after(&["R", "U", "R'", "U", "R", "U2", "R'"]);
        let h = compute_hint(&st, None);
        assert_eq!(h.stage, "oll");
        let name = h.case_name.expect("gợi ý OLL phải kèm tên ca");
        // Ten phai la mot ca CO THAT trong bang, khong phai chuoi bia.
        assert!(
            crate::oll_algorithms::all_algorithms().iter().any(|(n, _)| *n == name),
            "tên ca {name} không có trong bảng OLL"
        );
        // Va phai khop voi ten ma bang tra ra cho chinh trang thai nay.
        let full = full_state::from_facelets(&st);
        let (_, _, tra) = crate::oll_algorithms::solve_oll_with_auf(full).unwrap();
        assert_eq!(name, tra, "tên trong gợi ý lệch với tên bảng tra");
    }

    #[test]
    fn goi_y_pll_kem_ten_ca() {
        // T-perm: lop U da dinh huong xong, chi con hoan vi -> giai doan PLL.
        let st = after(&[
            "R", "U", "R'", "U'", "R'", "F", "R2", "U'", "R'", "U'", "R", "U", "R'", "F'",
        ]);
        let h = compute_hint(&st, None);
        assert_eq!(h.stage, "pll");
        let name = h.case_name.expect("gợi ý PLL phải kèm tên ca");
        assert!(
            crate::pll_algorithms::all_algorithms().iter().any(|(n, _)| *n == name),
            "tên ca {name} không có trong bảng PLL"
        );
    }

    #[test]
    fn goi_y_cross_va_f2l_khong_co_ten_ca() {
        // Khoi xao lung tung -> dang o Cross hoac F2L, khong phai mot ca
        // OLL/PLL nao ca, nen khong duoc ghi gi vao nhat ky.
        let st = after(&["R", "U", "F", "L2", "D", "B'", "R2", "U'", "F2", "L"]);
        let h = compute_hint(&st, None);
        assert!(
            h.stage == "cross" || h.stage == "f2l",
            "trang thai thu nghiem khong con o Cross/F2L: {}",
            h.stage
        );
        assert!(h.case_name.is_none(), "Cross/F2L không được kèm tên ca");
    }

    #[test]
    fn khoi_da_giai_thi_khong_co_ten_ca() {
        let h = compute_hint(&CubeState::solved(), None);
        assert_eq!(h.stage, "done");
        assert!(h.case_name.is_none());
    }
}

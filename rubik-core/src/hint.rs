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
            Hint { label: "Cross".to_string(), moves: crate::move_simplify::simplify(&mvs), stage }
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
                Some(mvs) => Hint { label: format!("F2L - cap {target}"), moves: crate::move_simplify::simplify(&mvs), stage },
                None => Hint {
                    label: format!("F2L - cap {target} (chua tim duoc, thu lai)"),
                    moves: vec![],
                    stage,
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
                    return Hint { label: format!("OLL - {name} (1-look)"), moves: crate::move_simplify::simplify(&all), stage };
                }
            }
            if !full_state::u_edges_oriented(&full) {
                match oll_solver::solve_phase_a(full, retry_seed) {
                    Some(mvs) => Hint { label: "OLL - Dinh huong 4 canh".to_string(), moves: crate::move_simplify::simplify(&mvs), stage },
                    None => Hint {
                        label: "OLL - Dinh huong 4 canh (chua tim duoc, thu lai)".to_string(),
                        moves: vec![],
                        stage,
                    },
                }
            } else {
                let mvs = if retry_seed.is_none() {
                    macro_solver::solve_ocll_macro(full, 4).or_else(|| oll_solver::solve_phase_b(full, retry_seed))
                } else {
                    oll_solver::solve_phase_b(full, retry_seed)
                };
                match mvs {
                    Some(mvs) => Hint { label: "OLL - Dinh huong 4 goc".to_string(), moves: crate::move_simplify::simplify(&mvs), stage },
                    None => Hint {
                        label: "OLL - Dinh huong 4 goc (chua tim duoc, thu lai)".to_string(),
                        moves: vec![],
                        stage,
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
                    return Hint { label: format!("PLL - {}", name.unwrap()), moves: crate::move_simplify::simplify(&m), stage };
                }
            }
            match macro_solver::solve_pll_macro(full, 4) {
                Some(m) => Hint { label: "PLL - Hoan vi cuoi cung".to_string(), moves: crate::move_simplify::simplify(&m), stage },
                None => Hint {
                    label: "PLL (chua tim duoc, thu lai)".to_string(),
                    moves: vec![],
                    stage,
                },
            }
        }
        _ => Hint { label: "Cube da giai xong!".to_string(), moves: vec![], stage },
    }
}

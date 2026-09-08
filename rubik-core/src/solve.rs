//! solve.rs — gộp Cross->F2L->OLL->PLL thành 1 hàm duy nhất cho UI gọi,
//! rút gọn từ main.rs của rubik-rs (cùng logic, cùng retry F2L khi bí).

use crate::cross_solver;
use crate::cube::CubeState;
use crate::f2l_solver;
use crate::full_state;
use crate::macro_solver;
use crate::oll_algorithms;
use crate::oll_solver;
use crate::pll_algorithms;

pub fn solve_oll(full: full_state::FullState) -> Option<Vec<&'static str>> {
    if let Some((prefix, mvs, _name)) = oll_algorithms::solve_oll_with_auf(full) {
        let mut all = prefix;
        all.extend(mvs);
        return Some(all);
    }
    if full_state::u_edges_oriented(&full) {
        if let Some(mvs) = macro_solver::solve_ocll_macro(full, 4) {
            return Some(mvs);
        }
    }
    oll_solver::solve_oll_search(full, None)
}

pub fn solve_pll(full: full_state::FullState) -> Option<Vec<&'static str>> {
    let (mvs, _name) = pll_algorithms::solve_pll_lookup_named(full);
    if let Some(m) = mvs {
        return Some(m);
    }
    macro_solver::solve_pll_macro(full, 4)
}

/// Giải trọn vẹn `state` hiện tại, trả về TOÀN BỘ chuỗi nước đi (không áp
/// dụng lên `state` -- caller tự animate rồi apply). Thử tối đa
/// `max_retries` lần xáo trộn F2L nếu OLL/PLL bí. None nếu cả mọi lần đều
/// thất bại (hiếm, ~5-10% theo benchmark ở rubik-rs).
pub fn solve_full_cube(state: &CubeState, max_retries: u32) -> Option<Vec<String>> {
    let cm = cross_solver::solve_cross(state, 20);
    let mut s0 = *state;
    s0.apply_sequence(&cm);
    let f_after_cross = full_state::from_facelets(&s0);

    for attempt in 0..=max_retries {
        let retry_seed = if attempt == 0 { None } else { Some(attempt as u64 * 7919) };
        let res = f2l_solver::solve_f2l_seeded(f_after_cross, &[8, 10, 12, 14], 400_000, retry_seed);
        if res.solved_slots.len() != 4 {
            continue;
        }
        let mut s = s0;
        s.apply_sequence(&res.moves);

        let f2 = full_state::from_facelets(&s);
        let oll_mvs = match solve_oll(f2) {
            Some(x) => x,
            None => continue,
        };
        s.apply_sequence(&oll_mvs);

        let f3 = full_state::from_facelets(&s);
        let pll_mvs = match solve_pll(f3) {
            Some(x) => x,
            None => continue,
        };
        s.apply_sequence(&pll_mvs);

        if s.is_solved() {
            let mut all = cm.clone();
            all.extend(res.moves);
            all.extend(oll_mvs);
            all.extend(pll_mvs);
            return Some(crate::move_simplify::simplify(&all));
        }
    }
    None
}

//! macro_solver.rs — port of the macro-search fallback trong oll_solver.py
//! (_solve_ocll_macro) và pll_solver.py (_solve_pll_macro).
//!
//! Ý tưởng: thay vì tìm kiếm tổng quát từng nước đơn (15^depth, dễ chậm/
//! thất bại — xem ghi chú trong oll_solver.py), duyệt trên KHÔNG GIAN MACRO
//! (mỗi bước = 1 trong vài "chiêu" CFOP thật nổi tiếng, đã biết chắc chắn
//! bảo toàn Cross+F2L). Không gian nhỏ hẳn (12-20^depth thay vì 15^depth)
//! nên nhanh và gần như luôn thành công — đúng cách người chơi CFOP thật
//! xử lý khi không nhận diện được case cụ thể.

use crate::full_state::{apply_move, u_corners_oriented, u_corners_home, u_edges_home, FullState};

// ── OCLL macro (Sune / Anti-Sune × 4 AUF) — dùng khi bảng OLL đầy đủ miss,
// pha B (định hướng góc) của kiến trúc 2-look. ─────────────────────────────
const SUNE: [&str; 7] = ["R", "U", "R'", "U", "R", "U2", "R'"];
const ANTISUNE: [&str; 7] = ["R", "U2", "R'", "U'", "R", "U'", "R'"];

fn ocll_macros() -> Vec<Vec<&'static str>> {
    let mut out = Vec::new();
    for auf in [vec![], vec!["U"], vec!["U2"], vec!["U'"]] {
        for alg in [SUNE.to_vec(), ANTISUNE.to_vec()] {
            let mut m = auf.clone();
            m.extend(alg);
            out.push(m);
        }
    }
    out
}

/// Tìm chuỗi macro (AUF + Sune/Anti-Sune) ngắn nhất định hướng 4 góc U.
/// Giả định cạnh U đã dùng hướng sẵn (2-look phase B). Không gian tìm
/// kiếm chỉ 8^depth (8 macro mỗi bước) nên rất nhanh (<1ms thực tế).
pub fn solve_ocll_macro(full: FullState, max_depth: u32) -> Option<Vec<&'static str>> {
    if u_corners_oriented(&full) {
        return Some(vec![]);
    }
    let macros = ocll_macros();
    let mut frontier: Vec<(FullState, Vec<&'static str>)> = vec![(full, vec![])];
    for _ in 0..max_depth {
        let mut next_frontier = Vec::new();
        for (state, path) in frontier {
            for mv_seq in &macros {
                let mut s = state;
                for &mv in mv_seq {
                    s = apply_move(&s, mv);
                }
                if u_corners_oriented(&s) {
                    let mut full_path = path.clone();
                    full_path.extend(mv_seq.iter().copied());
                    return Some(full_path);
                }
                let mut full_path = path.clone();
                full_path.extend(mv_seq.iter().copied());
                next_frontier.push((s, full_path));
            }
        }
        frontier = next_frontier;
    }
    None
}

// ── PLL macro (T-perm / T-perm-mirror / Y-perm / Corner-3-cycle / Edge-3-
// cycle × 4 AUF) — lưới an toàn khi bảng 21 PLL chuẩn miss. Theo comment
// gốc: 200/200 case tổng hợp + 10/10 case thật đều thành công, luôn <0.13s.
const TPERM: [&str; 14] = ["R", "U", "R'", "U'", "R'", "F", "R2", "U'", "R'", "U'", "R", "U", "R'", "F'"];
const TPERM_MIRROR: [&str; 14] = ["L'", "U'", "L", "U", "L", "F'", "L2", "U", "L", "U", "L'", "U'", "L", "F"];
const YPERM: [&str; 17] = ["F", "R", "U'", "R'", "U'", "R", "U", "R'", "F'", "R", "U", "R'", "U'", "R'", "F", "R", "F'"];
const CORNER3: [&str; 9] = ["R'", "F", "R'", "B2", "R", "F'", "R'", "B2", "R2"];
const EDGE3: [&str; 11] = ["R", "U'", "R", "U", "R", "U", "R", "U'", "R'", "U'", "R2"];

fn pll_macros() -> Vec<Vec<&'static str>> {
    let mut out = Vec::new();
    for auf in [vec![], vec!["U"], vec!["U2"], vec!["U'"]] {
        for alg in [
            TPERM.to_vec(),
            TPERM_MIRROR.to_vec(),
            YPERM.to_vec(),
            CORNER3.to_vec(),
            EDGE3.to_vec(),
        ] {
            let mut m = auf.clone();
            m.extend(alg);
            out.push(m);
        }
    }
    out
}

fn pll_done(full: &FullState) -> bool {
    u_corners_home(full) && u_edges_home(full)
}

/// Tìm chuỗi macro giải TOÀN BỘ PLL cùng lúc (không tách 2-look). Không
/// gian chỉ 20^depth (20 macro mỗi bước, 4 AUF × 5 generator).
pub fn solve_pll_macro(full: FullState, max_depth: u32) -> Option<Vec<&'static str>> {
    if pll_done(&full) {
        return Some(vec![]);
    }
    let macros = pll_macros();
    let mut frontier: Vec<(FullState, Vec<&'static str>)> = vec![(full, vec![])];
    for _ in 0..max_depth {
        let mut next_frontier = Vec::new();
        for (state, path) in frontier {
            for mv_seq in &macros {
                let mut s = state;
                for &mv in mv_seq {
                    s = apply_move(&s, mv);
                }
                let mut full_path = path.clone();
                full_path.extend(mv_seq.iter().copied());
                if pll_done(&s) {
                    return Some(full_path);
                }
                next_frontier.push((s, full_path));
            }
        }
        frontier = next_frontier;
    }
    None
}

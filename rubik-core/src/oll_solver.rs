//! oll_solver.rs — port of the search-based fallback trong solver/oll_solver.py
//! (dùng SAU KHI bảng 55-case đầy đủ VÀ OCLL macro đều miss -- trường hợp
//! hiếm, ~2.5% theo benchmark trước đó). 2 pha:
//!   A. Định hướng 4 cạnh U (khong quan tam vi tri) -- A* voi heuristic =
//!      max(PDB nhom canh, can duoi Cross+F2L).
//!   B. Định hướng 4 góc U (sau khi cạnh đã xong) -- thử OCLL macro trước
//!      (macro_solver.rs), chỉ dùng A* nếu macro cũng thất bại (rất hiếm).

use crate::corner_model as CM;
use crate::cross_solver;
use crate::cube::move_index;
use crate::edge_model as EM;
use crate::f2l_solver;
use crate::full_state::{apply_move, cross_f2l_ok, no_d_moves, u_corners_oriented, u_edges_oriented, FullState};
use crate::macro_solver;
use crate::pdb::{build_group_pdb_corners_anyperm, build_group_pdb_edges_anyperm, CornerGroupPdb, CrossPdb};
use crate::search::a_star_ladder;
use std::sync::OnceLock;

static EDGE_PDB: OnceLock<CrossPdb> = OnceLock::new();
static CORNER_PDB: OnceLock<CornerGroupPdb> = OnceLock::new();

fn edge_pdb() -> &'static CrossPdb {
    EDGE_PDB.get_or_init(build_group_pdb_edges_anyperm)
}
fn corner_pdb() -> &'static CornerGroupPdb {
    CORNER_PDB.get_or_init(build_group_pdb_corners_anyperm)
}

const F2L_ORDER: [&str; 4] = ["DFR", "DFL", "DBR", "DBL"];
fn f2l_edge_of(slot: &str) -> &'static str {
    match slot {
        "DFR" => "FR",
        "DFL" => "FL",
        "DBR" => "BR",
        "DBL" => "BL",
        _ => unreachable!(),
    }
}

/// Cận dưới THỰC SỰ (không phải cờ 0/1) cho số bước cần để phục hồi
/// Cross+F2L nếu đang bị vỡ -- tái sử dụng nguyên PDB của cross_solver.rs
/// và f2l_solver.rs (không xây PDB mới). Giống _cross_f2l_lower_bound().
fn cross_f2l_lower_bound(full: &FullState) -> u32 {
    let (ep, eo, cp, co) = full;
    let cross_pos: [usize; 4] = [ep[4], ep[5], ep[6], ep[7]];
    let cross_ori: [u8; 4] = [eo[4], eo[5], eo[6], eo[7]];
    let mut best = cross_solver::get_pdb().get(&cross_pos, &cross_ori).unwrap_or(8) as u32;
    for &slot in F2L_ORDER.iter() {
        let ci = CM::slot_index(slot);
        let ei = EM::slot_index(f2l_edge_of(slot));
        let d = f2l_solver::pair_pdb_get(slot, ep[ei], eo[ei], cp[ci], co[ci]).unwrap_or(8) as u32;
        if d > best {
            best = d;
        }
    }
    best
}

fn edge_key(full: &FullState) -> ([usize; 4], [u8; 4]) {
    let (ep, eo, _, _) = full;
    ([ep[0], ep[1], ep[2], ep[3]], [eo[0], eo[1], eo[2], eo[3]])
}
fn corner_key(full: &FullState) -> ([usize; 4], [u8; 4]) {
    let (_, _, cp, co) = full;
    ([cp[0], cp[1], cp[2], cp[3]], [co[0], co[1], co[2], co[3]])
}

fn heuristic_a(full: &FullState) -> u32 {
    let (pos, ori) = edge_key(full);
    let base = edge_pdb().get(&pos, &ori).unwrap_or(8) as u32;
    base.max(cross_f2l_lower_bound(full))
}
fn goal_a(full: &FullState) -> bool {
    cross_f2l_ok(full) && u_edges_oriented(full)
}

fn heuristic_b(full: &FullState) -> u32 {
    let (pos, ori) = corner_key(full);
    let mut base = corner_pdb().get(&pos, &ori).unwrap_or(8) as u32;
    base = base.max(cross_f2l_lower_bound(full));
    if !u_edges_oriented(full) {
        base = base.max(1);
    }
    base
}
fn goal_b(full: &FullState) -> bool {
    cross_f2l_ok(full) && u_edges_oriented(full) && u_corners_oriented(full)
}

/// Pha A: định hướng 4 cạnh U bằng A* tổng quát (PDB nhóm + cận dưới
/// Cross+F2L). Dùng KHI bảng 55-case OLL đầy đủ miss. `retry_seed`:
/// Some(seed) xáo thứ tự duyệt nước đi (tìm kiếm vốn tất định nên bấm lại
/// y hệt sẽ ra y hệt -- xem should_retry_hint trong main.rs).
/// Hai công thức định hướng cạnh của 2-look OLL. Chỉ dùng 18 nước cơ bản
/// (bắt buộc: `apply_move` không hiểu `f`/`M`).
const EO_ALGS: [&[&str]; 2] = [
    &["F", "R", "U", "R'", "U'", "F'"],
    &["F", "U", "R", "U'", "R'", "F'"],
];
const EO_AUF: [&[&str]; 4] = [&[], &["U"], &["U2"], &["U'"]];

/// Định hướng 4 cạnh lớp U bằng đúng hai công thức trên, thử mọi tổ hợp
/// AUF, tối đa hai lượt.
///
/// VÌ SAO CÓ HÀM NÀY: pha A trước đây dùng A* tổng quát, và nó THẤT BẠI ở
/// cả 8 thế "chấm" (4 cạnh sai hướng) — đo được 8/57 thế OLL không giải
/// được, mất 7,6 giây mỗi lần rồi mới chịu thua. Nguyên nhân: quãng đường
/// thật của thế chấm là hơn 6 nước, mà chỉ riêng duyệt tới độ sâu 6 đã cần
/// hơn 2 triệu trạng thái, trong khi thang tìm kiếm dừng ở 300k nút. Nới
/// thang lên thì chậm tới hàng phút, không dùng được.
///
/// Định hướng cạnh vốn chỉ có 3 thế (line / L / chấm) nên A* tổng quát là
/// thừa. Hàm này thử 8 khả năng một lượt và 64 khả năng hai lượt — vài
/// chục phép áp nước, xong tức thì.
///
/// TỰ KIỂM CHỨNG: chỉ trả về chuỗi đã ÁP THẬT và xác nhận 4 cạnh U đúng
/// hướng, Cross/F2L còn nguyên. Không dựa vào bảng tra hay quy ước khoá
/// nào cả.
fn solve_eo_two_look(full: FullState) -> Option<Vec<&'static str>> {
    let apply = |s: FullState, seq: &[&'static str]| -> FullState {
        let mut o = s;
        for &mv in seq {
            o = apply_move(&o, mv);
        }
        o
    };
    let done = |s: &FullState| cross_f2l_ok(s) && u_edges_oriented(s);
    if done(&full) {
        return Some(vec![]);
    }
    // Một lượt.
    for auf in EO_AUF {
        for alg in EO_ALGS {
            let mut out: Vec<&'static str> = auf.to_vec();
            out.extend(alg.iter().copied());
            if done(&apply(full, &out)) {
                return Some(out);
            }
        }
    }
    // Hai lượt (thế "chấm" cần đúng hai lượt).
    for auf1 in EO_AUF {
        for alg1 in EO_ALGS {
            let mut mid: Vec<&'static str> = auf1.to_vec();
            mid.extend(alg1.iter().copied());
            let s1 = apply(full, &mid);
            for auf2 in EO_AUF {
                for alg2 in EO_ALGS {
                    let mut out = mid.clone();
                    out.extend(auf2.iter().copied());
                    out.extend(alg2.iter().copied());
                    if done(&apply(full, &out)) {
                        return Some(out);
                    }
                }
            }
            let _ = s1;
        }
    }
    None
}

pub fn solve_phase_a(full: FullState, retry_seed: Option<u64>) -> Option<Vec<&'static str>> {
    // Thử cách 2-look trước: nhanh và phủ trọn. Chỉ khi nó trượt mới rơi
    // về A* tổng quát (giữ lại làm lưới an toàn, và để `retry_seed` vẫn
    // sinh được lời giải khác cho nút "thử lại").
    if retry_seed.is_none() {
        if let Some(mvs) = solve_eo_two_look(full) {
            return Some(mvs);
        }
    }
    let mut no_d = no_d_moves();
    if let Some(seed) = retry_seed {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;
        no_d.shuffle(&mut rand::rngs::StdRng::seed_from_u64(seed));
    }
    let move_table: Vec<(usize, &'static str, char)> =
        no_d.iter().map(|&m| (move_index(m), m, m.chars().next().unwrap())).collect();
    a_star_ladder(full, goal_a, heuristic_a, &[(80_000, 8), (200_000, 10), (300_000, 12)], &move_table)
}

/// Ban debug: cho phep chi dinh tiers tuy y (dung de do xem can do sau bao
/// nhieu thi giai duoc cac case kho).
pub fn solve_phase_a_tiers(full: FullState, tiers: &[(u32, u32)]) -> Option<Vec<&'static str>> {
    let no_d = no_d_moves();
    let move_table: Vec<(usize, &'static str, char)> =
        no_d.iter().map(|&m| (move_index(m), m, m.chars().next().unwrap())).collect();
    a_star_ladder(full, goal_a, heuristic_a, tiers, &move_table)
}

/// Pha B: định hướng 4 góc U (giả sử cạnh đã xong). Ưu tiên OCLL macro
/// (nhanh, đã kiểm chứng); chỉ rơi về A* tổng quát nếu macro cũng miss.
pub fn solve_phase_b(full: FullState, retry_seed: Option<u64>) -> Option<Vec<&'static str>> {
    if retry_seed.is_none() {
        if let Some(mvs) = macro_solver::solve_ocll_macro(full, 4) {
            return Some(mvs);
        }
    }
    let mut no_d = no_d_moves();
    if let Some(seed) = retry_seed {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;
        no_d.shuffle(&mut rand::rngs::StdRng::seed_from_u64(seed));
    }
    let move_table: Vec<(usize, &'static str, char)> =
        no_d.iter().map(|&m| (move_index(m), m, m.chars().next().unwrap())).collect();
    a_star_ladder(full, goal_b, heuristic_b, &[(80_000, 9), (150_000, 11), (300_000, 13)], &move_table)
}

/// Giải OLL bằng kiến trúc 2-look đầy đủ (pha A rồi pha B) -- dùng làm lưới
/// an toàn cuối cùng khi CẢ bảng 55-case CHỈNH lẫn OCLL-macro-trực-tiếp
/// đều miss (trường hợp hiếm: cạnh U chưa đúng hướng VÀ không khớp bảng).
pub fn solve_oll_search(full: FullState, retry_seed: Option<u64>) -> Option<Vec<&'static str>> {
    let edge_moves = solve_phase_a(full, retry_seed)?;
    let mut s = full;
    for &mv in &edge_moves {
        s = crate::full_state::apply_move(&s, mv);
    }
    let corner_moves = solve_phase_b(s, retry_seed)?;
    let mut all = edge_moves;
    all.extend(corner_moves);
    Some(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oll_algorithms;

    fn state_with(k: oll_algorithms::OriKey) -> FullState {
        let mut ep = [0usize; 12];
        let mut cp = [0usize; 8];
        for i in 0..12 {
            ep[i] = i;
        }
        for i in 0..8 {
            cp[i] = i;
        }
        let mut eo = [0u8; 12];
        let mut co = [0u8; 8];
        for i in 0..4 {
            eo[i] = k.0[i];
            co[i] = k.1[i];
        }
        (ep, eo, cp, co)
    }

    #[test]
    fn luoi_an_toan_giai_duoc_ca_57_the() {
        // `solve_oll_search` là lưới an toàn cuối cùng khi bảng OLL miss.
        // Trước đây nó THẤT BẠI ở cả 8 thế "chấm" (49/57) và mất 7,6 giây
        // mỗi lần mới chịu thua — tức là lưới thủng đúng chỗ cần nhất.
        // Giờ phải giải được cả 57, và phải nhanh.
        let t = oll_algorithms::table();
        let mut fail = Vec::new();
        for key in t.table.keys() {
            let name = t.name_table.get(key).copied().unwrap_or("?");
            match solve_oll_search(state_with(*key), None) {
                None => fail.push(name),
                Some(mvs) => {
                    // Không tin lời giải: áp thật rồi kiểm tra.
                    let mut s = state_with(*key);
                    for &mv in &mvs {
                        s = apply_move(&s, mv);
                    }
                    assert!(
                        cross_f2l_ok(&s) && u_edges_oriented(&s) && u_corners_oriented(&s),
                        "{name}: lời giải không định hướng xong lớp U"
                    );
                }
            }
        }
        assert!(fail.is_empty(), "không giải được {} thế: {fail:?}", fail.len());
    }

    #[test]
    fn buoc_dinh_huong_canh_phu_moi_the() {
        // Chỉ có 4 lớp hướng cạnh (đã xong / line / L / chấm) nên hai công
        // thức 2-look phải phủ hết. Quét mọi kiểu lật cạnh hợp lệ.
        for e in 0..16u8 {
            let eo4 = [e & 1, (e >> 1) & 1, (e >> 2) & 1, (e >> 3) & 1];
            if eo4.iter().sum::<u8>() % 2 != 0 {
                continue; // số cạnh lật phải chẵn
            }
            let st = state_with((eo4, [0, 0, 0, 0]));
            let got = solve_eo_two_look(st);
            assert!(got.is_some(), "không định hướng được cạnh {eo4:?}");
            let mut s = st;
            for mv in got.unwrap() {
                s = apply_move(&s, mv);
            }
            assert!(cross_f2l_ok(&s) && u_edges_oriented(&s), "cạnh {eo4:?} giải sai");
        }
    }
}

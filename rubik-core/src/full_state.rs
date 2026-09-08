//! full_state.rs — port of solver/full_state.py

use crate::corner_model as CM;
use crate::cube::{move_index, CubeState, ALL_MOVES};
use crate::edge_model as EM;

/// (edge_pos[12], edge_ori[12], corner_pos[8], corner_ori[8])
pub type FullState = ([usize; 12], [u8; 12], [usize; 8], [u8; 8]);

/// 15 nước không dùng D (bảo toàn lớp D trong khi ghép F2L).
pub fn no_d_moves() -> Vec<&'static str> {
    ALL_MOVES.iter().copied().filter(|m| !m.starts_with('D')).collect()
}

pub fn from_facelets(state: &CubeState) -> FullState {
    let all_e: Vec<usize> = (0..12).collect();
    let all_c: Vec<usize> = (0..8).collect();
    let (ep, eo) = EM::edges_from_state(state, &all_e);
    let (cp, co) = CM::corners_from_state(state, &all_c);
    let mut fep = [0usize; 12];
    let mut feo = [0u8; 12];
    let mut fcp = [0usize; 8];
    let mut fco = [0u8; 8];
    fep.copy_from_slice(&ep);
    feo.copy_from_slice(&eo);
    fcp.copy_from_slice(&cp);
    fco.copy_from_slice(&co);
    (fep, feo, fcp, fco)
}

pub fn apply_move(full: &FullState, mv: &str) -> FullState {
    apply_move_idx(full, move_index(mv))
}

#[inline]
pub fn apply_move_idx(full: &FullState, mi: usize) -> FullState {
    let (ep, eo, cp, co) = full;
    let (nep, neo) = EM::apply_move_idx_full(ep, eo, mi);
    let (ncp, nco) = CM::apply_move_idx_full(cp, co, mi);
    (nep, neo, ncp, nco)
}

pub fn cross_ok(full: &FullState) -> bool {
    let (ep, eo, _, _) = full;
    ep[4] == 4 && ep[5] == 5 && ep[6] == 6 && ep[7] == 7 && eo[4] == 0 && eo[5] == 0 && eo[6] == 0 && eo[7] == 0
}

/// slot F2L (tên viết hoa, VD "DFR") -> tên khe edge tương ứng.
pub fn f2l_edge_of(slot: &str) -> &'static str {
    match slot {
        "DFR" => "FR",
        "DFL" => "FL",
        "DBR" => "BR",
        "DBL" => "BL",
        _ => panic!("unknown F2L slot {slot}"),
    }
}

pub fn pair_ok(full: &FullState, slot: &str) -> bool {
    let (ep, eo, cp, co) = full;
    let ci = CM::slot_index(slot);
    if cp[ci] != ci || co[ci] != 0 {
        return false;
    }
    let ei = EM::slot_index(f2l_edge_of(slot));
    ep[ei] == ei && eo[ei] == 0
}

pub const F2L_ORDER: [&str; 4] = ["DFR", "DFL", "DBR", "DBL"];

/// True nếu Cross + toàn bộ 4 cặp F2L đều nguyên vẹn — dùng để định hướng
/// tìm kiếm tránh phá hỏng phần đã giải (mismatch penalty).
pub fn cross_f2l_ok(full: &FullState) -> bool {
    let (ep, eo, cp, co) = full;
    ep[4] == 4 && ep[5] == 5 && ep[6] == 6 && ep[7] == 7
        && ep[8] == 8 && ep[9] == 9 && ep[10] == 10 && ep[11] == 11
        && eo[4] == 0 && eo[5] == 0 && eo[6] == 0 && eo[7] == 0
        && eo[8] == 0 && eo[9] == 0 && eo[10] == 0 && eo[11] == 0
        && cp[4] == 4 && cp[5] == 5 && cp[6] == 6 && cp[7] == 7
        && co[4] == 0 && co[5] == 0 && co[6] == 0 && co[7] == 0
}

/// 4 cạnh lớp U (UF,UB,UL,UR) đã đúng hướng (không quan tâm vị trí).
pub fn u_edges_oriented(full: &FullState) -> bool {
    let eo = &full.1;
    eo[0] == 0 && eo[1] == 0 && eo[2] == 0 && eo[3] == 0
}

/// 4 góc lớp U (UFR,UFL,UBR,UBL) đã đúng hướng (không quan tâm vị trí).
pub fn u_corners_oriented(full: &FullState) -> bool {
    let co = &full.3;
    co[0] == 0 && co[1] == 0 && co[2] == 0 && co[3] == 0
}

/// 4 góc lớp U đúng cả vị trí lẫn hướng (PLL: pha góc đã xong).
pub fn u_corners_home(full: &FullState) -> bool {
    let (_, _, cp, co) = full;
    cp[0] == 0 && cp[1] == 1 && cp[2] == 2 && cp[3] == 3
        && co[0] == 0 && co[1] == 0 && co[2] == 0 && co[3] == 0
}

/// 4 cạnh lớp U đúng cả vị trí lẫn hướng (PLL: pha cạnh đã xong).
pub fn u_edges_home(full: &FullState) -> bool {
    let (ep, eo, _, _) = full;
    ep[0] == 0 && ep[1] == 1 && ep[2] == 2 && ep[3] == 3
        && eo[0] == 0 && eo[1] == 0 && eo[2] == 0 && eo[3] == 0
}

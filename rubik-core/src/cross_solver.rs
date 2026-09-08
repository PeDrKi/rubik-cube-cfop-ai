//! cross_solver.rs — port of solver/cross_solver.py
//! PDB cho khoảng cách CHÍNH XÁC -> từ bất kỳ trạng thái nào luôn có ít
//! nhất 1 nước làm khoảng cách giảm đúng 1, nên chỉ cần leo dốc tham lam.

use crate::cube::{CubeState, ALL_MOVES};
use crate::edge_model as EM;
use crate::pdb::{build_cross_pdb, CrossPdb};
use std::sync::OnceLock;

pub const TRACKED: [usize; 4] = [4, 5, 6, 7]; // DF, DB, DL, DR

static PDB: OnceLock<CrossPdb> = OnceLock::new();

pub fn get_pdb() -> &'static CrossPdb {
    PDB.get_or_init(|| build_cross_pdb(&TRACKED))
}

/// Trả về chuỗi nước Singmaster giải Cross tối ưu từ state hiện tại.
/// Không thay đổi state truyền vào.
pub fn solve_cross(state: &CubeState, max_moves: usize) -> Vec<&'static str> {
    let pdb = get_pdb();
    let (pos_v, ori_v) = EM::edges_from_state(state, &TRACKED);
    let mut pos: [usize; 4] = [pos_v[0], pos_v[1], pos_v[2], pos_v[3]];
    let mut ori: [u8; 4] = [ori_v[0], ori_v[1], ori_v[2], ori_v[3]];
    let mut d = pdb.get(&pos, &ori).expect("trạng thái cross không hợp lệ / không nằm trong PDB");

    let mut moves = Vec::new();
    let mut guard = 0;
    while d > 0 && guard < max_moves {
        guard += 1;
        let mut advanced = false;
        for &mv in ALL_MOVES.iter() {
            let (np, no) = EM::apply_move_to_edges(&pos, &ori, mv);
            let np4: [usize; 4] = [np[0], np[1], np[2], np[3]];
            let no4: [u8; 4] = [no[0], no[1], no[2], no[3]];
            if let Some(nd) = pdb.get(&np4, &no4) {
                if nd == d - 1 {
                    pos = np4;
                    ori = no4;
                    moves.push(mv);
                    d = nd;
                    advanced = true;
                    break;
                }
            }
        }
        if !advanced {
            panic!("Không tìm được nước giảm khoảng cách (bug PDB?)");
        }
    }
    moves
}

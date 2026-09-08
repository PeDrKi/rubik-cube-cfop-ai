//! edge_model.rs — port of solver/edge_model.py
//!
//! 12 khe cạnh, mỗi khe = 2 toạ độ facelet (face,row,col). Bảng hoán vị
//! MOVE_PERM/MOVE_FLIP được suy ra THỰC NGHIỆM (giống bản Python): áp mỗi
//! move lên cube đã giải rồi so khớp facelet — không hard-code quan hệ nào,
//! nên tự động đúng nếu cube.rs đúng.

use crate::cube::{move_index, CubeState, ALL_MOVES};
use std::sync::OnceLock;

pub const SLOT_NAMES: [&str; 12] = [
    "UF", "UB", "UL", "UR", "DF", "DB", "DL", "DR", "FR", "FL", "BR", "BL",
];

#[inline]
pub fn slot_index(name: &str) -> usize {
    SLOT_NAMES.iter().position(|&s| s == name).unwrap()
}

// (face_idx, row, col) cho mỗi khe — face_idx theo cube::FACES = U D F B L R = 0..6
const U: usize = 0;
const D: usize = 1;
const F: usize = 2;
const B: usize = 3;
const L: usize = 4;
const R: usize = 5;

type Coord = (usize, usize, usize);
const EDGE_SLOTS: [(Coord, Coord); 12] = [
    ((U, 2, 1), (F, 0, 1)), // UF
    ((U, 0, 1), (B, 0, 1)), // UB
    ((U, 1, 0), (L, 0, 1)), // UL
    ((U, 1, 2), (R, 0, 1)), // UR
    ((D, 0, 1), (F, 2, 1)), // DF
    ((D, 2, 1), (B, 2, 1)), // DB
    ((D, 1, 0), (L, 2, 1)), // DL
    ((D, 1, 2), (R, 2, 1)), // DR
    ((F, 1, 2), (R, 1, 0)), // FR
    ((F, 1, 0), (L, 1, 2)), // FL
    ((R, 1, 2), (B, 1, 0)), // BR
    ((L, 1, 0), (B, 1, 2)), // BL
];

/// (edge_gốc_index, flipped) đang chiếm slot `slot_idx` trong `state`.
fn identify(state: &CubeState, slot_idx: usize, solved: &CubeState) -> (usize, bool) {
    let (c1, c2) = EDGE_SLOTS[slot_idx];
    let v1 = state.facelet_at(c1.0, c1.1, c1.2);
    let v2 = state.facelet_at(c2.0, c2.1, c2.2);
    for orig in 0..12 {
        let (o1, o2) = EDGE_SLOTS[orig];
        let sv1 = solved.facelet_at(o1.0, o1.1, o1.2);
        let sv2 = solved.facelet_at(o2.0, o2.1, o2.2);
        let same_pair = (sv1 == v1 && sv2 == v2) || (sv1 == v2 && sv2 == v1);
        if same_pair {
            let flipped = v1 != sv1;
            return (orig, flipped);
        }
    }
    panic!("edge identify failed at slot {slot_idx}: v1={v1} v2={v2}");
}

pub struct EdgeMoveTables {
    pub perm: [[usize; 12]; 18],
    pub flip: [[u8; 12]; 18],
}

fn build_move_tables() -> EdgeMoveTables {
    let solved = CubeState::solved();
    let mut perm = [[0usize; 12]; 18];
    let mut flip = [[0u8; 12]; 18];
    for (mi, mv) in ALL_MOVES.iter().enumerate() {
        let mut after = CubeState::solved();
        after.do_move(mv);
        for slot_idx in 0..12 {
            // tìm slot nào trong `after` hiện chứa edge gốc = slot_idx
            let mut found = false;
            for cand in 0..12 {
                let (orig, flipped) = identify(&after, cand, &solved);
                if orig == slot_idx {
                    perm[mi][slot_idx] = cand;
                    flip[mi][slot_idx] = flipped as u8;
                    found = true;
                    break;
                }
            }
            assert!(found, "move table inference failed move={mv} slot={slot_idx}");
        }
    }
    EdgeMoveTables { perm, flip }
}

static TABLES: OnceLock<EdgeMoveTables> = OnceLock::new();
pub fn tables() -> &'static EdgeMoveTables {
    TABLES.get_or_init(build_move_tables)
}

pub fn apply_move_to_edges(
    positions: &[usize],
    orients: &[u8],
    mv: &str,
) -> (Vec<usize>, Vec<u8>) {
    let mi = move_index(mv);
    apply_move_idx(positions, orients, mi)
}

#[inline]
pub fn apply_move_idx(positions: &[usize], orients: &[u8], mi: usize) -> (Vec<usize>, Vec<u8>) {
    let t = tables();
    let new_pos: Vec<usize> = positions.iter().map(|&p| t.perm[mi][p]).collect();
    let new_ori: Vec<u8> = positions
        .iter()
        .zip(orients.iter())
        .map(|(&p, &o)| (o + t.flip[mi][p]) % 2)
        .collect();
    (new_pos, new_ori)
}

/// Bản KHÔNG cấp phát heap cho trường hợp theo dõi đủ cả 12 cạnh (đường
/// nóng của A* trong f2l_solver — mỗi lần áp 1 nước đi trước đây cấp phát
/// 2 Vec mới; giờ chỉ ghi vào mảng có sẵn trên stack).
#[inline]
pub fn apply_move_idx_full(positions: &[usize; 12], orients: &[u8; 12], mi: usize) -> ([usize; 12], [u8; 12]) {
    let t = tables();
    let mut new_pos = [0usize; 12];
    let mut new_ori = [0u8; 12];
    for i in 0..12 {
        let p = positions[i];
        new_pos[i] = t.perm[mi][p];
        new_ori[i] = (orients[i] + t.flip[mi][p]) % 2;
    }
    (new_pos, new_ori)
}

/// Đọc trực tiếp từ facelet state: vị trí+hướng của các cạnh gốc trong
/// `tracked` (danh sách slot_index), giống edges_from_state() bản Python.
pub fn edges_from_state(state: &CubeState, tracked: &[usize]) -> (Vec<usize>, Vec<u8>) {
    let solved = CubeState::solved();
    let mut loc_of = [(0usize, false); 12]; // edge gốc -> (slot hiện tại, flipped)
    for cand in 0..12 {
        let (orig, flipped) = identify(state, cand, &solved);
        loc_of[orig] = (cand, flipped);
    }
    let mut positions = Vec::with_capacity(tracked.len());
    let mut orients = Vec::with_capacity(tracked.len());
    for &name in tracked {
        let (cand, flipped) = loc_of[name];
        positions.push(cand);
        orients.push(flipped as u8);
    }
    (positions, orients)
}

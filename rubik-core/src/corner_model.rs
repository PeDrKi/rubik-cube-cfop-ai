//! corner_model.rs — port of solver/corner_model.py
//! Giống edge_model.rs nhưng cho 8 góc, hướng nhận giá trị 0/1/2 (mod 3).

use crate::cube::{move_index, CubeState, ALL_MOVES};
use std::sync::OnceLock;

pub const SLOT_NAMES: [&str; 8] = ["UFR", "UFL", "UBR", "UBL", "DFR", "DFL", "DBR", "DBL"];

#[inline]
pub fn slot_index(name: &str) -> usize {
    SLOT_NAMES.iter().position(|&s| s == name).unwrap()
}

const U: usize = 0;
const D: usize = 1;
const F: usize = 2;
const B: usize = 3;
const L: usize = 4;
const R: usize = 5;

type Coord = (usize, usize, usize);
const CORNER_SLOTS: [(Coord, Coord, Coord); 8] = [
    ((U, 2, 2), (F, 0, 2), (R, 0, 0)), // UFR
    ((U, 2, 0), (L, 0, 2), (F, 0, 0)), // UFL
    ((U, 0, 2), (R, 0, 2), (B, 0, 0)), // UBR
    ((U, 0, 0), (B, 0, 2), (L, 0, 0)), // UBL
    ((D, 0, 2), (R, 2, 0), (F, 2, 2)), // DFR
    ((D, 0, 0), (F, 2, 0), (L, 2, 2)), // DFL
    ((D, 2, 2), (B, 2, 0), (R, 2, 2)), // DBR
    ((D, 2, 0), (L, 2, 0), (B, 2, 2)), // DBL
];

/// (corner_gốc_index, orientation 0..3) đang chiếm slot `slot_idx`.
/// orientation = vị trí (0,1,2) của facelet mang giá trị U hoặc D trong bộ
/// 3 facelet đọc tại slot hiện tại — đúng quy ước bản Python.
fn identify(state: &CubeState, slot_idx: usize, solved: &CubeState) -> (usize, u8) {
    let (c1, c2, c3) = CORNER_SLOTS[slot_idx];
    let v = [
        state.facelet_at(c1.0, c1.1, c1.2),
        state.facelet_at(c2.0, c2.1, c2.2),
        state.facelet_at(c3.0, c3.1, c3.2),
    ];
    for orig in 0..8 {
        let (o1, o2, o3) = CORNER_SLOTS[orig];
        let sv = [
            solved.facelet_at(o1.0, o1.1, o1.2),
            solved.facelet_at(o2.0, o2.1, o2.2),
            solved.facelet_at(o3.0, o3.1, o3.2),
        ];
        let mut sv_sorted = sv;
        let mut v_sorted = v;
        sv_sorted.sort_unstable();
        v_sorted.sort_unstable();
        if sv_sorted == v_sorted {
            let ori = v.iter().position(|&x| x == U as u8 || x == D as u8).unwrap();
            return (orig, ori as u8);
        }
    }
    panic!("corner identify failed at slot {slot_idx}: v={v:?}");
}

pub struct CornerMoveTables {
    pub perm: [[usize; 8]; 18],
    pub ori_delta: [[u8; 8]; 18],
}

fn build_move_tables() -> CornerMoveTables {
    let solved = CubeState::solved();
    let mut perm = [[0usize; 8]; 18];
    let mut ori_delta = [[0u8; 8]; 18];
    for (mi, mv) in ALL_MOVES.iter().enumerate() {
        let mut after = CubeState::solved();
        after.do_move(mv);
        for slot_idx in 0..8 {
            let mut found = false;
            for cand in 0..8 {
                let (orig, ori) = identify(&after, cand, &solved);
                if orig == slot_idx {
                    perm[mi][slot_idx] = cand;
                    ori_delta[mi][slot_idx] = ori;
                    found = true;
                    break;
                }
            }
            assert!(found, "corner table inference failed move={mv} slot={slot_idx}");
        }
    }
    CornerMoveTables { perm, ori_delta }
}

static TABLES: OnceLock<CornerMoveTables> = OnceLock::new();
pub fn tables() -> &'static CornerMoveTables {
    TABLES.get_or_init(build_move_tables)
}

pub fn apply_move_to_corners(
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
        .map(|(&p, &o)| (o + t.ori_delta[mi][p]) % 3)
        .collect();
    (new_pos, new_ori)
}

/// Bản không cấp phát heap cho 8 góc đầy đủ — xem edge_model::apply_move_idx_full.
#[inline]
pub fn apply_move_idx_full(positions: &[usize; 8], orients: &[u8; 8], mi: usize) -> ([usize; 8], [u8; 8]) {
    let t = tables();
    let mut new_pos = [0usize; 8];
    let mut new_ori = [0u8; 8];
    for i in 0..8 {
        let p = positions[i];
        new_pos[i] = t.perm[mi][p];
        new_ori[i] = (orients[i] + t.ori_delta[mi][p]) % 3;
    }
    (new_pos, new_ori)
}

pub fn corners_from_state(state: &CubeState, tracked: &[usize]) -> (Vec<usize>, Vec<u8>) {
    let solved = CubeState::solved();
    let mut loc_of = [(0usize, 0u8); 8];
    for cand in 0..8 {
        let (orig, ori) = identify(state, cand, &solved);
        loc_of[orig] = (cand, ori);
    }
    let mut positions = Vec::with_capacity(tracked.len());
    let mut orients = Vec::with_capacity(tracked.len());
    for &name in tracked {
        let (cand, ori) = loc_of[name];
        positions.push(cand);
        orients.push(ori);
    }
    (positions, orients)
}

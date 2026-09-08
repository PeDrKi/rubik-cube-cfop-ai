//! oll_algorithms.rs — port of solver/oll_algorithms.py
//!
//! Bảng công thức OLL đầy đủ (dinh hướng cả cạnh lẫn góc lớp U trong 1
//! bước). Y hệt cách Python làm: mỗi công thức được ÁP THẬT lên cube đã
//! giải, đọc delta hướng thật sự tạo ra, rồi suy ra key = (-delta) mod
//! (2 hoặc 3) — KHÔNG chép tay key, tự kiểm chứng lúc khởi tạo. Công thức
//! nào phá vỡ Cross+F2L sẽ tự động bị loại khỏi bảng.

use crate::cube::CubeState;
use crate::full_state::{apply_move, cross_f2l_ok, from_facelets, FullState};
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

/// (tên case, chuỗi Singmaster) — dữ liệu thô y hệt _RAW_ALGS bản Python.
const RAW_ALGS: &[(&str, &str)] = &[
    ("Sune", "R U R' U R U2 R'"),
    ("AntiSune", "R U2 R' U' R U' R'"),
    ("Dot_variant1", "F R' F' R U R U' R' U F R U R' U' F'"),
    ("case_auto_01", "R U2 R' U' R U' R' U R U R' U R U2 R'"),
    ("case_auto_02", "U R U2 R' U' R U' R' U' R U R' U R U2 R'"),
    ("case_auto_04", "U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("case_auto_06", "R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_07", "R U2 R' U' R U' R' R U2 R' U' R U' R'"),
    ("case_auto_08", "L' B' U' B U L U R U2 R' U' R U' R' U' R U R' U R U2 R'"),
    ("case_auto_09", "L' B' U' B U L R U2 R' U' R U' R' R U2 R' U' R U' R'"),
    ("case_auto_10", "L' B' U' B U L R U R' U R U2 R' U R U R' U R U2 R'"),
    ("case_auto_11", "L' B' U' B U L U' R U2 R' U' R U' R'"),
    ("case_auto_12", "L' B' U' B U L R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_13", "L' B' U' B U L U2 R U2 R' U' R U' R'"),
    ("case_auto_14", "L' B' U' B U L U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("case_auto_15", "L' B' U' B U L U R U2 R' U' R U' R'"),
    ("case_auto_16", "L' B' U' B U L R U R' U R U2 R' U' R U2 R' U' R U' R'"),
    ("case_auto_17", "L' B' U' B U L U2 R U R' U R U2 R'"),
    ("case_auto_18", "L' B' U' B U L R U2 R' U' R U' R'"),
    ("case_auto_19", "L' B' U' B U L R U R' U R U2 R' U R U2 R' U' R U' R'"),
    ("case_auto_20", "L' B' U' B U L U R U R' U R U2 R' U' R U2 R' U' R U' R'"),
    ("case_auto_21", "L' B' U' B U L U2 R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_22", "L' B' U' B U L U R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_23", "B' U' R' U R B U2 R U2 R' U' R U' R' U' R U R' U R U2 R'"),
    ("case_auto_24", "B' U' R' U R B U2 R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("case_auto_25", "B' U' R' U R B U R U R' U R U2 R'"),
    ("case_auto_26", "B' U' R' U R B R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_27", "B' U' R' U R B U' R U R' U R U2 R'"),
    ("case_auto_28", "B' U' R' U R B U2 R U R' U R U2 R'"),
    ("case_auto_29", "B' U' R' U R B R U R' U R U2 R' R U R' U R U2 R'"),
    ("case_auto_30", "B' U' R' U R B U R U R' U R U2 R' U' R U2 R' U' R U' R'"),
    ("case_auto_31", "B' U' R' U R B R U R' U R U2 R' U R U R' U R U2 R'"),
    ("case_auto_32", "B' U' R' U R B U' R U2 R' U' R U' R'"),
    ("case_auto_33", "B' U' R' U R B R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("case_auto_34", "B' U' R' U R B R U R' U R U2 R' U' R U2 R' U' R U' R'"),
    ("case_auto_35", "B' U' R' U R B U R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_36", "B' U' R' U R B R U R' U R U2 R' U R U2 R' U' R U' R'"),
    ("case_auto_37", "B' U' R' U R B R U2 R' U' R U' R' R U2 R' U' R U' R'"),
    ("case_auto_38", "B' U' R' U R B R U2 R' U' R U' R'"),
    ("case_auto_39", "B' U' R' U R B U R U2 R' U' R U' R'"),
    ("case_auto_40", "B' U' R' U R B U2 R U R' U R U2 R' U' R U R' U R U2 R'"),
    ("case_auto_41", "B' U' R' U R B R U2 R' U' R U' R' U R U R' U R U2 R'"),
    ("case_auto_42", "B' U' R' U R B"),
    ("case_auto_43", "B' U' R' U R B U R U2 R' U' R U' R' U' R U R' U R U2 R'"),
    ("case_auto_44", "B' U' R' U R B U' R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("case_auto_45", "B' U' R' U R B U2 R U R' U R U2 R' U' R U2 R' U' R U' R'"),
    ("case_auto_46", "B' U' R' U R B R U R' U R U2 R'"),
    ("case_auto_47", "B' U' R' U R B U2 R U2 R' U' R U' R'"),
    ("case_auto_48", "B' U' R' U R B R U2 R' U' R U' R' U' R U R' U R U2 R'"),
    ("case_auto_49", "B' U' R' U R B U R U R' U R U2 R' U2 R U2 R' U' R U' R'"),
    ("Dot_OLL1", "R U2 R2 F R F' U2 R' F R F'"),
    ("Dot_OLL3", "F U R U' R' F' U F R U R' U' F'"),
    ("Dot_OLL4", "F' U' L' U L F U F R U R' U' F'"),
    ("Dot_OLL17", "R U R' U R' F R F' U2 R' F R F'"),
    ("Dot_OLL19", "F R' F' R U R U' R' U' F R U R' U' F'"),
];

pub type OriKey = ([u8; 4], [u8; 4]); // (eo_key, co_key) slot-indexed

pub struct OllTable {
    pub table: FxHashMap<OriKey, Vec<&'static str>>,
    pub name_table: FxHashMap<OriKey, &'static str>,
}

/// vị trí hiện tại (piece-indexed) -> hướng hiện tại TẠI vị trí đó
/// (slot-indexed), giống oll_recognition._slot_indexed().
fn slot_indexed<const N: usize>(pos: &[usize], ori: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    for piece_id in 0..pos.len() {
        let slot = pos[piece_id];
        if slot < N {
            out[slot] = ori[piece_id];
        }
    }
    out
}

fn delta_of(seq: &[&str]) -> ([u8; 4], [u8; 4], bool) {
    let mut st = CubeState::solved();
    st.apply_sequence(seq);
    let full = from_facelets(&st);
    let (_, eo, _, co) = full;
    let valid = cross_f2l_ok(&full);
    ([eo[0], eo[1], eo[2], eo[3]], [co[0], co[1], co[2], co[3]], valid)
}

fn build_table() -> OllTable {
    let mut table = FxHashMap::default();
    let mut name_table = FxHashMap::default();
    for &(name, seqstr) in RAW_ALGS {
        let seq: Vec<&str> = seqstr.split(' ').collect();
        let (eo_delta, co_delta, valid) = delta_of(&seq);
        if !valid {
            continue; // giống Python: loại khỏi bảng nếu phá vỡ Cross+F2L
        }
        let eo_key: [u8; 4] = std::array::from_fn(|i| (2 - eo_delta[i] % 2) % 2);
        let co_key: [u8; 4] = std::array::from_fn(|i| (3 - co_delta[i] % 3) % 3);
        let key: OriKey = (eo_key, co_key);
        table.insert(key, seq.clone());
        name_table.insert(key, name);
    }
    OllTable { table, name_table }
}

static TABLE: OnceLock<OllTable> = OnceLock::new();
pub fn table() -> &'static OllTable {
    TABLE.get_or_init(build_table)
}

/// Thử cả 4 AUF (không xoay / U / U2 / U'), trả về (auf_prefix, solve_moves,
/// case_name) cho lần đầu khớp bảng, hoặc None nếu không case nào khớp.
pub fn solve_oll_with_auf(full: FullState) -> Option<(Vec<&'static str>, Vec<&'static str>, &'static str)> {
    let t = table();
    for auf in ["", "U", "U2", "U'"] {
        let state = if auf.is_empty() { full } else { apply_move(&full, auf) };
        let (ep, eo, cp, co) = state;
        let eo_slot: [u8; 4] = slot_indexed::<4>(&ep, &eo);
        let co_slot: [u8; 4] = slot_indexed::<4>(&cp, &co);
        let key: OriKey = (eo_slot, co_slot);
        if let Some(mvs) = t.table.get(&key) {
            let prefix: Vec<&'static str> = if auf.is_empty() { vec![] } else { vec![auf] };
            let name = *t.name_table.get(&key).unwrap();
            return Some((prefix, mvs.clone(), name));
        }
    }
    None
}

/// Toàn bộ công thức thô (tên, chuỗi Singmaster) — dùng cho bảng tra cứu
/// trong giao diện. Trả về đúng dữ liệu gốc, không qua xử lý.
pub fn all_algorithms() -> &'static [(&'static str, &'static str)] {
    RAW_ALGS
}

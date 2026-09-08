//! pll_algorithms.rs — port of solver/pll_algorithms.py
//! 21 thuật toán PLL chuẩn, mỗi cái tạo ra 4 mục bảng (thuận/nghịch/gương/
//! gương-nghịch) sau khi tự kiểm chứng bằng cách áp thật lên cube đã giải.

use crate::cube::CubeState;
use crate::full_state::{apply_move, cross_f2l_ok, from_facelets, FullState};
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

const RAW_ALGS: &[(&str, &str)] = &[
    ("Aa", "R' F R' B2 R F' R' B2 R2"),
    ("Ab", "R2 B2 R F R' B2 R F' R"),
    ("E", "R B' R F2 R' B R F2 R2"),
    ("Ua", "R U' R U R U R U' R' U' R2"),
    ("Ub", "R2 U R U R' U' R' U' R' U R'"),
    ("H", "R2 U2 R U2 R2 U2 R2 U2 R U2 R2"),
    ("T", "R U R' U' R' F R2 U' R' U' R U R' F'"),
    ("Y", "F R U' R' U' R U R' F' R U R' U' R' F R F'"),
    ("F", "R' U' F' R U R' U' R' F R2 U' R' U' R U R' U R"),
    ("Ja", "R' U L' U2 R U' R' U2 R L"),
    ("Jb", "R U R' F' R U R' U' R' F R2 U' R'"),
    ("Ra", "R U' R' U' R U R D R' U' R D' R' U2 R'"),
    ("Rb", "R' U2 R U2 R' F R U R' U' R' F' R2"),
    ("Ga", "R2 U R' U R' U' R U' R2 U' D R' U R D'"),
    ("Gb", "R' U' R U D' R2 U R' U R U' R U' R2 D"),
    ("Gc", "R2 U' R U' R U R' U R2 U D' R U' R' D"),
    ("Gd", "R U R' U' D R2 U' R U' R' U R' U R2 D'"),
    ("V", "R' U R' U' B' R' B2 U' B' U B' R B R"),
    ("Na", "R U R' U R U R' F' R U R' U' R' F R2 U' R' U2 R U' R'"),
    ("Nb", "R' U R U' R' F' U' F R U R' F R' F' R U' R"),
    ("Z", "U2 R U R' U R' U' R' U R U' R' U' R2 U R"),
];

fn mirror_move(m: &str) -> &'static str {
    match m {
        "R" => "L'", "R'" => "L", "R2" => "L2",
        "L" => "R'", "L'" => "R", "L2" => "R2",
        "U" => "U'", "U'" => "U", "U2" => "U2",
        "D" => "D'", "D'" => "D", "D2" => "D2",
        "F" => "F'", "F'" => "F", "F2" => "F2",
        "B" => "B'", "B'" => "B", "B2" => "B2",
        other => panic!("unmapped move for mirror: {other}"),
    }
}
fn mirror_seq(seq: &[&'static str]) -> Vec<&'static str> {
    seq.iter().map(|&m| mirror_move(m)).collect()
}

fn inv_move(m: &str) -> &'static str {
    match m {
        "R" => "R'", "R'" => "R", "R2" => "R2",
        "L" => "L'", "L'" => "L", "L2" => "L2",
        "U" => "U'", "U'" => "U", "U2" => "U2",
        "D" => "D'", "D'" => "D", "D2" => "D2",
        "F" => "F'", "F'" => "F", "F2" => "F2",
        "B" => "B'", "B'" => "B", "B2" => "B2",
        other => panic!("unexpected move {other}"),
    }
}
fn inv_seq(seq: &[&'static str]) -> Vec<&'static str> {
    seq.iter().rev().map(|&m| inv_move(m)).collect()
}

pub type PermKey = ([usize; 4], [usize; 4]); // (cp[0:4], ep[0:4])

pub struct PllTable {
    pub table: FxHashMap<PermKey, Vec<&'static str>>,
    pub name_table: FxHashMap<PermKey, &'static str>,
}

fn pattern_of(seq: &[&str]) -> ([usize; 4], [usize; 4], bool) {
    let mut st = CubeState::solved();
    st.apply_sequence(seq);
    let full = from_facelets(&st);
    let (ep, eo, cp, co) = full;
    let valid = eo[0..4] == [0, 0, 0, 0] && co[0..4] == [0, 0, 0, 0] && cross_f2l_ok(&full);
    ([cp[0], cp[1], cp[2], cp[3]], [ep[0], ep[1], ep[2], ep[3]], valid)
}

fn add_variant(
    table: &mut FxHashMap<PermKey, Vec<&'static str>>,
    name_table: &mut FxHashMap<PermKey, &'static str>,
    seq: &[&'static str],
    name: &'static str,
) -> bool {
    let (cp, ep, valid) = pattern_of(seq);
    if !valid {
        return false;
    }
    let inv = inv_seq(seq);
    table.insert((cp, ep), inv.clone());
    name_table.insert((cp, ep), name);
    let (icp, iep, ivalid) = pattern_of(&inv);
    if ivalid {
        table.insert((icp, iep), seq.to_vec());
        name_table.insert((icp, iep), name);
    }
    true
}

fn build_table() -> PllTable {
    let mut table = FxHashMap::default();
    let mut name_table = FxHashMap::default();
    for &(name, seqstr) in RAW_ALGS {
        let seq: Vec<&'static str> = seqstr.split(' ').collect();
        if add_variant(&mut table, &mut name_table, &seq, name) {
            let mirrored = mirror_seq(&seq);
            add_variant(&mut table, &mut name_table, &mirrored, name);
        }
    }
    PllTable { table, name_table }
}

static TABLE: OnceLock<PllTable> = OnceLock::new();
pub fn table() -> &'static PllTable {
    TABLE.get_or_init(build_table)
}

const AUF_OPTIONS: [(&str, &[&str]); 4] = [("", &[]), ("U", &["U"]), ("U2", &["U2"]), ("U'", &["U'"])];

/// Trả về (moves, case_name) hoặc (None, None) nếu không case nào khớp.
/// case_name = "solved" nếu PLL đã xong sẵn (không cần nước nào).
pub fn solve_pll_lookup_named(full: FullState) -> (Option<Vec<&'static str>>, Option<&'static str>) {
    let (ep0, _eo0, cp0, _co0) = full;
    if cp0[0..4] == [0, 1, 2, 3] && ep0[0..4] == [0, 1, 2, 3] {
        return (Some(vec![]), Some("solved"));
    }
    let t = table();
    for (auf_name, auf) in AUF_OPTIONS {
        let mut s = full;
        for &mv in auf {
            s = apply_move(&s, mv);
        }
        let (ep, _eo, cp, _co) = s;
        let key: PermKey = ([cp[0], cp[1], cp[2], cp[3]], [ep[0], ep[1], ep[2], ep[3]]);
        if let Some(mvs) = t.table.get(&key) {
            let mut full_moves: Vec<&'static str> = if auf_name.is_empty() { vec![] } else { vec![auf[0]] };
            full_moves.extend(mvs.iter().copied());
            let name = *t.name_table.get(&key).unwrap();
            return (Some(full_moves), Some(name));
        }
    }
    (None, None)
}

/// Toàn bộ công thức thô (tên, chuỗi Singmaster) — dùng cho bảng tra cứu
/// trong giao diện. Trả về đúng dữ liệu gốc, không qua xử lý.
pub fn all_algorithms() -> &'static [(&'static str, &'static str)] {
    RAW_ALGS
}

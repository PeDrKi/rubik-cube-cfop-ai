//! f2l_solver.rs — port of solver/f2l_solver.py, tối ưu sâu so với bản port thẳng:
//!
//! 1. Pair-PDB là mảng phẳng [u16;576] (pdb.rs) thay vì HashMap -> tra bảng
//!    O(1) thật, không hash.
//! 2. best_g dùng FxHashMap (hash không mã hoá, nhanh hơn nhiều SipHash mặc
//!    định của Rust cho khoá không phải input từ bên ngoài/không cần chống
//!    DoS) thay vì std HashMap.
//! 3. KHÔNG clone Vec<đường đi> ở mỗi node (bản port thẳng làm y hệt Python
//!    `path + (mv,)` -- O(độ sâu) mỗi lần mở rộng, cộng dồn thành O(node *
//!    độ sâu) tổng cộng). Thay bằng "arena" phẳng lưu (node cha, nước đi),
//!    mỗi node mở rộng chỉ ghi 2 giá trị O(1); truy vết đường đi bằng cách
//!    đi ngược con trỏ cha CHỈ MỘT LẦN khi tìm thấy đích.
//! 4. Sửa 1 lỗi hiệu năng trong bản port thẳng: tra ngược tên nước đi từ
//!    chỉ số move bằng `.position()` (O(n) mỗi lần) — giờ giữ luôn cặp
//!    (chỉ số, tên, mặt) nên tra là O(1).

use crate::corner_model as CM;
use crate::cube::move_index;
use crate::edge_model as EM;
use crate::full_state::{apply_move_idx, cross_ok, f2l_edge_of, no_d_moves, pair_ok, FullState};
use crate::pdb::{build_pdb_pair, PairPdb};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::OnceLock;

pub static NODES_SEEN: AtomicU64 = AtomicU64::new(0);

pub const F2L_ORDER: [&str; 4] = ["DFR", "DFL", "DBR", "DBL"];

struct PairPdbCache {
    map: FxHashMap<&'static str, PairPdb>,
}
static PAIR_PDB: OnceLock<PairPdbCache> = OnceLock::new();

fn pair_pdb(slot: &'static str) -> &'static PairPdb {
    let cache = PAIR_PDB.get_or_init(|| {
        let mut map = FxHashMap::default();
        for &s in F2L_ORDER.iter() {
            map.insert(s, build_pdb_pair(s, f2l_edge_of(s)));
        }
        PairPdbCache { map }
    });
    cache.map.get(slot).unwrap()
}

/// Buộc build toàn bộ 4 pair-PDB ngay (dùng để đo thời gian warm-up tách
/// biệt khỏi thời gian solve trong benchmark).
pub fn warm_pair_pdbs() {
    let _ = pair_pdb(F2L_ORDER[0]);
}

/// Tra cứu pair-PDB công khai cho 1 slot F2L cụ thể -- dùng lại bởi
/// oll_solver.rs để tính cận dưới "Cross+F2L còn cách bao xa nếu bị vỡ",
/// khớp _cross_f2l_lower_bound() trong oll_solver.py (không xây PDB mới).
pub fn pair_pdb_get(slot: &'static str, ep: usize, eo: u8, cp: usize, co: u8) -> Option<u16> {
    pair_pdb(slot).get(ep, eo, cp, co)
}

fn mismatch_penalty(full: &FullState, done_slots: &[&str]) -> u32 {
    let (ep, eo, cp, co) = full;
    let mut bad = if ep[4] == 4 && ep[5] == 5 && ep[6] == 6 && ep[7] == 7
        && eo[4] == 0 && eo[5] == 0 && eo[6] == 0 && eo[7] == 0
    {
        0
    } else {
        1
    };
    for &s in done_slots {
        let ci = CM::slot_index(s);
        if cp[ci] != ci || co[ci] != 0 {
            bad += 1;
        }
        let ei = EM::slot_index(f2l_edge_of(s));
        if ep[ei] != ei || eo[ei] != 0 {
            bad += 1;
        }
    }
    bad
}

#[inline]
fn heuristic(full: &FullState, slot: &'static str, done_slots: &[&str]) -> u32 {
    let (ep, eo, cp, co) = full;
    let ci = CM::slot_index(slot);
    let ei = EM::slot_index(f2l_edge_of(slot));
    let base = pair_pdb(slot).get(ep[ei], eo[ei], cp[ci], co[ci]).unwrap_or(8) as u32;
    base + 2 * mismatch_penalty(full, done_slots)
}

#[derive(PartialEq, Eq)]
struct HeapItem {
    f: u32,
    g: u32,
    counter: u64,
    state: FullState,
    node_idx: u32,
    last_face: Option<char>,
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| other.counter.cmp(&self.counter))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// arena[i] = (node cha, nước đi dẫn tới node i). arena[0] là root (không
/// có nước đi dẫn tới) -- dừng truy vết khi gặp lại index 0, không đọc
/// placeholder move của root.
fn reconstruct(arena: &[(u32, &'static str)], mut idx: u32) -> Vec<&'static str> {
    let mut rev = Vec::new();
    while idx != 0 {
        let (parent, mv) = arena[idx as usize];
        rev.push(mv);
        idx = parent;
    }
    rev.reverse();
    rev
}

fn solve_pair(
    full_start: FullState,
    slot: &'static str,
    done_slots: &[&str],
    max_nodes: u32,
    max_depth: u32,
    moves: &[(usize, &'static str, char)], // (move_index, tên, mặt)
) -> Option<Vec<&'static str>> {
    let goal = |full: &FullState| -> bool {
        if !cross_ok(full) {
            return false;
        }
        for &s in done_slots {
            if !pair_ok(full, s) {
                return false;
            }
        }
        pair_ok(full, slot)
    };

    if goal(&full_start) {
        return Some(vec![]);
    }

    let mut arena: Vec<(u32, &'static str)> = vec![(0, "")]; // arena[0] = root sentinel

    let h0 = heuristic(&full_start, slot, done_slots);
    let mut counter: u64 = 0;
    let mut heap = BinaryHeap::new();
    heap.push(HeapItem { f: h0, g: 0, counter, state: full_start, node_idx: 0, last_face: None });
    let mut best_g: FxHashMap<FullState, u32> = FxHashMap::default();
    best_g.insert(full_start, 0);
    let mut nodes: u32 = 0;

    while let Some(item) = heap.pop() {
        let HeapItem { g, state: cur, node_idx, last_face, .. } = item;
        if g > *best_g.get(&cur).unwrap_or(&u32::MAX) {
            continue;
        }
        if g >= max_depth {
            continue;
        }
        nodes += 1;
        if nodes % crate::cancel::CHECK_EVERY == 0 && crate::cancel::is_cancelled() {
            return None;
        }
        if nodes > max_nodes {
            NODES_SEEN.fetch_add(nodes as u64, std::sync::atomic::Ordering::Relaxed);
            return None;
        }
        for &(mi, name, face) in moves {
            if Some(face) == last_face {
                continue;
            }
            let nxt = apply_move_idx(&cur, mi);
            let ng = g + 1;
            if ng < *best_g.get(&nxt).unwrap_or(&u32::MAX) {
                best_g.insert(nxt, ng);
                arena.push((node_idx, name));
                let nxt_idx = (arena.len() - 1) as u32;
                if goal(&nxt) {
                    NODES_SEEN.fetch_add(nodes as u64, AtomicOrdering::Relaxed);
                    return Some(reconstruct(&arena, nxt_idx));
                }
                counter += 1;
                let h = heuristic(&nxt, slot, done_slots);
                heap.push(HeapItem { f: ng + h, g: ng, counter, state: nxt, node_idx: nxt_idx, last_face: Some(face) });
            }
        }
    }
    NODES_SEEN.fetch_add(nodes as u64, AtomicOrdering::Relaxed);
    None
}

fn solve_pair_ladder(
    full: FullState,
    slot: &'static str,
    done: &[&str],
    depths: &[u32],
    nodes_per_depth: u32,
    move_table: &[(usize, &'static str, char)],
) -> Option<Vec<&'static str>> {
    for &depth in depths {
        if let Some(mvs) = solve_pair(full, slot, done, nodes_per_depth, depth, move_table) {
            return Some(mvs);
        }
    }
    None
}

pub struct F2lResult {
    pub moves: Vec<&'static str>,
    pub per_slot: FxHashMap<&'static str, Option<Vec<&'static str>>>,
    pub solved_slots: Vec<&'static str>,
}

pub fn solve_f2l(mut full: FullState, depths: &[u32], nodes_per_depth: u32) -> F2lResult {
    solve_f2l_seeded(full, depths, nodes_per_depth, None)
}

/// Giống solve_f2l, nhưng nếu `retry_seed` là Some(seed), thứ tự duyệt
/// nước đi bị xáo trộn (có seed để tái lập) -- giống move_order(moves,
/// shuffled=True) trong search_utils.py bản Python: tìm kiếm vốn tất
/// định nên thử lại với cùng ngân sách sẽ ra y hệt lần trước; xáo trộn
/// thứ tự giúp khám phá nhánh khác, hội tụ về 1 kết quả F2L KHÁC (vẫn
/// tối ưu/gần tối ưu) -- dùng khi case OLL/PLL theo sau hoá ra quá khó.
pub fn solve_f2l_seeded(mut full: FullState, depths: &[u32], nodes_per_depth: u32, retry_seed: Option<u64>) -> F2lResult {
    let mut no_d = no_d_moves();
    if let Some(seed) = retry_seed {
        use rand::SeedableRng;
        use rand::seq::SliceRandom;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        no_d.shuffle(&mut rng);
    }
    let move_table: Vec<(usize, &'static str, char)> =
        no_d.iter().map(|&m| (move_index(m), m, m.chars().next().unwrap())).collect();

    let mut all_moves = Vec::new();
    let mut per_slot = FxHashMap::default();
    let mut done: Vec<&'static str> = F2L_ORDER.iter().copied().filter(|&s| pair_ok(&full, s)).collect();

    for &slot in F2L_ORDER.iter() {
        if pair_ok(&full, slot) {
            per_slot.insert(slot, Some(vec![]));
            // QUAN TRONG: slot co the vua tro thanh "da xong" NGAY TRONG
            // vong lap nay (tac dung phu cua 1 slot truoc do), khong chi
            // luc khoi tao -- neu chua co trong `done` thi PHAI them vao,
            // neu khong cac slot sau se khong biet phai BAO VE no, tu do
            // pha vo no ma khong bi phat (mismatch penalty khong biet).
            // Day la bug that da tim thay: thieu dong nay khien F2L bao
            // "3/4 xong" nhung 1 trong so do da bi pha lai sau do.
            if !done.contains(&slot) {
                done.push(slot);
            }
            continue;
        }
        let mvs = solve_pair_ladder(full, slot, &done, depths, nodes_per_depth, &move_table);
        match mvs {
            Some(ms) => {
                for &mv in &ms {
                    full = apply_move_idx(&full, move_index(mv));
                }
                all_moves.extend(ms.iter().copied());
                per_slot.insert(slot, Some(ms));
                done.push(slot);
            }
            None => {
                per_slot.insert(slot, None);
                break;
            }
        }
    }

    F2lResult { moves: all_moves, per_slot, solved_slots: done }
}

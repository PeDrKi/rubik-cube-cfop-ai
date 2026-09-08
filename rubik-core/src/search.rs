//! search.rs — port of solver/search_utils.py (chỉ phần a_star; IDA* KHÔNG
//! port -- xem ghi chú cuối file). Cùng kiến trúc đã tối ưu trong
//! f2l_solver.rs (arena thay vì clone Vec đường đi, FxHashMap thay vì
//! std::HashMap), tổng quát hoá qua goal_fn/heuristic_fn dạng closure để
//! dùng lại cho OLL phase A/B thay vì viết riêng một bản nữa.

use crate::full_state::{apply_move_idx, FullState};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

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

/// A* tổng quát: goal/heuristic là closure tuỳ ý (heuristic PHẢI admissible
/// để đường đi tìm được là tối ưu, giống yêu cầu của bản Python). `moves`
/// là danh sách (move_index, tên, mặt) — thường truyền NO_D_MOVES để giữ
/// nguyên Cross trong khi giải OLL/PLL.
pub fn a_star<G, H>(
    full_start: FullState,
    goal: G,
    heuristic: H,
    max_nodes: u32,
    max_depth: u32,
    moves: &[(usize, &'static str, char)],
) -> Option<Vec<&'static str>>
where
    G: Fn(&FullState) -> bool,
    H: Fn(&FullState) -> u32,
{
    if goal(&full_start) {
        return Some(vec![]);
    }

    let mut arena: Vec<(u32, &'static str)> = vec![(0, "")];
    let h0 = heuristic(&full_start);
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
                    return Some(reconstruct(&arena, nxt_idx));
                }
                counter += 1;
                let h = heuristic(&nxt);
                heap.push(HeapItem { f: ng + h, g: ng, counter, state: nxt, node_idx: nxt_idx, last_face: Some(face) });
            }
        }
    }
    None
}

/// Thử nhiều ngân sách (node, độ sâu) tăng dần, giống move_order+ladder
/// trong search_utils.py/oll_solver.py. KHÔNG port IDA* (dự phòng cuối
/// cùng của Python cho trường hợp A* hết ngân sách RAM) -- ở Rust,
/// FxHashMap rẻ hơn dict Python nhiều nên A* với ngân sách lớn hơn đã đủ
/// dùng trong thực tế; nếu 1 case nào đó vẫn miss cả 3 tier, ta trả None
/// (đã đo: không xảy ra trên các batch test đã chạy).
pub fn a_star_ladder<G, H>(
    full_start: FullState,
    goal: G,
    heuristic: H,
    tiers: &[(u32, u32)],
    moves: &[(usize, &'static str, char)],
) -> Option<Vec<&'static str>>
where
    G: Fn(&FullState) -> bool,
    H: Fn(&FullState) -> u32,
{
    for &(nodes, depth) in tiers {
        if let Some(mvs) = a_star(full_start, &goal, &heuristic, nodes, depth, moves) {
            return Some(mvs);
        }
    }
    None
}

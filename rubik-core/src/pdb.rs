//! pdb.rs — port of solver/pdb_builder.py (chỉ phần cần cho Cross + F2L pair).
//! BFS ngược từ trạng thái đã giải trên không gian combinatorial rút gọn
//! (vị trí+hướng của tập quân theo dõi) — cho khoảng cách CHÍNH XÁC, dùng
//! làm heuristic admissible.
//!
//! TỐI ƯU so với bản đầu: thay vì HashMap<key,dist> (mỗi lần tra phải hash
//! + so khớp key), ta MÃ HOÁ key thành 1 chỉ số nguyên và dùng Vec phẳng
//! (Vec<u16>) — tra bảng là 1 phép index O(1) thật sự, không hash. Đây là
//! kỹ thuật PDB kinh điển (Korf 1997 dùng chính xác cách này, không phải
//! HashMap) — bản Python dùng dict vì Python không tiện làm mảng phẳng
//! nhanh, nhưng ở Rust đây là điều nên làm ngay từ đầu.

use crate::corner_model as CM;
use crate::edge_model as EM;
use std::collections::VecDeque;

const NONE: u16 = u16::MAX;

// ── Cross PDB: 4 cạnh theo dõi trong 12 khe ────────────────────────────────
// Encode: pos_code = ((p0*12+p1)*12+p2)*12+p3   (0..20736, thưa nhưng đơn giản)
//         ori_code = ((o0*2+o1)*2+o2)*2+o3       (0..16)
//         index = pos_code*16 + ori_code          (0..331776)
const CROSS_POS_RADIX: usize = 12;
const CROSS_TABLE_SIZE: usize = 12 * 12 * 12 * 12 * 16; // 331,776

#[inline]
fn cross_encode(pos: &[usize], ori: &[u8]) -> usize {
    let pos_code = ((pos[0] * CROSS_POS_RADIX + pos[1]) * CROSS_POS_RADIX + pos[2]) * CROSS_POS_RADIX + pos[3];
    let ori_code = ((ori[0] as usize * 2 + ori[1] as usize) * 2 + ori[2] as usize) * 2 + ori[3] as usize;
    pos_code * 16 + ori_code
}

pub struct CrossPdb {
    table: Vec<u16>,
}
impl CrossPdb {
    #[inline]
    pub fn get(&self, pos: &[usize], ori: &[u8]) -> Option<u16> {
        let idx = cross_encode(pos, ori);
        let v = self.table[idx];
        if v == NONE {
            None
        } else {
            Some(v)
        }
    }
    pub fn len(&self) -> usize {
        self.table.iter().filter(|&&v| v != NONE).count()
    }
    pub fn max_dist(&self) -> u16 {
        self.table.iter().filter(|&&v| v != NONE).copied().max().unwrap()
    }
}

pub fn build_cross_pdb(tracked: &[usize; 4]) -> CrossPdb {
    let mut table = vec![NONE; CROSS_TABLE_SIZE];
    let start_pos: [usize; 4] = *tracked;
    let start_ori: [u8; 4] = [0; 4];
    let start_idx = cross_encode(&start_pos, &start_ori);
    table[start_idx] = 0;

    let mut queue: VecDeque<([usize; 4], [u8; 4])> = VecDeque::new();
    queue.push_back((start_pos, start_ori));

    while let Some((pos, ori)) = queue.pop_front() {
        let d = table[cross_encode(&pos, &ori)];
        for mi in 0..18 {
            let (np, no) = EM::apply_move_idx(&pos, &ori, mi);
            let np4: [usize; 4] = [np[0], np[1], np[2], np[3]];
            let no4: [u8; 4] = [no[0], no[1], no[2], no[3]];
            let idx = cross_encode(&np4, &no4);
            if table[idx] == NONE {
                table[idx] = d + 1;
                queue.push_back((np4, no4));
            }
        }
    }
    CrossPdb { table }
}

// ── Pair PDB: 1 corner + 1 edge (24*24 = 576 trạng thái) ───────────────────
// Encode: index = ((ep*2+eo)*8 + cp)*3 + co   (0..576)
#[inline]
fn pair_encode(ep: usize, eo: u8, cp: usize, co: u8) -> usize {
    ((ep * 2 + eo as usize) * 8 + cp) * 3 + co as usize
}

pub struct PairPdb {
    table: [u16; 576],
}
impl PairPdb {
    #[inline]
    pub fn get(&self, ep: usize, eo: u8, cp: usize, co: u8) -> Option<u16> {
        let v = self.table[pair_encode(ep, eo, cp, co)];
        if v == NONE {
            None
        } else {
            Some(v)
        }
    }
    pub fn len(&self) -> usize {
        self.table.iter().filter(|&&v| v != NONE).count()
    }
    pub fn max_dist(&self) -> u16 {
        self.table.iter().filter(|&&v| v != NONE).copied().max().unwrap()
    }
}

pub fn build_pdb_pair(corner_slot: &str, edge_slot: &str) -> PairPdb {
    let esi = EM::slot_index(edge_slot);
    let csi = CM::slot_index(corner_slot);
    let mut table = [NONE; 576];
    let start_idx = pair_encode(esi, 0, csi, 0);
    table[start_idx] = 0;

    let mut queue: VecDeque<(usize, u8, usize, u8)> = VecDeque::new();
    queue.push_back((esi, 0, csi, 0));

    while let Some((ep, eo, cp, co)) = queue.pop_front() {
        let d = table[pair_encode(ep, eo, cp, co)];
        for mi in 0..18 {
            let (nep, neo) = EM::apply_move_idx(&[ep], &[eo], mi);
            let (ncp, nco) = CM::apply_move_idx(&[cp], &[co], mi);
            let idx = pair_encode(nep[0], neo[0], ncp[0], nco[0]);
            if table[idx] == NONE {
                table[idx] = d + 1;
                queue.push_back((nep[0], neo[0], ncp[0], nco[0]));
            }
        }
    }
    PairPdb { table }
}

// ── Group PDB (OLL): "4 cạnh/góc U, BẤT KỲ hoán vị nào giữa chúng, miễn
// hướng đều = 0" -- không quan tâm quân nào ở khe nào (khớp đúng định
// nghĩa u_edges_oriented()/u_corners_oriented() trong full_state.rs: chỉ
// kiểm tra hướng, không kiểm tra vị trí). Port build_pdb_edges_group_anyperm
// / build_pdb_corners_group_anyperm — multi-source BFS: MỌI hoán vị 4-phần-
// tử-phân-biệt trong 12 (hoặc 8) khe, hướng=0, đều là nguồn cách=0.

fn permutations_4_of_n(n: usize) -> Vec<[usize; 4]> {
    let mut out = Vec::new();
    for a in 0..n {
        for b in 0..n {
            if b == a {
                continue;
            }
            for c in 0..n {
                if c == a || c == b {
                    continue;
                }
                for d in 0..n {
                    if d == a || d == b || d == c {
                        continue;
                    }
                    out.push([a, b, c, d]);
                }
            }
        }
    }
    out
}

/// PDB nhóm cho 4 cạnh (dùng chung mã hoá với CrossPdb -- cùng không gian
/// vị trí/hướng, chỉ khác cách seed BFS: nhiều nguồn thay vì 1 nguồn).
pub fn build_group_pdb_edges_anyperm() -> CrossPdb {
    let mut table = vec![NONE; CROSS_TABLE_SIZE];
    let mut queue: VecDeque<([usize; 4], [u8; 4])> = VecDeque::new();
    for p in permutations_4_of_n(12) {
        let ori = [0u8; 4];
        let idx = cross_encode(&p, &ori);
        if table[idx] == NONE {
            table[idx] = 0;
            queue.push_back((p, ori));
        }
    }
    while let Some((pos, ori)) = queue.pop_front() {
        let d = table[cross_encode(&pos, &ori)];
        for mi in 0..18 {
            let (np, no) = EM::apply_move_idx_full(
                &{
                    let mut full = [0usize; 12];
                    for i in 0..4 {
                        full[i] = pos[i];
                    }
                    full
                },
                &{
                    let mut full = [0u8; 12];
                    for i in 0..4 {
                        full[i] = ori[i];
                    }
                    full
                },
                mi,
            );
            let np4: [usize; 4] = [np[0], np[1], np[2], np[3]];
            let no4: [u8; 4] = [no[0], no[1], no[2], no[3]];
            let idx = cross_encode(&np4, &no4);
            if table[idx] == NONE {
                table[idx] = d + 1;
                queue.push_back((np4, no4));
            }
        }
    }
    CrossPdb { table }
}

// ── Corner group PDB: pos_code = ((p0*8+p1)*8+p2)*8+p3 (0..4096),
//    ori_code = ((o0*3+o1)*3+o2)*3+o3 (0..81), index = pos_code*81+ori_code.
const CORNER_GROUP_TABLE_SIZE: usize = 8 * 8 * 8 * 8 * 81; // 331,776

#[inline]
fn corner_group_encode(pos: &[usize], ori: &[u8]) -> usize {
    let pos_code = ((pos[0] * 8 + pos[1]) * 8 + pos[2]) * 8 + pos[3];
    let ori_code = ((ori[0] as usize * 3 + ori[1] as usize) * 3 + ori[2] as usize) * 3 + ori[3] as usize;
    pos_code * 81 + ori_code
}

pub struct CornerGroupPdb {
    table: Vec<u16>,
}
impl CornerGroupPdb {
    #[inline]
    pub fn get(&self, pos: &[usize], ori: &[u8]) -> Option<u16> {
        let v = self.table[corner_group_encode(pos, ori)];
        if v == NONE {
            None
        } else {
            Some(v)
        }
    }
    pub fn len(&self) -> usize {
        self.table.iter().filter(|&&v| v != NONE).count()
    }
    pub fn max_dist(&self) -> u16 {
        self.table.iter().filter(|&&v| v != NONE).copied().max().unwrap()
    }
}

pub fn build_group_pdb_corners_anyperm() -> CornerGroupPdb {
    let mut table = vec![NONE; CORNER_GROUP_TABLE_SIZE];
    let mut queue: VecDeque<([usize; 4], [u8; 4])> = VecDeque::new();
    for p in permutations_4_of_n(8) {
        let ori = [0u8; 4];
        let idx = corner_group_encode(&p, &ori);
        if table[idx] == NONE {
            table[idx] = 0;
            queue.push_back((p, ori));
        }
    }
    while let Some((pos, ori)) = queue.pop_front() {
        let d = table[corner_group_encode(&pos, &ori)];
        for mi in 0..18 {
            let mut full_pos = [0usize; 8];
            let mut full_ori = [0u8; 8];
            full_pos[..4].copy_from_slice(&pos);
            full_ori[..4].copy_from_slice(&ori);
            let (np, no) = CM::apply_move_idx_full(&full_pos, &full_ori, mi);
            let np4: [usize; 4] = [np[0], np[1], np[2], np[3]];
            let no4: [u8; 4] = [no[0], no[1], no[2], no[3]];
            let idx = corner_group_encode(&np4, &no4);
            if table[idx] == NONE {
                table[idx] = d + 1;
                queue.push_back((np4, no4));
            }
        }
    }
    CornerGroupPdb { table }
}

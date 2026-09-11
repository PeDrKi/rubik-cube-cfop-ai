//! pll_algorithms.rs — port of solver/pll_algorithms.py
//! 21 thuật toán PLL chuẩn. Mỗi cái sinh 4 mục bảng — công thức kèm 4 khả
//! năng hậu-AUF (không xoay / U / U2 / U') — sau khi tự kiểm chứng bằng
//! cách áp thật lên cube đã giải. Khoá luôn là trạng thái MÀ CÔNG THỨC ĐÓ
//! GIẢI ĐƯỢC, nên tên ca không thể lệch.

use crate::cube::CubeState;
use crate::full_state::{apply_move, cross_f2l_ok, from_facelets, FullState};
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

const RAW_ALGS: &[(&str, &str)] = &[
    ("Aa", "R' F R' B2 R F' R' B2 R2"),
    ("Ab", "R2 B2 R F R' B2 R F' R"),
    // E-perm: 2 cặp góc đối chỗ, cạnh đứng yên.
    //
    // Chuỗi cũ `R B' R F2 R' B R F2 R2` KHÔNG phải E-perm — đo được nó giải
    // đúng ca của `Ab` (góc 3-chu-trình, cạnh đứng yên), tức là một A-perm
    // sao chép sai. Hậu quả: 21 tên chỉ ứng 20 ca, E-perm thật không có
    // công thức nào nên 8/288 trạng thái phải rơi xuống `macro_solver`, và
    // 4 trạng thái vốn là `Ab` bị ghi nhãn "E".
    //
    // Chuỗi dưới đây là E-perm cổ điển, vốn viết kèm phép xoay x; ở đây đã
    // thay sang mặt tương ứng để chỉ còn 18 nước cơ bản (bắt buộc:
    // `inv_move` và `full_state::apply_move` không hiểu x/M/r).
    ("E", "R B' R' F R B R' F' R B R' F R B' R' F'"),
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

// GHI CHU: `mirror_move` / `mirror_seq` DA BI XOA.
//
// Ban cu dung chung de nap them ban doi xung cua moi cong thuc, nhung do
// chinh la nguon goc loi gan nham ten: ban doi xung cua mot ca la ca KHAC,
// vay ma no duoc nap duoi CUNG mot ten. Gio bang sinh tu 4x4 to hop AUF
// truoc/sau nen phu tron ma khong can guong -- va ten khong the lech.

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

/// Nước U chèn TRƯỚC và nối SAU công thức.
///
/// Hậu-AUF là chiều mà bảng cũ thiếu hẳn: bộ tra chỉ thử AUF *trước* khi
/// tra bảng (`AUF_OPTIONS`), nên nếu công thức chuẩn đưa lớp cuối về đúng
/// hoán vị mà còn lệch một nước U thì trạng thái đó KHÔNG khớp khoá nào.
/// Đo được 99/288 trạng thái (34%) lọt như vậy, và trên ván thật là 32% số
/// ván phải rơi xuống `macro_solver` — vừa chậm, vừa mất tên ca.
///
/// Tiền-AUF nhìn thì trùng việc với `AUF_OPTIONS`, nhưng sinh cả 4×4 tổ
/// hợp mới phủ trọn: chỉ hậu-AUF thôi vẫn để lọt 8 trạng thái (đo được).
const AUF_MOVES: [&[&str]; 4] = [&[], &["U"], &["U2"], &["U'"]];

/// Ba trạng thái "đã xong nhưng lớp cuối bị lệch một nước U".
///
/// Không ca PLL nào giải chúng (chúng thuộc quỹ đạo của khối đã giải), mà
/// `solve_pll_lookup_named` chỉ nhận ra đúng hoán vị đơn vị, nên trước đây
/// chúng cũng lọt xuống `macro_solver`. Đặt tên "AUF" cho thật: việc phải
/// làm chỉ là xoay lớp U.
const AUF_ONLY: [&str; 3] = ["U", "U2", "U'"];

fn build_table() -> PllTable {
    let mut table = FxHashMap::default();
    let mut name_table = FxHashMap::default();

    // ── Tên có thẩm quyền ────────────────────────────────────────────────
    // Nguyên tắc: trạng thái mà công thức X giải được thì mang TÊN CỦA X.
    //
    // Bảng cũ làm ngược ở một nửa: nó nạp khoá = pattern(X) (trạng thái do
    // X *tạo ra* từ khối đã giải) rồi gán tên X, trong khi trạng thái đó
    // phải do inv(X) giải -- tức là ca NGHỊCH ĐẢO của X. Với ca tự nghịch
    // đảo (T, H, E, Z, V, F, Y, N) thì trùng nhau nên không lộ, nhưng với
    // cặp nghịch đảo (Aa/Ab, Ua/Ub, Ga/Gb, Gc/Gd) thì mục sau ghi đè mục
    // trước, làm 5 tên Aa, Ua, Ga, Gc, Na VĨNH VIỄN không bao giờ xuất
    // hiện -- ca Aa bị ghi thành "Ab", v.v. (đo được 5/21 sai tên).
    //
    // Ở đây khoá luôn là pattern(inv(full)) = đúng trạng thái mà `full`
    // giải được, nên tên không thể lệch.
    for &(name, seqstr) in RAW_ALGS {
        let base: Vec<&'static str> = seqstr.split(' ').collect();
        for pre in AUF_MOVES {
            for post in AUF_MOVES {
                let mut full: Vec<&'static str> = Vec::new();
                full.extend(pre.iter().copied());
                full.extend(base.iter().copied());
                full.extend(post.iter().copied());
                // Trạng thái mà `full` giải được = áp NGHỊCH ĐẢO của `full`
                // lên khối đã giải. Nhờ vậy tên ca không thể lệch.
                let inv = inv_seq(&full);
                let (cp, ep, valid) = pattern_of(&inv);
                if !valid {
                    continue;
                }
                // Giữ lời giải NGẮN NHẤT cho mỗi khoá.
                let better = table
                    .get(&(cp, ep))
                    .map(|old: &Vec<&'static str>| full.len() < old.len())
                    .unwrap_or(true);
                if better {
                    table.insert((cp, ep), full);
                }
                name_table.insert((cp, ep), name);
            }
        }
    }

    // ── Trạng thái chỉ cần xoay U ────────────────────────────────────────
    for mv in AUF_ONLY {
        let seq: Vec<&'static str> = vec![mv];
        let inv = inv_seq(&seq);
        let (cp, ep, valid) = pattern_of(&inv);
        if valid {
            table.insert((cp, ep), seq);
            name_table.insert((cp, ep), "AUF");
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

#[cfg(test)]
mod tests {
    use super::*;

    const AUF4: [&[&str]; 4] = [&[], &["U"], &["U2"], &["U'"]];

    fn perms4() -> Vec<[usize; 4]> {
        let mut out = Vec::new();
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let p = [a, b, c, d];
                        let mut seen = [false; 4];
                        if p.iter().all(|x| {
                            let f = !seen[*x];
                            seen[*x] = true;
                            f
                        }) {
                            out.push(p);
                        }
                    }
                }
            }
        }
        out
    }

    fn parity(p: &[usize; 4]) -> usize {
        let mut n = 0;
        for i in 0..4 {
            for j in i + 1..4 {
                if p[i] > p[j] {
                    n += 1;
                }
            }
        }
        n % 2
    }

    fn state_of(cp4: [usize; 4], ep4: [usize; 4]) -> FullState {
        let mut ep = [0usize; 12];
        let mut cp = [0usize; 8];
        for i in 0..12 {
            ep[i] = i;
        }
        for i in 0..8 {
            cp[i] = i;
        }
        for i in 0..4 {
            cp[i] = cp4[i];
            ep[i] = ep4[i];
        }
        (ep, [0u8; 12], cp, [0u8; 8])
    }

    fn ll_solved(s: &FullState) -> bool {
        let (ep, _, cp, _) = s;
        cp[0..4] == [0, 1, 2, 3] && ep[0..4] == [0, 1, 2, 3]
    }

    /// Mọi hoán vị lớp cuối hợp lệ: tính chẵn lẻ của góc phải bằng của cạnh.
    fn all_states() -> Vec<([usize; 4], [usize; 4])> {
        let ps = perms4();
        let mut out = Vec::new();
        for cp4 in &ps {
            for ep4 in &ps {
                if parity(cp4) == parity(ep4) {
                    out.push((*cp4, *ep4));
                }
            }
        }
        out
    }

    #[test]
    fn co_288_trang_thai() {
        assert_eq!(all_states().len(), 288);
    }

    #[test]
    fn tra_duoc_moi_trang_thai() {
        // Không trạng thái nào được phép lọt xuống `macro_solver`: trước khi
        // sửa có 99/288 (34%) lọt, và trên ván thật là 32% số ván.
        let mut missing = Vec::new();
        for (cp4, ep4) in all_states() {
            let full = state_of(cp4, ep4);
            let (mvs, name) = solve_pll_lookup_named(full);
            if mvs.is_none() || name.is_none() {
                missing.push((cp4, ep4));
            }
        }
        assert!(missing.is_empty(), "{} trạng thái không tra được: {missing:?}", missing.len());
    }

    #[test]
    fn moi_loi_giai_that_su_giai_xong() {
        for (cp4, ep4) in all_states() {
            let full = state_of(cp4, ep4);
            let (mvs, name) = solve_pll_lookup_named(full);
            let Some(mvs) = mvs else { continue };
            let mut s = full;
            for mv in &mvs {
                s = apply_move(&s, mv);
            }
            assert!(
                ll_solved(&s),
                "{:?} (góc {cp4:?} cạnh {ep4:?}) không giải xong sau {} nước",
                name,
                mvs.len()
            );
        }
    }

    #[test]
    fn ten_ca_khong_lech() {
        // Dựng đúng trạng thái mà từng công thức giải được rồi tra ngược:
        // phải ra CHÍNH tên đó.
        //
        // Bảng cũ sai 5/21 (Aa→Ab, Ua→Ub, Ga→Gb, Gc→Gd, Na→Nb) vì nó nạp
        // khoá = trạng thái công thức TẠO RA rồi gán tên công thức, trong
        // khi trạng thái đó phải do nghịch đảo của nó giải.
        for (name, seqstr) in RAW_ALGS {
            let seq: Vec<&'static str> = seqstr.split(' ').collect();
            let inv = inv_seq(&seq);
            let mut st = CubeState::solved();
            st.apply_sequence(&inv);
            let (_, got) = solve_pll_lookup_named(from_facelets(&st));
            assert_eq!(got, Some(*name), "ca {name} bị tra ra tên khác");
        }
    }

    #[test]
    fn khong_co_hai_ten_cung_mot_ca() {
        // Nếu hai tên là cùng một ca thì có một ca THẬT không công thức nào
        // giải được. Chuỗi `E` cũ trùng ca với `Ab` đúng vì lỗi này.
        for (name, seqstr) in RAW_ALGS {
            let seq: Vec<&'static str> = seqstr.split(' ').collect();
            let inv = inv_seq(&seq);
            let mut st = CubeState::solved();
            st.apply_sequence(&inv);
            let target = from_facelets(&st);
            for (n2, s2) in RAW_ALGS {
                if n2 == name {
                    continue;
                }
                let base: Vec<&'static str> = s2.split(' ').collect();
                for pre in AUF4 {
                    for post in AUF4 {
                        let mut s = target;
                        for &mv in pre {
                            s = apply_move(&s, mv);
                        }
                        for &mv in &base {
                            s = apply_move(&s, mv);
                        }
                        for &mv in post {
                            s = apply_move(&s, mv);
                        }
                        assert!(!ll_solved(&s), "{name} và {n2} là cùng một ca");
                    }
                }
            }
        }
    }

    #[test]
    fn du_21_ten_deu_duoc_dung() {
        let mut used: Vec<&str> = Vec::new();
        for (cp4, ep4) in all_states() {
            if let (Some(_), Some(n)) = solve_pll_lookup_named(state_of(cp4, ep4)) {
                if !used.contains(&n) {
                    used.push(n);
                }
            }
        }
        for (name, _) in RAW_ALGS {
            assert!(used.contains(name), "tên {name} không bao giờ được dùng");
        }
    }
}

//! oll_algorithms.rs — port of solver/oll_algorithms.py
//!
//! Bảng công thức OLL ĐỦ 57 THẾ (dinh hướng cả cạnh lẫn góc lớp U trong 1
//! bước). Y hệt cách Python làm: mỗi công thức được ÁP THẬT lên cube đã
//! giải, đọc delta hướng thật sự tạo ra, rồi suy ra key = (-delta) mod
//! (2 hoặc 3) — KHÔNG chép tay key, tự kiểm chứng lúc khởi tạo. Công thức
//! nào phá vỡ Cross+F2L sẽ tự động bị loại khỏi bảng.

use crate::cube::CubeState;
use crate::full_state::{apply_move, cross_f2l_ok, from_facelets, FullState};
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

/// (tên case, chuỗi Singmaster) — ĐỦ 57 thế OLL, đặt tên theo cách đánh
/// số chuẩn OLL 1–57, độ dài 6–14 nước (trung bình 9,9).
///
/// VỀ TÊN: tên cũ là `case_auto_01`…`case_auto_49`, sinh tự động nên nhìn
/// vào không biết là thế nào. Số OLL chuẩn được xác định bằng cách áp công
/// thức chuẩn của từng số lên khối đã giải rồi so khoá — KHÔNG chép tay.
/// Hai nguồn độc lập khớp nhau ở mọi số chung, 57 số rơi vào đúng 57 thế
/// khác nhau, và kết quả tự kiểm chéo: Sune ra OLL 27, AntiSune ra OLL 26,
/// các mục Dot_OLL1/3/4/17/19 mà người viết trước đặt tay đều ra đúng số
/// đó. Xem examples/oll_number.rs.
///
/// VỀ CÔNG THỨC: bảng chỉ chứa 18 nước cơ bản, vì `full_state::apply_move`
/// không hiểu `M`/`r`/`x` — dùng vào sẽ panic ở đường FullState. Mà phần
/// lớn công thức chuẩn NGẮN lại cần đúng những nước đó. Lối ra: VIẾT LẠI
/// chúng thành nước cơ bản.
///
/// Viết lại được là nhờ một chuyện đo được (examples/rewrite_probe.rs):
/// mỗi nước wide = 1 nước cơ bản + 1 phép xoay khối, mỗi slice = 2 nước +
/// 1 phép xoay, mỗi phép xoay = 0 nước. Nên thay vì THỰC HIỆN phép xoay,
/// ta ghi nhớ nó rồi đổi tên mặt cho các nước phía sau — độ dài gần như
/// không tăng. Nhờ vậy tổng số nước 994 → 565 (giảm 43%), và không còn
/// công thức nào dài quá 14 nước (trước đây có mục tới 25 nước).
///
/// Mọi công thức đều qua HAI lớp kiểm chứng độc lập trước khi vào đây:
/// chuỗi viết lại phải cho trạng thái cuối GIỐNG HỆT chuỗi gốc, và áp thật
/// lên trạng thái của khoá thì lớp U định hướng xong, Cross/F2L còn nguyên
/// (xem test `moi_muc_giai_dung_that` và examples/oll_shorten.rs).
const RAW_ALGS: &[(&str, &str)] = &[
    ("OLL01", "R U2 R2 F R F' U2 R' F R F'"),   // 11 nuoc, dang dung
    ("OLL02", "L F L' U2 R U2 R' U2 L F' L'"),   // 11 nuoc, chuan B
    ("OLL03", "R' F2 R2 U2 R' F R U2 R2 F2 R"),   // 11 nuoc, dang dung
    ("OLL04", "R' F2 R2 U2 R' F' R U2 R2 F2 R"),   // 11 nuoc, dang dung
    ("OLL05", "L' B2 R B R' B L"),   // 7 nuoc, chuan B
    ("OLL06", "L F2 R' F' R F' L'"),   // 7 nuoc, chuan A
    ("OLL07", "L F R' F R F2 L'"),   // 7 nuoc, chuan A
    ("OLL08", "R' F' L F' L' F2 R"),   // 7 nuoc, chuan A
    ("OLL09", "R U R' U' R' F R2 U R' U' F'"),   // 11 nuoc, dang dung
    ("OLL10", "R U R' U R' F R F' R U2 R'"),   // 11 nuoc, dang dung
    ("OLL11", "F' B U L U' L U L2 U' L2 F B'"),   // 12 nuoc, chuan B
    ("OLL12", "F' B U' L' U L' U' L2 U L2 F B'"),   // 12 nuoc, chuan B
    ("OLL13", "F U R U2 R' U' R U R' F'"),   // 10 nuoc, dang dung
    ("OLL14", "R' F R U R' F' R F U' F'"),   // 10 nuoc, dang dung
    ("OLL15", "R' F' R L' U' L U R' F R"),   // 10 nuoc, chuan B
    ("OLL16", "L F L' R U R' U' L F' L'"),   // 10 nuoc, chuan B
    ("OLL17", "F R' F' R U F B' D R' D' F' B"),   // 12 nuoc, chuan B
    ("OLL18", "R U2 R2 F R F' U2 L R' F R F' L'"),   // 13 nuoc, chuan A
    ("OLL19", "F B' D R D' F' B U' R' F R F'"),   // 12 nuoc, chuan A
    ("OLL20", "F' B U' L' U L U L U L' U' F B'"),   // 13 nuoc, chuan B
    ("OLL21", "R U R' U R U' R' U R U2 R'"),   // 11 nuoc, dang dung
    ("OLL22", "R U2 R2 U' R2 U' R2 U2 R"),   // 9 nuoc, dang dung
    ("OLL23", "R2 D' R U2 R' D R U2 R"),   // 9 nuoc, dang dung
    ("OLL24", "L F R' F' L' F R F'"),   // 8 nuoc, chuan A
    ("OLL25", "F R' F' L F R F' L'"),   // 8 nuoc, chuan A
    ("OLL26", "R U2 R' U' R U' R'"),   // 7 nuoc, dang dung
    ("OLL27", "R U R' U R U2 R'"),   // 7 nuoc, dang dung
    ("OLL28", "R' F R F' B U' F' U F B'"),   // 10 nuoc, chuan B
    ("OLL29", "L2 U' L B L' U L2 U' L' B' L"),   // 11 nuoc, chuan B
    ("OLL30", "L' F' L U' L' F L2 F' L' U L F L'"),   // 13 nuoc, chuan B
    ("OLL31", "R' U' F U R U' R' F' R"),   // 9 nuoc, dang dung
    ("OLL32", "F' B U L U' L' U' F U B'"),   // 10 nuoc, chuan A
    ("OLL33", "R U R' U' R' F R F'"),   // 8 nuoc, dang dung
    ("OLL34", "B U B' U' L' B' R B L R'"),   // 10 nuoc, chuan B
    ("OLL35", "R U2 R2 F R F' R U2 R'"),   // 9 nuoc, dang dung
    ("OLL36", "L' U' L U' L' U L U L F' L' F"),   // 12 nuoc, dang dung
    ("OLL37", "F R' F' R U R U' R'"),   // 8 nuoc, dang dung
    ("OLL38", "R U R' U R U' R' U' R' F R F'"),   // 12 nuoc, dang dung
    ("OLL39", "L F' L' U' L U F U' L'"),   // 9 nuoc, dang dung
    ("OLL40", "R' F R U R' U' F' U R"),   // 9 nuoc, dang dung
    ("OLL41", "R U R' U R U2 R' F R U R' U' F'"),   // 13 nuoc, dang dung
    ("OLL42", "B' U' R' U R B R U2 R' U' R U' R'"),   // 13 nuoc, dang dung
    ("OLL43", "B' U' R' U R B"),   // 6 nuoc, dang dung
    ("OLL44", "F U R U' R' F'"),   // 6 nuoc, dang dung
    ("OLL45", "F R U R' U' F'"),   // 6 nuoc, dang dung
    ("OLL46", "R' U' R' F R F' U R"),   // 8 nuoc, dang dung
    ("OLL47", "F' L' U' L U L' U' L U F"),   // 10 nuoc, dang dung
    ("OLL48", "F R U R' U' R U R' U' F'"),   // 10 nuoc, dang dung
    ("OLL49", "L F' L2 B L2 F L2 B' L"),   // 9 nuoc, chuan B
    ("OLL50", "L' B L2 F' L2 B' L2 F L'"),   // 9 nuoc, chuan B
    ("OLL51", "F U R U' R' U R U' R' F'"),   // 10 nuoc, dang dung
    ("OLL52", "R' F' U' F U' R U R' U R"),   // 10 nuoc, dang dung
    ("OLL53", "R' F' L F' L' F L F' L' F2 R"),   // 11 nuoc, chuan A
    ("OLL54", "L F R' F R F' R' F R F2 L'"),   // 11 nuoc, chuan B
    ("OLL55", "R' F U R U' R2 F' R2 U R' U' R"),   // 12 nuoc, dang dung
    ("OLL56", "L F L' U R U' R' U R U' R' L F' L'"),   // 14 nuoc, chuan A
    ("OLL57", "F' B U' F U F B' R' F' R"),   // 10 nuoc, chuan B
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::full_state::apply_move;

    /// Phép hoán vị ô mà MỘT nước U gây ra trên khoá OLL.
    ///
    /// KHÔNG phải xoay vòng 0→1→2→3 như trực giác: đo thật trên khối thì
    /// cạnh đi theo chu trình (0 2 1 3) và góc theo (0 1 3 2) — hai chu
    /// trình KHÁC nhau. Dùng sai phép xoay sẽ gom quỹ đạo sai và cho ra
    /// kết luận "thiếu 18 thế" trong khi thật ra chỉ thiếu 2.
    /// `new[σ(i)] = old[i]`, nên `new[j] = old[σ⁻¹(j)]`.
    fn rot(k: OriKey) -> OriKey {
        let (e, c) = k;
        ([e[3], e[2], e[0], e[1]], [c[2], c[0], c[3], c[1]])
    }

    /// Mọi hướng lớp U hợp lệ: số cạnh lật phải chẵn, tổng xoắn góc chia hết 3.
    fn all_orientations() -> Vec<OriKey> {
        let mut out = Vec::new();
        for e in 0..16u8 {
            let eo = [e & 1, (e >> 1) & 1, (e >> 2) & 1, (e >> 3) & 1];
            if eo.iter().sum::<u8>() % 2 != 0 {
                continue;
            }
            for c in 0..81u32 {
                let co = [(c % 3) as u8, (c / 3 % 3) as u8, (c / 9 % 3) as u8, (c / 27 % 3) as u8];
                if co.iter().map(|x| *x as u32).sum::<u32>() % 3 != 0 {
                    continue;
                }
                out.push((eo, co));
            }
        }
        out
    }

    /// Gom thành các lớp tương đương theo AUF. Mỗi lớp = một thế OLL.
    fn orbits() -> Vec<Vec<OriKey>> {
        let mut seen: Vec<OriKey> = Vec::new();
        let mut out: Vec<Vec<OriKey>> = Vec::new();
        for s in all_orientations() {
            if seen.contains(&s) {
                continue;
            }
            let mut orb = Vec::new();
            let mut k = s;
            for _ in 0..4 {
                if !orb.contains(&k) {
                    orb.push(k);
                }
                seen.push(k);
                k = rot(k);
            }
            out.push(orb);
        }
        out
    }

    fn state_with(k: OriKey) -> FullState {
        let mut ep = [0usize; 12];
        let mut cp = [0usize; 8];
        for i in 0..12 {
            ep[i] = i;
        }
        for i in 0..8 {
            cp[i] = i;
        }
        let mut eo = [0u8; 12];
        let mut co = [0u8; 8];
        for i in 0..4 {
            eo[i] = k.0[i];
            co[i] = k.1[i];
        }
        (ep, eo, cp, co)
    }

    #[test]
    fn dung_58_lop_tuong_duong() {
        // 57 thế OLL + 1 lớp đã định hướng xong. Nếu số này đổi thì `rot`
        // sai, không phải bảng sai.
        assert_eq!(orbits().len(), 58);
    }

    #[test]
    fn phu_du_57_the() {
        let t = table();
        let solved: OriKey = ([0, 0, 0, 0], [0, 0, 0, 0]);
        let mut missing = Vec::new();
        let mut covered = 0;
        for orb in orbits() {
            if orb.contains(&solved) {
                continue;
            }
            if orb.iter().any(|k| t.table.contains_key(k)) {
                covered += 1;
            } else {
                missing.push(orb[0]);
            }
        }
        assert!(missing.is_empty(), "thiếu {} thế OLL: {missing:?}", missing.len());
        assert_eq!(covered, 57);
    }

    #[test]
    fn moi_muc_giai_dung_that() {
        // Dựng lại trạng thái từ từng khoá rồi ÁP công thức đã lưu: lớp U
        // phải định hướng xong và Cross/F2L phải còn nguyên. Đây là phép
        // kiểm quan trọng nhất — nó bắt được công thức sai chỗ.
        let t = table();
        for (key, seq) in t.table.iter() {
            let name = t.name_table.get(key).copied().unwrap_or("?");
            let mut s = state_with(*key);
            for &mv in seq {
                s = apply_move(&s, mv);
            }
            let (ep, eo, cp, co) = s;
            for id in 0..12 {
                assert!(
                    !(ep[id] < 4 && eo[id] != 0),
                    "{name}: còn cạnh sai hướng sau khi áp công thức"
                );
            }
            for id in 0..8 {
                assert!(
                    !(cp[id] < 4 && co[id] != 0),
                    "{name}: còn góc sai hướng sau khi áp công thức"
                );
            }
            assert!(cross_f2l_ok(&s), "{name}: phá vỡ Cross/F2L");
        }
    }

    #[test]
    fn moi_ten_ung_dung_mot_the() {
        // Một thế không được tra ra nhiều tên khác nhau tuỳ góc xoay, nếu
        // không thống kê sẽ đếm một thế thành nhiều dòng.
        let t = table();
        let solved: OriKey = ([0, 0, 0, 0], [0, 0, 0, 0]);
        let lookup = |start: OriKey| -> Option<&'static str> {
            let mut k = start;
            for _ in 0..4 {
                if let Some(n) = t.name_table.get(&k) {
                    return Some(*n);
                }
                k = rot(k);
            }
            None
        };
        let mut used: Vec<&str> = Vec::new();
        for orb in orbits() {
            if orb.contains(&solved) {
                continue;
            }
            let mut ns: Vec<&str> = orb.iter().filter_map(|k| lookup(*k)).collect();
            ns.sort();
            ns.dedup();
            assert_eq!(ns.len(), 1, "một thế tra ra nhiều tên: {ns:?}");
            used.push(ns[0]);
        }
        used.sort();
        let n0 = used.len();
        used.dedup();
        assert_eq!(used.len(), n0, "có tên dùng cho nhiều thế");
        assert_eq!(used.len(), 57);
    }

    #[test]
    fn ten_la_so_oll_chuan_01_den_57() {
        // Tên phải là OLL01..OLL57, đủ 57 số, không trùng, không thiếu.
        // Cách đánh số này xác định bằng máy (xem examples/oll_number.rs),
        // không chép tay -- test giữ cho nó không bị sửa lung tung.
        let mut nums: Vec<u8> = RAW_ALGS
            .iter()
            .map(|(n, _)| {
                let d = n.strip_prefix("OLL").unwrap_or_else(|| panic!("tên lạ: {n}"));
                d.parse::<u8>().unwrap_or_else(|_| panic!("tên lạ: {n}"))
            })
            .collect();
        nums.sort();
        assert_eq!(nums, (1..=57).collect::<Vec<u8>>());
    }

    #[test]
    fn khong_cong_thuc_nao_qua_dai() {
        // Bảng từng có mục tới 25 nước vì công thức sinh tự động bằng cách
        // ghép Sune/AntiSune. Sau khi viết lại công thức chuẩn sang nước cơ
        // bản thì dài nhất còn 14. Test giữ mốc đó — nếu ai thêm một công
        // thức ghép dài vào thì biết ngay.
        for (name, seq) in RAW_ALGS {
            let n = seq.split(' ').count();
            assert!(n <= 14, "{name} dài {n} nước, quá 14");
        }
        let tong: usize = RAW_ALGS.iter().map(|(_, s)| s.split(' ').count()).sum();
        assert!(tong <= 600, "tổng số nước {tong} vượt 600");
    }

    #[test]
    fn moi_cong_thuc_deu_vao_bang() {
        // Công thức phá vỡ Cross/F2L bị `build_table` loại âm thầm — nếu
        // ai thêm một công thức sai vào RAW_ALGS thì test này bắt được.
        assert_eq!(table().table.len(), RAW_ALGS.len());
    }
}

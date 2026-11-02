//! Bảng hoán vị 54 ô cho từng nước đi, sinh từ `CubeState::do_move`.
//! Dùng để giao diện web tự tính nước xoay (xem `web/move_table.json`).

use crate::cube::CubeState;

/// Tên nước cơ bản: xoay mặt, lớp giữa, xoay cả khối, nước hai lớp.
pub const BASES: &str = "UDFBLRMESxyzudfblr";

/// Hoán vị của một nước (`mv` như "R", "R'", "R2"): mới[i] = cũ[perm[i]].
pub fn perm_of(mv: &str) -> [u8; 54] {
    let mut s = CubeState {
        faces: [[0u8; 9]; 6],
    };
    for f in 0..6 {
        for i in 0..9 {
            s.faces[f][i] = (f * 9 + i) as u8;
        }
    }
    s.do_move(mv);
    let mut out = [0u8; 54];
    for f in 0..6 {
        for i in 0..9 {
            out[f * 9 + i] = s.faces[f][i];
        }
    }
    out
}

/// JSON gọn: mỗi nước một dòng, 3 biến thể (thuận, nghịch, nửa vòng); `X2'` dùng chung với `X2`.
pub fn json() -> String {
    let mut lines = Vec::new();
    for b in BASES.chars() {
        for suf in ["", "'", "2"] {
            let tok = format!("{b}{suf}");
            let p = perm_of(&tok)
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!("\"{tok}\":[{p}]"));
        }
    }
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

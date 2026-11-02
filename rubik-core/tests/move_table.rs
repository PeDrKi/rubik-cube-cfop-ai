//! Bảng hoán vị cho giao diện web phải khớp engine, và file đã lưu (`web/move_table.json`) không được cũ.
use rubik_core::cube::CubeState;
use rubik_core::move_table::{self, perm_of, BASES};

#[test]
fn file_da_luu_khop_engine() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../web/move_table.json");
    let saved = std::fs::read_to_string(path).expect("thiếu web/move_table.json");
    assert_eq!(
        saved.replace("\r\n", "\n"),
        move_table::json(),
        "web/move_table.json đã cũ: chạy `cargo run -p rubik-core --example gen_move_table > web/move_table.json` rồi `node web/build.js`"
    );
}

#[test]
fn ap_bang_cho_cung_ket_qua_voi_do_move() {
    // Trạng thái có 54 nhãn khác nhau cho phép so sánh từng ô.
    let mut base = CubeState { faces: [[0; 9]; 6] };
    for f in 0..6 {
        for i in 0..9 {
            base.faces[f][i] = (f * 9 + i) as u8;
        }
    }
    for b in BASES.chars() {
        for suf in ["", "'", "2", "2'"] {
            let tok = format!("{b}{suf}");
            let mut s = base;
            s.do_move(&tok);
            let p = perm_of(&tok);
            for k in 0..54 {
                assert_eq!(
                    s.faces[k / 9][k % 9],
                    base.faces[(p[k] / 9) as usize][(p[k] % 9) as usize],
                    "{tok} ô {k}"
                );
            }
        }
    }
}

#[test]
fn moi_hoan_vi_la_song_anh() {
    for b in BASES.chars() {
        for suf in ["", "'", "2"] {
            let mut seen = [false; 54];
            for v in perm_of(&format!("{b}{suf}")) {
                assert!(!seen[v as usize]);
                seen[v as usize] = true;
            }
        }
    }
}

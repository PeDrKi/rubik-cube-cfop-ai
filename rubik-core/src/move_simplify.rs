//! move_simplify.rs — port của solver/move_simplify.py
//!
//! Kỹ thuật: 2 nước trên MẶT ĐỐI DIỆN (U/D, F/B, L/R) GIAO HOÁN được với
//! nhau (đổi thứ tự không đổi kết quả), nên có thể "bubble" 1 nước qua các
//! nước đối diện liền kề để tìm cơ hội gộp VỚI nước CÙNG MẶT ở xa hơn. Sau
//! khi gộp, các nước cùng mặt được CỘNG DỒN (VD R + R = R2, R + R' = huỷ,
//! R2 + R' = R). Lặp lại tới khi không còn rút gọn được nữa.

fn opposite(face: char) -> char {
    match face {
        'U' => 'D', 'D' => 'U',
        'F' => 'B', 'B' => 'F',
        'L' => 'R', 'R' => 'L',
        _ => face,
    }
}

fn face_of(mv: &str) -> char {
    mv.chars().next().unwrap()
}

fn turns_of(mv: &str) -> i32 {
    if mv.ends_with('2') {
        2
    } else if mv.ends_with('\'') {
        3
    } else {
        1
    }
}

fn make(face: char, turns: i32) -> Option<String> {
    let turns = turns.rem_euclid(4);
    match turns {
        0 => None,
        1 => Some(face.to_string()),
        2 => Some(format!("{face}2")),
        _ => Some(format!("{face}'")),
    }
}

/// Rút gọn 1 danh sách nước đi Singmaster. Trả về danh sách mới (có thể
/// ngắn hơn), KHÔNG thay đổi hiệu ứng cuối cùng lên cube.
pub fn simplify(moves: &[&str]) -> Vec<String> {
    let mut moves: Vec<String> = moves.iter().map(|s| s.to_string()).collect();
    let mut changed = true;
    while changed {
        changed = false;

        // bước 1: gộp 2 nước CÙNG MẶT liền kề (R,R->R2 ; R,R'->huỷ ; ...)
        let mut out: Vec<String> = Vec::with_capacity(moves.len());
        let mut i = 0;
        while i < moves.len() {
            if i + 1 < moves.len() && face_of(&moves[i]) == face_of(&moves[i + 1]) {
                let nm = make(face_of(&moves[i]), turns_of(&moves[i]) + turns_of(&moves[i + 1]));
                if let Some(nm) = nm {
                    out.push(nm);
                }
                i += 2;
                changed = true;
            } else {
                out.push(moves[i].clone());
                i += 1;
            }
        }
        moves = out;

        // bước 2: thử đổi chỗ 1 cặp nước trên MẶT ĐỐI DIỆN liền kề (giao
        // hoán, không đổi kết quả) NẾU làm vậy tạo ra 1 cặp CÙNG MẶT mới để
        // gộp ở bước sau. Chỉ đổi 1 cặp mỗi lần để giữ đơn giản & an toàn.
        if moves.len() >= 3 {
            for i in 0..moves.len() - 2 {
                let fa = face_of(&moves[i]);
                let fb = face_of(&moves[i + 1]);
                let fc = face_of(&moves[i + 2]);
                if fb == opposite(fa) && fa == fc {
                    moves.swap(i, i + 1);
                    changed = true;
                    break;
                }
            }
        }
    }
    moves
}

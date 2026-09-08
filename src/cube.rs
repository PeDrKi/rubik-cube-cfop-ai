//! cube.rs — Rust port of cube_engine.py
//!
//! Trạng thái: 6 mặt, mỗi mặt là mảng 9 ô [u8;9] đọc theo hàng (row-major,
//! giống numpy array 3x3 ở bản Python). Facelet value = index mặt màu
//! (0..6), giữ đúng nghĩa "chữ màu" của bản gốc nhưng rẻ hơn nhiều so với
//! String/char.
//!
//! Không có unsafe, không phụ thuộc ngoài (rand chỉ dùng cho scramble).

use std::fmt;

pub const FACES: [char; 6] = ['U', 'D', 'F', 'B', 'L', 'R'];

#[inline]
fn face_idx(c: char) -> usize {
    match c {
        'U' => 0,
        'D' => 1,
        'F' => 2,
        'B' => 3,
        'L' => 4,
        'R' => 5,
        _ => panic!("invalid face char: {c}"),
    }
}

/// Trạng thái cube: state[face][row*3+col], row/col 0..3 giống numpy (3,3).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CubeState {
    pub faces: [[u8; 9]; 6],
}

impl CubeState {
    pub fn solved() -> Self {
        let mut faces = [[0u8; 9]; 6];
        for (f, row) in faces.iter_mut().enumerate() {
            *row = [f as u8; 9];
        }
        CubeState { faces }
    }

    #[inline]
    fn get(&self, f: usize, r: usize, c: usize) -> u8 {
        self.faces[f][r * 3 + c]
    }
    #[inline]
    fn set(&mut self, f: usize, r: usize, c: usize, v: u8) {
        self.faces[f][r * 3 + c] = v;
    }

    pub fn is_solved(&self) -> bool {
        self.faces.iter().all(|f| f.iter().all(|&x| x == f[0]))
    }

    /// rot90(k=-1) numpy tương đương xoay CW một mặt 3x3.
    fn rot_cw(m: [u8; 9]) -> [u8; 9] {
        let g = |r: usize, c: usize| m[r * 3 + c];
        let mut out = [0u8; 9];
        for r in 0..3 {
            for c in 0..3 {
                out[r * 3 + c] = g(2 - c, r);
            }
        }
        out
    }
    fn rot_ccw(m: [u8; 9]) -> [u8; 9] {
        let g = |r: usize, c: usize| m[r * 3 + c];
        let mut out = [0u8; 9];
        for r in 0..3 {
            for c in 0..3 {
                out[r * 3 + c] = g(c, 2 - r);
            }
        }
        out
    }

    fn row(&self, f: usize, r: usize) -> [u8; 3] {
        [self.get(f, r, 0), self.get(f, r, 1), self.get(f, r, 2)]
    }
    fn col(&self, f: usize, c: usize) -> [u8; 3] {
        [self.get(f, 0, c), self.get(f, 1, c), self.get(f, 2, c)]
    }
    fn set_row(&mut self, f: usize, r: usize, v: [u8; 3]) {
        for c in 0..3 {
            self.set(f, r, c, v[c]);
        }
    }
    fn set_col(&mut self, f: usize, c: usize, v: [u8; 3]) {
        for r in 0..3 {
            self.set(f, r, c, v[r]);
        }
    }
    fn rev(v: [u8; 3]) -> [u8; 3] {
        [v[2], v[1], v[0]]
    }

    const U: usize = 0;
    const D: usize = 1;
    const F: usize = 2;
    const B: usize = 3;
    const L: usize = 4;
    const R: usize = 5;

    fn u(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::U] = if prime {
            Self::rot_ccw(s.faces[Self::U])
        } else {
            Self::rot_cw(s.faces[Self::U])
        };
        if !prime {
            self.set_row(Self::F, 0, s.row(Self::R, 0));
            self.set_row(Self::R, 0, s.row(Self::B, 0));
            self.set_row(Self::B, 0, s.row(Self::L, 0));
            self.set_row(Self::L, 0, s.row(Self::F, 0));
        } else {
            self.set_row(Self::F, 0, s.row(Self::L, 0));
            self.set_row(Self::L, 0, s.row(Self::B, 0));
            self.set_row(Self::B, 0, s.row(Self::R, 0));
            self.set_row(Self::R, 0, s.row(Self::F, 0));
        }
    }

    fn d(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::D] = if prime {
            Self::rot_ccw(s.faces[Self::D])
        } else {
            Self::rot_cw(s.faces[Self::D])
        };
        if !prime {
            self.set_row(Self::F, 2, s.row(Self::L, 2));
            self.set_row(Self::L, 2, s.row(Self::B, 2));
            self.set_row(Self::B, 2, s.row(Self::R, 2));
            self.set_row(Self::R, 2, s.row(Self::F, 2));
        } else {
            self.set_row(Self::F, 2, s.row(Self::R, 2));
            self.set_row(Self::R, 2, s.row(Self::B, 2));
            self.set_row(Self::B, 2, s.row(Self::L, 2));
            self.set_row(Self::L, 2, s.row(Self::F, 2));
        }
    }

    fn f(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::F] = if prime {
            Self::rot_ccw(s.faces[Self::F])
        } else {
            Self::rot_cw(s.faces[Self::F])
        };
        if !prime {
            self.set_row(Self::U, 2, Self::rev(s.col(Self::L, 2)));
            self.set_col(Self::L, 2, s.row(Self::D, 0));
            self.set_row(Self::D, 0, Self::rev(s.col(Self::R, 0)));
            self.set_col(Self::R, 0, s.row(Self::U, 2));
        } else {
            self.set_row(Self::U, 2, s.col(Self::R, 0));
            self.set_col(Self::R, 0, Self::rev(s.row(Self::D, 0)));
            self.set_row(Self::D, 0, s.col(Self::L, 2));
            self.set_col(Self::L, 2, Self::rev(s.row(Self::U, 2)));
        }
    }

    fn b(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::B] = if prime {
            Self::rot_ccw(s.faces[Self::B])
        } else {
            Self::rot_cw(s.faces[Self::B])
        };
        if !prime {
            self.set_row(Self::U, 0, s.col(Self::R, 2));
            self.set_col(Self::R, 2, Self::rev(s.row(Self::D, 2)));
            self.set_row(Self::D, 2, s.col(Self::L, 0));
            self.set_col(Self::L, 0, Self::rev(s.row(Self::U, 0)));
        } else {
            self.set_row(Self::U, 0, Self::rev(s.col(Self::L, 0)));
            self.set_col(Self::L, 0, s.row(Self::D, 2));
            self.set_row(Self::D, 2, Self::rev(s.col(Self::R, 2)));
            self.set_col(Self::R, 2, s.row(Self::U, 0));
        }
    }

    fn l(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::L] = if prime {
            Self::rot_ccw(s.faces[Self::L])
        } else {
            Self::rot_cw(s.faces[Self::L])
        };
        if !prime {
            self.set_col(Self::F, 0, s.col(Self::U, 0));
            self.set_col(Self::U, 0, Self::rev(s.col(Self::B, 2)));
            self.set_col(Self::B, 2, Self::rev(s.col(Self::D, 0)));
            self.set_col(Self::D, 0, s.col(Self::F, 0));
        } else {
            self.set_col(Self::F, 0, s.col(Self::D, 0));
            self.set_col(Self::D, 0, Self::rev(s.col(Self::B, 2)));
            self.set_col(Self::B, 2, Self::rev(s.col(Self::U, 0)));
            self.set_col(Self::U, 0, s.col(Self::F, 0));
        }
    }

    fn r(&mut self, prime: bool) {
        let s = *self;
        self.faces[Self::R] = if prime {
            Self::rot_ccw(s.faces[Self::R])
        } else {
            Self::rot_cw(s.faces[Self::R])
        };
        if !prime {
            self.set_col(Self::F, 2, s.col(Self::D, 2));
            self.set_col(Self::D, 2, Self::rev(s.col(Self::B, 0)));
            self.set_col(Self::B, 0, Self::rev(s.col(Self::U, 2)));
            self.set_col(Self::U, 2, s.col(Self::F, 2));
        } else {
            self.set_col(Self::F, 2, s.col(Self::U, 2));
            self.set_col(Self::U, 2, Self::rev(s.col(Self::B, 0)));
            self.set_col(Self::B, 0, Self::rev(s.col(Self::D, 2)));
            self.set_col(Self::D, 2, s.col(Self::F, 2));
        }
    }

    fn m(&mut self, prime: bool) {
        let s = *self;
        if !prime {
            self.set_col(Self::F, 1, s.col(Self::U, 1));
            self.set_col(Self::U, 1, Self::rev(s.col(Self::B, 1)));
            self.set_col(Self::B, 1, Self::rev(s.col(Self::D, 1)));
            self.set_col(Self::D, 1, s.col(Self::F, 1));
        } else {
            self.set_col(Self::F, 1, s.col(Self::D, 1));
            self.set_col(Self::D, 1, Self::rev(s.col(Self::B, 1)));
            self.set_col(Self::B, 1, Self::rev(s.col(Self::U, 1)));
            self.set_col(Self::U, 1, s.col(Self::F, 1));
        }
    }

    fn e(&mut self, prime: bool) {
        let s = *self;
        if !prime {
            self.set_row(Self::F, 1, s.row(Self::L, 1));
            self.set_row(Self::L, 1, s.row(Self::B, 1));
            self.set_row(Self::B, 1, s.row(Self::R, 1));
            self.set_row(Self::R, 1, s.row(Self::F, 1));
        } else {
            self.set_row(Self::F, 1, s.row(Self::R, 1));
            self.set_row(Self::R, 1, s.row(Self::B, 1));
            self.set_row(Self::B, 1, s.row(Self::L, 1));
            self.set_row(Self::L, 1, s.row(Self::F, 1));
        }
    }

    fn s(&mut self, prime: bool) {
        let s = *self;
        if !prime {
            self.set_row(Self::U, 1, Self::rev(s.col(Self::L, 1)));
            self.set_col(Self::L, 1, s.row(Self::D, 1));
            self.set_row(Self::D, 1, Self::rev(s.col(Self::R, 1)));
            self.set_col(Self::R, 1, s.row(Self::U, 1));
        } else {
            self.set_row(Self::U, 1, s.col(Self::R, 1));
            self.set_col(Self::R, 1, Self::rev(s.row(Self::D, 1)));
            self.set_row(Self::D, 1, s.col(Self::L, 1));
            self.set_col(Self::L, 1, Self::rev(s.row(Self::U, 1)));
        }
    }

    fn x(&mut self, prime: bool) {
        self.r(prime);
        self.m(!prime);
        self.l(!prime);
    }
    fn y(&mut self, prime: bool) {
        self.u(prime);
        self.e(!prime);
        self.d(!prime);
    }
    fn z(&mut self, prime: bool) {
        self.f(prime);
        self.s(prime);
        self.b(!prime);
    }

    fn do_single(&mut self, mv: &str) {
        let prime = mv.ends_with('\'');
        let base = if prime { &mv[..mv.len() - 1] } else { mv };
        match base {
            "U" => self.u(prime),
            "D" => self.d(prime),
            "F" => self.f(prime),
            "B" => self.b(prime),
            "L" => self.l(prime),
            "R" => self.r(prime),
            "M" => self.m(prime),
            "E" => self.e(prime),
            "S" => self.s(prime),
            "x" => self.x(prime),
            "y" => self.y(prime),
            "z" => self.z(prime),
            "u" => {
                self.u(prime);
                self.e(!prime);
            }
            "d" => {
                self.d(prime);
                self.e(prime);
            }
            "f" => {
                self.f(prime);
                self.s(prime);
            }
            "b" => {
                self.b(prime);
                self.s(!prime);
            }
            "l" => {
                self.l(prime);
                self.m(prime);
            }
            "r" => {
                self.r(prime);
                self.m(!prime);
            }
            other => panic!("invalid base move: {other}"),
        }
    }

    /// Áp một nước đi Singmaster đầy đủ (hỗ trợ suffix ', 2, 2', '2).
    pub fn do_move(&mut self, mv: &str) {
        if mv.ends_with("2'") || mv.ends_with("'2") {
            let base = &mv[..mv.len() - 2];
            self.do_single(base);
            self.do_single(base);
        } else if let Some(base) = mv.strip_suffix('2') {
            self.do_single(base);
            self.do_single(base);
        } else {
            self.do_single(mv);
        }
    }

    pub fn apply_sequence(&mut self, moves: &[&str]) {
        for m in moves {
            self.do_move(m);
        }
    }
}

impl fmt::Debug for CubeState {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, fc) in FACES.iter().enumerate() {
            writeln!(out, "{fc}: {:?}", self.faces[i])?;
        }
        Ok(())
    }
}

// ── Scramble ────────────────────────────────────────────────────────────────

pub const ALL_MOVES: [&str; 18] = [
    "U", "U'", "U2", "D", "D'", "D2", "F", "F'", "F2", "B", "B'", "B2", "L", "L'", "L2", "R",
    "R'", "R2",
];

/// Scramble n nước ngẫu nhiên, tránh 2 nước liên tiếp cùng mặt.
/// Trả về danh sách nước đã đi (giống random_scramble_moves bản Python).
pub fn random_scramble_moves(
    state: &mut CubeState,
    n: usize,
    rng: &mut impl rand::Rng,
) -> Vec<&'static str> {
    let mut prev_face: Option<char> = None;
    let mut moves = Vec::with_capacity(n);
    for _ in 0..n {
        let candidates: Vec<&'static str> = match prev_face {
            Some(pf) => ALL_MOVES
                .iter()
                .copied()
                .filter(|m| m.chars().next().unwrap() != pf)
                .collect(),
            None => ALL_MOVES.to_vec(),
        };
        let mv = candidates[rng.gen_range(0..candidates.len())];
        state.do_move(mv);
        moves.push(mv);
        prev_face = mv.chars().next();
    }
    moves
}

// ── Singmaster parser ──────────────────────────────────────────────────────

const VALID_BASES: &str = "UDFBLRMESxyzudfblr";

/// Mở rộng nhóm lặp `(...)n`, lồng nhau, giới hạn 10 vòng như bản Python.
pub fn expand_repeats(text: &str) -> String {
    let mut text = text.to_string();
    for _ in 0..10 {
        let Some(open) = text.find('(') else { break };
        // tìm ')' khớp gần nhất không chứa '(' khác ở giữa (nhóm không lồng trực tiếp,
        // giống regex [^()]+ của bản gốc)
        let Some(rel_close) = text[open..].find(')') else {
            break;
        };
        let close = open + rel_close;
        if text[open + 1..close].contains('(') {
            break; // không khớp dạng [^()]+ -- dừng như regex gốc sẽ không match
        }
        let inner = text[open + 1..close].trim().to_string();
        // đọc số lặp ngay sau ')'
        let rest = &text[close + 1..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            break;
        }
        let count: usize = digits.parse().unwrap();
        let expanded = std::iter::repeat(inner.as_str())
            .take(count)
            .collect::<Vec<_>>()
            .join(" ");
        text = format!(
            "{}{}{}",
            &text[..open],
            expanded,
            &text[close + 1 + digits.len()..]
        );
    }
    text
}

fn is_valid_base(c: char) -> bool {
    VALID_BASES.contains(c)
}

/// Parse chuỗi Singmaster. Ok(moves) hoặc Err(thông báo lỗi).
pub fn parse_singmaster(text: &str) -> Result<Vec<String>, String> {
    let expanded = expand_repeats(text);
    let chars: Vec<char> = expanded.chars().collect();
    let mut i = 0;
    let mut moves = Vec::new();
    let mut leftover = String::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == ',' || c == '(' || c == ')' {
            i += 1;
            continue;
        }
        if is_valid_base(c) {
            let mut tok = String::new();
            tok.push(c);
            i += 1;
            // suffix: 2' | '2 | 2 | '
            if i + 1 < chars.len() && chars[i] == '2' && chars[i + 1] == '\'' {
                tok.push_str("2'");
                i += 2;
            } else if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '2' {
                tok.push_str("2'"); // chuẩn hóa '2 -> 2'
                i += 2;
            } else if i < chars.len() && chars[i] == '2' {
                tok.push('2');
                i += 1;
            } else if i < chars.len() && chars[i] == '\'' {
                tok.push('\'');
                i += 1;
            }
            moves.push(tok);
        } else {
            leftover.push(c);
            i += 1;
        }
    }

    if !leftover.is_empty() {
        return Err(format!("Unknown: '{leftover}'"));
    }
    Ok(moves)
}

/// Đảo ngược chuỗi moves.
pub fn invert_moves(moves: &[String]) -> Vec<String> {
    fn inv_one(m: &str) -> String {
        if let Some(base) = m.strip_suffix("2'") {
            format!("{base}2")
        } else if let Some(base) = m.strip_suffix('2') {
            format!("{base}2")
        } else if let Some(base) = m.strip_suffix('\'') {
            base.to_string()
        } else {
            format!("{m}'")
        }
    }
    moves.iter().rev().map(|m| inv_one(m)).collect()
}

#[allow(dead_code)]
pub fn face_char(f: usize) -> char {
    FACES[f]
}
#[allow(dead_code)]
pub fn idx_of(c: char) -> usize {
    face_idx(c)
}

impl CubeState {
    /// Đọc trực tiếp 1 facelet theo toạ độ (face_idx, row, col). Dùng bởi
    /// edge_model/corner_model để "nhận diện" cubie giống hệt cách Python
    /// đọc state[f][r,c].
    pub fn facelet_at(&self, face: usize, row: usize, col: usize) -> u8 {
        self.get(face, row, col)
    }
}

/// Toàn bộ 18 nước outer-move, thứ tự cố định — dùng làm chỉ số (move_index)
/// xuyên suốt các bảng hoán vị edge/corner, giống ALL_MOVES trong Python.
pub fn move_index(mv: &str) -> usize {
    ALL_MOVES.iter().position(|&m| m == mv).unwrap_or_else(|| panic!("unknown move {mv}"))
}

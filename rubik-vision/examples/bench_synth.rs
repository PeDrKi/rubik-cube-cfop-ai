//! Bộ thử tổng hợp cho phần nhận diện màu: sinh 6 ảnh mặt (9 mẫu RGB mỗi mặt) từ một khối đã xáo,
//! áp ánh sáng/nhiễu/loá/xoay mặt ngẫu nhiên theo từng mức khó, rồi chạy `resolve_scan` và so với đáp án.
//! Dùng: cargo run --release -p rubik-vision --example bench_synth [số_ca_mỗi_mức]
//! Kết quả: đúng (ra đúng khối) / sai âm thầm (ra khối hợp lệ nhưng SAI, nguy hiểm nhất) / báo lỗi (người dùng quét lại).

use rubik_core::cube::CubeState;
use rubik_vision::scan::{resolve_scan, RawFace, RawScan, ScanError};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    fn gauss(&mut self) -> f32 {
        let (u1, u2) = (self.f().max(1e-6), self.f());
        (-2.0 * u1.ln()).sqrt() * (6.2831853 * u2).cos()
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Màu sticker thật khi chụp dưới ánh sáng chuẩn: U D F B L R = trắng, vàng, xanh lá, xanh dương, cam, đỏ.
const BASE: [[f32; 3]; 6] = [
    [235.0, 235.0, 230.0],
    [235.0, 205.0, 25.0],
    [25.0, 150.0, 70.0],
    [20.0, 75.0, 190.0],
    [240.0, 110.0, 25.0],
    [185.0, 25.0, 40.0],
];

struct Level {
    name: &'static str,
    gain: (f32, f32),
    cast: f32,
    noise: f32,
    shade: f32,
    glare_p: f32,
    lamp: [f32; 3],
}

const ABL: [Level; 6] = [
    Level { name: "chỉ độ sáng mặt 0.5-1.3", gain: (0.5, 1.3), cast: 0.0, noise: 0.0, shade: 0.0, glare_p: 0.0, lamp: [1.0, 1.0, 1.0] },
    Level { name: "chỉ lệch màu mặt ±15%   ", gain: (1.0, 1.0), cast: 0.15, noise: 0.0, shade: 0.0, glare_p: 0.0, lamp: [1.0, 1.0, 1.0] },
    Level { name: "chỉ đèn vàng ấm         ", gain: (1.0, 1.0), cast: 0.0, noise: 0.0, shade: 0.0, glare_p: 0.0, lamp: [1.2, 0.98, 0.7] },
    Level { name: "chỉ nhiễu σ=12          ", gain: (1.0, 1.0), cast: 0.0, noise: 12.0, shade: 0.0, glare_p: 0.0, lamp: [1.0, 1.0, 1.0] },
    Level { name: "chỉ đổ bóng 0.4         ", gain: (1.0, 1.0), cast: 0.0, noise: 0.0, shade: 0.4, glare_p: 0.0, lamp: [1.0, 1.0, 1.0] },
    Level { name: "chỉ loá (80% mặt)       ", gain: (1.0, 1.0), cast: 0.0, noise: 0.0, shade: 0.0, glare_p: 0.8, lamp: [1.0, 1.0, 1.0] },
];

/// "Khó" bỏ lần lượt từng yếu tố: cho thấy yếu tố nào còn gây lỗi nhiều nhất.
const LOO: [Level; 6] = [
    Level { name: "khó, bỏ đổi độ sáng mặt ", gain: (1.0, 1.0), cast: 0.2, noise: 10.0, shade: 0.4, glare_p: 0.6, lamp: [1.12, 0.98, 0.78] },
    Level { name: "khó, bỏ lệch màu mặt    ", gain: (0.45, 1.3), cast: 0.0, noise: 10.0, shade: 0.4, glare_p: 0.6, lamp: [1.12, 0.98, 0.78] },
    Level { name: "khó, bỏ đèn vàng        ", gain: (0.45, 1.3), cast: 0.2, noise: 10.0, shade: 0.4, glare_p: 0.6, lamp: [1.0, 1.0, 1.0] },
    Level { name: "khó, bỏ nhiễu           ", gain: (0.45, 1.3), cast: 0.2, noise: 0.0, shade: 0.4, glare_p: 0.6, lamp: [1.12, 0.98, 0.78] },
    Level { name: "khó, bỏ đổ bóng         ", gain: (0.45, 1.3), cast: 0.2, noise: 10.0, shade: 0.0, glare_p: 0.6, lamp: [1.12, 0.98, 0.78] },
    Level { name: "khó, bỏ loá             ", gain: (0.45, 1.3), cast: 0.2, noise: 10.0, shade: 0.4, glare_p: 0.0, lamp: [1.12, 0.98, 0.78] },
];

const LEVELS: [Level; 4] = [
    Level { name: "dễ     ", gain: (0.85, 1.1), cast: 0.04, noise: 4.0, shade: 0.08, glare_p: 0.0, lamp: [1.0, 1.0, 1.0] },
    Level { name: "vừa    ", gain: (0.6, 1.2), cast: 0.12, noise: 7.0, shade: 0.25, glare_p: 0.3, lamp: [1.0, 0.97, 0.9] },
    Level { name: "khó    ", gain: (0.45, 1.3), cast: 0.2, noise: 10.0, shade: 0.4, glare_p: 0.6, lamp: [1.12, 0.98, 0.78] },
    Level { name: "rất khó", gain: (0.35, 1.4), cast: 0.28, noise: 14.0, shade: 0.5, glare_p: 0.8, lamp: [1.2, 0.98, 0.7] },
];

fn rotate9<T: Copy + Default>(m: [T; 9], k: usize) -> [T; 9] {
    let mut out = m;
    for _ in 0..k % 4 {
        let g = out;
        let mut next = [T::default(); 9];
        for r in 0..3 {
            for c in 0..3 {
                next[r * 3 + c] = g[(2 - c) * 3 + r];
            }
        }
        out = next;
    }
    out
}

fn make_scan(state: &CubeState, lv: &Level, rng: &mut Rng) -> RawScan {
    let faces = std::array::from_fn(|f| {
        let gain = rng.range(lv.gain.0, lv.gain.1);
        let cast: [f32; 3] = std::array::from_fn(|k| lv.lamp[k] * rng.range(1.0 - lv.cast, 1.0 + lv.cast));
        let (sx, sy) = (rng.range(-1.0, 1.0) * lv.shade, rng.range(-1.0, 1.0) * lv.shade);
        let glare_n = if rng.f() < lv.glare_p { 1 + rng.below(2) } else { 0 };
        let mut glare = [false; 9];
        for _ in 0..glare_n {
            glare[rng.below(9)] = true;
        }
        let mut conf = [1.0f32; 9];
        let samples: [[u8; 3]; 9] = std::array::from_fn(|s| {
            let base = BASE[state.faces[f][s] as usize];
            let (x, y) = ((s % 3) as f32 - 1.0, (s / 3) as f32 - 1.0);
            let shade = (1.0 + sx * x + sy * y).max(0.2);
            let mut rgb: [f32; 3] = std::array::from_fn(|k| base[k] * gain * cast[k] * shade + rng.gauss() * lv.noise);
            if glare[s] {
                let w = rng.range(0.3, 0.7);
                for c in rgb.iter_mut() {
                    *c = *c * (1.0 - w) + 255.0 * w;
                }
                conf[s] = 0.5; // cảm biến thật đo được tỉ lệ pixel cháy sáng nên hạ độ tin cậy
            }
            std::array::from_fn(|k| rgb[k].clamp(0.0, 255.0) as u8)
        });
        let k = rng.below(4);
        RawFace { samples: rotate9(samples, k), confidence: rotate9(conf, k) } // độ tin cậy quay theo ô
    });
    RawScan { faces }
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    println!("{:<24} {:>7} {:>10} {:>9}   (n = {n} ca mỗi mức)", "mức", "đúng", "sai âm thầm", "báo lỗi");
    let mut tot = (0usize, 0usize, 0usize);
    let abl = std::env::args().nth(2).as_deref() == Some("abl");
    let loo = std::env::args().nth(2).as_deref() == Some("loo");
    let levels: &[Level] = if abl { &ABL } else if loo { &LOO } else { &LEVELS };
    for lv in levels {
        let mut rng = Rng(0x9E3779B97F4A7C15);
        let (mut ok, mut wrong, mut err) = (0, 0, 0);
        for _ in 0..n {
            let mut st = CubeState::solved();
            let moves = ["U", "D", "L", "R", "F", "B", "U'", "D'", "L'", "R'", "F'", "B'", "U2", "R2", "F2"];
            for _ in 0..25 {
                st.do_move(moves[rng.below(moves.len())]);
            }
            let scan = make_scan(&st, lv, &mut rng);
            match resolve_scan(&scan) {
                Ok(rep) if rep.state.faces == st.faces => ok += 1,
                Ok(_) => wrong += 1,
                Err(ScanError::NoValidArrangement { .. }) | Err(ScanError::Ambiguous { .. }) => err += 1,
            }
        }
        println!(
            "{:<24} {:>6.1}% {:>9.1}% {:>8.1}%",
            lv.name,
            100.0 * ok as f32 / n as f32,
            100.0 * wrong as f32 / n as f32,
            100.0 * err as f32 / n as f32
        );
        tot = (tot.0 + ok, tot.1 + wrong, tot.2 + err);
    }
    let all = (tot.0 + tot.1 + tot.2) as f32;
    println!("tổng     {:>6.1}% {:>9.1}% {:>8.1}%", 100.0 * tot.0 as f32 / all, 100.0 * tot.1 as f32 / all, 100.0 * tot.2 as f32 / all);
}

//! Đo thời gian xử lý MỘT khung hình quét: dò mặt -> duỗi phối cảnh -> lấy mẫu.
//! Dùng: cargo run [--release] -p rubik-vision --example bench_frame [rộng] [cao] [số_lần]
use image::{Rgb, RgbImage};
use rubik_vision::detect::detect_cube_face;
use rubik_vision::perspective::warp_to_square;
use rubik_vision::sample::{sample_grid_with_confidence, SquareRegion};
use std::time::Instant;

fn scene(w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(w, h, Rgb([18, 18, 22]));
    let cols = [[220, 40, 40], [40, 180, 60], [40, 70, 220], [240, 220, 40], [240, 130, 30], [235, 235, 235]];
    let side = h as f32 * 0.6;
    let (x0, y0) = (w as f32 * 0.5 - side / 2.0, h as f32 * 0.5 - side / 2.0);
    let cell = side / 3.0;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 - x0, y as f32 - y0);
            if fx < 0.0 || fy < 0.0 || fx >= side || fy >= side { continue; }
            let (cx, cy) = ((fx / cell) as usize, (fy / cell) as usize);
            let (ix, iy) = (fx % cell, fy % cell);
            if ix < 3.0 || iy < 3.0 { continue; }
            let c = cols[(cy * 3 + cx) % 6];
            img.put_pixel(x, y, Rgb([c[0], c[1], c[2]]));
        }
    }
    img
}

fn main() {
    let a: Vec<u32> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let (w, h, n) = (*a.first().unwrap_or(&320), *a.get(1).unwrap_or(&240), *a.get(2).unwrap_or(&30));
    let img = scene(w, h);
    let (mut td, mut tw, mut ts) = (0.0, 0.0, 0.0);
    for _ in 0..n {
        let t = Instant::now();
        let d = detect_cube_face(&img).expect("không dò được mặt");
        td += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let r = warp_to_square(&img, d.quad, 300).expect("warp");
        tw += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let _ = sample_grid_with_confidence(&r, SquareRegion { x: 0, y: 0, size: 300 });
        ts += t.elapsed().as_secs_f64();
    }
    let n = n as f64;
    println!("{w}x{h}: dò {:.2} ms | duỗi {:.2} ms | lấy mẫu {:.2} ms | tổng {:.2} ms/khung",
        td / n * 1e3, tw / n * 1e3, ts / n * 1e3, (td + tw + ts) / n * 1e3);
}

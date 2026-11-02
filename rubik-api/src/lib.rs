//! rubik-api: toàn bộ "route" của bộ giải, dùng chung cho rubik-wasm (trình duyệt) và rubik-web (axum).
//! Không phụ thuộc wasm_bindgen hay axum. Mọi nước đi đi qua parse_singmaster (chỉ sinh base hợp lệ) trước khi do_move.

use rand::{rngs::StdRng, SeedableRng};
use rubik_core::breakdown::{full_solve_breakdown, StageStatus};
use rubik_core::cube::{self, CubeState};
use rubik_core::{hint, oll_algorithms, pll_algorithms};
use rubik_vision::detect::detect_cube_face;
use rubik_vision::perspective::warp_to_square;
use rubik_vision::sample::{aggregate_frames, sample_grid_with_confidence, SquareRegion};
use rubik_vision::scan::{resolve_scan, validate_facelets, RawFace, RawScan, ScanError};
use serde::Deserialize;
use serde_json::{json, Value};

fn encode(s: &CubeState) -> String {
    s.faces
        .iter()
        .flat_map(|f| f.iter().map(|&v| cube::face_char(v as usize)))
        .collect()
}

fn decode(text: &str) -> Result<CubeState, String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() != 54 || chars.iter().any(|c| !"UDFBLR".contains(*c)) {
        return Err("Trạng thái phải gồm đúng 54 ký tự trong U D F B L R".into());
    }
    let mut faces = [[0u8; 9]; 6];
    for (i, c) in chars.iter().enumerate() {
        faces[i / 9][i % 9] = cube::idx_of(*c) as u8;
    }
    validate_facelets(faces)
}

fn parse<T: for<'a> Deserialize<'a>>(body: &str) -> Result<T, String> {
    serde_json::from_str(if body.trim().is_empty() { "{}" } else { body })
        .map_err(|e| e.to_string())
}

fn stage_json(st: &StageStatus) -> Value {
    match st {
        StageStatus::Done => json!({ "status": "done", "moves": [] }),
        StageStatus::Moves { moves, case_name } => {
            json!({ "status": "moves", "moves": moves, "case_name": case_name })
        }
        StageStatus::Failed => json!({ "status": "failed", "moves": [] }),
        StageStatus::Pending => json!({ "status": "pending", "moves": [] }),
    }
}

pub fn route(name: &str, body: &str) -> Result<Value, String> {
    match name {
        "warmup" => {
            hint::warmup();
            Ok(json!({}))
        }
        "solved" => Ok(json!({ "state": encode(&CubeState::solved()) })),
        "scramble" => {
            #[derive(Deserialize)]
            struct Q {
                n: Option<usize>,
                seed: Option<u64>,
            }
            let q: Q = parse(body)?;
            let mut s = CubeState::solved();
            // Không truyền seed thì xáo ngẫu nhiên thật (trước đây mặc định seed = 1 nên luôn ra cùng một đề).
            let mut rng = q
                .seed
                .map_or_else(StdRng::from_entropy, StdRng::seed_from_u64);
            let mv = cube::random_scramble_moves(&mut s, q.n.unwrap_or(25).clamp(1, 60), &mut rng);
            Ok(json!({ "state": encode(&s), "moves": mv }))
        }
        "apply" => {
            #[derive(Deserialize)]
            struct Q {
                state: String,
                moves: String,
            }
            let q: Q = parse(body)?;
            let mut s = decode(&q.state)?;
            let moves = cube::parse_singmaster(&q.moves)?;
            for m in &moves {
                s.do_move(m);
            }
            Ok(json!({ "state": encode(&s), "moves": moves, "solved": s.is_solved() }))
        }
        "validate" => {
            #[derive(Deserialize)]
            struct Q {
                state: String,
            }
            let s = decode(&parse::<Q>(body)?.state)?;
            Ok(json!({ "state": encode(&s) }))
        }
        "hint" => {
            #[derive(Deserialize)]
            struct Q {
                state: String,
                seed: Option<u64>,
            }
            let q: Q = parse(body)?;
            let s = decode(&q.state)?;
            let h = hint::compute_hint(&s, q.seed);
            Ok(
                json!({ "label": h.label, "moves": h.moves, "stage": h.stage, "case_name": h.case_name }),
            )
        }
        "solve" => {
            #[derive(Deserialize)]
            struct Q {
                state: String,
            }
            let s = decode(&parse::<Q>(body)?.state)?;
            let b = full_solve_breakdown(&s);
            let f2l: Vec<Value> = b
                .f2l
                .iter()
                .map(|(slot, st)| {
                    let mut v = stage_json(st);
                    v["slot"] = json!(slot);
                    v
                })
                .collect();
            Ok(
                json!({ "cross": stage_json(&b.cross), "f2l": f2l, "oll": stage_json(&b.oll), "pll": stage_json(&b.pll), "total": b.total_moves() }),
            )
        }
        "algos" => {
            let pack = |v: &[(&str, &str)]| {
                v.iter()
                    .map(|(n, a)| json!({ "name": n, "alg": a }))
                    .collect::<Vec<_>>()
            };
            Ok(
                json!({ "oll": pack(oll_algorithms::all_algorithms()), "pll": pack(pll_algorithms::all_algorithms()) }),
            )
        }
        "scan_aggregate" => {
            #[derive(Deserialize)]
            struct Q {
                frames: Vec<FaceIn>,
            }
            let q: Q = parse(body)?;
            if q.frames.is_empty() {
                return Err("Không có khung hình nào để gộp".into());
            }
            let v: Vec<([[u8; 3]; 9], [f32; 9])> =
                q.frames.iter().map(|f| (f.samples, f.conf)).collect();
            let (samples, conf) = aggregate_frames(&v);
            Ok(json!({ "samples": samples, "conf": conf }))
        }
        "scan_resolve" => {
            #[derive(Deserialize)]
            struct Q {
                faces: Vec<FaceIn>,
            }
            let q: Q = parse(body)?;
            if q.faces.len() != 6 {
                return Err("Cần đủ 6 mặt".into());
            }
            let faces: [RawFace; 6] = std::array::from_fn(|i| RawFace {
                samples: q.faces[i].samples,
                confidence: q.faces[i].conf,
            });
            Ok(match resolve_scan(&RawScan { faces }) {
                Ok(r) => {
                    json!({ "kind": "ok", "state": encode(&r.state), "rotations": r.rotations_applied, "auto_fixed": r.auto_fixed })
                }
                Err(ScanError::NoValidArrangement { suspicious }) => {
                    json!({ "kind": "none", "suspicious": suspicious })
                }
                Err(ScanError::Ambiguous { candidates }) => {
                    json!({ "kind": "ambiguous", "candidates": candidates.iter().map(encode).collect::<Vec<_>>() })
                }
            })
        }
        other => Err(format!("Route không tồn tại: {other}")),
    }
}

#[derive(Deserialize)]
struct FaceIn {
    samples: [[u8; 3]; 9],
    conf: [f32; 9],
}

/// Dựng sẵn các bảng dữ liệu nặng (gọi 1 lần ở thread nền / lúc khởi động).
pub fn warmup() {
    hint::warmup();
}

const WARP_SIZE: u32 = 300;

/// Giống `sample_for_region` trong scan_ui.rs: hiệu chỉnh phối cảnh nếu có tứ giác rồi mới lấy mẫu 3x3.
fn sample_for_region(
    img: &image::RgbImage,
    region: SquareRegion,
    quad: Option<[(f32, f32); 4]>,
) -> ([[u8; 3]; 9], [f32; 9]) {
    if let Some(q) = quad {
        if let Some(rectified) = warp_to_square(img, q, WARP_SIZE) {
            return sample_grid_with_confidence(
                &rectified,
                SquareRegion {
                    x: 0,
                    y: 0,
                    size: WARP_SIZE,
                },
            );
        }
    }
    sample_grid_with_confidence(img, region)
}

/// Giống `fallback_region` trong scan_ui.rs: khung vuông canh tay ở giữa ảnh.
fn fallback_region(img: &image::RgbImage, region_scale: f32) -> SquareRegion {
    let (w, h) = (img.width(), img.height());
    let side = (w.min(h) as f32 * region_scale.clamp(0.4, 0.95)) as u32;
    SquareRegion {
        x: w.saturating_sub(side) / 2,
        y: h.saturating_sub(side) / 2,
        size: side,
    }
}

/// Giống `face_looks_valid` trong scan_ui.rs.
fn face_looks_valid(samples: &[[u8; 3]; 9]) -> bool {
    if samples.contains(&[128, 128, 128]) {
        return false;
    }
    let avg: u32 = samples
        .iter()
        .map(|c| c[0] as u32 + c[1] as u32 + c[2] as u32)
        .sum::<u32>()
        / (9 * 3);
    avg >= 15
}

/// Xử lý 1 khung hình RGBA: dò mặt khối bằng hình học, hiệu chỉnh phối cảnh, lấy mẫu 9 ô + độ tin cậy.
pub fn scan_frame(w: u32, h: u32, rgba: &[u8], region_scale: f32) -> Result<Value, String> {
    if w == 0 || h == 0 || rgba.len() != (w as usize) * (h as usize) * 4 {
        return Err("Kích thước khung hình không khớp dữ liệu".into());
    }
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    let img = image::RgbImage::from_raw(w, h, rgb).ok_or("Ảnh không hợp lệ")?;
    let (region, quad, auto) = match detect_cube_face(&img) {
        Some(d) => (d.region, Some(d.quad), true),
        None => (fallback_region(&img, region_scale), None, false),
    };
    let (samples, conf) = sample_for_region(&img, region, quad);
    Ok(json!({
        "auto": auto,
        "region": { "x": region.x, "y": region.y, "size": region.size },
        "quad": quad.map(|q| q.map(|(x, y)| [x, y])),
        "samples": samples,
        "conf": conf,
        "valid": face_looks_valid(&samples),
    }))
}

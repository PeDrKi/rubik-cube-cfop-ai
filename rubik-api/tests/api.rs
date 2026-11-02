//! Kiểm thử đầu-cuối cho rubik-api, chạy ở đúng cấu hình của bản wasm (rubik-core KHÔNG bật rayon).
//! Chạy: cargo test -p rubik-api --release -- --nocapture
use rubik_api::{route, scan_frame};
use serde_json::{json, Value};

const SOLVED: &str = "UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR";
// Màu của app (mesh.rs face_color), thứ tự U D F B L R.
const PAL: [[u8; 3]; 6] = [
    [255, 210, 0],
    [255, 255, 255],
    [15, 160, 55],
    [15, 75, 210],
    [210, 30, 45],
    [255, 100, 0],
];

fn ok(name: &str, body: Value) -> Value {
    route(name, &body.to_string()).unwrap_or_else(|e| panic!("route {name} lỗi: {e}"))
}
fn scramble(seed: u64) -> String {
    ok("scramble", json!({ "n": 25, "seed": seed }))["state"]
        .as_str()
        .unwrap()
        .to_string()
}
fn state_samples(state: &str) -> Vec<Value> {
    (0..6)
        .map(|f| {
            let s: Vec<[u8; 3]> = state[f * 9..f * 9 + 9]
                .chars()
                .map(|c| PAL["UDFBLR".find(c).unwrap()])
                .collect();
            json!({ "samples": s, "conf": vec![1.0f32; 9] })
        })
        .collect()
}

#[test]
fn co_ban() {
    assert_eq!(ok("solved", json!({}))["state"], SOLVED);
    let a = ok("algos", json!({}));
    assert_eq!(a["oll"].as_array().unwrap().len(), 57);
    assert_eq!(a["pll"].as_array().unwrap().len(), 21);
    assert_eq!(scramble(7), scramble(7), "cùng seed phải cho cùng kết quả");
    assert_ne!(scramble(7), SOLVED);
    assert_ne!(scramble(7), scramble(8));
}

#[test]
fn nuoc_di() {
    let sexy = ok("apply", json!({ "state": SOLVED, "moves": "(R U R' U')6" }));
    assert_eq!(sexy["state"], SOLVED);
    assert_eq!(sexy["solved"], true);
    for m in ["R R'", "R2 R2", "x x' ", "M M'", "u u'", "z2 z2", "R2' R2"] {
        assert_eq!(
            ok("apply", json!({ "state": SOLVED, "moves": m }))["state"],
            SOLVED,
            "{m}"
        );
    }
    assert_ne!(
        ok("apply", json!({ "state": SOLVED, "moves": "R" }))["state"],
        SOLVED
    );
    assert!(route(
        "apply",
        &json!({ "state": SOLVED, "moves": "Q" }).to_string()
    )
    .is_err());
    assert!(route(
        "apply",
        &json!({ "state": "UUU", "moves": "R" }).to_string()
    )
    .is_err());
    let lech = SOLVED.replacen('D', "U", 1); // sai số lượng màu
    assert!(route("validate", &json!({ "state": lech }).to_string()).is_err());
}

#[test]
fn giai_that_su_ra_khoi_hoan_chinh() {
    for seed in [1u64, 2, 3] {
        let st = scramble(seed);
        let t0 = std::time::Instant::now();
        let r = ok("solve", json!({ "state": st }));
        let mut all: Vec<String> = Vec::new();
        let mut stages: Vec<&Value> = vec![&r["cross"]];
        stages.extend(r["f2l"].as_array().unwrap().iter());
        stages.push(&r["oll"]);
        stages.push(&r["pll"]);
        for s in &stages {
            assert_ne!(
                s["status"], "failed",
                "seed {seed}: có chặng không giải được: {s}"
            );
            assert_ne!(s["status"], "pending", "seed {seed}: chặng chưa tính: {s}");
            all.extend(
                s["moves"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m.as_str().unwrap().to_string()),
            );
        }
        assert_eq!(all.len() as u64, r["total"].as_u64().unwrap());
        let end = ok("apply", json!({ "state": st, "moves": all.join(" ") }));
        assert_eq!(
            end["solved"], true,
            "seed {seed}: áp dụng lời giải KHÔNG ra khối đã giải"
        );
        println!(
            "seed {seed}: {} nước, {:.1}s",
            all.len(),
            t0.elapsed().as_secs_f32()
        );
    }
}

#[test]
fn goi_y_hop_le() {
    let st = scramble(5);
    let h = ok("hint", json!({ "state": st, "seed": null }));
    let mv: Vec<String> = h["moves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap().to_string())
        .collect();
    println!("gợi ý: {} -> {:?}", h["label"], mv);
    assert!(!mv.is_empty(), "khối xáo phải có gợi ý");
    assert!(route(
        "apply",
        &json!({ "state": st, "moves": mv.join(" ") }).to_string()
    )
    .is_ok());
    let done = ok("hint", json!({ "state": SOLVED }));
    assert!(done["moves"].as_array().unwrap().is_empty());
}

#[test]
fn quet_tu_mau_ly_tuong() {
    for st in [SOLVED.to_string(), scramble(11), scramble(12)] {
        let r = ok("scan_resolve", json!({ "faces": state_samples(&st) }));
        assert_eq!(r["kind"], "ok", "{r}");
        assert_eq!(
            r["state"],
            st.as_str(),
            "quét màu lý tưởng phải ra đúng trạng thái"
        );
        assert!(r["rotations"].as_array().unwrap().iter().all(|v| v == 0));
    }
    // Trộn nhầm 2 ô màu khác nhau: phải bị báo không hợp lệ hoặc tự sửa, không được trả bậy.
    let mut f = state_samples(&scramble(11));
    let a = f[0]["samples"][0].clone();
    f[0]["samples"][0] = f[1]["samples"][0].clone();
    f[1]["samples"][0] = a;
    let r = ok("scan_resolve", json!({ "faces": f }));
    assert_ne!(
        r["kind"], "ok",
        "ghép sai 2 ô mà vẫn báo thành công là sai: {r}"
    );
}

#[test]
fn gop_khung() {
    let one = json!({ "samples": vec![[10u8, 20, 30]; 9], "conf": vec![1.0f32; 9] });
    let r = ok(
        "scan_aggregate",
        json!({ "frames": [one.clone(), one.clone(), one] }),
    );
    assert_eq!(r["samples"][4], json!([10, 20, 30]));
    assert!(route("scan_aggregate", &json!({ "frames": [] }).to_string()).is_err());
}

#[test]
fn quet_khung_hinh() {
    // Ảnh 300x300 nền xám tối, mặt khối 3x3 ở giữa (210x210) với màu chuẩn.
    let (w, h) = (300u32, 300u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for p in rgba.chunks_exact_mut(4) {
        p.copy_from_slice(&[40, 40, 40, 255]);
    }
    let order = [2usize, 0, 3, 5, 1, 4, 2, 0, 3];
    let (x0, y0, side) = (45u32, 45u32, 210u32);
    for y in 0..side {
        for x in 0..side {
            let c = PAL[order[((y / 70) * 3 + x / 70) as usize]];
            let i = (((y0 + y) * w + x0 + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    let r = scan_frame(w, h, &rgba, 0.7).expect("scan_frame lỗi");
    println!(
        "auto={} valid={} region={}",
        r["auto"], r["valid"], r["region"]
    );
    assert_eq!(r["valid"], true);
    for (i, s) in r["samples"].as_array().unwrap().iter().enumerate() {
        let want = PAL[order[i]];
        for k in 0..3 {
            assert!(
                (s[k].as_i64().unwrap() - want[k] as i64).abs() <= 40,
                "ô {i} sai màu: {s} vs {want:?}"
            );
        }
    }
    assert!(scan_frame(10, 10, &[0u8; 5], 0.7).is_err());
}

#[test]
fn xao_khong_seed_thi_ngau_nhien_that() {
    let a = ok("scramble", json!({ "n": 25 }))["state"].clone();
    let b = ok("scramble", json!({ "n": 25 }))["state"].clone();
    assert_ne!(a, b, "không truyền seed thì hai lần xáo phải khác nhau");
}

#[test]
fn apply_chan_cong_thuc_doc_hai() {
    for bad in ["(R)99999999999999999999999", "(R U R' U')999999999", "R Q"] {
        let e = route(
            "apply",
            &json!({ "state": SOLVED, "moves": bad }).to_string(),
        );
        assert!(e.is_err(), "phải trả lỗi cho {bad}");
    }
    let r = ok("apply", json!({ "state": SOLVED, "moves": "(R U R' U')6" }));
    assert_eq!(r["state"], SOLVED, "sexy move x6 về lại trạng thái giải");
}

/// Vectơ chuẩn cho bảng hoán vị của giao diện web (web/move_table.json). `rubik-web/tests/ui.test.js` kiểm tra
/// JavaScript cho ra đúng y hệt; nếu sửa engine mà đổi kết quả thì cả hai test cùng báo lỗi.
pub const GOLDEN_R: &str = "UUFUUFUUFDDBDDBDDBFFDFFDFFDUBBUBBUBBLLLLLLLLLRRRRRRRRR";
pub const GOLDEN_MIX: &str = "LFBDDLUDBLBBUULRDRFRDDRRDRRDRULLUFLFBBLBBFUFFLURUFFUBD";

#[test]
fn vecto_chuan_cho_giao_dien() {
    let solved = "UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR";
    let r = route("apply", &format!(r#"{{"state":"{solved}","moves":"R"}}"#)).unwrap();
    assert_eq!(r["state"], GOLDEN_R);
    let r = route(
        "apply",
        &format!(r#"{{"state":"{solved}","moves":"R U2 F' r M' x z2 d b'"}}"#),
    )
    .unwrap();
    assert_eq!(r["state"], GOLDEN_MIX);
}

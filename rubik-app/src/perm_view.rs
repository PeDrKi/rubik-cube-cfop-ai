//! perm_view.rs — sơ đồ hoán vị: 54 ô nằm trên 9 vòng tròn (bản egui của khung "Sơ đồ hoán vị" ở bản web).
//!
//! Hình học đo từ video gốc: 3 họ vòng tròn đồng tâm (mỗi trục x/y/z một họ, mỗi vòng là một lớp). Mỗi ô dán nằm ở giao
//! điểm của 2 vòng thuộc hai trục mà ô đó KHÔNG hướng theo. Một nước xoay đẩy 12 ô chạy dọc vòng của lớp đó; 8 ô trên mặt
//! đang xoay nhảy quanh tâm mặt. Chỉ số ô (0..53) là chỉ số facelet của rubik-core: mặt U D F B L R, mỗi mặt đọc theo hàng.

use crate::mesh::face_color;
use crate::solve_flow::PendingMove;
use rubik_core::cube::{self, CubeState};
use std::f32::consts::PI;
use std::sync::OnceLock;
use three_d::egui;

type P3 = [i32; 3];

/// Chỉ số ô -> (x, y, z, mặt): vị trí khối con và mặt mà ô hướng ra (hệ toạ độ y lên, z về phía người xem).
const FL: [(i32, i32, i32, char); 54] = [
    (-1, 1, -1, 'U'),
    (0, 1, -1, 'U'),
    (1, 1, -1, 'U'),
    (-1, 1, 0, 'U'),
    (0, 1, 0, 'U'),
    (1, 1, 0, 'U'),
    (-1, 1, 1, 'U'),
    (0, 1, 1, 'U'),
    (1, 1, 1, 'U'),
    (-1, -1, 1, 'D'),
    (0, -1, 1, 'D'),
    (1, -1, 1, 'D'),
    (-1, -1, 0, 'D'),
    (0, -1, 0, 'D'),
    (1, -1, 0, 'D'),
    (-1, -1, -1, 'D'),
    (0, -1, -1, 'D'),
    (1, -1, -1, 'D'),
    (-1, 1, 1, 'F'),
    (0, 1, 1, 'F'),
    (1, 1, 1, 'F'),
    (-1, 0, 1, 'F'),
    (0, 0, 1, 'F'),
    (1, 0, 1, 'F'),
    (-1, -1, 1, 'F'),
    (0, -1, 1, 'F'),
    (1, -1, 1, 'F'),
    (1, 1, -1, 'B'),
    (0, 1, -1, 'B'),
    (-1, 1, -1, 'B'),
    (1, 0, -1, 'B'),
    (0, 0, -1, 'B'),
    (-1, 0, -1, 'B'),
    (1, -1, -1, 'B'),
    (0, -1, -1, 'B'),
    (-1, -1, -1, 'B'),
    (-1, 1, -1, 'L'),
    (-1, 1, 0, 'L'),
    (-1, 1, 1, 'L'),
    (-1, 0, -1, 'L'),
    (-1, 0, 0, 'L'),
    (-1, 0, 1, 'L'),
    (-1, -1, -1, 'L'),
    (-1, -1, 0, 'L'),
    (-1, -1, 1, 'L'),
    (1, 1, 1, 'R'),
    (1, 1, 0, 'R'),
    (1, 1, -1, 'R'),
    (1, 0, 1, 'R'),
    (1, 0, 0, 'R'),
    (1, 0, -1, 'R'),
    (1, -1, 1, 'R'),
    (1, -1, 0, 'R'),
    (1, -1, -1, 'R'),
];

const DOT: f32 = 7.4;
/// Vùng vẽ (toạ độ gốc đo từ video): x 110..610, y 520..930.
const VB_X: f32 = 110.0;
const VB_Y: f32 = 520.0;
const VB_W: f32 = 500.0;
const VB_H: f32 = 410.0;

fn center(ax: usize) -> [f32; 2] {
    match ax {
        0 => [417.2, 773.8],
        1 => [359.6, 674.2],
        _ => [302.1, 773.6],
    }
}

/// Vòng của lớp `l` trên trục `ax`: 1 = ngoài cùng (U, L, B), -1 = trong cùng (D, R, F).
fn ring_of(ax: usize, l: i32) -> ([f32; 2], f32) {
    let rank = if ax == 1 { l } else { -l };
    (center(ax), 92.0 + 24.65 * (rank as f32 + 1.0))
}

fn rot(v: P3, ax: usize, q: i32) -> P3 {
    let [x, y, z] = v;
    match ax {
        0 => [x, -z * q, y * q],
        1 => [z * q, y, -x * q],
        _ => [-y * q, x * q, z],
    }
}

fn normal(f: char) -> P3 {
    match f {
        'U' => [0, 1, 0],
        'D' => [0, -1, 0],
        'F' => [0, 0, 1],
        'B' => [0, 0, -1],
        'R' => [1, 0, 0],
        _ => [-1, 0, 0],
    }
}

fn crossing(c1: [f32; 2], r1: f32, c2: [f32; 2], r2: f32) -> ([f32; 2], [f32; 2]) {
    let (dx, dy) = (c2[0] - c1[0], c2[1] - c1[1]);
    let d = dx.hypot(dy);
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let (mx, my) = (c1[0] + a * dx / d, c1[1] + a * dy / d);
    let (ux, uy) = (dx / d, dy / d);
    ([mx - uy * h, my + ux * h], [mx + uy * h, my - ux * h])
}

/// Vị trí (toạ độ gốc) của ô nằm ở khối con `p`, hướng ra `n`.
fn slot(p: P3, n: P3) -> [f32; 2] {
    let na = if n[0] != 0 {
        0
    } else if n[1] != 0 {
        1
    } else {
        2
    };
    let pos = n[na] > 0;
    let others: Vec<usize> = (0..3).filter(|&k| k != na).collect();
    let (a, b) = (others[0], others[1]);
    let (ca, ra) = ring_of(a, p[a]);
    let (cb, rb) = ring_of(b, p[b]);
    let (p1, p2) = crossing(ca, ra, cb, rb);
    let first = match na {
        1 => (p1[1] < p2[1]) == pos,                 // U ở trên, D ở dưới
        2 => (p1[0] - p1[1] < p2[0] - p2[1]) == pos, // F dưới-trái, B trên-phải
        _ => (p1[0] + p1[1] > p2[0] + p2[1]) == pos, // R dưới-phải, L trên-trái
    };
    if first {
        p1
    } else {
        p2
    }
}

fn slots() -> &'static Vec<[f32; 2]> {
    static S: OnceLock<Vec<[f32; 2]>> = OnceLock::new();
    S.get_or_init(|| {
        FL.iter()
            .map(|&(x, y, z, f)| slot([x, y, z], normal(f)))
            .collect()
    })
}

/// Chữ cái nước -> (trục, các lớp bị xoay, dấu quay quanh +trục của nước không có dấu '). Khớp `move_spec` của app.
fn spec(mv: char) -> Option<(usize, &'static [i32], i32)> {
    Some(match mv {
        'U' => (1, &[1], -1),
        'D' => (1, &[-1], 1),
        'R' => (0, &[1], -1),
        'L' => (0, &[-1], 1),
        'F' => (2, &[1], -1),
        'B' => (2, &[-1], 1),
        'M' => (0, &[0], 1),
        'E' => (1, &[0], 1),
        'S' => (2, &[0], -1),
        'u' => (1, &[1, 0], -1),
        'd' => (1, &[-1, 0], 1),
        'r' => (0, &[1, 0], -1),
        'l' => (0, &[-1, 0], 1),
        'f' => (2, &[1, 0], -1),
        'b' => (2, &[-1, 0], 1),
        'x' => (0, &[-1, 0, 1], -1),
        'y' => (1, &[-1, 0, 1], -1),
        'z' => (2, &[-1, 0, 1], -1),
        _ => return None,
    })
}

/// Một chặng 90° của một ô.
#[derive(Clone, Copy)]
struct Leg {
    from: [f32; 2],
    to: [f32; 2],
    a0: f32,
    da: f32,
}

struct Frame {
    pos: Vec<[f32; 2]>,
    arcs: Vec<([f32; 2], f32, f32, f32, usize)>, // tâm, bán kính, góc đầu, góc cuối, chỉ số ô (lấy màu)
    label_axis: usize,
}

/// Vị trí các chấm khi nước `mv` đang ở tiến độ `e` (0..1, đã làm mượt).
fn pose(mv: char, prime: bool, double: bool, e: f32) -> Option<Frame> {
    let (ax, layers, q0) = spec(mv)?;
    let q = if prime { -q0 } else { q0 };
    let turns = if double { 2 } else { 1 };
    let sl = slots();
    let mut pos: Vec<[f32; 2]> = sl.clone();
    let le = e * turns as f32;
    let leg_i = ((le.floor() as usize).min(turns - 1)) as usize;
    let t = le - leg_i as f32;
    let mut arcs = Vec::new();

    for &layer in layers {
        let (c, r) = ring_of(ax, layer);
        // các chặng của từng ô trong lớp này
        let mut legs: Vec<(usize, bool, Vec<Leg>, [f32; 2])> = Vec::new();
        for (i, &(x, y, z, f)) in FL.iter().enumerate() {
            let (mut p, mut n) = ([x, y, z], normal(f));
            if p[ax] != layer {
                continue;
            }
            let on_ring = n[ax] == 0;
            let mut from = sl[i];
            let mut ls = Vec::new();
            for _ in 0..turns {
                p = rot(p, ax, q);
                n = rot(n, ax, q);
                let to = slot(p, n);
                ls.push(Leg {
                    from,
                    to,
                    a0: 0.0,
                    da: 0.0,
                });
                from = to;
            }
            let mut ctr = [0.0, 0.0];
            if !on_ring {
                let mut cp = [0, 0, 0];
                cp[ax] = normal(f)[ax];
                ctr = slot(cp, cp);
            }
            legs.push((i, on_ring, ls, ctr));
        }
        // góc trên vòng cho các ô chạy dọc vòng; chiều quay lấy theo đa số để mọi ô cùng chiều
        for k in 0..turns {
            let mut pos_n = 0;
            let mut cnt = 0;
            for (_, on_ring, ls, _) in legs.iter_mut() {
                if !*on_ring {
                    continue;
                }
                let l = &mut ls[k];
                l.a0 = (l.from[1] - c[1]).atan2(l.from[0] - c[0]);
                let d = (l.to[1] - c[1]).atan2(l.to[0] - c[0]) - l.a0;
                l.da = d.sin().atan2(d.cos());
                cnt += 1;
                if l.da > 0.0 {
                    pos_n += 1;
                }
            }
            let sg = if pos_n * 2 >= cnt { 1.0 } else { -1.0 };
            for (_, on_ring, ls, _) in legs.iter_mut() {
                if *on_ring && ls[k].da * sg < 0.0 {
                    ls[k].da += sg * 2.0 * PI;
                }
            }
        }
        let u = 1.0 - t;
        let mut ring_items: Vec<(f32, usize)> = Vec::new();
        for (i, on_ring, ls, ctr) in &legs {
            let l = ls[leg_i];
            if *on_ring {
                let th = l.a0 + l.da * t;
                pos[*i] = [c[0] + r * th.cos(), c[1] + r * th.sin()];
                ring_items.push((th.rem_euclid(2.0 * PI), *i));
            } else {
                let (cx, cy) = ((l.from[0] + l.to[0]) / 2.0, (l.from[1] + l.to[1]) / 2.0);
                let (qx, qy) = (cx + (ctr[0] - cx) * 0.8, cy + (ctr[1] - cy) * 0.8);
                pos[*i] = [
                    u * u * l.from[0] + 2.0 * u * t * qx + t * t * l.to[0],
                    u * u * l.from[1] + 2.0 * u * t * qy + t * t * l.to[1],
                ];
            }
        }
        ring_items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let n = ring_items.len();
        for k in 0..n {
            let (cur, idx) = ring_items[k];
            let prev = ring_items[(k + n - 1) % n].0;
            let next = ring_items[(k + 1) % n].0;
            let t0 = cur - ((cur - prev + 2.0 * PI).rem_euclid(2.0 * PI)) / 2.0;
            let t1 = cur + ((next - cur + 2.0 * PI).rem_euclid(2.0 * PI)) / 2.0;
            arcs.push((c, r, t0, t1, idx));
        }
    }
    Some(Frame {
        pos,
        arcs,
        label_axis: ax,
    })
}

fn sticker_color(state: &CubeState, i: usize) -> egui::Color32 {
    let v = state.facelet_at(i / 9, (i % 9) / 3, i % 3);
    let [r, g, b] = face_color(cube::face_char(v as usize));
    egui::Color32::from_rgb(r, g, b)
}

/// Vẽ sơ đồ hoán vị. `pending`: nước đang xoay (nếu có) để chạy hiệu ứng; `state` là trạng thái TRƯỚC nước đó.
/// `sel_face`: mặt đang chọn ở sơ đồ 6 mặt, 9 ô của mặt đó được làm sáng.
pub(crate) fn draw_perm_diagram(
    ui: &mut egui::Ui,
    state: &CubeState,
    pending: Option<&PendingMove>,
    sel_face: Option<char>,
) {
    let w = ui.available_width();
    let scale = w / VB_W;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, VB_H * scale), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let to = |p: [f32; 2]| {
        egui::pos2(
            rect.min.x + (p[0] - VB_X) * scale,
            rect.min.y + (p[1] - VB_Y) * scale,
        )
    };
    painter.rect_filled(
        rect,
        egui::Rounding::same(8.0),
        egui::Color32::from_rgb(10, 11, 17),
    );

    let ring_col = egui::Color32::from_rgb(111, 111, 124);
    for ax in [1usize, 0, 2] {
        for l in [1, 0, -1] {
            let (c, r) = ring_of(ax, l);
            painter.circle_stroke(
                to(c),
                r * scale,
                egui::Stroke::new((2.6 * scale).max(1.0), ring_col),
            );
        }
    }

    let mut frame: Option<Frame> = None;
    let mut e = 0.0f32;
    if let Some(p) = pending {
        let k = (p.current / p.target).clamp(0.0, 1.0);
        e = k * k * (3.0 - 2.0 * k); // smoothstep
        frame = pose(p.mv, p.prime, p.double, e);
    }
    let vis = (e * 7.0).min(1.0) * if e > 0.8 { (1.0 - e) / 0.2 } else { 1.0 };

    if let Some(f) = &frame {
        for &(c, r, t0, t1, idx) in &f.arcs {
            let col = sticker_color(state, idx);
            let a = (255.0 * vis) as u8;
            let col = egui::Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), a);
            let steps = (((t1 - t0).abs() / 0.08).ceil() as usize).max(2);
            let pts: Vec<egui::Pos2> = (0..=steps)
                .map(|s| {
                    let th = t0 + (t1 - t0) * s as f32 / steps as f32;
                    to([c[0] + r * th.cos(), c[1] + r * th.sin()])
                })
                .collect();
            painter.add(egui::Shape::line(
                pts,
                egui::Stroke::new((4.6 * scale).max(1.5), col),
            ));
        }
    }

    let sl = slots();
    let grow = 1.0 + 0.3 * (PI * e).sin();
    // Mặt đang chọn ở sơ đồ 6 mặt: 9 ô của mặt đó sáng (viền trắng, to hơn), các ô còn lại mờ đi.
    let sel_idx = sel_face.map(cube::idx_of);
    for i in 0..54 {
        let p = frame.as_ref().map_or(sl[i], |f| f.pos[i]);
        let c = to(p);
        let on = sel_idx == Some(i / 9);
        let dim = sel_idx.is_some() && !on;
        let base = sticker_color(state, i);
        let fill = if dim {
            egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 90)
        } else {
            base
        };
        let rad = DOT * scale * grow * if on { 1.3 } else { 1.0 };
        painter.circle_filled(c, rad, fill);
        let stroke = if on {
            egui::Stroke::new(2.0_f32, egui::Color32::WHITE)
        } else {
            egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgba_unmultiplied(10, 10, 13, if dim { 90 } else { 255 }),
            )
        };
        painter.circle_stroke(c, rad, stroke);
    }

    if let (Some(f), Some(p)) = (&frame, pending) {
        let (lx, ly, left) = match f.label_axis {
            1 => (196.0, 586.0, true),
            2 => (142.0, 742.0, true),
            _ => (578.0, 742.0, false),
        };
        let name = format!(
            "{}{}",
            p.mv,
            if p.double {
                "2"
            } else if p.prime {
                "′"
            } else {
                ""
            }
        );
        let anchor = to([if left { lx - 14.0 } else { lx + 14.0 }, ly]);
        let a = (255.0 * vis) as u8;
        let box_rect = egui::Rect::from_center_size(anchor, egui::vec2(30.0 * scale, 26.0 * scale));
        painter.rect_filled(
            box_rect,
            egui::Rounding::same(5.0),
            egui::Color32::from_rgba_unmultiplied(17, 17, 22, a),
        );
        painter.text(
            anchor,
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional((22.0 * scale).max(11.0)),
            egui::Color32::from_rgba_unmultiplied(236, 236, 244, a),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 54 ô phải có 54 vị trí khác nhau và nằm trong vùng vẽ.
    #[test]
    fn bon_muoi_bon_o_khac_vi_tri() {
        let s = slots();
        assert_eq!(s.len(), 54);
        for (i, a) in s.iter().enumerate() {
            assert!(
                a[0] > VB_X && a[0] < VB_X + VB_W && a[1] > VB_Y && a[1] < VB_Y + VB_H,
                "ô {i} ngoài vùng vẽ"
            );
            for b in &s[i + 1..] {
                assert!((a[0] - b[0]).hypot(a[1] - b[1]) > 5.0, "hai ô trùng vị trí");
            }
        }
    }

    /// Với mọi nước, phép xoay hình học đưa ô i tới đúng ô mà engine (do_move) đưa tới:
    /// trên một khối đã xáo, so kết quả do_move với phép xoay hình học trong sơ đồ.
    #[test]
    fn xoay_hinh_hoc_khop_engine() {
        let toks = [
            "U", "D", "L", "R", "F", "B", "M", "E", "S", "u", "d", "l", "r", "f", "b", "x", "y",
            "z",
        ];
        for t in toks {
            for suf in ["", "'", "2"] {
                let mv = format!("{t}{suf}");
                // trạng thái xáo sẵn để các màu ít bị trùng; st0 = trước nước, st1 = sau nước
                let mut st0 = CubeState::solved();
                for m in [
                    "R", "U", "F'", "L2", "D", "B'", "M", "E'", "R'", "U2", "S", "F", "L'", "D2",
                    "x",
                ] {
                    st0.do_move(m);
                }
                let mut st1 = st0;
                st1.do_move(&mv);
                let ch = t.chars().next().unwrap();
                let (ax, layers, q0) = spec(ch).unwrap();
                let q = if suf == "'" { -q0 } else { q0 };
                let reps = if suf == "2" { 2 } else { 1 };
                for (i, &(x, y, z, f)) in FL.iter().enumerate() {
                    let (mut p, mut n) = ([x, y, z], normal(f));
                    if layers.contains(&p[ax]) {
                        for _ in 0..reps {
                            p = rot(p, ax, q);
                            n = rot(n, ax, q);
                        }
                    }
                    let j = FL
                        .iter()
                        .position(|&(a, b, c, g)| [a, b, c] == p && normal(g) == n)
                        .unwrap();
                    // sau nước mv, ô j mang màu mà ô i có trước đó
                    assert_eq!(
                        st1.facelet_at(j / 9, (j % 9) / 3, j % 3),
                        st0.facelet_at(i / 9, (i % 9) / 3, i % 3),
                        "nước {mv}: ô {i} -> {j}"
                    );
                }
            }
        }
    }
}

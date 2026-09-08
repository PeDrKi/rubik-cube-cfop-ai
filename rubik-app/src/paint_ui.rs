//! paint_ui.rs — cửa sổ "Điền màu thủ công": hiển thị 6 mặt cube dạng
//! khai triển (net, giống sơ đồ hướng dẫn ở scan_ui.rs), 6 ô TÂM tự
//! điền sẵn theo đúng quy ước màu = chỉ số mặt (không cho sửa, vì tâm
//! không bao giờ đổi vị trí trên cube thật), 48 ô còn lại người dùng
//! chọn 1 trong 6 màu rồi bấm vào ô để tô -- mô phỏng trạng thái cube
//! mà không cần camera. Tối đa 9 ô mỗi màu (kể cả tâm), khớp đúng số
//! sticker thật của 1 màu trên cube 3x3.
//!
//! Sau khi điền đủ 54 ô, dùng chung `rubik_vision::scan::validate_facelets`
//! (đã viết cho luồng quét camera) để kiểm tra ghép cubie hợp lệ + giải
//! được trước khi áp dụng vào cube thật trong app.

use rubik_core::corner_model::CORNER_SLOTS;
use rubik_core::cube::CubeState;
use rubik_core::edge_model::EDGE_SLOTS;
use rubik_vision::scan::validate_facelets;
use three_d::egui;

const COLOR_NAMES: [&str; 6] = ["Vàng", "Trắng", "Xanh lá", "Xanh dương", "Đỏ", "Cam"];
/// Bảng màu ĐÚNG NHƯ app đang dùng ở mọi nơi khác (sơ đồ 6 mặt, khối 3D)
/// -- xem `face_color()` trong main.rs. Copy y hệt giá trị RGB để nhất
/// quán, tránh người dùng thấy 2 bảng màu khác nhau cho cùng 1 mặt.
const DISPLAY_COLORS: [egui::Color32; 6] = [
    egui::Color32::from_rgb(255, 210, 0),  // U = vàng
    egui::Color32::from_rgb(255, 255, 255), // D = trắng
    egui::Color32::from_rgb(15, 160, 55),  // F = xanh lá
    egui::Color32::from_rgb(15, 75, 210),  // B = xanh dương
    egui::Color32::from_rgb(210, 30, 45),  // L = đỏ
    egui::Color32::from_rgb(255, 100, 0),  // R = cam
];
const FACE_LABELS: [&str; 6] = ["U", "D", "F", "B", "L", "R"];

pub struct PaintState {
    pub open: bool,
    /// faces[mặt][ô 0..9] = Some(chỉ số màu) đã điền, None = trống.
    /// Ô tâm (index 4) LUÔN Some(mặt) ngay từ đầu và không cho sửa.
    faces: [[Option<u8>; 9]; 6],
    /// Màu đang chọn để tô khi bấm vào 1 ô.
    current_color: u8,
    error: Option<String>,
}

impl Default for PaintState {
    fn default() -> Self {
        let mut faces = [[None; 9]; 6];
        for (f, face) in faces.iter_mut().enumerate() {
            face[4] = Some(f as u8);
        }
        Self { open: false, faces, current_color: 0, error: None }
    }
}

impl PaintState {
    fn count_of(&self, color: u8) -> usize {
        self.faces.iter().flatten().filter(|&&c| c == Some(color)).count()
    }
}

/// Đọc màu tại 1 toạ độ (mặt, hàng, cột) trong bảng đang điền dở.
fn get(faces: &[[Option<u8>; 9]; 6], (f, r, c): (usize, usize, usize)) -> Option<u8> {
    faces[f][r * 3 + c]
}

/// Kiểm tra các mảnh (cạnh/góc) ĐÃ ĐIỀN ĐỦ MÀU tính đến thời điểm hiện
/// tại có vi phạm luật ghép cube thật hay không -- KHÔNG cần chờ điền
/// hết 54 ô, hễ 1 mảnh vừa đủ dữ liệu để kiểm tra là báo ngay:
/// - 1 cạnh (2 mặt) không thể có 2 màu giống nhau.
/// - 1 góc (3 mặt) không thể có 2 trong 3 màu giống nhau.
/// - Không có 2 cạnh khác nhau nào trùng hệt cặp màu (cube thật không
///   có 2 mảnh cạnh giống hệt nhau), tương tự với góc (bộ 3 màu).
fn find_conflicts(faces: &[[Option<u8>; 9]; 6]) -> Vec<String> {
    let mut msgs = Vec::new();

    let mut edge_pairs: Vec<(u8, u8)> = Vec::new();
    for &(p1, p2) in EDGE_SLOTS.iter() {
        let (Some(a), Some(b)) = (get(faces, p1), get(faces, p2)) else { continue };
        if a == b {
            msgs.push(format!(
                "⚠ Cạnh {}-{} đang bị tô CÙNG 1 màu ({}) -- 1 cạnh thật luôn gồm 2 màu khác nhau.",
                FACE_LABELS[p1.0], FACE_LABELS[p2.0], COLOR_NAMES[a as usize]
            ));
            continue;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        if edge_pairs.contains(&pair) {
            msgs.push(format!(
                "⚠ Có 2 cạnh khác nhau cùng mang cặp màu {}-{} -- cube thật không có 2 mảnh cạnh giống hệt nhau.",
                COLOR_NAMES[pair.0 as usize], COLOR_NAMES[pair.1 as usize]
            ));
        } else {
            edge_pairs.push(pair);
        }
    }

    let mut corner_sets: Vec<[u8; 3]> = Vec::new();
    for &(p1, p2, p3) in CORNER_SLOTS.iter() {
        let (Some(a), Some(b), Some(c)) = (get(faces, p1), get(faces, p2), get(faces, p3)) else { continue };
        if a == b || b == c || a == c {
            msgs.push(format!(
                "⚠ Góc {}-{}-{} đang có 2 mặt trùng màu -- 1 góc thật luôn gồm 3 màu khác nhau đôi một.",
                FACE_LABELS[p1.0], FACE_LABELS[p2.0], FACE_LABELS[p3.0]
            ));
            continue;
        }
        let mut set = [a, b, c];
        set.sort_unstable();
        if corner_sets.contains(&set) {
            msgs.push(format!(
                "⚠ Có 2 góc khác nhau cùng mang bộ 3 màu {}-{}-{} -- cube thật không có 2 mảnh góc giống hệt nhau.",
                COLOR_NAMES[set[0] as usize], COLOR_NAMES[set[1] as usize], COLOR_NAMES[set[2] as usize]
            ));
        } else {
            corner_sets.push(set);
        }
    }

    msgs
}

/// Vẽ và xử lý cửa sổ điền màu. Trả về `Some(state)` đúng 1 lần khi
/// người dùng bấm "Áp dụng vào cube" với 1 cách điền hợp lệ.
pub fn show(ctx: &egui::Context, s: &mut PaintState) -> Option<CubeState> {
    if !s.open {
        return None;
    }
    let mut applied: Option<CubeState> = None;
    let mut window_open = s.open;
    let mut reset_requested = false;

    egui::Window::new("🎨 Điền màu thủ công").open(&mut window_open).default_width(440.0).show(ctx, |ui| {
        ui.label("Chọn 1 màu bên dưới rồi bấm vào từng ô để tô. Bấm lại đúng màu đang có trên 1 ô để xoá ô đó.");
        ui.add_space(4.0);

        // ── Bảng chọn màu + đếm số ô đã dùng / tối đa 9 ─────────────
        ui.horizontal_wrapped(|ui| {
            for c in 0u8..6 {
                let count = s.count_of(c);
                let maxed = count >= 9;
                let bg = DISPLAY_COLORS[c as usize];
                let brightness = bg.r() as u32 + bg.g() as u32 + bg.b() as u32;
                let text_color = if brightness > 400 { egui::Color32::BLACK } else { egui::Color32::WHITE };
                let label = format!("{} {}/9", COLOR_NAMES[c as usize], count);
                let mut button = egui::Button::new(egui::RichText::new(label).color(text_color)).fill(bg);
                if s.current_color == c {
                    button = button.stroke(egui::Stroke::new(3.0, egui::Color32::WHITE));
                }
                if ui.add_enabled(!maxed || s.current_color == c, button).clicked() {
                    s.current_color = c;
                }
            }
        });

        let conflicts = find_conflicts(&s.faces);
        if !conflicts.is_empty() {
            ui.add_space(4.0);
            for msg in &conflicts {
                ui.colored_label(egui::Color32::from_rgb(220, 80, 70), msg);
            }
        }
        ui.separator();

        // ── Lưới khai triển 6 mặt (U trên / L-F-R-B hàng giữa / D dưới) ──
        const CELL: f32 = 26.0;
        const GAP: f32 = 2.0;
        let indent = CELL * 3.0 + GAP * 2.0 + 10.0;

        let draw_face = |ui: &mut egui::Ui, s: &mut PaintState, f: usize| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
                ui.label(FACE_LABELS[f]);
                for row in 0..3 {
                    ui.horizontal(|ui| {
                        for col in 0..3 {
                            let cell = row * 3 + col;
                            let (rect, resp) =
                                ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                            let color = s.faces[f][cell]
                                .map(|c| DISPLAY_COLORS[c as usize])
                                .unwrap_or(egui::Color32::from_gray(55));
                            ui.painter().rect_filled(rect, 3.0, color);
                            ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(15)));

                            if resp.clicked() {
                                if cell == 4 {
                                    // Ô tâm không cho sửa -- bấm vào chỉ để chọn nhanh màu của mặt đó.
                                    s.current_color = f as u8;
                                } else if s.faces[f][cell] == Some(s.current_color) {
                                    s.faces[f][cell] = None;
                                } else if s.count_of(s.current_color) < 9 {
                                    s.faces[f][cell] = Some(s.current_color);
                                }
                            }
                        }
                    });
                }
            });
        };

        ui.horizontal(|ui| {
            ui.add_space(indent);
            draw_face(ui, s, 0); // U
        });
        let old_spacing = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.horizontal(|ui| {
            draw_face(ui, s, 4); // L
            draw_face(ui, s, 2); // F
            draw_face(ui, s, 5); // R
            draw_face(ui, s, 3); // B
        });
        ui.spacing_mut().item_spacing.x = old_spacing;
        ui.horizontal(|ui| {
            ui.add_space(indent);
            draw_face(ui, s, 1); // D
        });

        ui.separator();
        let filled = s.faces.iter().flatten().filter(|c| c.is_some()).count();
        ui.label(format!("Đã điền: {filled}/54"));

        if let Some(err) = &s.error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 70), err);
        }

        ui.horizontal(|ui| {
            if ui.button("↺ Xoá hết").clicked() {
                reset_requested = true;
            }
            let can_apply = filled == 54 && conflicts.is_empty();
            if ui.add_enabled(can_apply, egui::Button::new("✅ Áp dụng vào cube")).clicked() {
                let arr: [[u8; 9]; 6] =
                    std::array::from_fn(|f| std::array::from_fn(|c| s.faces[f][c].unwrap()));
                match validate_facelets(arr) {
                    Ok(state) => {
                        applied = Some(state);
                        s.error = None;
                    }
                    Err(e) => s.error = Some(e),
                }
            }
        });
    });

    if reset_requested {
        let open = s.open;
        *s = PaintState::default();
        s.open = open;
    }
    s.open = window_open;
    if applied.is_some() {
        s.open = false;
        *s = PaintState::default();
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_same_color_edge() {
        let mut faces = [[None; 9]; 6];
        for (f, face) in faces.iter_mut().enumerate() {
            face[4] = Some(f as u8);
        }
        // U-F edge = facelet (U,row2,col1) và (F,row0,col1) -- tô cùng màu 0 (vàng)
        // để chắc đúng toạ độ, cứ thử vài cặp và assert có ít nhất 1 cảnh báo khi cố tình gán trùng.
        faces[0][7] = Some(0); // 1 ô cạnh của U, tô trùng màu U (test coarse: any same-face-color edge)
        faces[2][1] = Some(0); // 1 ô cạnh của F, cũng tô màu 0 -> nếu đây đúng là 1 cặp cạnh thật, phải bị báo
        let msgs = find_conflicts(&faces);
        assert!(!msgs.is_empty(), "phải phát hiện được cạnh trùng màu, msgs={msgs:?}");
    }

    #[test]
    fn no_conflict_on_valid_partial_fill() {
        let mut faces = [[None; 9]; 6];
        for (f, face) in faces.iter_mut().enumerate() {
            face[4] = Some(f as u8);
        }
        // Chỉ có 6 tâm, chưa điền gì thêm -> không thể có xung đột.
        let msgs = find_conflicts(&faces);
        assert!(msgs.is_empty(), "6 tâm mặc định không được báo lỗi, msgs={msgs:?}");
    }
}

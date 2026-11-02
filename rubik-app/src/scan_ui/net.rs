//! Vẽ hình khai triển 6 mặt đã chụp và các ô màu mẫu.

use super::*;

/// Vẽ hình khai triển ("net") CẢ 6 MẶT cùng lúc — bố cục hình chữ thập
/// quen thuộc (U trên cùng, D dưới cùng, hàng giữa L-F-R-B), mỗi mặt là
/// lưới 3x3 màu THẬT đã chụp được. Tô viền màu `highlight_color` cho các
/// ô trong `highlight` (ô đáng ngờ hoặc vừa được tự sửa) — thay vì chỉ
/// nêu "mặt X ô số Y" bằng chữ, người dùng thấy ngay ô đó nằm ở đâu
/// trong tổng thể khối, giữa các ô xung quanh màu gì.
///
/// MỌI ô (không chỉ ô được tô viền) đều bấm được — trả về `Some((mặt,
/// ô))` ở đúng khung hình người dùng vừa bấm, để bên gọi cho phép sửa cả
/// những ô app không tự nghi ngờ nhưng người dùng nhìn thấy sai bằng mắt.
pub(super) fn draw_cube_net(
    ui: &mut egui::Ui,
    captured: &[Option<RawFace>; 6],
    highlight: &[(usize, usize)],
    highlight_color: egui::Color32,
) -> Option<(usize, usize)> {
    // Tự tính toạ độ pixel cho TOÀN BỘ khối bằng 1 vùng cấp phát duy
    // nhất + vẽ trực tiếp bằng painter, KHÔNG lồng `egui::Grid` trong
    // `horizontal`/`vertical` -- cách lồng widget trước đó bị lệch canh
    // vì mỗi widget egui tự cộng thêm khoảng đệm/khoảng cách nội bộ
    // riêng, rất khó đoán trước để bù chính xác bằng `add_space`. Tính
    // tay như dưới đây đảm bảo U/D luôn thẳng cột với F và khoảng cách
    // đều tuyệt đối giữa mọi mặt.
    const CELL: f32 = 16.0;
    const CELL_GAP: f32 = 1.5;
    const FACE_GAP: f32 = 6.0;
    const FACE: f32 = CELL * 3.0 + CELL_GAP * 2.0;

    let total_size = egui::vec2(FACE * 4.0 + FACE_GAP * 3.0, FACE * 3.0 + FACE_GAP * 2.0);
    let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());
    let origin = rect.min;
    let painter = ui.painter_at(rect);
    let pointer_pos = response.interact_pointer_pos();
    let was_clicked = response.clicked();

    // Cột/hàng trong bố cục chữ thập: L,F,R,B ở hàng giữa (hàng 1); U ở
    // hàng trên (hàng 0), D ở hàng dưới (hàng 2) -- cả 2 đều thẳng cột
    // với F (cột 1).
    let face_grid_pos = |face_idx: usize| -> Option<(u32, u32)> {
        match face_idx {
            0 => Some((1, 0)), // U
            4 => Some((0, 1)), // L
            2 => Some((1, 1)), // F
            5 => Some((2, 1)), // R
            3 => Some((3, 1)), // B
            1 => Some((1, 2)), // D
            _ => None,
        }
    };

    let mut clicked: Option<(usize, usize)> = None;
    for f in 0..6 {
        let Some((col, row)) = face_grid_pos(f) else {
            continue;
        };
        let face_min = egui::pos2(
            origin.x + col as f32 * (FACE + FACE_GAP),
            origin.y + row as f32 * (FACE + FACE_GAP),
        );
        for r in 0..3 {
            for c in 0..3 {
                let sidx = r * 3 + c;
                let cell_min = egui::pos2(
                    face_min.x + c as f32 * (CELL + CELL_GAP),
                    face_min.y + r as f32 * (CELL + CELL_GAP),
                );
                let cell_rect = egui::Rect::from_min_size(cell_min, egui::vec2(CELL, CELL));
                let color = captured[f]
                    .map(|face| face.samples[sidx])
                    .unwrap_or([70, 70, 70]);
                painter.rect_filled(
                    cell_rect,
                    2.0,
                    egui::Color32::from_rgb(color[0], color[1], color[2]),
                );
                let is_hl = highlight.contains(&(f, sidx));
                let stroke = if is_hl {
                    egui::Stroke::new(2.5_f32, highlight_color)
                } else {
                    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(70))
                };
                painter.rect_stroke(cell_rect, 2.0, stroke);
                if was_clicked && pointer_pos.is_some_and(|p| cell_rect.contains(p)) {
                    clicked = Some((f, sidx));
                }
            }
        }
    }

    clicked
}

/// Vẽ 9 ô màu nhỏ (3x3) đúng như những gì hệ thống ĐANG đọc được từ
/// khung hình hiện tại — phản hồi trực quan tức thời để người dùng biết
/// ngay có canh đúng/đủ sáng hay không, không phải đợi xong cả 6 mặt.
pub(super) fn draw_swatches(ui: &mut egui::Ui, samples: &[[u8; 3]; 9]) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Màu đang đọc được:").size(11.0).weak());
    egui::Grid::new("swatch_grid")
        .spacing([3.0, 3.0])
        .show(ui, |ui| {
            for row in 0..3 {
                for col in 0..3 {
                    let c = samples[row * 3 + col];
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(60)),
                    );
                }
                ui.end_row();
            }
        });
    ui.add_space(4.0);
}

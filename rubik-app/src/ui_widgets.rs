//! ui_widgets.rs — các hàm vẽ/soi (picking) dùng bên trong vòng lặp
//! egui của main.rs, tách riêng cho main.rs đỡ dài. Vẫn nhận
//! `&mut egui::Ui`/`&Context` từ nơi gọi chứ không tự giữ state riêng.

use crate::mesh::{face_color, face_info, FACES};
use rubik_core::cube::{self, CubeState};
use three_d::*;

/// Bắn tia từ vị trí chuột (pixel vật lý) qua camera, tìm điểm chạm gần
/// nhất trên `model` (dùng three_d::pick() -- raycasting GPU có sẵn,
/// không cần tự viết ray-plane), rồi suy ra mặt nào trong 6 mặt lớn (U D
/// F B L R) chứa điểm đó: mặt có dot(diem_cham, normal_mat) LỚN NHẤT
/// (điểm luôn nằm trên đúng 1 trong 6 mặt ngoài vì mesh tĩnh dùng để
/// pick chỉ vẽ panel/sticker mặt ngoài, xem `mesh::build_cube_mesh`).
pub(crate) fn pick_face(
    context: &Context,
    camera: &Camera,
    model: &Gm<Mesh, ColorMaterial>,
    pixel: PhysicalPoint,
) -> Option<char> {
    let hit = three_d::pick(context, camera, pixel, std::iter::once(model))?;
    let mut best_face = 'U';
    let mut best_dot = f32::NEG_INFINITY;
    for &face in &FACES {
        let (n, _, _) = face_info(face);
        let d = hit.dot(n);
        if d > best_dot {
            best_dot = d;
            best_face = face;
        }
    }
    Some(best_face)
}

/// Vẽ khung 6 mặt (bố cục chữ thập: U trên, L F R B một hàng, D dưới) --
/// dùng egui::Painter vẽ hình vuông màu trực tiếp, không cần 3D. Đọc
/// facelet TRỰC TIẾP từ CubeState thật (cube.rs) qua facelet_at()/idx_of().
pub(crate) fn draw_six_face_net(ui: &mut egui::Ui, state: &CubeState, sel_face: Option<char>) {
    // Ô co giãn theo bề rộng thực tế còn lại: sơ đồ gồm 4 cột mặt, mỗi
    // cột = 3 ô + 2 khe + lề. Giải ngược ra cỡ ô để sơ đồ luôn vừa khít
    // panel dù cửa sổ to nhỏ thế nào (trước đây cố định 21px nên panel
    // rộng thì thừa chỗ, panel hẹp thì tràn).
    let avail = ui.available_width();
    let gap = 2.5;
    let pad = 7.0;
    let cell = (((avail / 4.0) - pad - 2.0 * gap) / 3.0).clamp(13.0, 30.0);
    let panel = cell * 3.0 + gap * 2.0;
    let grid_pos: [(char, i32, i32); 6] = [
        ('U', 1, 0),
        ('L', 0, 1),
        ('F', 1, 1),
        ('R', 2, 1),
        ('B', 3, 1),
        ('D', 1, 2),
    ];
    let origin = ui.cursor().min;
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(4.0 * (panel + pad), 3.0 * (panel + pad)),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    for (face, gc, gr) in grid_pos {
        let fx = origin.x + gc as f32 * (panel + pad);
        let fy = origin.y + gr as f32 * (panel + pad);
        let selected = Some(face) == sel_face;

        // Nền mờ phía sau mỗi mặt cho tách bạch khỏi nền panel
        let panel_rect = egui::Rect::from_min_size(
            egui::pos2(fx - 3.0, fy - 3.0),
            egui::vec2(panel + 6.0, panel + 6.0),
        );
        painter.rect_filled(
            panel_rect,
            egui::Rounding::same(5.0),
            egui::Color32::from_rgb(16, 17, 25),
        );

        for r in 0..3usize {
            for c in 0..3usize {
                let ck = cube::face_char(state.facelet_at(cube::idx_of(face), r, c) as usize);
                let [cr, cg, cb] = face_color(ck);
                let x = fx + c as f32 * (cell + gap);
                let y = fy + r as f32 * (cell + gap);
                let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell, cell));
                painter.rect_filled(
                    cell_rect,
                    egui::Rounding::same((cell * 0.17).clamp(2.0, 6.0)),
                    egui::Color32::from_rgb(cr, cg, cb),
                );
            }
        }

        // Chữ tên mặt ở ô giữa (ô tâm luôn giữ nguyên màu mặt đó)
        let center = egui::pos2(fx + panel / 2.0, fy + panel / 2.0);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            face,
            egui::FontId::proportional((cell * 0.52).clamp(8.0, 15.0)),
            egui::Color32::from_black_alpha(150),
        );

        if selected {
            painter.rect_stroke(
                panel_rect,
                egui::Rounding::same(5.0),
                egui::Stroke::new(2.0_f32, crate::theme::ACCENT),
            );
        }
    }
}

/// Vẽ 1 dòng chặng trong bảng công thức. `header` để trống nếu không cần
/// tiêu đề nhóm riêng (VD các cặp F2L nằm chung dưới 1 tiêu đề).
pub(crate) fn stage_row(
    ui: &mut egui::Ui,
    header: &str,
    label: &str,
    st: &rubik_core::breakdown::StageStatus,
) {
    use rubik_core::breakdown::StageStatus as S;
    if !header.is_empty() {
        ui.label(
            egui::RichText::new(header)
                .size(11.5)
                .strong()
                .color(crate::theme::ACCENT),
        );
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{label}"))
                .size(12.5)
                .strong()
                .color(crate::theme::TEXT),
        );
        match st {
            S::Done => {
                ui.label(
                    egui::RichText::new("✔ đã xong sẵn")
                        .size(12.0)
                        .color(crate::theme::SUCCESS),
                );
            }
            S::Moves { moves, case_name } => {
                if let Some(n) = case_name {
                    ui.label(
                        egui::RichText::new(format!("[{n}]"))
                            .size(11.0)
                            .color(crate::theme::INFO),
                    );
                }
                ui.label(
                    egui::RichText::new(moves.join(" "))
                        .monospace()
                        .size(12.5)
                        .color(crate::theme::ACCENT),
                );
                ui.label(
                    egui::RichText::new(format!("({} nước)", moves.len()))
                        .size(11.0)
                        .color(crate::theme::TEXT_DIM),
                );
            }
            S::Failed => {
                ui.label(
                    egui::RichText::new("chưa tìm được — thử mở lại bảng (T)")
                        .size(12.0)
                        .color(crate::theme::DANGER),
                );
            }
            S::Pending => {
                ui.label(
                    egui::RichText::new("chưa tới lượt — cần xong bước trước")
                        .size(12.0)
                        .color(crate::theme::TEXT_DIM),
                );
            }
        }
    });
}

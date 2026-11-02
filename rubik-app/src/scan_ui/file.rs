//! Chế độ nạp ảnh từ file và vẽ ảnh kèm lưới 3x3.

use super::*;
use super::{net::*, window::*};

pub(super) fn show_file_capture(ui: &mut egui::Ui, s: &mut ScanState) {
    draw_orientation_guide(ui, s.current_face);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Đường dẫn file:");
        ui.text_edit_singleline(&mut s.file_path);
        if ui.button("Nạp ảnh").clicked() {
            match image::open(&s.file_path) {
                Ok(dyn_img) => {
                    let rgb = dyn_img.to_rgb8();
                    let d = run_detection(s, &rgb);
                    s.detected_region = d.as_ref().map(|d| d.region);
                    s.detected_quad = d.and_then(|d| d.quad);
                    s.current_image = Some(rgb);
                    s.file_error = None;
                    s.texture_dirty = true;
                }
                Err(e) => {
                    s.file_error = Some(format!("Không đọc được ảnh: {e}"));
                    s.current_image = None;
                }
            }
        }
    });
    if let Some(err) = &s.file_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 70), err);
    }

    if let Some(img) = s.current_image.take() {
        let ctx = ui.ctx().clone();
        let (region, auto_mode) = match s.detected_region {
            Some(r) => (r, true),
            None => (fallback_region(&img, s.region_scale), false),
        };
        let stroke = if auto_mode {
            egui::Color32::from_rgb(60, 200, 90)
        } else {
            egui::Color32::from_rgb(255, 210, 0)
        };
        draw_image_with_grid(
            &ctx,
            ui,
            &img,
            region,
            s.detected_quad,
            stroke,
            &mut s.live_texture,
            &mut s.texture_dirty,
            "file_preview",
        );
        if auto_mode {
            ui.colored_label(
                egui::Color32::from_rgb(60, 190, 90),
                "✔ Đã tự phát hiện vị trí khối rubik.",
            );
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(230, 170, 40),
                "⚠ Chưa tự phát hiện được — dùng khung vàng, chỉnh thanh trượt cho khít.",
            );
            ui.add(egui::Slider::new(&mut s.region_scale, 0.4..=0.95).text("Kích thước khung"));
        }
        let (samples, confidence) = sample_for_region(&img, region, s.detected_quad);
        draw_swatches(ui, &samples);
        let ctx2 = ui.ctx().clone();
        let space_pressed =
            !ctx2.wants_keyboard_input() && ctx2.input(|i| i.key_pressed(egui::Key::Space));
        let mut captured_now = false;
        let clicked = ui
            .add_sized(
                [ui.available_width(), 34.0],
                egui::Button::new("✔  Dùng ảnh này cho mặt hiện tại (Space)"),
            )
            .clicked();
        if clicked || space_pressed {
            s.captured[s.current_face] = Some(RawFace {
                samples,
                confidence,
            });
            s.file_path.clear();
            captured_now = true;
            if s.current_face < 5 {
                s.current_face += 1;
            }
        }
        if !captured_now {
            s.current_image = Some(img);
        }
    }
}

/// Vẽ `img` trong `ui` (co theo bề rộng khung egui đang có), phủ lưới 3x3
/// lên trên. NẾU có `quad` (tứ giác ước lượng, xem `DetectedFace`) --
/// VẼ ĐÚNG THEO GÓC NGHIÊNG đó (4 cạnh tứ giác + lưới nội suy song tuyến
/// giữa các cạnh, xem ghi chú bên dưới) để phản ánh trung thực việc lấy
/// mẫu màu đang thực hiện qua `sample_for_region` (đã hiệu chỉnh phối
/// cảnh theo đúng `quad` này). Không có `quad` -- vẽ hình vuông thẳng
/// trục như trước đây (khung canh tay thủ công, hoặc dò bằng ML).
pub(super) fn draw_image_with_grid(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    img: &RgbImage,
    region: SquareRegion,
    quad: Option<[(f32, f32); 4]>,
    stroke_color: egui::Color32,
    texture: &mut Option<egui::TextureHandle>,
    dirty: &mut bool,
    texture_key: &str,
) {
    let (w, h) = (img.width(), img.height());

    let handle = match texture {
        // Đã có texture VÀ ảnh không đổi từ lần vẽ trước -> dùng lại
        // nguyên, không tốn CPU encode lại / GPU upload lại. Đây là
        // phần cốt lõi để hết giật: trước đây bước này chạy ở MỌI khung
        // hình vẽ của app, giờ chỉ chạy khi ảnh thực sự đổi (~webcam
        // 30fps, hoặc đúng 1 lần khi nạp file).
        Some(t) if !*dirty => t,
        _ => {
            // Giảm độ phân giải RIÊNG cho ảnh xem trước (không đụng tới
            // `img` gốc — sample_grid vẫn lấy mẫu trên ảnh gốc đầy đủ
            // để giữ độ chính xác màu) -- giảm hẳn dung lượng phải nén +
            // đẩy lên GPU mỗi lần cập nhật.
            const MAX_PREVIEW_W: u32 = 320;
            let preview = if w > MAX_PREVIEW_W {
                image::imageops::resize(
                    img,
                    MAX_PREVIEW_W,
                    (h as u64 * MAX_PREVIEW_W as u64 / w as u64) as u32,
                    image::imageops::FilterType::Triangle,
                )
            } else {
                img.clone()
            };
            let color_image = egui::ColorImage::from_rgb(
                [preview.width() as usize, preview.height() as usize],
                preview.as_raw(),
            );
            *dirty = false;
            match texture {
                Some(t) => {
                    t.set(color_image, egui::TextureOptions::LINEAR);
                    t
                }
                None => {
                    *texture = Some(ctx.load_texture(
                        texture_key,
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                    texture.as_mut().unwrap()
                }
            }
        }
    };

    let avail_w = ui.available_width().min(420.0);
    let disp_size = egui::vec2(avail_w, avail_w * h as f32 / w as f32);
    let img_rect = ui.image((handle.id(), disp_size)).rect;

    let disp_scale = disp_size.x / w as f32;
    let painter = ui.painter_at(img_rect);
    let stroke = egui::Stroke::new(2.5_f32, stroke_color);
    let to_screen = |p: (f32, f32)| {
        egui::pos2(
            img_rect.left() + p.0 * disp_scale,
            img_rect.top() + p.1 * disp_scale,
        )
    };

    if let Some([tl, tr, br, bl]) = quad {
        // Vẽ theo ĐÚNG tứ giác đã dò (không phải hình vuông thẳng trục)
        // -- nội suy song tuyến (bilinear) giữa 4 góc để ra lưới 3x3
        // "trông giống" phối cảnh thật. LƯU Ý: đây CHỈ để hiển thị, khác
        // với phép phối cảnh (homography) THẬT SỰ dùng lúc lấy mẫu màu ở
        // `sample_for_region`/`warp_to_square` -- 2 phép này trùng nhau ở
        // 4 góc và rất gần nhau ở giữa với góc nghiêng vừa phải (trường
        // hợp thực tế khi quét cube), nhưng KHÔNG toán học tương đương;
        // chấp nhận sai số nhỏ này vì vẽ đúng lưới phối cảnh cần thêm 1
        // phép biến đổi nữa cho mỗi điểm, không đáng để tăng độ phức tạp
        // chỉ cho phần vẽ overlay.
        let point_at = |u: f32, v: f32| {
            let top = (tl.0 + (tr.0 - tl.0) * u, tl.1 + (tr.1 - tl.1) * u);
            let bottom = (bl.0 + (br.0 - bl.0) * u, bl.1 + (br.1 - bl.1) * u);
            (
                top.0 + (bottom.0 - top.0) * v,
                top.1 + (bottom.1 - top.1) * v,
            )
        };
        let corners = [to_screen(tl), to_screen(tr), to_screen(br), to_screen(bl)];
        painter.add(egui::Shape::closed_line(corners.to_vec(), stroke));
        for k in 1..3 {
            let t = k as f32 / 3.0;
            painter.line_segment(
                [to_screen(point_at(t, 0.0)), to_screen(point_at(t, 1.0))],
                stroke,
            );
            painter.line_segment(
                [to_screen(point_at(0.0, t)), to_screen(point_at(1.0, t))],
                stroke,
            );
        }
        return;
    }

    // Không có quad (khung canh tay thủ công, hoặc dò bằng ML) -- vẽ
    // hình vuông thẳng trục như trước đây.
    let square_rect = egui::Rect::from_min_size(
        to_screen((region.x as f32, region.y as f32)),
        egui::vec2(
            region.size as f32 * disp_scale,
            region.size as f32 * disp_scale,
        ),
    );
    painter.rect_stroke(square_rect, 0.0, stroke);
    for k in 1..3 {
        let t = k as f32 / 3.0;
        let x = square_rect.left() + square_rect.width() * t;
        painter.line_segment(
            [
                egui::pos2(x, square_rect.top()),
                egui::pos2(x, square_rect.bottom()),
            ],
            stroke,
        );
        let y = square_rect.top() + square_rect.height() * t;
        painter.line_segment(
            [
                egui::pos2(square_rect.left(), y),
                egui::pos2(square_rect.right(), y),
            ],
            stroke,
        );
    }
}

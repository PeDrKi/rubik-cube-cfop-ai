//! Cửa sổ quét chính (`show`) và hình hướng dẫn cách cầm khối.

use super::*;
use super::{file::*, net::*, webcam::*};

/// Vẽ và xử lý cửa sổ quét. Trả về `Some(state)` đúng 1 lần, ở frame mà
/// người dùng bấm "Áp dụng vào cube" sau khi nhận diện thành công.
pub fn show(ctx: &egui::Context, s: &mut ScanState) -> Option<CubeState> {
    if !s.open {
        return None;
    }

    let mut applied: Option<CubeState> = None;
    let mut window_open = s.open;

    egui::Window::new("📷 Quét trạng thái từ camera")
        .open(&mut window_open)
        .default_width(460.0)
        .show(ctx, |ui| {
            // ── Chọn nguồn ảnh + chế độ chụp ─────────────────────────
            ui.horizontal(|ui| {
                ui.label("Nguồn ảnh:");
                if ui.selectable_label(s.mode == Mode::Webcam, "Webcam").clicked() {
                    s.mode = Mode::Webcam;
                }
                if ui.selectable_label(s.mode == Mode::File, "Nạp file ảnh").clicked() {
                    s.mode = Mode::File;
                    s.camera = None; // rời webcam thì đóng camera luôn, tránh chiếm dụng ngầm
                }
            });
            if s.mode == Mode::Webcam {
                ui.horizontal(|ui| {
                    ui.label("Chụp:");
                    if ui.selectable_label(s.capture_mode == CaptureMode::Manual, "Thủ công (phím Space)").clicked() {
                        s.capture_mode = CaptureMode::Manual;
                        s.stable_frames = 0;
                        s.stability_ref = None;
                        s.frame_history.clear();
                    }
                    if ui.selectable_label(s.capture_mode == CaptureMode::Auto, "Tự động khi giữ yên").clicked() {
                        s.capture_mode = CaptureMode::Auto;
                        s.stable_frames = 0;
                        s.stability_ref = None;
                        s.frame_history.clear();
                    }
                });
            }
            // ── Chọn cách dò vị trí ──────────────────────────────────
            //
            // LƯU Ý: lựa chọn ML KHÔNG phải bản nâng cấp, nó ĐANG KÉM HƠN
            // mặc định — nên chỗ này phải nói thẳng ra, chứ cái tên "thử
            // nghiệm" dễ làm người dùng tưởng là bản tốt hơn.
            //
            // Lý do, có số liệu: `detect_cube_face` (Hình học) trả về TỨ
            // GIÁC 4 đỉnh, `sample_for_region` dùng nó để hiệu chỉnh phối
            // cảnh rồi mới lấy mẫu lưới 3×3. `MlDetector` chỉ trả về hộp
            // VUÔNG, nên lưới 3×3 song song trục bị đặt vào hộp bao của
            // một hình thoi — lấy mẫu trúng viền đen hoặc ô bên cạnh, NGAY
            // CẢ khi hộp vuông chính xác tuyệt đối.
            ui.horizontal(|ui| {
                ui.label("Dò vị trí bằng:");
                if ui
                    .selectable_label(s.detect_method == DetectMethod::Heuristic, "Hình học (mặc định)")
                    .on_hover_text(
                        "Trả về tứ giác 4 đỉnh nên hiệu chỉnh được phối cảnh. \
                         Đo đầu-cuối trên bộ ảnh Bielefeld: đọc đúng 97,3% số ô.",
                    )
                    .clicked()
                {
                    s.detect_method = DetectMethod::Heuristic;
                    s.detected_region = None;
                    s.detected_quad = None;
                }
                if ui
                    .selectable_label(s.detect_method == DetectMethod::Ml, "ML (kém hơn)")
                    .on_hover_text(
                        "KÉM HƠN mặc định, chỉ để so sánh. Model chỉ trả hộp vuông, \
                         không có góc nghiêng nên không hiệu chỉnh được phối cảnh. \
                         Đo đầu-cuối: 32,9% số ô (so với 97,3% của Hình học).",
                    )
                    .clicked()
                {
                    s.detect_method = DetectMethod::Ml;
                    s.detected_region = None;
                    s.detected_quad = None;
                    if s.ml_detector.is_none() {
                        s.ml_detector = ml_bridge::Detector::load();
                    }
                }
            });
            if s.detect_method == DetectMethod::Ml {
                if s.ml_detector.is_none() {
                    ui.colored_label(
                        egui::Color32::from_rgb(230, 170, 40),
                        "⚠ Bản build này chưa bật tính năng ML (cần build với --features ml_detect).",
                    );
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(235, 92, 92),
                        "⚠ Lựa chọn này ĐANG KÉM HƠN mặc định — chỉ nên dùng để so sánh.",
                    );
                    ui.label(
                        egui::RichText::new(
                            "Mặt cube nhìn nghiêng là hình thoi. Model chỉ trả hộp vuông, \
                             nên lưới 3×3 thẳng trục đặt vào hộp bao sẽ lấy mẫu trúng viền \
                             hoặc ô bên cạnh — ngay cả khi hộp vuông hoàn hảo. Đo đầu-cuối \
                             trên bộ ảnh Bielefeld (đọc màu 27 ô): Hình học 97,3% · hộp vuông \
                             hoàn hảo 38,9% · hộp vuông do ML 32,9%.",
                        )
                        .size(11.0)
                        .color(crate::theme::TEXT_DIM),
                    );
                }
            }
            ui.separator();

            if s.resolve_result.is_none() && !s.captured.iter().all(|c| c.is_some()) {
                match s.mode {
                    Mode::Webcam => show_webcam_capture(ctx, ui, s),
                    Mode::File => show_file_capture(ui, s),
                }
            }

            ui.separator();

            // ── Tiến độ 6 mặt ────────────────────────────────────────
            ui.horizontal_wrapped(|ui| {
                let short = ["U", "D", "F", "B", "L", "R"];
                for i in 0..6 {
                    let done = s.captured[i].is_some();
                    let mark = if done { "✅" } else if i == s.current_face { "▶" } else { "○" };
                    let text = format!("{mark}{}", short[i]);
                    if ui.selectable_label(i == s.current_face, text).clicked() && s.resolve_result.is_none() {
                        s.current_face = i;
                        s.stable_frames = 0;
                        s.stability_ref = None;
                        s.frame_history.clear();
                        s.capture_cooldown = 0;
                    }
                }
            });

            if s.captured.iter().all(|c| c.is_some()) && s.resolve_result.is_none() {
                ui.add_space(6.0);
                if ui.button("🔍  Nhận diện trạng thái").clicked() {
                    let faces: [RawFace; 6] = std::array::from_fn(|i| s.captured[i].unwrap());
                    s.resolve_result = Some(rubik_vision::scan::resolve_scan(&RawScan { faces }));
                }
            }

            // ── Kết quả nhận diện ────────────────────────────────────
            let mut reset_requested = false;
            // (mặt, ô, chỉ số mặt màu người dùng vừa chọn để sửa tay) --
            // gom lại ở đây rồi áp dụng SAU khối `match` bên dưới, vì
            // bên trong khối đó `s.resolve_result` đang bị mượn bất biến
            // (không thể vừa đọc `suspicious` vừa sửa `s.captured` cùng lúc).
            let mut manual_fix: Option<(usize, usize, usize)> = None;
            if let Some(result) = &s.resolve_result {
                ui.add_space(8.0);
                match result {
                    Ok(report) => {
                        let rotated: Vec<String> = report
                            .rotations_applied
                            .iter()
                            .enumerate()
                            .filter(|(_, &r)| r != 0)
                            .map(|(i, &r)| format!("{} (đã tự xoay {}°)", FACE_LABELS[i], r as u32 * 90))
                            .collect();
                        ui.colored_label(egui::Color32::from_rgb(60, 190, 90), "✔ Nhận diện thành công — trạng thái hợp lệ và giải được.");
                        if !rotated.is_empty() {
                            ui.label(format!("Đã tự sửa hướng ảnh cho: {}", rotated.join(", ")));
                        }
                        if !report.auto_fixed.is_empty() {
                            ui.colored_label(
                                egui::Color32::from_rgb(230, 170, 40),
                                format!(
                                    "⚠ {} ô màu mơ hồ đã được tự đoán lại để ghép ra được trạng thái hợp lệ (viền vàng dưới đây). Kiểm tra lại nếu nghi ngờ.",
                                    report.auto_fixed.len()
                                ),
                            );
                            ui.add_space(4.0);
                            draw_cube_net(
                                ui,
                                &s.captured,
                                &report.auto_fixed,
                                egui::Color32::from_rgb(230, 170, 40),
                            );
                            ui.add_space(4.0);
                        }
                        ui.horizontal(|ui| {
                            if ui.button("✅  Áp dụng vào cube").clicked() {
                                applied = Some(report.state);
                            }
                            if ui.button("↺  Quét lại").clicked() {
                                reset_requested = true;
                            }
                        });
                    }
                    Err(ScanError::NoValidArrangement { suspicious }) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 80, 70),
                            "✘ Không tìm được cách ghép hợp lệ — có thể do phân loại màu sai ở vài ô.",
                        );
                        ui.label("Gợi ý: chụp lại dưới ánh sáng đều hơn, tránh bóng đổ/phản chiếu lên sticker.");
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(
                            "Viền đỏ = ô đáng ngờ. Bấm vào BẤT KỲ ô nào dưới đây (kể cả ô không viền đỏ, nếu bạn thấy sai bằng mắt) để sửa màu ngay, không cần quét lại:",
                        ).size(12.0));
                        ui.add_space(4.0);
                        let net_clicked = draw_cube_net(
                            ui,
                            &s.captured,
                            suspicious,
                            egui::Color32::from_rgb(220, 80, 70),
                        );
                        if let Some(pick) = net_clicked {
                            s.net_fix_target = Some(pick);
                        }
                        if let Some((f, sidx)) = s.net_fix_target {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "Đang sửa {} ô #{} → chọn đúng màu:",
                                    FACE_LABELS[f].split(" (").next().unwrap_or(""),
                                    sidx + 1
                                ));
                                for g in 0..6 {
                                    let Some(g_face) = &s.captured[g] else { continue };
                                    let c = g_face.samples[4]; // ô tâm = màu chuẩn đã biết của mặt g
                                    let (r2, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                                    ui.painter().rect_filled(r2, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                                    ui.painter().rect_stroke(r2, 3.0, egui::Stroke::new(1.0_f32, egui::Color32::from_gray(90)));
                                    if resp.on_hover_text(FACE_LABELS[g]).clicked() {
                                        manual_fix = Some((f, sidx, g));
                                    }
                                }
                            });
                        }
                        ui.add_space(4.0);
                        if ui.button("↺  Chụp lại từ đầu").clicked() {
                            reset_requested = true;
                        }
                    }
                    Err(ScanError::Ambiguous { candidates }) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(230, 170, 40),
                            format!("⚠ Có {} cách ghép cùng hợp lệ — chọn cách đúng với cube thật của bạn:", candidates.len()),
                        );
                        ui.add(egui::Slider::new(&mut s.candidate_pick, 0..=candidates.len().saturating_sub(1)).text("Phương án"));
                        ui.horizontal(|ui| {
                            if ui.button("✅  Dùng phương án này").clicked() {
                                if let Some(st) = candidates.get(s.candidate_pick) {
                                    applied = Some(*st);
                                }
                            }
                            if ui.button("↺  Chụp lại").clicked() {
                                reset_requested = true;
                            }
                        });
                    }
                }
            }
            if let Some((f, sidx, g)) = manual_fix {
                // Ghi đè ô đáng ngờ bằng ĐÚNG màu ô tâm của mặt màu người
                // dùng vừa chọn -- đảm bảo phân loại lại chắc chắn ra
                // đúng màu đó (khoảng cách 0 tới chính tâm mặt g), rồi
                // thử ghép lại NGAY mà không bắt quét lại toàn bộ 6 mặt.
                // Lấy màu ra trước (RawFace là Copy) để tránh giữ mượn
                // `s.captured[g]` trong lúc mượn-mutable `s.captured[f]`.
                let color = s.captured[g].map(|face| face.samples[4]);
                if let (Some(color), Some(target_face)) = (color, s.captured[f].as_mut()) {
                    target_face.samples[sidx] = color;
                    // Người dùng đã xác nhận màu này bằng tay -> coi như
                    // tin cậy tuyệt đối, tránh bị thuật toán cân bằng
                    // Hungarian (lần ghép lại ngay dưới đây) đổi ngược
                    // lại chỉ vì "rẻ" hơn về khoảng cách Lab.
                    target_face.confidence[sidx] = 1.0;
                }
                let faces: [RawFace; 6] = std::array::from_fn(|i| s.captured[i].unwrap());
                s.resolve_result = Some(rubik_vision::scan::resolve_scan(&RawScan { faces }));
                s.net_fix_target = None;
            }
            if reset_requested {
                s.reset_progress();
            }
        });

    s.open = window_open;
    if !window_open {
        s.close();
    }
    if applied.is_some() {
        s.close();
        s.reset_progress();
    }
    applied
}

/// Hướng dẫn ngắn cho từng bước, ĐÚNG thứ tự bắt buộc U,D,F,B,L,R. "Tư
/// thế ban đầu" = cầm cube thoải mái, ghi nhớ mặt nào đang hướng lên
/// trời (dùng cho bước U) và mặt nào đang hướng về phía người chụp
/// (dùng cho bước F) — 4 bước còn lại đều quay lại tư thế này.
pub(super) const ORIENTATION_HINTS: [&str; 6] = [
    "Cầm cube ở tư thế thoải mái bất kỳ — GHI NHỚ tư thế này. Mặt đang hướng LÊN TRỜI ngay bây giờ = U.",
    "Từ đúng tư thế ban đầu, LẬT NGƯỢC cube ra sau (như lật một quyển sách) — mặt giờ hướng lên trời = D.",
    "Quay lại ĐÚNG tư thế ban đầu. Mặt đang hướng VỀ PHÍA BẠN = F.",
    "Từ tư thế ban đầu, XOAY cube 180° quanh trục thẳng đứng (như vặn nắp chai) — mặt giờ hướng về phía bạn = B.",
    "Từ tư thế ban đầu, mặt bên TRÁI của cube = L.",
    "Từ tư thế ban đầu, mặt bên PHẢI của cube (đối diện L) = R.",
];

/// Sơ đồ khai triển 6 mặt cube (U ở trên, D ở dưới, L-F-R-B thành 1
/// hàng) — tô đậm mặt đang cần chụp, giúp hình dung "mặt này nằm ở đâu
/// so với các mặt kia" thay vì chỉ đọc chữ U/D/F/B/L/R trừu tượng.
pub(super) fn draw_orientation_guide(ui: &mut egui::Ui, current_face: usize) {
    let short = ["U", "D", "F", "B", "L", "R"];
    let cell = |ui: &mut egui::Ui, idx: usize| {
        let active = idx == current_face;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
        let fill = if active {
            egui::Color32::from_rgb(60, 200, 90)
        } else {
            egui::Color32::from_gray(50)
        };
        ui.painter().rect_filled(rect, 4.0, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            short[idx],
            egui::FontId::proportional(14.0),
            if active {
                egui::Color32::BLACK
            } else {
                egui::Color32::from_gray(210)
            },
        );
    };
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.add_space(34.0);
            cell(ui, 0); // U
        });
        ui.horizontal(|ui| {
            cell(ui, 4); // L
            cell(ui, 2); // F
            cell(ui, 5); // R
            cell(ui, 3); // B
        });
        ui.horizontal(|ui| {
            ui.add_space(34.0);
            cell(ui, 1); // D
        });
    });
    ui.add_space(4.0);
    ui.label(egui::RichText::new(ORIENTATION_HINTS[current_face]).size(12.0));
}

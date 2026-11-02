//! Chế độ chụp bằng webcam trực tiếp.

use super::*;
use super::{file::*, net::*, window::*};

pub(super) fn show_webcam_capture(ctx: &egui::Context, ui: &mut egui::Ui, s: &mut ScanState) {
    ui.horizontal(|ui| {
        draw_orientation_guide(ui, s.current_face);
    });
    ui.add_space(4.0);

    if s.camera.is_none() {
        if ui.button("▶  Mở webcam").clicked() {
            s.camera = Some(CameraHandle::start(0));
            s.camera_error = None;
        }
        let names = CameraHandle::list_available();
        if !names.is_empty() {
            ui.label(format!("Camera phát hiện: {}", names.join(", ")));
        }
        return;
    }

    // App chỉ vẽ lại khi có yêu cầu (không lặp vô điều kiện, xem ghi chú
    // ở đầu vòng lặp render trong main.rs) — nên khi đang xem webcam
    // phải chủ động xin vẽ lại. Hạ xuống ~10fps (100ms) thay vì 30fps
    // trước đây — canh khung bằng mắt không cần nhanh hơn vậy, và trên
    // máy yếu mỗi lần vẽ lại kéo theo cả sample_grid + dò vị trí + có
    // thể cả resize ảnh + upload GPU, nên giảm tần suất là cách hạ tải
    // hiệu quả nhất.
    ctx.request_repaint_after(std::time::Duration::from_millis(100));

    if let Some(err) = s.camera.as_ref().and_then(|c| c.take_error()) {
        s.camera_error = Some(err);
    }
    if let Some(err) = &s.camera_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 70), err);
    }

    // CHỈ đọc + clone frame khi camera thực sự có frame MỚI (so version,
    // rẻ) — trước đây clone ảnh ~1MB mỗi khung hình VẼ của app (có thể
    // 60-144 Hz) dù camera chỉ thực sự đổi ảnh ~30 lần/giây, gây giật.
    let mut got_new_frame = false;
    if let Some(cam) = &s.camera {
        let v = cam.version();
        if v != s.shown_camera_version {
            if let Some(frame) = cam.latest_frame() {
                s.current_image = Some(frame);
                s.shown_camera_version = v;
                s.texture_dirty = true;
                got_new_frame = true;
            }
        }
    }

    ui.add_space(2.0);

    if let Some(img) = s.current_image.take() {
        // Tự dò vị trí mặt cube -- CHỈ chạy lại khi có frame MỚI (đây
        // là phép tính nặng nhất trong toàn bộ luồng: hạ độ phân giải +
        // gán nhãn vùng liên thông), không chạy lặp vô ích mỗi khung
        // hình vẽ.
        if got_new_frame {
            let d = run_detection(s, &img);
            s.detected_region = d.as_ref().map(|d| d.region);
            s.detected_quad = d.and_then(|d| d.quad);
        }

        let (region, auto_mode) = match s.detected_region {
            Some(r) => (r, true),
            None => (fallback_region(&img, s.region_scale), false),
        };
        let stroke = if auto_mode {
            egui::Color32::from_rgb(60, 200, 90) // xanh lá = đã TỰ dò được
        } else {
            egui::Color32::from_rgb(255, 210, 0) // vàng = khung dự phòng canh tay
        };
        draw_image_with_grid(
            ctx,
            ui,
            &img,
            region,
            s.detected_quad,
            stroke,
            &mut s.live_texture,
            &mut s.texture_dirty,
            "webcam_preview",
        );
        if !auto_mode {
            ui.add(
                egui::Slider::new(&mut s.region_scale, 0.4..=0.95)
                    .text("Chưa tự dò được — chỉnh khung tay"),
            );
        }

        // Chỉ tính lại màu 9 ô khi vừa có frame MỚI -- máy yếu thì phép
        // lấy mẫu này cũng đáng kể nếu lặp lại vô ích mỗi lần vẽ dù ảnh
        // chưa đổi. Nếu có tứ giác ước lượng (`detected_quad`), hiệu
        // chỉnh phối cảnh trước khi lấy mẫu (xem `sample_for_region`).
        let (samples, confidence) = if got_new_frame || s.cached_samples.is_none() {
            let sc = sample_for_region(&img, region, s.detected_quad);
            s.cached_samples = Some(sc);
            sc
        } else {
            s.cached_samples.unwrap()
        };
        draw_swatches(ui, &samples);

        let valid = face_looks_valid(&samples);

        // Tích luỹ LỊCH SỬ khung hình "giữ yên" -- dùng chung cho CẢ 2 chế
        // độ chụp (trước đây chỉ chế độ Tự động mới theo dõi "giữ yên",
        // chế độ Thủ công reset mỗi khung nên không bao giờ tích luỹ được
        // gì). Chỉ cập nhật khi có frame MỚI (tránh đẩy trùng cùng 1 khung
        // nhiều lần) và ảnh đủ sáng/rõ (không lẫn khung "rác" vào kết quả
        // gộp cuối cùng). Phát hiện chuyển động (màu đổi hẳn so với khung
        // ngay trước) hoặc ảnh không đạt -> coi như bắt đầu lại từ đầu.
        if got_new_frame {
            if !valid {
                s.frame_history.clear();
                s.stability_ref = None;
            } else {
                let matches_prev = s
                    .stability_ref
                    .map(|prev| samples_close(&samples, &prev))
                    .unwrap_or(false);
                if !matches_prev {
                    s.frame_history.clear();
                }
                s.frame_history.push((samples, confidence));
                if s.frame_history.len() > FRAME_HISTORY_CAP {
                    s.frame_history.remove(0);
                }
                s.stability_ref = Some(samples);
            }
        }
        s.stable_frames = s.frame_history.len() as u32;

        let space_pressed =
            !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space));
        let mut do_capture = false;

        if s.capture_mode == CaptureMode::Manual {
            // Chế độ thủ công: không tự chụp, nhưng VẪN tận dụng lịch sử
            // khung hình vừa tích luỹ ở trên khi người dùng thật sự bấm
            // chụp (xem khối `if do_capture` bên dưới).
            if s.capture_cooldown > 0 {
                s.capture_cooldown -= 1;
            }
            if !valid {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 170, 40),
                    "⚠ Chưa đủ sáng/rõ để đọc màu — chỉnh ánh sáng hoặc khoảng cách rồi thử lại.",
                );
            } else if auto_mode {
                ui.colored_label(
                    egui::Color32::from_rgb(60, 190, 90),
                    "✅ Đã thấy khối rubik — bấm Space hoặc nút để chụp.",
                );
            } else {
                ui.label("Canh khung vàng cho khít mặt cube rồi bấm Space hoặc nút để chụp.");
            }
            if s.frame_history.len() > 1 {
                ui.label(format!(
                    "📊 Đang giữ yên — sẽ gộp {} khung hình gần nhất để tăng độ chính xác.",
                    s.frame_history.len()
                ));
            }
            if space_pressed {
                do_capture = true;
            }
        } else {
            // Chế độ tự động: đếm số khung hình liên tiếp ổn định (nay
            // chính là độ dài `frame_history`), tự chụp khi đủ ngưỡng —
            // Space vẫn hoạt động để chụp ngay lập tức nếu người dùng
            // không muốn chờ.
            if s.capture_cooldown > 0 {
                s.capture_cooldown -= 1;
                ui.colored_label(
                    egui::Color32::from_rgb(60, 190, 90),
                    "✔ Vừa chụp — chuẩn bị mặt tiếp theo…",
                );
            } else if !valid {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 170, 40),
                    "⚠ Chưa đủ sáng/rõ để đọc màu — chỉnh ánh sáng hoặc khoảng cách rồi thử lại.",
                );
            } else if s.stable_frames >= STABLE_FRAMES_NEEDED {
                do_capture = true;
            } else {
                let label = if auto_mode {
                    "✅ Đã thấy khối rubik — giữ yên…"
                } else {
                    "Giữ yên…"
                };
                ui.add(
                    egui::ProgressBar::new(s.stable_frames as f32 / STABLE_FRAMES_NEEDED as f32)
                        .text(label),
                );
            }
            if space_pressed {
                do_capture = true;
            }
        }

        if !do_capture
            && ui
                .add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new("📸  Chụp (Space)"),
                )
                .clicked()
        {
            do_capture = true;
        }

        if do_capture {
            // Gộp TOÀN BỘ lịch sử khung hình vừa giữ yên (xem
            // `aggregate_frames`) thay vì chỉ dùng đúng khung hình tại
            // thời điểm bấm chụp -- giảm nhiễu cảm biến/rung sáng ngẫu
            // nhiên giữa các khung. Lịch sử rỗng chỉ xảy ra trong trường
            // hợp hiếm (vd bấm Space ngay khung hình đầu tiên trước khi
            // `valid` từng đúng lần nào) -- khi đó lùi về đúng khung hiện
            // tại như hành vi trước đây.
            let (final_samples, final_confidence) = if s.frame_history.is_empty() {
                (samples, confidence)
            } else {
                aggregate_frames(&s.frame_history)
            };
            s.captured[s.current_face] = Some(RawFace {
                samples: final_samples,
                confidence: final_confidence,
            });
            if s.current_face < 5 {
                s.current_face += 1;
            }
            s.stable_frames = 0;
            s.stability_ref = None;
            s.frame_history.clear();
            s.capture_cooldown = COOLDOWN_FRAMES;
        }
        s.current_image = Some(img);
    } else {
        ui.label("Đang chờ frame từ camera…");
        ctx.request_repaint();
    }
}

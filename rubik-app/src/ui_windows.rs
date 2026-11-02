//! Cửa sổ nổi nhỏ: bảng công thức, icon bánh răng, phím tắt, hộp thoại quét/điền màu.

use crate::app::{App, Req};
use crate::ui_widgets::stage_row;
use crate::{paint_ui, scan_ui, theme};
use three_d::*;

impl App {
    pub(crate) fn ui_formula_table(&mut self, gui_context: &egui::Context, req: &mut Req) {
        // ── Bảng công thức (phím T) ─────────────────────────
        // Cửa sổ nổi, KHÔNG chặn thao tác khác (giống bản Python:
        // "không modal") -- vẫn xoay được khối, vẫn gõ được.
        if self.formula_open {
            if let Some((_, bd)) = &self.formula_table {
                let mut open = true;
                egui::Window::new("Bảng công thức CFOP")
                    .open(&mut open)
                    .default_width(390.0)
                    .default_pos(egui::pos2(req.actual_left_w + 24.0, 24.0))
                    .collapsible(false)
                    .frame(
                        egui::Frame::none()
                            .fill(theme::BG_PANEL)
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(13.0))
                            .stroke(egui::Stroke::new(1.0_f32, theme::BG_CARD_HI)),
                    )
                    .show(gui_context, |ui| {
                        ui.label(
                            egui::RichText::new(format!("Tổng cộng {} nước", bd.total_moves()))
                                .size(12.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical()
                            .max_height(430.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                stage_row(ui, "1) CROSS", "Cross", &bd.cross);
                                ui.add_space(9.0);
                                ui.label(
                                    egui::RichText::new("2) F2L  (4 cặp góc–cạnh)")
                                        .size(11.5)
                                        .strong()
                                        .color(theme::ACCENT),
                                );
                                for (slot, st) in &bd.f2l {
                                    stage_row(ui, "", slot, st);
                                }
                                ui.add_space(9.0);
                                stage_row(ui, "3) OLL  (định hướng lớp cuối)", "OLL", &bd.oll);
                                ui.add_space(9.0);
                                stage_row(ui, "4) PLL  (hoán vị lớp cuối)", "PLL", &bd.pll);
                            });
                    });
                if !open {
                    req.close_formula = true;
                }
            }
        }
    }

    pub(crate) fn ui_shortcuts(&mut self, gui_context: &egui::Context, req: &mut Req) {
        // ── Icon bánh răng ở góc màn hình -- mở bảng phím tắt ──
        egui::Area::new("shortcuts_gear_icon")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, 14.0))
            .order(egui::Order::Foreground)
            .show(gui_context, |ui| {
                let resp = ui.add(
                    egui::Button::new(egui::RichText::new("⚙").size(18.0))
                        .fill(theme::BG_CARD)
                        .stroke(egui::Stroke::new(1.0_f32, theme::BG_CARD_HI))
                        .rounding(egui::Rounding::same(8.0)),
                );
                if resp.on_hover_text("Phím tắt & hướng dẫn").clicked() {
                    self.shortcuts_open = !self.shortcuts_open;
                }
            });

        // ── Cửa sổ phím tắt & hướng dẫn (mở bằng icon bánh răng) ──
        if self.shortcuts_open {
            let mut open = true;
            egui::Window::new("⚙ Phím tắt & hướng dẫn")
                .open(&mut open)
                .default_width(340.0)
                .default_pos(egui::pos2(req.actual_left_w + 40.0, 60.0))
                .collapsible(false)
                .frame(
                    egui::Frame::none()
                        .fill(theme::BG_PANEL)
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(13.0))
                        .stroke(egui::Stroke::new(1.0_f32, theme::BG_CARD_HI)),
                )
                .show(gui_context, |ui| {
                    let row = |ui: &mut egui::Ui, k: &str, d: &str| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(k)
                                    .monospace()
                                    .size(11.5)
                                    .color(theme::ACCENT),
                            );
                            ui.label(egui::RichText::new(d).size(11.5).color(theme::TEXT_DIM));
                        });
                    };
                    row(ui, "U D F B L R", "xoay lớp tương ứng");
                    row(ui, "Shift + phím", "xoay ngược lại (U', R'…)");
                    row(
                        ui,
                        "Space",
                        "xáo ngẫu nhiên 25 nước, hiện công thức xáo ở panel bên trái",
                    );
                    row(
                        ui,
                        "Enter",
                        "AI tự giải, hiện công thức từng bước (Cross → F2L → OLL → PLL)",
                    );
                    row(ui, "H", "gợi ý 1 bước");
                    row(ui, "Ctrl (bấm rồi thả)", "thực hiện gợi ý đang hiển thị");
                    row(
                        ui,
                        "Alt (bấm rồi thả)",
                        "chạy từng NƯỚC một của gợi ý (H); hết thì tự tính gợi ý tiếp theo",
                    );
                    row(ui, "Ctrl+T", "bảng công thức cho ván này");
                    row(ui, "Ctrl+J", "tra toàn bộ công thức chuẩn");
                    row(ui, "Ctrl+Y", "thống kê OLL/PLL đã gặp");
                    row(ui, "Ctrl+K", "quét trạng thái từ self.camera");
                    row(
                        ui,
                        "Esc",
                        "huỷ khi đang tính · dừng giải tự động giữa chừng",
                    );
                    row(ui, "Ctrl+Z", "hoàn tác");
                    row(ui, "F11", "toàn màn hình");
                    row(
                        ui,
                        "[  ]",
                        &format!(
                            "tốc độ quay (hiện tại {:.2}s / bước 90°)",
                            std::f32::consts::FRAC_PI_2 / self.anim_speed
                        ),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Kéo chuột để xoay góc nhìn · cuộn để phóng to\n\
                             Bấm vào một ô màu rồi dùng phím mũi tên để xoay mặt đó",
                        )
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                    );
                });
            if !open {
                self.shortcuts_open = false;
            }
        }
    }

    pub(crate) fn ui_dialogs(&mut self, gui_context: &egui::Context, req: &mut Req) {
        // ── Cửa sổ quét trạng thái từ self.camera ─────────────────
        if let Some(new_state) = scan_ui::show(gui_context, &mut self.scan_state) {
            req.scanned_state = Some(new_state);
            req.state_source_label = "self.camera";
        }

        // ── Cửa sổ điền màu thủ công ──────────────────────────
        if let Some(new_state) = paint_ui::show(gui_context, &mut self.paint_state) {
            req.scanned_state = Some(new_state);
            req.state_source_label = "điền màu thủ công";
        }

        // Chặn phím tắt toàn cục khi BẤT KỲ ô nhập nào của egui
        // đang nhận bàn phím (ô công thức ở panel, ô tìm kiếm
        // trong bảng tra…). Trước đây chỉ chặn riêng ô công
        // thức, nên gõ vào ô tìm kiếm bị hiểu nhầm thành phím
        // tắt (gõ "Sune" làm chữ "u" xoay mặt U) -- lỗi thật đã
        // phát hiện khi thử.
        if gui_context.wants_keyboard_input() {
            req.editing_formula = true;
        }
    }
}

//! Panel trái (sơ đồ 6 mặt, nút hành động, ô công thức, trạng thái).

use crate::app::{App, Req};
use crate::theme;
use crate::ui_widgets::draw_six_face_net;
use three_d::*;

impl App {
    pub(crate) fn ui_left_panel(&mut self, gui_context: &egui::Context, req: &mut Req) {
        let panel_resp = egui::SidePanel::left("faces_panel")
            .default_width(req.left_w)
            .width_range(280.0..=640.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(13.0, 12.0)),
            )
            .show(gui_context, |ui| {
                // Cuộn được khi cửa sổ thấp / nội dung dài hơn
                // chiều cao panel (trước đây bị cắt mất phần dưới).
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // ── Tiêu đề ứng dụng ────────────────────────
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("RUBIK")
                                    .size(21.0)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            ui.label(
                                egui::RichText::new("CFOP Solver")
                                    .size(11.5)
                                    .color(theme::TEXT_DIM),
                            );
                        });
                        ui.add_space(11.0);

                        // ── Sơ đồ 6 mặt ─────────────────────────────
                        theme::card(ui, |ui| {
                            ui.horizontal(|ui| {
                                theme::section_label(ui, "Sơ đồ 6 mặt");
                                if let Some(f) = self.sel_face {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("● mặt {f}"))
                                                    .size(11.0)
                                                    .color(theme::ACCENT),
                                            );
                                        },
                                    );
                                }
                            });
                            ui.add_space(7.0);
                            draw_six_face_net(ui, &self.state, self.sel_face);
                        });
                        ui.add_space(10.0);

                        // ── Đầu vào ──────────────────────────────────
                        theme::card(ui, |ui| {
                            theme::section_label(ui, "Đầu vào");
                            ui.add_space(7.0);
                            ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 30.0],
                                        egui::Button::new("🎨  Điền màu thủ công"),
                                    )
                                    .on_hover_text("Tự chọn màu từng ô để mô phỏng trạng thái cube")
                                    .clicked()
                                {
                                    self.paint_state.open = true;
                                }
                            });
                        });
                        ui.add_space(10.0);

                        // ── Hành động chính ─────────────────────────
                        theme::card(ui, |ui| {
                            theme::section_label(ui, "Điều khiển");
                            ui.add_space(7.0);
                            ui.horizontal(|ui| {
                                let w = (ui.available_width() - 8.0) / 2.0;
                                ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                    if ui
                                        .add_sized([w, 32.0], egui::Button::new("🔀  Xáo"))
                                        .on_hover_text("Phím tắt: Space")
                                        .clicked()
                                    {
                                        req.request_scramble = true;
                                    }
                                });
                                if self.auto_solve {
                                    // Nút này LUÔN bấm được (không bị
                                    // req.is_busy/req.is_animating khoá) để dừng
                                    // được auto-solve bất kỳ lúc nào --
                                    // kể cả đang tính hay đang xoay dở.
                                    let text = egui::RichText::new("⏹  Dừng giải tự động")
                                        .strong()
                                        .color(theme::TEXT);
                                    if ui
                                        .add_sized(
                                            [w, 32.0],
                                            egui::Button::new(text)
                                                .fill(theme::DANGER)
                                                .rounding(egui::Rounding::same(8.0)),
                                        )
                                        .on_hover_text("Phím tắt: Esc")
                                        .clicked()
                                    {
                                        self.auto_solve = false;
                                        self.auto_plan_boundaries.clear();
                                        self.queue.clear();
                                        if self.job_rx.is_some() {
                                            rubik_core::cancel::request();
                                        }
                                        self.status = "Đã dừng giải tự động".to_string();
                                    }
                                } else {
                                    ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                        let text = egui::RichText::new("▶  Giải tự động")
                                            .strong()
                                            .color(egui::Color32::from_rgb(28, 20, 6));
                                        if ui
                                            .add_sized(
                                                [w, 32.0],
                                                egui::Button::new(text)
                                                    .fill(theme::ACCENT)
                                                    .rounding(egui::Rounding::same(8.0)),
                                            )
                                            .on_hover_text("Phím tắt: Enter")
                                            .clicked()
                                        {
                                            req.request_solve = true;
                                        }
                                    });
                                }
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let w = (ui.available_width() - 8.0) / 2.0;
                                ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                    if ui
                                        .add_sized([w, 30.0], egui::Button::new("💡  Gợi ý"))
                                        .on_hover_text("Gợi ý 1 bước tiếp theo — Phím tắt: H")
                                        .clicked()
                                    {
                                        req.request_hint = true;
                                    }
                                });
                                ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                    if ui
                                .add_sized([w, 30.0], egui::Button::new("📋  Bảng công thức"))
                                .on_hover_text(
                                    "Xem lời giải tách theo từng chặng CFOP — Phím tắt: Ctrl+T",
                                )
                                .clicked()
                            {
                                req.request_table = true;
                            }
                                });
                            });
                            ui.add_space(6.0);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 28.0],
                                    egui::Button::new("📊  Thống kê OLL / PLL"),
                                )
                                .on_hover_text("Số lần đã gặp từng thế OLL/PLL — Phím tắt: Ctrl+Y")
                                .clicked()
                            {
                                req.request_stats = true;
                            }
                        });
                        ui.add_space(10.0);

                        // ── Công thức xáo (nếu đã xáo ít nhất 1 lần) ─
                        if let Some(mvs) = &self.scramble_formula {
                            theme::card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🔀").size(13.0));
                                    ui.label(
                                        egui::RichText::new("Công thức xáo")
                                            .size(13.0)
                                            .strong()
                                            .color(theme::INFO),
                                    );
                                });
                                ui.add_space(5.0);
                                ui.label(
                                    egui::RichText::new(mvs.join("  "))
                                        .monospace()
                                        .size(13.5)
                                        .color(theme::ACCENT),
                                );
                            });
                            ui.add_space(10.0);
                        }

                        // ── Kết quả gợi ý ───────────────────────────
                        // Đặt TRƯỚC ô nhập công thức: khi vừa bấm H,
                        // gợi ý là thứ người dùng đang chờ đọc, nên nó
                        // phải nằm ngay dưới cụm nút thay vì bị đẩy
                        // xuống dưới ô nhập. Khối này chỉ hiện khi CÓ
                        // gợi ý, nên lúc không có thì bố cục y như cũ.
                        if let Some(lbl) = &self.hint_label {
                            theme::card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("💡").size(13.0));
                                    ui.label(
                                        egui::RichText::new(lbl)
                                            .size(13.0)
                                            .strong()
                                            .color(theme::INFO),
                                    );
                                });
                                if !self.hint_moves.is_empty() {
                                    ui.add_space(5.0);
                                    ui.label(
                                        egui::RichText::new(self.hint_moves.join("  "))
                                            .monospace()
                                            .size(13.5)
                                            .color(theme::ACCENT),
                                    );
                                    ui.add_space(6.0);
                                    ui.add_enabled_ui(!req.is_busy && !req.is_animating, |ui| {
                                        if theme::wide_button(
                                            ui,
                                            "Thực hiện gợi ý này",
                                            "Phím tắt: bấm rồi thả Ctrl",
                                        )
                                        .clicked()
                                        {
                                            req.apply_hint = true;
                                        }
                                    });
                                }
                            });
                            ui.add_space(10.0);
                        }

                        // ── Nhập công thức ──────────────────────────
                        theme::card(ui, |ui| {
                            theme::section_label(ui, "Công thức Singmaster");
                            ui.add_space(6.0);
                            let resp = ui.add_sized(
                                [ui.available_width(), 26.0],
                                egui::TextEdit::singleline(&mut self.formula_text)
                                    .hint_text("VD:  R U R' U'   hoặc   (R U)3"),
                            );
                            req.editing_formula = resp.has_focus() || resp.lost_focus();
                            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                req.submit_formula = true;
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let w = (ui.available_width() - 8.0) / 2.0;
                                if ui
                                    .add_sized([w, 26.0], egui::Button::new("Áp dụng"))
                                    .clicked()
                                {
                                    req.submit_formula = true;
                                }
                                if ui.add_sized([w, 26.0], egui::Button::new("Xoá")).clicked() {
                                    self.formula_text.clear();
                                }
                            });
                        });

                        // ── Trạng thái (luôn ở cuối) ────────────────
                        ui.add_space(10.0);
                        theme::card(ui, |ui| {
                            ui.set_max_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                if req.is_busy {
                                    ui.spinner();
                                }
                                let color = if req.is_busy {
                                    theme::ACCENT
                                } else if self.status.contains("Không")
                                    || self.status.contains("Lỗi")
                                {
                                    theme::DANGER
                                } else if req.is_solved_now {
                                    theme::SUCCESS
                                } else {
                                    theme::TEXT_DIM
                                };
                                ui.label(egui::RichText::new(&self.status).size(12.0).color(color));
                            });
                            if !self.queue.is_empty() || req.is_animating {
                                ui.add_space(5.0);
                                let done = self.total_queued.saturating_sub(self.queue.len());
                                let frac = if self.total_queued > 0 {
                                    done as f32 / self.total_queued as f32
                                } else {
                                    0.0
                                };
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_height(16.0)
                                        .fill(theme::ACCENT)
                                        .text(
                                            egui::RichText::new(format!(
                                                "{done}/{} nước",
                                                self.total_queued
                                            ))
                                            .size(10.0),
                                        ),
                                );
                            }
                            if req.is_solved_now && !req.is_animating && self.queue.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("✔  Khối đã hoàn thành")
                                        .size(12.5)
                                        .strong()
                                        .color(theme::SUCCESS),
                                );
                            }
                        });

                        // ── Phím tắt: đã chuyển sang cửa sổ riêng, mở
                        // bằng icon bánh răng ở góc màn hình (xem phía
                        // dưới, gần "Bảng công thức").
                    }); // het ScrollArea
            });
        req.actual_left_w = panel_resp.response.rect.width();
    }
}

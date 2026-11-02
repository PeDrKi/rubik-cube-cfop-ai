//! Cửa sổ bảng tra toàn bộ công thức chuẩn (phím V).

use crate::app::{App, Req};
use crate::theme;
use three_d::*;

impl App {
    pub(crate) fn ui_library(&mut self, gui_context: &egui::Context, req: &mut Req) {
        // ── Bảng tra toàn bộ công thức chuẩn (phím V) ───────
        if self.library_open {
            let mut open = true;
            egui::Window::new("Tra công thức chuẩn")
                .open(&mut open)
                .default_width(420.0)
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
                    let oll = rubik_core::oll_algorithms::all_algorithms();
                    let pll = rubik_core::pll_algorithms::all_algorithms();
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(!self.library_tab_pll, format!("OLL ({})", oll.len()))
                            .clicked()
                        {
                            self.library_tab_pll = false;
                        }
                        if ui
                            .selectable_label(self.library_tab_pll, format!("PLL ({})", pll.len()))
                            .clicked()
                        {
                            self.library_tab_pll = true;
                        }
                    });
                    ui.add_space(7.0);
                    ui.add_sized(
                        [ui.available_width(), 25.0],
                        egui::TextEdit::singleline(&mut self.library_query)
                            .hint_text("Tìm theo tên hoặc nước đi… VD: Sune, R U R'"),
                    );
                    ui.add_space(8.0);

                    let list = if self.library_tab_pll { pll } else { oll };
                    let q = self.library_query.trim().to_lowercase();
                    let filtered: Vec<&(&str, &str)> = list
                        .iter()
                        .filter(|(n, m)| {
                            q.is_empty()
                                || n.to_lowercase().contains(&q)
                                || m.to_lowercase().contains(&q)
                        })
                        .collect();

                    ui.label(
                        egui::RichText::new(format!("{} công thức", filtered.len()))
                            .size(11.0)
                            .color(theme::TEXT_DIM),
                    );
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (name, moves) in filtered {
                                egui::Frame::none()
                                    .fill(theme::BG_CARD)
                                    .rounding(egui::Rounding::same(7.0))
                                    .inner_margin(egui::Margin::symmetric(9.0, 7.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(*name)
                                                    .size(12.5)
                                                    .strong()
                                                    .color(theme::INFO),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .small_button("Dùng")
                                                        .on_hover_text(
                                                            "Nạp vào ô công thức bên trái",
                                                        )
                                                        .clicked()
                                                    {
                                                        req.load_into_bar = Some(moves.to_string());
                                                    }
                                                },
                                            );
                                        });
                                        ui.label(
                                            egui::RichText::new(*moves)
                                                .monospace()
                                                .size(12.0)
                                                .color(theme::ACCENT),
                                        );
                                    });
                                ui.add_space(5.0);
                            }
                        });
                });
            if !open {
                req.close_library = true;
            }
        }
    }
}

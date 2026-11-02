//! Cửa sổ thống kê OLL/PLL đã gặp (Ctrl+Y).

use crate::app::{App, Req};
use crate::{history, theme};
use three_d::*;

impl App {
    pub(crate) fn ui_stats(&mut self, gui_context: &egui::Context, req: &mut Req) {
        // ── Thống kê OLL/PLL đã gặp (Ctrl+Y) ────────────────
        // Liệt kê TOÀN BỘ thế trong bảng chuẩn, kể cả thế chưa
        // gặp lần nào (số 0) -- để thấy được mình còn thiếu thế
        // nào, chứ không chỉ thấy những thế đã gặp.
        if self.stats_open {
            let mut open = true;
            egui::Window::new("📊 Thống kê OLL / PLL")
                .open(&mut open)
                .default_width(330.0)
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
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.stats_tab_pll, false, "OLL");
                        ui.selectable_value(&mut self.stats_tab_pll, true, "PLL");
                    });
                    ui.add_space(6.0);

                    let kind = if self.stats_tab_pll { "PLL" } else { "OLL" };
                    let algs = if self.stats_tab_pll {
                        rubik_core::pll_algorithms::all_algorithms()
                    } else {
                        rubik_core::oll_algorithms::all_algorithms()
                    };
                    // Ghép bảng chuẩn với số đếm rồi sắp giảm dần.
                    // Thế cùng số lần thì xếp theo tên cho ổn định
                    // (không nhảy lung tung giữa các lần mở).
                    let mut rows: Vec<(&str, u32)> = algs
                        .iter()
                        .map(|(name, _)| {
                            let n = self
                                .stats_counts
                                .get(&(kind.to_string(), name.to_string()))
                                .copied()
                                .unwrap_or(0);
                            (*name, n)
                        })
                        .collect();
                    let n_standard = rows.len();
                    // Nhật ký có thể chứa tên KHÔNG nằm trong bảng
                    // chuẩn: khi không tra được thế, breakdown lùi
                    // về cách giải 2 bước ("2-look") hoặc tìm kiếm
                    // ("macro"). Phải hiện ra, nếu không thì tổng
                    // số lần sẽ lớn hơn tổng các dòng mà không hiểu
                    // vì sao.
                    let mut extra: Vec<(String, u32)> = self
                        .stats_counts
                        .iter()
                        .filter(|((k, name), _)| k == kind && !algs.iter().any(|(a, _)| a == name))
                        .map(|((_, name), n)| (name.clone(), *n))
                        .collect();
                    extra.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

                    let total = history::total(&self.stats_counts, kind);
                    let seen = rows.iter().filter(|(_, n)| *n > 0).count();
                    ui.label(
                        egui::RichText::new(format!(
                            "Đã gặp {seen}/{n_standard} thế · tổng {total} lần"
                        ))
                        .size(12.5)
                        .color(theme::TEXT_DIM),
                    );
                    ui.add_space(8.0);

                    let max = rows
                        .first()
                        .map(|(_, n)| *n)
                        .unwrap_or(0)
                        .max(extra.first().map(|(_, n)| *n).unwrap_or(0))
                        .max(1);
                    // Vẽ chung một vòng: bảng chuẩn trước, rồi tới
                    // các tên ngoài bảng (nếu có).
                    let all_rows: Vec<(&str, u32, bool)> = rows
                        .iter()
                        .map(|(n, c)| (*n, *c, false))
                        .chain(extra.iter().map(|(n, c)| (n.as_str(), *c, true)))
                        .collect();
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut sep_done = false;
                            for (name, n, is_extra) in &all_rows {
                                if *is_extra && !sep_done {
                                    sep_done = true;
                                    ui.add_space(8.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Ngoài bảng chuẩn (giải bằng cách dự phòng)",
                                        )
                                        .size(10.5)
                                        .color(theme::TEXT_DIM),
                                    );
                                }
                                ui.horizontal(|ui| {
                                    let col = if *n > 0 { theme::TEXT } else { theme::TEXT_DIM };
                                    // Cột tên co giãn theo bề rộng
                                    // cửa sổ (thay vì cắt cứng ở
                                    // 124px) và cho phép xuống
                                    // dòng khi tên dài + cửa sổ
                                    // hẹp -- chừa sẵn chiều cao
                                    // đủ cho 2 dòng để không đè
                                    // lên hàng dưới.
                                    let name_col_w =
                                        (ui.available_width() * 0.42).clamp(90.0, 260.0);
                                    ui.add_sized(
                                        [name_col_w, 28.0],
                                        egui::Label::new(
                                            egui::RichText::new(*name).size(11.5).color(col),
                                        ),
                                    );
                                    ui.add_sized(
                                        [30.0, 16.0],
                                        egui::Label::new(
                                            egui::RichText::new(n.to_string())
                                                .monospace()
                                                .size(11.5)
                                                .color(if *n > 0 {
                                                    theme::ACCENT
                                                } else {
                                                    theme::TEXT_DIM
                                                }),
                                        ),
                                    );
                                    // Thanh ngang so sánh tương đối
                                    // với thế gặp nhiều nhất.
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width().max(4.0), 8.0),
                                        egui::Sense::hover(),
                                    );
                                    if *n > 0 {
                                        let w = rect.width() * (*n as f32 / max as f32);
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                rect.left_center() - egui::vec2(0.0, 3.0),
                                                egui::vec2(w.max(2.0), 6.0),
                                            ),
                                            2.0,
                                            theme::ACCENT,
                                        );
                                    }
                                });
                            }
                        });
                    ui.add_space(8.0);
                    let where_txt = match history::path() {
                        Some(p) => format!("Nhật ký: {}", p.display()),
                        None => "Chưa xác định được nơi lưu nhật ký".to_string(),
                    };
                    ui.label(
                        egui::RichText::new(where_txt)
                            .size(10.5)
                            .color(theme::TEXT_DIM),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Mỗi lần bấm “Bảng công thức” cho một ván mới sẽ ghi thêm \
                             một dòng cho thế OLL và thế PLL của ván đó.",
                        )
                        .size(10.5)
                        .color(theme::TEXT_DIM),
                    );
                });
            if !open {
                req.close_stats = true;
            }
        }
    }
}

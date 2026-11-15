//! Chế độ tập trung (Focus): toàn màn hình, ẩn mọi thứ trừ khối Rubik ở giữa (vẫn xoay/kéo/cuộn như thường).
//! Chức năng nằm trong 2 popover nổi ở 2 mép: đưa chuột vào mép TRÁI (Điều khiển — chính là panel trái cũ)
//! hoặc mép PHẢI (Công cụ), hoặc bấm phím `,` / `.`. Phím `V` bật/tắt chế độ. `Esc` đóng popover.

use crate::app::{App, Req};
use crate::theme;
use three_d::*;
use winit::window::Fullscreen;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FocusSide {
    Left,
    Right,
}

/// Bề rộng vùng nhạy ở mép màn hình (đơn vị logic của egui).
const EDGE_PT: f32 = 12.0;
/// Trễ trước khi tự đóng popover mở bằng chuột sau khi con trỏ rời đi.
const CLOSE_DELAY: f64 = 0.35;
const MARGIN: f32 = 12.0;
const RIGHT_W: f32 = 270.0;

/// Con trỏ ở x (trong màn hình rộng `width`) có chạm mép nào không.
pub(crate) fn edge_hit(x: f32, width: f32) -> Option<FocusSide> {
    if x <= EDGE_PT {
        Some(FocusSide::Left)
    } else if x >= width - EDGE_PT {
        Some(FocusSide::Right)
    } else {
        None
    }
}

impl App {
    pub(crate) fn set_focus(&mut self, on: bool) {
        if on == self.focus {
            return;
        }
        self.focus = on;
        self.focus_open = None;
        self.focus_pinned = false;
        self.focus_leave = None;
        self.focus_edge = None;
        if on {
            // Chỉ tự bật toàn màn hình nếu đang ở cửa sổ thường; khi thoát chỉ trả lại nếu chính mình đã bật.
            self.focus_set_fs = !self.is_fullscreen;
            if self.focus_set_fs {
                self.is_fullscreen = true;
                self.winit_window
                    .set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
            self.status =
                "Chế độ tập trung: chạm mép trái/phải hoặc bấm , . để mở chức năng · V để thoát"
                    .to_string();
        } else if self.focus_set_fs {
            self.is_fullscreen = false;
            self.winit_window.set_fullscreen(None);
            self.focus_set_fs = false;
        }
        self.needs_redraw = true;
        self.redraw_next = true;
    }

    pub(crate) fn toggle_focus_side(&mut self, side: FocusSide) {
        if !self.focus {
            return;
        }
        if self.focus_open == Some(side) {
            self.focus_open = None;
            self.focus_pinned = false;
        } else {
            self.focus_open = Some(side);
            self.focus_pinned = true;
        }
        self.focus_leave = None;
        self.needs_redraw = true;
        self.redraw_next = true;
    }

    /// Vẽ popover đang mở (nếu có) và xử lý mở/đóng theo con trỏ.
    pub(crate) fn ui_focus_popovers(&mut self, ctx: &egui::Context, req: &mut Req) {
        let screen = ctx.screen_rect();
        let (ptr, dragging, pressed, now) = ctx.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.any_down(),
                i.pointer.primary_pressed(),
                i.time,
            )
        });
        let near = ptr.and_then(|p| edge_hit(p.x, screen.width()));
        // Chạm mép (không phải đang kéo xoay khối) -> mở popover bên đó.
        if let (Some(side), false) = (near, dragging) {
            // Chỉ khi con trỏ MỚI chạm mép — nếu không, con trỏ đang nằm ở mép sẽ ghi đè phím , / . mỗi khung hình.
            if self.focus_edge != near && self.focus_open != Some(side) {
                self.focus_open = Some(side);
                self.focus_pinned = false;
            }
            self.focus_leave = None;
        }
        self.focus_edge = near;
        let Some(side) = self.focus_open else { return };

        let h = (screen.height() - 2.0 * MARGIN).max(100.0);
        let (pos, w) = match side {
            FocusSide::Left => (egui::pos2(MARGIN, MARGIN), (req.left_w).clamp(300.0, 430.0)),
            FocusSide::Right => (
                egui::pos2(screen.right() - MARGIN - RIGHT_W, MARGIN),
                RIGHT_W,
            ),
        };
        let area = egui::Area::new(egui::Id::new("focus_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(theme::BG_PANEL)
                    .rounding(egui::Rounding::same(14.0))
                    .stroke(egui::Stroke::new(1.0_f32, theme::BG_CARD_HI))
                    .inner_margin(egui::Margin::symmetric(13.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_width(w - 26.0);
                        ui.set_height(h - 24.0);
                        match side {
                            FocusSide::Left => {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false; 2])
                                    .show(ui, |ui| self.left_panel_body(ui, req));
                            }
                            FocusSide::Right => self.focus_tools_body(ui, req),
                        }
                    });
            });
        let rect = area.response.rect;

        // Bấm ra ngoài (vào khối 3D) -> đóng cả khi đã "ghim" bằng phím.
        if pressed && !ctx.is_pointer_over_area() {
            self.focus_open = None;
            self.focus_pinned = false;
            self.focus_leave = None;
            return;
        }
        // Mở bằng chuột (không ghim): đóng sau một nhịp ngắn khi con trỏ rời cả popover lẫn mép.
        if !self.focus_pinned && near != Some(side) {
            let inside = ptr.map_or(false, |p| rect.expand(6.0).contains(p));
            if inside || ctx.wants_keyboard_input() {
                self.focus_leave = None;
            } else {
                let since = *self.focus_leave.get_or_insert(now);
                if now - since >= CLOSE_DELAY {
                    self.focus_open = None;
                    self.focus_leave = None;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    /// Nội dung popover phải: các công cụ / cửa sổ phụ.
    fn focus_tools_body(&mut self, ui: &mut egui::Ui, req: &mut Req) {
        ui.label(
            egui::RichText::new("CÔNG CỤ  ·  phím .")
                .size(10.5)
                .color(theme::TEXT_DIM)
                .strong(),
        );
        ui.add_space(8.0);
        ui.checkbox(&mut self.perm_open, "Sơ đồ hoán vị (P)")
            .on_hover_text("Hiện trong popover Điều khiển (bên trái)");
        ui.add_space(6.0);
        let idle = !req.is_busy && !req.is_animating;
        let mut acted = false;
        if theme::wide_button(ui, "📋  Bảng công thức", "Ctrl+T").clicked() {
            req.request_table = true;
            acted = true;
        }
        if theme::wide_button(ui, "📊  Thống kê OLL / PLL", "Ctrl+Y").clicked() {
            req.request_stats = true;
            acted = true;
        }
        if theme::wide_button(ui, "📚  Thư viện công thức", "Ctrl+J").clicked() {
            self.library_open = !self.library_open;
            acted = true;
        }
        ui.add_enabled_ui(idle, |ui| {
            if theme::wide_button(ui, "📷  Quét bằng camera", "Ctrl+K").clicked() {
                self.scan_state.open = true;
                acted = true;
            }
            if theme::wide_button(ui, "🎨  Điền màu thủ công", "").clicked() {
                self.paint_state.open = true;
                acted = true;
            }
        });
        if theme::wide_button(ui, "⚙  Phím tắt & hướng dẫn", "").clicked() {
            self.shortcuts_open = !self.shortcuts_open;
            acted = true;
        }
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        if theme::wide_button(ui, "⤢  Thoát chế độ tập trung (V)", "").clicked() {
            self.set_focus(false);
            return;
        }
        if acted {
            // Cửa sổ vừa mở nằm giữa màn hình — đóng popover cho khỏi che.
            self.focus_open = None;
            self.focus_pinned = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_hit_detects_left_and_right_edges_only() {
        assert_eq!(edge_hit(0.0, 1000.0), Some(FocusSide::Left));
        assert_eq!(edge_hit(EDGE_PT, 1000.0), Some(FocusSide::Left));
        assert_eq!(edge_hit(EDGE_PT + 0.5, 1000.0), None);
        assert_eq!(edge_hit(500.0, 1000.0), None);
        assert_eq!(edge_hit(1000.0 - EDGE_PT, 1000.0), Some(FocusSide::Right));
        assert_eq!(edge_hit(1000.0, 1000.0), Some(FocusSide::Right));
        assert_eq!(edge_hit(1000.0 - EDGE_PT - 0.5, 1000.0), None);
    }
}

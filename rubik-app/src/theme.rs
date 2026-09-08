//! theme.rs — font tiếng Việt + bảng màu và style cho giao diện.
//!
//! Font mặc định của egui (Ubuntu-Light) KHÔNG có đủ ký tự tiếng Việt --
//! đã kiểm chứng: "ể", "ạ", "ấ" hiện thành ô vuông ☐. Vì vậy nhúng
//! DejaVuSans (giấy phép tự do, hỗ trợ đầy đủ Latin Extended Additional)
//! để hiển thị được tiếng Việt có dấu.

use three_d::egui;

// ── Bảng màu ────────────────────────────────────────────────────────────
pub const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(18, 19, 28);
pub const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(26, 28, 40);
pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(34, 37, 52);
pub const BG_CARD_HI: egui::Color32 = egui::Color32::from_rgb(44, 48, 66);

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 168, 46);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(186, 120, 30);
pub const SUCCESS: egui::Color32 = egui::Color32::from_rgb(74, 205, 128);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(235, 92, 92);
pub const INFO: egui::Color32 = egui::Color32::from_rgb(96, 170, 255);

pub const TEXT: egui::Color32 = egui::Color32::from_rgb(228, 231, 242);
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(146, 152, 175);

/// Nạp font tiếng Việt. Gọi 1 lần lúc khởi động.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "vn".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/DejaVuSans.ttf")),
    );
    fonts.font_data.insert(
        "vn_bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/DejaVuSans-Bold.ttf")),
    );
    // Đặt font tiếng Việt LÊN ĐẦU danh sách ưu tiên, giữ font gốc phía sau
    // làm dự phòng cho các ký tự DejaVu không có (VD emoji).
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "vn".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "vn".to_owned());
    ctx.set_fonts(fonts);
}

/// Áp bảng màu + bo góc + cỡ chữ. Gọi 1 lần lúc khởi động.
pub fn install_style(ctx: &egui::Context) {
    use egui::{FontFamily::Proportional, FontId, TextStyle};

    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, Proportional)),
        (TextStyle::Body, FontId::new(13.5, Proportional)),
        (TextStyle::Button, FontId::new(14.0, Proportional)),
        (TextStyle::Small, FontId::new(11.5, Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, Proportional)),
    ]
    .into();

    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_PANEL;
    v.extreme_bg_color = BG_DEEP;
    v.faint_bg_color = BG_CARD;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT_DIM;

    let r = egui::Rounding::same(7.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.noninteractive.bg_fill = BG_CARD;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_DIM);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BG_CARD_HI);

    v.widgets.inactive.rounding = r;
    v.widgets.inactive.bg_fill = BG_CARD_HI;
    v.widgets.inactive.weak_bg_fill = BG_CARD_HI;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);

    v.widgets.hovered.rounding = r;
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 66, 90);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(60, 66, 90);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT_DIM);

    v.widgets.active.rounding = r;
    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.weak_bg_fill = ACCENT_DIM;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);

    ctx.set_style(style);
}

/// Khung "thẻ" bo góc, dùng để nhóm các phần trong panel cho dễ đọc.
pub fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(BG_CARD)
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(11.0, 10.0))
        .stroke(egui::Stroke::new(1.0, BG_CARD_HI))
        .show(ui, add);
}

/// Tiêu đề nhỏ của một mục (chữ hoa, mờ, cỡ nhỏ).
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(10.5)
            .color(TEXT_DIM)
            .strong(),
    );
}

/// Nút lớn, nổi bật (dùng cho hành động chính).
pub fn primary_button(ui: &mut egui::Ui, label: &str, hint: &str) -> egui::Response {
    let text = egui::RichText::new(label).size(14.0).strong().color(egui::Color32::from_rgb(28, 20, 6));
    ui.add_sized(
        [ui.available_width(), 34.0],
        egui::Button::new(text).fill(ACCENT).rounding(egui::Rounding::same(8.0)),
    )
    .on_hover_text(hint)
}

/// Nút thường, chiếm hết bề ngang phần còn lại.
pub fn wide_button(ui: &mut egui::Ui, label: &str, hint: &str) -> egui::Response {
    ui.add_sized([ui.available_width(), 30.0], egui::Button::new(label))
        .on_hover_text(hint)
}

//! scan_ui.rs — cửa sổ egui "Quét trạng thái từ camera": hướng dẫn chụp
//! lần lượt 6 mặt (U,D,F,B,L,R — đúng thứ tự mà `rubik_vision::scan`
//! yêu cầu), hỗ trợ cả webcam trực tiếp lẫn nạp ảnh từ file, rồi gọi
//! `rubik_vision::scan::resolve_scan` để ra `CubeState`.
//!
//! Thiết kế tách hẳn khỏi main.rs: main.rs chỉ cần giữ 1
//! `ScanState::default()`, gọi `scan_ui::show(...)` mỗi frame trong
//! closure `gui.update`, và nhận về `Option<CubeState>` khi người dùng
//! bấm "Áp dụng vào cube".

use image::RgbImage;
use rubik_core::cube::CubeState;
use rubik_vision::detect::detect_cube_face;
use rubik_vision::perspective::warp_to_square;
use rubik_vision::sample::{aggregate_frames, sample_grid_with_confidence, SquareRegion};
use rubik_vision::scan::{RawFace, RawScan, ScanError};
use three_d::egui;

use crate::camera_capture::CameraHandle;

/// Cầu nối tới `rubik-vision-ml` -- module giả lập RỖNG khi build KHÔNG
/// bật feature `ml_detect` (mặc định), để phần còn lại của file này
/// dùng `ml_bridge::Detector` bình thường mà không cần rải `#[cfg(...)]`
/// khắp nơi. Khi tắt feature, `load()` luôn trả `None` -> UI tự hiển
/// thị "chưa bật ML" thay vì crash/không compile.
#[cfg(feature = "ml_detect")]
mod ml_bridge {
    pub struct Detector(rubik_vision_ml::MlDetector);
    impl Detector {
        pub fn load() -> Option<Self> {
            rubik_vision_ml::MlDetector::load().ok().map(Detector)
        }
        pub fn detect(&mut self, img: &image::RgbImage) -> Option<rubik_vision::sample::SquareRegion> {
            self.0.detect(img)
        }
    }
}
#[cfg(not(feature = "ml_detect"))]
mod ml_bridge {
    pub struct Detector;
    impl Detector {
        pub fn load() -> Option<Self> {
            None
        }
        pub fn detect(&mut self, _img: &image::RgbImage) -> Option<rubik_vision::sample::SquareRegion> {
            None
        }
    }
}

/// Nhãn hiển thị cho từng mặt, ĐÚNG thứ tự chụp bắt buộc U,D,F,B,L,R —
/// thứ tự này phải khớp với quy ước của `rubik_vision::scan::RawScan`.
const FACE_LABELS: [&str; 6] = [
    "Mặt TRÊN (U — Up)",
    "Mặt DƯỚI (D — Down)",
    "Mặt TRƯỚC (F — Front)",
    "Mặt SAU (B — Back)",
    "Mặt TRÁI (L — Left)",
    "Mặt PHẢI (R — Right)",
];

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Webcam,
    File,
}

#[derive(PartialEq, Clone, Copy)]
enum CaptureMode {
    Manual,
    Auto,
}

#[derive(PartialEq, Clone, Copy)]
enum DetectMethod {
    Heuristic,
    Ml,
}

/// Kết quả dò vị trí mặt cube: hình vuông thẳng trục (dùng để vẽ khung/
/// khoanh vùng như trước đây) + tứ giác ước lượng NẾU CÓ (chỉ phương
/// pháp Hình học mới ước lượng được góc nghiêng; ML hiện chỉ trả hình
/// vuông thẳng trục, `quad: None`).
struct DetectedFace {
    region: SquareRegion,
    quad: Option<[(f32, f32); 4]>,
}

/// Dò vị trí mặt cube theo đúng phương pháp người dùng đang chọn
/// (`s.detect_method`) -- dùng chung cho cả webcam lẫn nạp file.
fn run_detection(s: &mut ScanState, img: &RgbImage) -> Option<DetectedFace> {
    match s.detect_method {
        DetectMethod::Heuristic => {
            detect_cube_face(img).map(|d| DetectedFace { region: d.region, quad: Some(d.quad) })
        }
        DetectMethod::Ml => s.ml_detector.as_mut().and_then(|d| d.detect(img)).map(|region| DetectedFace { region, quad: None }),
    }
}

/// Kích thước ảnh vuông sau khi "duỗi thẳng" phối cảnh -- đủ lớn để giữ
/// đủ chi tiết cho việc lấy mẫu 9 ô (mỗi ô ~1/3 cạnh này), không cần lớn
/// hơn vì bước lấy mẫu sau đó chỉ cần vài chục pixel/ô là đủ.
const WARP_SIZE: u32 = 300;

/// Lấy mẫu 9 ô (+ độ tin cậy) cho 1 vùng đã dò được. NẾU có tứ giác ước
/// lượng (`quad`, chỉ có ở phương pháp Hình học) -- HIỆU CHỈNH PHỐI CẢNH
/// trước (duỗi thẳng tứ giác thành ảnh vuông) rồi mới lấy mẫu trên ảnh đã
/// duỗi, xử lý đúng trường hợp mặt cube bị chụp lệch góc, thay vì giả
/// định nó đã vuông góc hoàn toàn với camera như khi lấy mẫu trực tiếp
/// trên `region` (hình vuông thẳng trục).
///
/// Lùi về lấy mẫu trực tiếp trên `region` (hành vi trước khi có bước này)
/// khi KHÔNG có `quad` (khung canh tay thủ công, hoặc dò bằng ML), hoặc
/// khi việc duỗi thất bại (tứ giác suy biến -- xem `warp_to_square`) --
/// cả 2 trường hợp đều KHÔNG coi là lỗi, chỉ đơn giản là chưa hiệu chỉnh
/// được nên dùng phương án cũ.
fn sample_for_region(img: &RgbImage, region: SquareRegion, quad: Option<[(f32, f32); 4]>) -> ([[u8; 3]; 9], [f32; 9]) {
    if let Some(q) = quad {
        if let Some(rectified) = warp_to_square(img, q, WARP_SIZE) {
            return sample_grid_with_confidence(&rectified, SquareRegion { x: 0, y: 0, size: WARP_SIZE });
        }
    }
    sample_grid_with_confidence(img, region)
}

pub struct ScanState {
    pub open: bool,
    mode: Mode,
    capture_mode: CaptureMode,
    detect_method: DetectMethod,
    ml_detector: Option<ml_bridge::Detector>,
    camera: Option<CameraHandle>,
    camera_error: Option<String>,
    live_texture: Option<egui::TextureHandle>,
    /// Số phiên bản frame webcam đã hiển thị lần gần nhất — dùng để chỉ
    /// upload texture khi có frame MỚI, tránh làm lại việc mỗi khung
    /// hình vẽ của app (đây là nguyên nhân chính gây giật/lag trước
    /// đây: encode + upload GPU ~1MB mỗi frame vẽ dù ảnh không đổi).
    shown_camera_version: u64,
    /// Đánh dấu ảnh hiện tại (webcam hoặc file) vừa đổi, cần upload lại
    /// texture ở lần vẽ tới; sau khi upload xong thì tắt cờ.
    texture_dirty: bool,
    /// Mẫu màu 9 ô (+ độ tin cậy từng ô) tính từ frame gần nhất — cache
    /// lại để KHÔNG phải tính lại `sample_grid_with_confidence` ở mỗi
    /// khung hình vẽ khi camera chưa có frame mới (máy yếu thì phép
    /// tính này cũng đáng kể nếu lặp vô ích 10-30 lần/giây).
    cached_samples: Option<([[u8; 3]; 9], [f32; 9])>,
    /// Vùng vuông TỰ DÒ được (toạ độ pixel ảnh gốc) từ frame gần nhất —
    /// None nghĩa là chưa dò được, phải dùng khung canh tay dự phòng.
    detected_region: Option<SquareRegion>,
    /// Tứ giác ước lượng đi kèm `detected_region` (xem `DetectedFace`) --
    /// None nếu chưa dò được HOẶC phương pháp dò hiện tại không ước
    /// lượng góc (ML). Dùng để hiệu chỉnh phối cảnh trước khi lấy mẫu
    /// màu (`sample_for_region`) và để vẽ khung khai triển đúng góc nhìn
    /// thay vì luôn vẽ hình vuông thẳng trục.
    detected_quad: Option<[(f32, f32); 4]>,
    /// Ảnh tĩnh hiện đang xem (frame webcam mới nhất, hoặc ảnh vừa nạp
    /// từ file) — dùng chung cho cả việc vẽ overlay lẫn lấy mẫu màu.
    current_image: Option<RgbImage>,
    /// Kích thước vùng vuông khoanh, tính theo tỉ lệ cạnh ngắn hơn của
    /// ảnh (0.4..0.95) — người dùng chỉnh bằng thanh trượt cho khít mặt
    /// cube trong khung hình.
    region_scale: f32,
    file_path: String,
    file_error: Option<String>,
    /// 6 mặt đã chụp, theo đúng thứ tự U,D,F,B,L,R. None = chưa chụp.
    captured: [Option<RawFace>; 6],
    current_face: usize,
    resolve_result: Option<Result<rubik_vision::scan::ScanReport, ScanError>>,
    candidate_pick: usize,
    /// Mẫu màu 9 ô của khung hình TRƯỚC — so với khung hiện tại để phát
    /// hiện "đang giữ yên" (tự động chụp), không cần bấm nút.
    stability_ref: Option<[[u8; 3]; 9]>,
    /// Số khung hình liên tiếp mẫu màu gần như không đổi -- LUÔN bằng
    /// `frame_history.len()` (xem field đó); giữ làm field riêng chỉ để
    /// không phải đổi các chỗ đang đọc `s.stable_frames` (thanh tiến độ,
    /// ngưỡng tự chụp).
    stable_frames: u32,
    /// Lịch sử các khung hình LIÊN TIẾP gần "giống nhau" (đang giữ yên
    /// cube) -- tích luỹ ở CẢ 2 chế độ chụp (thủ công lẫn tự động), không
    /// chỉ riêng chế độ tự động, để dù bấm Space thủ công cũng được lợi
    /// từ việc gộp nhiều khung. Bị xoá về rỗng (rồi seed lại từ khung hiện
    /// tại) ngay khi phát hiện chuyển động (không còn "giữ yên" nữa), và
    /// sau mỗi lần chụp xong 1 mặt. Giới hạn độ dài để không phình vô hạn
    /// nếu người dùng giữ yên rất lâu không bấm chụp.
    frame_history: Vec<([[u8; 3]; 9], [f32; 9])>,
    /// Số khung hình còn phải chờ trước khi bật lại dò ổn định — tránh
    /// chụp liên tiếp 2 lần cho cùng 1 mặt ngay sau khi vừa tự chụp.
    capture_cooldown: u32,
    /// (mặt, ô) người dùng vừa bấm chọn trên hình khai triển 6 mặt để
    /// sửa màu tay — None nghĩa là chưa chọn ô nào, chưa hiện bảng chọn
    /// màu bên dưới hình.
    net_fix_target: Option<(usize, usize)>,
}

/// Ngưỡng lệch màu tối đa (mỗi kênh RGB) để coi 2 khung hình là "giống
/// nhau" (đang giữ yên, không phải đang di chuyển/rung). Nới rộng hơn
/// trước để tay hơi run nhẹ không bị tính là "còn đang di chuyển".
const STABLE_THRESHOLD: i32 = 16;
/// Số khung hình liên tiếp ổn định cần để tự động chụp (~0.5s ở nhịp
/// 10fps). Giảm so với trước (8) vì người dùng phản ánh phải giữ yên
/// hơi lâu.
const STABLE_FRAMES_NEEDED: u32 = 5;
/// Số khung hình tối đa giữ lại trong `frame_history` để gộp -- vượt quá
/// mức này thì lợi ích giảm nhiễu đã bão hoà (trung bình N mẫu độc lập
/// giảm nhiễu theo 1/sqrt(N), càng về sau càng ít lợi hơn) trong khi vẫn
/// tốn bộ nhớ/tính toán mỗi khi gộp -- chặn để không phình vô hạn nếu
/// người dùng giữ yên cube rất lâu mà chưa bấm chụp.
const FRAME_HISTORY_CAP: usize = 10;
/// Số khung hình "nghỉ" sau khi tự chụp trước khi dò ổn định lại, để
/// người dùng có thời gian xoay cube sang mặt tiếp theo (~0.5s ở 10fps).
const COOLDOWN_FRAMES: u32 = 5;

impl Default for ScanState {
    fn default() -> Self {
        Self {
            open: false,
            mode: Mode::Webcam,
            capture_mode: CaptureMode::Manual,
            detect_method: DetectMethod::Heuristic,
            ml_detector: None,
            camera: None,
            camera_error: None,
            live_texture: None,
            shown_camera_version: 0,
            texture_dirty: false,
            cached_samples: None,
            detected_region: None,
            detected_quad: None,
            current_image: None,
            region_scale: 0.8,
            file_path: String::new(),
            file_error: None,
            captured: [None, None, None, None, None, None],
            current_face: 0,
            resolve_result: None,
            candidate_pick: 0,
            stability_ref: None,
            stable_frames: 0,
            frame_history: Vec::new(),
            capture_cooldown: 0,
            net_fix_target: None,
        }
    }
}

impl ScanState {
    /// Đóng cửa sổ VÀ giải phóng camera (nếu đang mở) — gọi khi người
    /// dùng bấm "Đóng"/"Huỷ", để không giữ webcam chiếm dụng ngầm khi
    /// cửa sổ quét không còn hiển thị.
    fn close(&mut self) {
        self.open = false;
        self.camera = None;
        self.live_texture = None;
        self.shown_camera_version = 0;
        self.texture_dirty = false;
        self.cached_samples = None;
        self.detected_region = None;
        self.detected_quad = None;
        self.net_fix_target = None;
        self.frame_history.clear();
    }

    fn reset_progress(&mut self) {
        self.captured = [None, None, None, None, None, None];
        self.current_face = 0;
        self.resolve_result = None;
        self.candidate_pick = 0;
        self.stability_ref = None;
        self.stable_frames = 0;
        self.frame_history.clear();
        self.capture_cooldown = 0;
        self.net_fix_target = None;
    }
}

/// Vẽ hình khai triển ("net") CẢ 6 MẶT cùng lúc — bố cục hình chữ thập
/// quen thuộc (U trên cùng, D dưới cùng, hàng giữa L-F-R-B), mỗi mặt là
/// lưới 3x3 màu THẬT đã chụp được. Tô viền màu `highlight_color` cho các
/// ô trong `highlight` (ô đáng ngờ hoặc vừa được tự sửa) — thay vì chỉ
/// nêu "mặt X ô số Y" bằng chữ, người dùng thấy ngay ô đó nằm ở đâu
/// trong tổng thể khối, giữa các ô xung quanh màu gì.
///
/// MỌI ô (không chỉ ô được tô viền) đều bấm được — trả về `Some((mặt,
/// ô))` ở đúng khung hình người dùng vừa bấm, để bên gọi cho phép sửa cả
/// những ô app không tự nghi ngờ nhưng người dùng nhìn thấy sai bằng mắt.
fn draw_cube_net(
    ui: &mut egui::Ui,
    captured: &[Option<RawFace>; 6],
    highlight: &[(usize, usize)],
    highlight_color: egui::Color32,
) -> Option<(usize, usize)> {
    // Tự tính toạ độ pixel cho TOÀN BỘ khối bằng 1 vùng cấp phát duy
    // nhất + vẽ trực tiếp bằng painter, KHÔNG lồng `egui::Grid` trong
    // `horizontal`/`vertical` -- cách lồng widget trước đó bị lệch canh
    // vì mỗi widget egui tự cộng thêm khoảng đệm/khoảng cách nội bộ
    // riêng, rất khó đoán trước để bù chính xác bằng `add_space`. Tính
    // tay như dưới đây đảm bảo U/D luôn thẳng cột với F và khoảng cách
    // đều tuyệt đối giữa mọi mặt.
    const CELL: f32 = 16.0;
    const CELL_GAP: f32 = 1.5;
    const FACE_GAP: f32 = 6.0;
    const FACE: f32 = CELL * 3.0 + CELL_GAP * 2.0;

    let total_size = egui::vec2(FACE * 4.0 + FACE_GAP * 3.0, FACE * 3.0 + FACE_GAP * 2.0);
    let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());
    let origin = rect.min;
    let painter = ui.painter_at(rect);
    let pointer_pos = response.interact_pointer_pos();
    let was_clicked = response.clicked();

    // Cột/hàng trong bố cục chữ thập: L,F,R,B ở hàng giữa (hàng 1); U ở
    // hàng trên (hàng 0), D ở hàng dưới (hàng 2) -- cả 2 đều thẳng cột
    // với F (cột 1).
    let face_grid_pos = |face_idx: usize| -> Option<(u32, u32)> {
        match face_idx {
            0 => Some((1, 0)), // U
            4 => Some((0, 1)), // L
            2 => Some((1, 1)), // F
            5 => Some((2, 1)), // R
            3 => Some((3, 1)), // B
            1 => Some((1, 2)), // D
            _ => None,
        }
    };

    let mut clicked: Option<(usize, usize)> = None;
    for f in 0..6 {
        let Some((col, row)) = face_grid_pos(f) else { continue };
        let face_min = egui::pos2(
            origin.x + col as f32 * (FACE + FACE_GAP),
            origin.y + row as f32 * (FACE + FACE_GAP),
        );
        for r in 0..3 {
            for c in 0..3 {
                let sidx = r * 3 + c;
                let cell_min = egui::pos2(
                    face_min.x + c as f32 * (CELL + CELL_GAP),
                    face_min.y + r as f32 * (CELL + CELL_GAP),
                );
                let cell_rect = egui::Rect::from_min_size(cell_min, egui::vec2(CELL, CELL));
                let color = captured[f].map(|face| face.samples[sidx]).unwrap_or([70, 70, 70]);
                painter.rect_filled(cell_rect, 2.0, egui::Color32::from_rgb(color[0], color[1], color[2]));
                let is_hl = highlight.contains(&(f, sidx));
                let stroke = if is_hl {
                    egui::Stroke::new(2.5, highlight_color)
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(70))
                };
                painter.rect_stroke(cell_rect, 2.0, stroke);
                if was_clicked && pointer_pos.is_some_and(|p| cell_rect.contains(p)) {
                    clicked = Some((f, sidx));
                }
            }
        }
    }

    clicked
}

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
                                    ui.painter().rect_stroke(r2, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(90)));
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
const ORIENTATION_HINTS: [&str; 6] = [
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
fn draw_orientation_guide(ui: &mut egui::Ui, current_face: usize) {
    let short = ["U", "D", "F", "B", "L", "R"];
    let cell = |ui: &mut egui::Ui, idx: usize| {
        let active = idx == current_face;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
        let fill = if active { egui::Color32::from_rgb(60, 200, 90) } else { egui::Color32::from_gray(50) };
        ui.painter().rect_filled(rect, 4.0, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            short[idx],
            egui::FontId::proportional(14.0),
            if active { egui::Color32::BLACK } else { egui::Color32::from_gray(210) },
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

fn show_webcam_capture(ctx: &egui::Context, ui: &mut egui::Ui, s: &mut ScanState) {
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
        draw_image_with_grid(ctx, ui, &img, region, s.detected_quad, stroke, &mut s.live_texture, &mut s.texture_dirty, "webcam_preview");
        if !auto_mode {
            ui.add(egui::Slider::new(&mut s.region_scale, 0.4..=0.95).text("Chưa tự dò được — chỉnh khung tay"));
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
                let matches_prev = s.stability_ref.map(|prev| samples_close(&samples, &prev)).unwrap_or(false);
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

        let space_pressed = !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space));
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
                ui.colored_label(egui::Color32::from_rgb(60, 190, 90), "✅ Đã thấy khối rubik — bấm Space hoặc nút để chụp.");
            } else {
                ui.label("Canh khung vàng cho khít mặt cube rồi bấm Space hoặc nút để chụp.");
            }
            if s.frame_history.len() > 1 {
                ui.label(format!("📊 Đang giữ yên — sẽ gộp {} khung hình gần nhất để tăng độ chính xác.", s.frame_history.len()));
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
                ui.colored_label(egui::Color32::from_rgb(60, 190, 90), "✔ Vừa chụp — chuẩn bị mặt tiếp theo…");
            } else if !valid {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 170, 40),
                    "⚠ Chưa đủ sáng/rõ để đọc màu — chỉnh ánh sáng hoặc khoảng cách rồi thử lại.",
                );
            } else if s.stable_frames >= STABLE_FRAMES_NEEDED {
                do_capture = true;
            } else {
                let label = if auto_mode { "✅ Đã thấy khối rubik — giữ yên…" } else { "Giữ yên…" };
                ui.add(egui::ProgressBar::new(s.stable_frames as f32 / STABLE_FRAMES_NEEDED as f32).text(label));
            }
            if space_pressed {
                do_capture = true;
            }
        }

        if !do_capture
            && ui.add_sized([ui.available_width(), 30.0], egui::Button::new("📸  Chụp (Space)")).clicked()
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
            let (final_samples, final_confidence) =
                if s.frame_history.is_empty() { (samples, confidence) } else { aggregate_frames(&s.frame_history) };
            s.captured[s.current_face] = Some(RawFace { samples: final_samples, confidence: final_confidence });
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

/// Khung vuông canh tay ở giữa ảnh, dùng khi CHƯA tự dò được vị trí
/// khối cube (fallback duy nhất, không tự động chụp trong chế độ này).
fn fallback_region(img: &RgbImage, region_scale: f32) -> SquareRegion {
    let (w, h) = (img.width(), img.height());
    let side = (w.min(h) as f32 * region_scale) as u32;
    SquareRegion { x: (w.saturating_sub(side)) / 2, y: (h.saturating_sub(side)) / 2, size: side }
}

/// Kiểm tra 9 mẫu màu đọc được có "hợp lệ" để tính vào bộ đếm giữ-yên
/// hay không — chặn 2 tình huống hay gặp nhất khi mới đưa cube vào
/// khung: khung vuông lệch ra ngoài ảnh (sample.rs trả về đúng
/// [128,128,128] khi 1 ô hoàn toàn nằm ngoài ảnh) và ảnh quá tối (ống
/// kính bị che, chưa đủ sáng). KHÔNG kiểm tra "có đúng màu cube không"
/// vì 1 mặt cube thật hoàn toàn có thể chỉ có 1-2 màu (sau khi giải 1
/// phần) — không thể coi "ít màu" là dấu hiệu sai.
fn face_looks_valid(samples: &[[u8; 3]; 9]) -> bool {
    let out_of_bounds = samples.iter().any(|&c| c == [128, 128, 128]);
    if out_of_bounds {
        return false;
    }
    let avg_brightness: u32 =
        samples.iter().map(|c| c[0] as u32 + c[1] as u32 + c[2] as u32).sum::<u32>() / (9 * 3);
    avg_brightness >= 15
}

/// So 2 bộ mẫu 9 ô: lệch tối đa mỗi kênh RGB ở MỌI ô đều trong ngưỡng ->
/// coi là "cùng 1 cảnh" (đang giữ yên), không phải đang di chuyển.
fn samples_close(a: &[[u8; 3]; 9], b: &[[u8; 3]; 9]) -> bool {
    a.iter().zip(b.iter()).all(|(ca, cb)| {
        ca.iter().zip(cb.iter()).all(|(&x, &y)| (x as i32 - y as i32).abs() <= STABLE_THRESHOLD)
    })
}

/// Vẽ 9 ô màu nhỏ (3x3) đúng như những gì hệ thống ĐANG đọc được từ
/// khung hình hiện tại — phản hồi trực quan tức thời để người dùng biết
/// ngay có canh đúng/đủ sáng hay không, không phải đợi xong cả 6 mặt.
fn draw_swatches(ui: &mut egui::Ui, samples: &[[u8; 3]; 9]) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Màu đang đọc được:").size(11.0).weak());
    egui::Grid::new("swatch_grid").spacing([3.0, 3.0]).show(ui, |ui| {
        for row in 0..3 {
            for col in 0..3 {
                let c = samples[row * 3 + col];
                let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));
            }
            ui.end_row();
        }
    });
    ui.add_space(4.0);
}

fn show_file_capture(ui: &mut egui::Ui, s: &mut ScanState) {
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
        let stroke =
            if auto_mode { egui::Color32::from_rgb(60, 200, 90) } else { egui::Color32::from_rgb(255, 210, 0) };
        draw_image_with_grid(&ctx, ui, &img, region, s.detected_quad, stroke, &mut s.live_texture, &mut s.texture_dirty, "file_preview");
        if auto_mode {
            ui.colored_label(egui::Color32::from_rgb(60, 190, 90), "✔ Đã tự phát hiện vị trí khối rubik.");
        } else {
            ui.colored_label(egui::Color32::from_rgb(230, 170, 40), "⚠ Chưa tự phát hiện được — dùng khung vàng, chỉnh thanh trượt cho khít.");
            ui.add(egui::Slider::new(&mut s.region_scale, 0.4..=0.95).text("Kích thước khung"));
        }
        let (samples, confidence) = sample_for_region(&img, region, s.detected_quad);
        draw_swatches(ui, &samples);
        let ctx2 = ui.ctx().clone();
        let space_pressed = !ctx2.wants_keyboard_input() && ctx2.input(|i| i.key_pressed(egui::Key::Space));
        let mut captured_now = false;
        let clicked = ui.add_sized([ui.available_width(), 34.0], egui::Button::new("✔  Dùng ảnh này cho mặt hiện tại (Space)")).clicked();
        if clicked || space_pressed {
            s.captured[s.current_face] = Some(RawFace { samples, confidence });
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
fn draw_image_with_grid(
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
            let color_image =
                egui::ColorImage::from_rgb([preview.width() as usize, preview.height() as usize], preview.as_raw());
            *dirty = false;
            match texture {
                Some(t) => {
                    t.set(color_image, egui::TextureOptions::LINEAR);
                    t
                }
                None => {
                    *texture = Some(ctx.load_texture(texture_key, color_image, egui::TextureOptions::LINEAR));
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
    let stroke = egui::Stroke::new(2.5, stroke_color);
    let to_screen =
        |p: (f32, f32)| egui::pos2(img_rect.left() + p.0 * disp_scale, img_rect.top() + p.1 * disp_scale);

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
            (top.0 + (bottom.0 - top.0) * v, top.1 + (bottom.1 - top.1) * v)
        };
        let corners = [to_screen(tl), to_screen(tr), to_screen(br), to_screen(bl)];
        painter.add(egui::Shape::closed_line(corners.to_vec(), stroke));
        for k in 1..3 {
            let t = k as f32 / 3.0;
            painter.line_segment([to_screen(point_at(t, 0.0)), to_screen(point_at(t, 1.0))], stroke);
            painter.line_segment([to_screen(point_at(0.0, t)), to_screen(point_at(1.0, t))], stroke);
        }
        return;
    }

    // Không có quad (khung canh tay thủ công, hoặc dò bằng ML) -- vẽ
    // hình vuông thẳng trục như trước đây.
    let square_rect = egui::Rect::from_min_size(
        to_screen((region.x as f32, region.y as f32)),
        egui::vec2(region.size as f32 * disp_scale, region.size as f32 * disp_scale),
    );
    painter.rect_stroke(square_rect, 0.0, stroke);
    for k in 1..3 {
        let t = k as f32 / 3.0;
        let x = square_rect.left() + square_rect.width() * t;
        painter.line_segment(
            [egui::pos2(x, square_rect.top()), egui::pos2(x, square_rect.bottom())],
            stroke,
        );
        let y = square_rect.top() + square_rect.height() * t;
        painter.line_segment(
            [egui::pos2(square_rect.left(), y), egui::pos2(square_rect.right(), y)],
            stroke,
        );
    }
}

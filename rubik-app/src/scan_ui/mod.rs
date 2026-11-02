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
        pub fn detect(
            &mut self,
            img: &image::RgbImage,
        ) -> Option<rubik_vision::sample::SquareRegion> {
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
        pub fn detect(
            &mut self,
            _img: &image::RgbImage,
        ) -> Option<rubik_vision::sample::SquareRegion> {
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
        DetectMethod::Heuristic => detect_cube_face(img).map(|d| DetectedFace {
            region: d.region,
            quad: Some(d.quad),
        }),
        DetectMethod::Ml => s
            .ml_detector
            .as_mut()
            .and_then(|d| d.detect(img))
            .map(|region| DetectedFace { region, quad: None }),
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
fn sample_for_region(
    img: &RgbImage,
    region: SquareRegion,
    quad: Option<[(f32, f32); 4]>,
) -> ([[u8; 3]; 9], [f32; 9]) {
    if let Some(q) = quad {
        if let Some(rectified) = warp_to_square(img, q, WARP_SIZE) {
            return sample_grid_with_confidence(
                &rectified,
                SquareRegion {
                    x: 0,
                    y: 0,
                    size: WARP_SIZE,
                },
            );
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

mod file;
mod net;
mod webcam;
mod window;

pub use window::show;

/// Khung vuông canh tay ở giữa ảnh, dùng khi CHƯA tự dò được vị trí
/// khối cube (fallback duy nhất, không tự động chụp trong chế độ này).
fn fallback_region(img: &RgbImage, region_scale: f32) -> SquareRegion {
    let (w, h) = (img.width(), img.height());
    let side = (w.min(h) as f32 * region_scale) as u32;
    SquareRegion {
        x: (w.saturating_sub(side)) / 2,
        y: (h.saturating_sub(side)) / 2,
        size: side,
    }
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
    let avg_brightness: u32 = samples
        .iter()
        .map(|c| c[0] as u32 + c[1] as u32 + c[2] as u32)
        .sum::<u32>()
        / (9 * 3);
    avg_brightness >= 15
}

/// So 2 bộ mẫu 9 ô: lệch tối đa mỗi kênh RGB ở MỌI ô đều trong ngưỡng ->
/// coi là "cùng 1 cảnh" (đang giữ yên), không phải đang di chuyển.
fn samples_close(a: &[[u8; 3]; 9], b: &[[u8; 3]; 9]) -> bool {
    a.iter().zip(b.iter()).all(|(ca, cb)| {
        ca.iter()
            .zip(cb.iter())
            .all(|(&x, &y)| (x as i32 - y as i32).abs() <= STABLE_THRESHOLD)
    })
}

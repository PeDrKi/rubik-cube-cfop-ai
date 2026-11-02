//! camera_capture.rs — mở webcam trên 1 thread nền, liên tục đọc frame
//! mới nhất và để UI (chạy trên thread render chính) đọc ra bất cứ lúc
//! nào mà không phải chờ — tránh việc gọi `Camera::frame()` (I/O chặn)
//! ngay trong vòng lặp vẽ khung hình, vốn sẽ làm giật toàn bộ app.
//!
//! Chỉ giữ FRAME MỚI NHẤT (không phải hàng đợi) vì UI chỉ cần xem trực
//! tiếp + chụp tại thời điểm bấm nút, không cần xử lý mọi frame.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use image::RgbImage;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
    Resolution,
};
use nokhwa::Camera;

/// Tay cầm điều khiển 1 phiên webcam đang chạy nền. Drop tay cầm này
/// (hoặc gọi `stop`) để dừng thread và giải phóng camera.
pub struct CameraHandle {
    latest: Arc<Mutex<Option<RgbImage>>>,
    /// Tăng dần mỗi khi có frame MỚI được lưu — cho phép UI kiểm tra
    /// "có gì mới không" bằng 1 phép so sánh số nguyên (rẻ), thay vì
    /// phải lock+clone cả ảnh (~1MB) mỗi khung hình vẽ chỉ để phát hiện
    /// không có gì thay đổi. Đây là nguyên nhân chính gây giật lag
    /// trước đây: preview bị re-upload lên GPU ở TẦN SỐ VẼ CỦA APP
    /// (có thể 60-144 Hz) thay vì tần số thật của camera (~30 Hz).
    version: Arc<AtomicU64>,
    error: Arc<Mutex<Option<String>>>,
    running: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl CameraHandle {
    /// Liệt kê camera khả dụng trên máy (tên hiển thị), để người dùng
    /// chọn khi có nhiều hơn 1 webcam.
    pub fn list_available() -> Vec<String> {
        nokhwa::query(ApiBackend::Auto)
            .map(|list| list.into_iter().map(|c| c.human_name()).collect())
            .unwrap_or_default()
    }

    /// Mở camera theo `index` (0 = camera mặc định) và bắt đầu đọc frame
    /// liên tục trên thread nền.
    pub fn start(index: u32) -> Self {
        let latest: Arc<Mutex<Option<RgbImage>>> = Arc::new(Mutex::new(None));
        let version = Arc::new(AtomicU64::new(0));
        let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let running = Arc::new(AtomicBool::new(true));

        let (latest_bg, version_bg, error_bg, running_bg) = (
            latest.clone(),
            version.clone(),
            error.clone(),
            running.clone(),
        );
        let join = std::thread::spawn(move || {
            // Yêu cầu THẲNG định dạng YUYV (thô) thay vì để nokhwa tự
            // chọn độ phân giải cao nhất — hầu hết webcam chỉ đạt độ
            // phân giải cao qua MJPEG, mà giải mã MJPEG cần crate
            // `mozjpeg` (C/C++, hay lỗi build trên Windows vì thiếu
            // NASM). YUYV được nokhwa giải mã bằng Rust thuần, không
            // cần build thêm gì.
            //
            // Độ phân giải hạ xuống 320x240 (trước là 640x480) -- việc
            // dò vị trí mặt cube (detect.rs) vốn đã tự resize ảnh xuống
            // chỉ còn ~220px cạnh dài trước khi xử lý, và việc lấy màu
            // trung bình mỗi ô sticker cũng không cần chi tiết cao, nên
            // 320x240 vẫn dư sức chính xác. Đổi lại: giải mã YUYV->RGB
            // (chạy liên tục trên thread nền) và upload ảnh preview lên
            // GPU mỗi khi có frame mới đều nhẹ đi ~4 lần số pixel --
            // khác biệt rõ rệt trên máy cấu hình thấp. FPS giữ ở 15 (đủ
            // mượt cho việc canh khung bằng mắt) để giảm tải CPU giải
            // mã liên tục trên máy cấu hình thấp — trước đây xin 30fps
            // trong khi việc canh cube hoàn toàn không cần nhanh vậy.
            let wanted = CameraFormat::new(Resolution::new(320, 240), FrameFormat::YUYV, 15);
            let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(wanted));
            let mut camera = match Camera::new(CameraIndex::Index(index), requested) {
                Ok(c) => c,
                Err(e) => {
                    *error_bg.lock().unwrap() = Some(format!("Không mở được camera: {e}"));
                    return;
                }
            };
            if let Err(e) = camera.open_stream() {
                *error_bg.lock().unwrap() = Some(format!("Không mở được luồng camera: {e}"));
                return;
            }

            while running_bg.load(Ordering::Relaxed) {
                match camera.frame() {
                    Ok(buffer) => match buffer.decode_image::<RgbFormat>() {
                        Ok(img) => {
                            *latest_bg.lock().unwrap() = Some(img);
                            version_bg.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            *error_bg.lock().unwrap() = Some(format!("Lỗi giải mã frame: {e}"))
                        }
                    },
                    Err(e) => {
                        *error_bg.lock().unwrap() = Some(format!("Lỗi đọc frame: {e}"));
                        // Lỗi đọc frame liên tục (vd rút camera giữa chừng) không nên
                        // vòng lặp busy-spin — nghỉ ngắn trước khi thử lại.
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                }
            }
            let _ = camera.stop_stream();
        });

        Self {
            latest,
            version,
            error,
            running,
            join: Some(join),
        }
    }

    /// Lấy bản sao frame mới nhất hiện có (None nếu chưa có frame nào,
    /// vd đang khởi động camera).
    pub fn latest_frame(&self) -> Option<RgbImage> {
        self.latest.lock().unwrap().clone()
    }

    /// Số phiên bản frame hiện tại — so sánh với lần đọc trước để biết
    /// có frame MỚI hay chưa mà không phải clone ảnh.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    /// Lỗi gần nhất (nếu có) — hiển thị cho người dùng, KHÔNG tự tắt
    /// camera khi có lỗi thoáng qua vì webcam có thể tự phục hồi.
    pub fn take_error(&self) -> Option<String> {
        self.error.lock().unwrap().take()
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for CameraHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

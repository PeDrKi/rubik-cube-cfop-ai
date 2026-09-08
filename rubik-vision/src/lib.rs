//! rubik-vision — nhận diện trạng thái Rubik từ ảnh chụp 6 mặt.
//!
//! Tách khỏi `rubik-core` để lõi thuật toán giải cube không phụ thuộc
//! `image`/camera. Luồng xử lý:
//!
//! 1. `sample`  — từ 1 ảnh (webcam hoặc file) + vùng vuông người dùng
//!    khoanh, lấy trung bình màu 9 ô lưới 3x3 -> `[[u8; 3]; 9]`.
//! 2. `color`   — không giả định "trắng là màu gì": dùng màu Ô TÂM của
//!    chính 6 ảnh làm 6 màu chuẩn (tâm không bao giờ đổi vị trí), rồi
//!    phân loại 54 ô còn lại theo khoảng cách Lab gần nhất. Tự thích
//!    nghi mọi color-scheme, mọi điều kiện ánh sáng tương đối.
//! 3. `scan`    — ghép 6 mặt đã phân loại thành `CubeState`. Vì hướng
//!    xoay của từng ảnh trong khung hình có thể sai (người dùng cầm
//!    lệch), tự thử tối đa 4^6 = 4096 tổ hợp xoay để tìm tổ hợp cho
//!    trạng thái hợp lệ + giải được, thay vì bắt buộc 1 quy trình cầm
//!    cube tuyệt đối chính xác.

pub mod color;
pub mod detect;
pub mod perspective;
pub mod sample;
pub mod scan;

pub use color::classify_scan;
pub use detect::{detect_cube_face, Detection};
pub use scan::{RawFace, RawScan, ScanError, ScanReport};

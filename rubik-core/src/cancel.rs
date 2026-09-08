//! cancel.rs — port của cơ chế hủy trong solver/search_utils.py: 1 cờ toàn
//! cục (module-level), vì kiến trúc chỉ chạy 1 job AI tại 1 thời điểm
//! (main.rs khoá UI trước khi cho phép bấm job mới, giống cfop_busy bên
//! Python). Nếu sau này cần nhiều job song song, phải đổi sang truyền
//! cancel token làm tham số tường minh cho từng hàm search.

use std::sync::atomic::{AtomicBool, Ordering};

static CANCEL: AtomicBool = AtomicBool::new(false);

/// Gọi 1 lần trước khi bắt đầu 1 job mới.
pub fn clear() {
    CANCEL.store(false, Ordering::Relaxed);
}

/// Gọi khi người dùng bấm Hủy.
pub fn request() {
    CANCEL.store(true, Ordering::Relaxed);
}

#[inline]
pub fn is_cancelled() -> bool {
    CANCEL.load(Ordering::Relaxed)
}

/// Kiểm tra mỗi N node thay vì mỗi node (tránh gọi atomic load quá nhiều) --
/// giống _CANCEL_CHECK_EVERY = 512 bên Python.
pub const CHECK_EVERY: u32 = 512;

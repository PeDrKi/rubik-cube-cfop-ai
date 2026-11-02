//! Sinh bảng hoán vị 54 ô của mọi nước đi, để trình duyệt tự tính nước xoay mà không cần gọi server.
//!
//!   cargo run -p rubik-core --example gen_move_table > web/move_table.json
//!
//! Bảng sinh từ chính engine Rust (không viết tay) và được `tests/move_table.rs` đối chiếu với file đã lưu,
//! nên không thể lệch với bộ giải. Định dạng: { "R": [54 số], "R'": [...], "R2": [...], ... } với
//! mới[i] = cũ[perm[i]] (chỉ số 0..53, thứ tự ô U D F B L R × 9 ô theo hàng, giống chuỗi trạng thái 54 ký tự).
fn main() {
    print!("{}", rubik_core::move_table::json());
}

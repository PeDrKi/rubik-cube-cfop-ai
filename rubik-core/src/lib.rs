//! rubik-core — lõi giải Rubik's Cube 3x3, port từ dự án Python gốc.
//!
//! Crate này chứa TOÀN BỘ phần thuật toán, độc lập hoàn toàn với giao
//! diện: cube engine, các mô hình cubie, pattern database, và pipeline
//! CFOP đầy đủ (Cross -> F2L -> OLL -> PLL).
//!
//! Trước đây phần này bị SAO CHÉP giữa 2 project riêng biệt (`rubik-rs`
//! và `rubik-render-windowed`), dẫn tới việc phải sửa lỗi 2 lần và code
//! đã bắt đầu lệch nhau. Gộp lại thành 1 crate thư viện để chỉ còn 1
//! nguồn sự thật duy nhất.
//!
//! # Ví dụ
//! ```no_run
//! use rubik_core::{cube::CubeState, solve};
//!
//! let mut state = CubeState::solved();
//! state.do_move("R");
//! state.do_move("U");
//! if let Some(moves) = solve::solve_full_cube(&state, 2) {
//!     println!("Lời giải {} nước: {}", moves.len(), moves.join(" "));
//! }
//! ```

pub mod breakdown;
pub mod cancel;
pub mod corner_model;
pub mod cross_solver;
pub mod cube;
pub mod edge_model;
pub mod f2l_solver;
pub mod full_state;
pub mod hint;
pub mod macro_solver;
pub mod move_simplify;
pub mod oll_algorithms;
pub mod oll_solver;
pub mod pdb;
pub mod pll_algorithms;
pub mod search;
pub mod solve;

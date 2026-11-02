//! Xử lý các yêu cầu do giao diện đặt ra trong khung hình (xáo, giải, gợi ý, nạp công thức...).

use crate::app::{App, Req};
use crate::history;
use crate::solve_flow::{queue_move, JobResult};
use rubik_core::cube::{self};

impl App {
    pub(crate) fn apply_requests(&mut self, req: &mut Req) {
        if let Some(new_state) = req.scanned_state.take() {
            self.push_undo();
            self.pending = None;
            self.queue.clear();
            self.state = new_state;
            self.move_count = 0;
            self.case_log.van_moi();
            self.formula_table = None;
            self.status = format!("Đã nạp trạng thái từ {}", req.state_source_label);
        }

        if req.close_formula {
            self.formula_open = false;
        }
        if req.close_stats {
            self.stats_open = false;
        }
        if req.request_stats {
            if self.stats_open {
                self.stats_open = false;
            } else {
                // Đọc lại file mỗi lần mở: nhật ký có thể vừa được ghi
                // thêm, và người dùng cũng có thể tự sửa file.
                self.stats_counts = history::counts();
                self.stats_open = true;
            }
        }
        if req.close_library {
            self.library_open = false;
        }
        if let Some(mvs) = req.load_into_bar.take() {
            self.formula_text = mvs;
            self.status = "Đã nạp công thức — bấm Áp dụng để chạy".to_string();
        }
        if req.request_table
            && self.job_rx.is_none()
            && self.queue.is_empty()
            && self.pending.is_none()
        {
            let fresh = self
                .formula_table
                .as_ref()
                .map(|(s, _)| *s == self.state)
                .unwrap_or(false);
            if fresh {
                self.formula_open = !self.formula_open;
            } else {
                rubik_core::cancel::clear();
                let state_copy = self.state;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let b = rubik_core::breakdown::full_solve_breakdown(&state_copy);
                    let _ = tx.send(JobResult::Breakdown(state_copy, b));
                });
                self.job_rx = Some(rx);
                self.status = "Đang lập bảng công thức…".to_string();
            }
        }
        if req.request_scramble && !req.is_busy && !req.is_animating && self.queue.is_empty() {
            self.push_undo();
            self.case_log.van_moi();
            self.formula_table = None;
            let mut rng = rand::thread_rng();
            let mut tmp = self.state;
            let mvs = cube::random_scramble_moves(&mut tmp, 25, &mut rng);
            self.scramble_formula = Some(mvs.iter().map(|s| s.to_string()).collect());
            self.queue.clear();
            for m in mvs {
                queue_move(&mut self.queue, m);
            }
            self.total_queued = self.queue.len();
            self.status = "Đang xáo…".to_string();
        }
        if req.request_solve && !req.is_busy && !req.is_animating && self.queue.is_empty() {
            if self.state.is_solved() {
                self.status = "Khối đã ở trạng thái hoàn thành".to_string();
            } else {
                rubik_core::cancel::clear();
                let state_copy = self.state;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let b = rubik_core::breakdown::full_solve_breakdown(&state_copy);
                    let _ = tx.send(JobResult::AutoPlan(state_copy, b));
                });
                self.job_rx = Some(rx);
                self.auto_solve = true;
                self.status = "Đang tính lời giải…".to_string();
            }
        }
        // Toàn bộ nước của "Giải tự động" đã được xếp hết vào `self.queue`
        // ngay từ lúc nhận JobResult::AutoPlan (xem bên dưới) -- nên
        // phát hiện XONG chỉ đơn giản là: đang ở chế độ self.auto_solve,
        // không còn job nào chạy, hàng đợi rỗng, không còn mốc chặng
        // nào chờ đổi nhãn nữa.
        if self.auto_solve
            && self.job_rx.is_none()
            && self.queue.is_empty()
            && self.pending.is_none()
            && self.auto_plan_boundaries.is_empty()
        {
            self.auto_solve = false;
            self.status = "Đã giải xong!".to_string();
        }

        if req.request_hint
            && self.job_rx.is_none()
            && self.queue.is_empty()
            && self.pending.is_none()
        {
            rubik_core::cancel::clear();
            let cur_stage = rubik_core::hint::stage_of(&self.state);
            let retry = self.last_hint_failed
                && self.last_hint_stage == Some(cur_stage)
                && self.last_hint_move_count == Some(self.move_count);
            let retry_seed = if retry {
                self.retry_counter += 1;
                Some(self.retry_counter)
            } else {
                None
            };
            let state_copy = self.state;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let h = rubik_core::hint::compute_hint(&state_copy, retry_seed);
                let _ = tx.send(JobResult::Hint(h));
            });
            self.job_rx = Some(rx);
            self.status = if retry {
                "Đang tính gợi ý (thử hướng khác)…".to_string()
            } else {
                "Đang tính gợi ý…".to_string()
            };
        }
        if req.apply_hint && !self.hint_moves.is_empty() {
            self.push_undo();
            self.queue.clear();
            for m in &self.hint_moves {
                queue_move(&mut self.queue, m);
            }
            self.status = "Đang thực hiện gợi ý…".to_string();
        }

        if req.submit_formula {
            match cube::parse_singmaster(&self.formula_text) {
                Ok(mvs) if !mvs.is_empty() => {
                    self.push_undo();
                    self.queue.clear();
                    for m in &mvs {
                        queue_move(&mut self.queue, m);
                    }
                    self.status = format!("Đã nạp {} nước từ công thức", mvs.len());
                }
                Ok(_) => {
                    self.status = "Công thức trống".to_string();
                }
                Err(e) => {
                    self.status = format!("Lỗi công thức: {e}");
                }
            }
            self.formula_text.clear();
        }
    }
}

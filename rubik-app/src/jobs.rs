//! Nhận kết quả của job nền (bảng công thức, gợi ý, kế hoạch giải tự động).

use crate::app::App;
use crate::history;
use crate::solve_flow::{build_auto_plan, build_boundaries, queue_move, JobResult};

impl App {
    pub(crate) fn poll_job(&mut self) {
        // ── Nhan ket qua job nen (khong chan khung hinh) ────────────
        if let Some(rx) = &self.job_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    JobResult::Breakdown(st, b) => {
                        self.status = format!("Bảng công thức: {} nước", b.total_moves());
                        // Ghi nhật ký NGAY tại đây, chỗ duy nhất bảng
                        // được tính xong. Vì `self.formula_table` chỉ tính lại
                        // khi trạng thái đổi, mở đi mở lại cùng một ván
                        // sẽ KHÔNG đếm trùng.
                        use rubik_core::breakdown::StageStatus;
                        let mut entries: Vec<(&str, String, usize)> = Vec::new();
                        for (kind, stage) in [("OLL", &b.oll), ("PLL", &b.pll)] {
                            if let StageStatus::Moves { moves, case_name } = stage {
                                if let Some(name) = case_name {
                                    // Chỉ ghi nếu ván này chưa ghi ca đó.
                                    if self.case_log.nen_ghi(kind, name) {
                                        entries.push((kind, name.clone(), moves.len()));
                                    }
                                }
                            }
                        }
                        if let Err(e) = history::append(&entries) {
                            // Không nuốt lỗi: nếu không ghi được (thư mục
                            // chỉ đọc chẳng hạn) thì phải nói ra, chứ để
                            // im lặng thì người dùng tưởng đã ghi.
                            self.status =
                                format!("Bảng công thức xong, nhưng KHÔNG ghi được nhật ký: {e}");
                        }
                        if self.stats_open {
                            self.stats_counts = history::counts();
                        }
                        self.formula_table = Some((st, b));
                        self.formula_open = true;
                    }
                    JobResult::AutoPlan(_st, b) => {
                        use rubik_core::breakdown::StageStatus;
                        // Ghi nhật ký case OLL/PLL y hệt Breakdown ở
                        // trên (dùng chung `full_solve_breakdown` nên
                        // dữ liệu cùng dạng) -- không đụng tới
                        // self.formula_table/self.formula_open, bảng công thức
                        // (Ctrl+T) là tính năng riêng.
                        let mut entries: Vec<(&str, String, usize)> = Vec::new();
                        for (kind, stage) in [("OLL", &b.oll), ("PLL", &b.pll)] {
                            if let StageStatus::Moves { moves, case_name } = stage {
                                if let Some(name) = case_name {
                                    if self.case_log.nen_ghi(kind, name) {
                                        entries.push((kind, name.clone(), moves.len()));
                                    }
                                }
                            }
                        }
                        let mut log_err: Option<String> = None;
                        if !entries.is_empty() {
                            if let Err(e) = history::append(&entries) {
                                log_err = Some(e.to_string());
                            }
                            if self.stats_open {
                                self.stats_counts = history::counts();
                            }
                        }

                        // Gom các chặng thành 1 kế hoạch phẳng theo
                        // đúng thứ tự CFOP (hàm thuần, có test riêng --
                        // xem build_auto_plan ở đầu file).
                        let (plan, failed_at) = build_auto_plan(&b);

                        if plan.is_empty() {
                            self.auto_solve = false;
                            self.status = match failed_at {
                                Some(name) => {
                                    format!("Không tìm được lời giải ở bước {name} — thử bấm lại")
                                }
                                None => "Khối đã ở trạng thái hoàn thành".to_string(),
                            };
                        } else {
                            self.push_undo();
                            self.queue.clear();
                            for (_, moves) in &plan {
                                for m in moves {
                                    queue_move(&mut self.queue, m);
                                }
                            }
                            self.total_queued = self.queue.len();
                            // Hiện nhãn của chặng ĐẦU TIÊN ngay bây giờ
                            // (không đợi nước nào chạy xong) -- phần còn
                            // lại nằm trong `self.auto_plan_boundaries`, tự
                            // đổi nhãn đúng lúc (xem chỗ `self.move_count += 1`).
                            let mut boundaries = build_boundaries(&plan);
                            let (_, first_label, first_moves) = boundaries.pop_front().unwrap();
                            self.hint_label = Some(first_label);
                            self.hint_moves = first_moves;
                            self.auto_plan_boundaries = boundaries;
                            self.auto_plan_start_count = self.move_count;
                            self.auto_solve = true;
                            self.status = match failed_at {
                                Some(name) => {
                                    format!(
                                        "Đang giải tự động ({} bước — riêng {name} chưa tìm được)…",
                                        plan.len()
                                    )
                                }
                                None => "Đang giải tự động…".to_string(),
                            };
                        }
                        if let Some(e) = log_err {
                            self.status = format!("{} (KHÔNG ghi được nhật ký: {e})", self.status);
                        }
                    }
                    JobResult::Hint(h) => {
                        // Gợi ý nhận ra một ca OLL/PLL cụ thể thì cũng
                        // tính vào nhật ký, y như bảng công thức. Chỉ
                        // ghi lần ĐẦU gặp ca đó trong ván này, nên bấm
                        // H bao nhiêu lần cũng không làm phồng số.
                        //
                        // Hai bước 2-look ("định hướng 4 cạnh/4 góc")
                        // có `case_name` là None vì chúng là bước lẻ
                        // chứ không phải một ca — không ghi gì cả.
                        let mut log_err: Option<String> = None;
                        if let Some(name) = &h.case_name {
                            let kind = match h.stage {
                                "oll" => Some("OLL"),
                                "pll" => Some("PLL"),
                                _ => None,
                            };
                            if let Some(kind) = kind {
                                if self.case_log.nen_ghi(kind, name) {
                                    let e = [(kind, name.clone(), h.moves.len())];
                                    if let Err(err) = history::append(&e) {
                                        // KHÔNG đặt `self.status` ở đây: đoạn
                                        // ngay dưới gán đè `self.status` bằng
                                        // nhãn gợi ý, nên lỗi sẽ bị nuốt
                                        // mất. Giữ lại rồi nối vào sau.
                                        log_err = Some(err);
                                    } else if self.stats_open {
                                        self.stats_counts = history::counts();
                                    }
                                }
                            }
                        }
                        self.last_hint_stage = Some(h.stage);
                        self.last_hint_move_count = Some(self.move_count);
                        self.last_hint_failed = h.moves.is_empty() && h.stage != "done";
                        self.status = if h.moves.is_empty() && h.stage == "done" {
                            h.label.clone()
                        } else if h.moves.is_empty() {
                            if rubik_core::cancel::is_cancelled() {
                                "Đã huỷ".to_string()
                            } else {
                                format!("{} — chưa tìm được", h.label)
                            }
                        } else {
                            format!("Gợi ý: {}", h.label)
                        };
                        if let Some(err) = log_err {
                            self.status =
                                format!("{} — KHÔNG ghi được nhật ký: {err}", self.status);
                        }
                        self.hint_label = Some(h.label);
                        self.hint_moves = h.moves;
                        if self.auto_apply_hint {
                            self.auto_apply_hint = false;
                            if !self.hint_moves.is_empty() {
                                self.push_undo();
                                self.queue.clear();
                                for m in &self.hint_moves {
                                    queue_move(&mut self.queue, m);
                                }
                                self.total_queued = self.queue.len();
                            }
                        } else if self.alt_step_pending {
                            // Alt vừa yêu cầu gợi ý mới (không có sẵn
                            // nước nào để chạy tiếp) -- giờ kết quả đã
                            // về, chạy luôn nước ĐẦU TIÊN của gợi ý mới
                            // này, giữ lại phần còn lại cho lần Alt kế.
                            self.alt_step_pending = false;
                            if !self.hint_moves.is_empty() {
                                self.push_undo();
                                let first = self.hint_moves.remove(0);
                                queue_move(&mut self.queue, &first);
                                self.total_queued = self.queue.len();
                                self.status = format!("Gợi ý (từng bước): {first}");
                            }
                        }
                    }
                }
                self.job_rx = None;
            }
        }
    }
}

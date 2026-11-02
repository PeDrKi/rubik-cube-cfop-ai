//! Phím tắt bàn phím (xoay mặt, gợi ý, giải, hoàn tác, mở cửa sổ...).

use crate::app::{App, Req};
use crate::history;
use crate::solve_flow::{queue_move, JobResult};
use rubik_core::cube;
use three_d::*;

impl App {
    pub(crate) fn handle_keys(&mut self, frame_input: &FrameInput, req: &Req) {
        for event in &frame_input.events {
            if req.editing_formula || self.scan_state.open || self.paint_state.open {
                break;
            }
            if let Event::KeyPress {
                kind,
                modifiers,
                handled,
                ..
            } = event
            {
                if *handled {
                    continue;
                }
                if modifiers.ctrl || modifiers.command {
                    // Đang giữ Ctrl mà có phím khác đi kèm => đây là một
                    // tổ hợp phím (Ctrl+T/J/K/Y/Z...), KHÔNG phải cú bấm
                    // Ctrl đơn để thực hiện gợi ý.
                    self.ctrl_combo_used = true;
                }
                if modifiers.alt {
                    // Tương tự: Alt đang giữ mà có phím khác đi kèm =>
                    // không tính là cú bấm Alt đơn để chạy 1 nước gợi ý.
                    self.alt_combo_used = true;
                }
                match kind {
                    // 6 mặt cơ bản. Kèm phím bổ trợ:
                    //   Shift = nước ngược (U')
                    //   Ctrl  = nước wide, xoay 2 lớp (u, r…)
                    //   Alt   = nước đúp 180° (U2) -- nạp 2 lần 90°
                    Key::U | Key::D | Key::F | Key::B | Key::L | Key::R => {
                        let base = match kind {
                            Key::U => 'U',
                            Key::D => 'D',
                            Key::F => 'F',
                            Key::B => 'B',
                            Key::L => 'L',
                            Key::R => 'R',
                            _ => unreachable!(),
                        };
                        // Ctrl -> chữ thường = nước wide (move_spec đã
                        // hỗ trợ sẵn u/d/f/b/l/r).
                        let mv = if modifiers.ctrl || modifiers.command {
                            base.to_ascii_lowercase()
                        } else {
                            base
                        };
                        self.push_undo();
                        self.queue.push_back((mv, modifiers.shift, modifiers.alt));
                    }
                    // Lớp giữa: M (giữa L-R), E (giữa U-D), S (giữa F-B).
                    Key::M => {
                        self.push_undo();
                        self.queue.push_back(('M', modifiers.shift, false));
                    }
                    Key::E => {
                        self.push_undo();
                        self.queue.push_back(('E', modifiers.shift, false));
                    }
                    Key::S => {
                        self.push_undo();
                        self.queue.push_back(('S', modifiers.shift, false));
                    }
                    // Xoay CẢ KHỐI: x (quanh trục R), y (quanh U), z (quanh F).
                    Key::X => {
                        self.push_undo();
                        self.queue.push_back(('x', modifiers.shift, false));
                    }
                    // Chốt chặn Ctrl giống hệt `Key::Z` ngay dưới: Ctrl+Y
                    // đã nhường cho cửa sổ thống kê. Không mất gì, vì
                    // nhánh này vốn BỎ QUA Ctrl -- Ctrl+Y trước đây chỉ là
                    // bản trùng lặp của Y, xoay y y hệt.
                    Key::Y if !(modifiers.ctrl || modifiers.command) => {
                        self.push_undo();
                        self.queue.push_back(('y', modifiers.shift, false));
                    }
                    Key::Z if !(modifiers.ctrl || modifiers.command) => {
                        self.push_undo();
                        self.queue.push_back(('z', modifiers.shift, false));
                    }
                    Key::Space => {
                        // Y hệt nút "🔀 Xáo" (kể cả điều kiện chặn khi
                        // đang bận/đang xoay) -- trước đây phím Space
                        // xáo được BẤT KỲ LÚC NÀO kể cả giữa lúc đang
                        // "Giải tự động", cắt ngang mà không cảnh báo.
                        // Cũng thiếu `self.case_log.van_moi()` (làm lệch số
                        // liệu thống kê OLL/PLL nếu ca vừa gặp ở ván
                        // trước lặp lại) và `self.formula_table = None`
                        // (bảng công thức cũ có thể còn treo đó).
                        if self.job_rx.is_none() && self.queue.is_empty() && self.pending.is_none()
                        {
                            self.push_undo();
                            self.case_log.van_moi();
                            self.formula_table = None;
                            let mut rng = rand::thread_rng();
                            let mut tmp = self.state;
                            let mvs = cube::random_scramble_moves(&mut tmp, 25, &mut rng);
                            self.scramble_formula =
                                Some(mvs.iter().map(|s| s.to_string()).collect());
                            self.queue.clear();
                            for m in mvs {
                                queue_move(&mut self.queue, m);
                            }
                            self.total_queued = self.queue.len();
                            self.status = "Đang xáo…".to_string();
                        } else if self.job_rx.is_none() {
                            self.status = "Đợi khối xoay xong đã".to_string();
                        }
                    }
                    Key::Enter => {
                        if self.job_rx.is_none() && self.queue.is_empty() && self.pending.is_none()
                        {
                            if self.state.is_solved() {
                                self.status = "Khối đã ở trạng thái hoàn thành".to_string();
                            } else {
                                // Y hệt khối req.request_solve bên trên,
                                // nhưng phải làm TRỰC TIẾP ở đây (không
                                // chỉ đặt cờ req.request_solve) vì vòng lặp
                                // phím này chạy SAU khối đó trong cùng
                                // 1 khung hình -- đặt cờ rồi sẽ không
                                // còn ai đọc lại ở khung này.
                                rubik_core::cancel::clear();
                                let state_copy = self.state;
                                let (tx, rx) = std::sync::mpsc::channel();
                                std::thread::spawn(move || {
                                    let b =
                                        rubik_core::breakdown::full_solve_breakdown(&state_copy);
                                    let _ = tx.send(JobResult::AutoPlan(state_copy, b));
                                });
                                self.job_rx = Some(rx);
                                self.auto_solve = true;
                                self.status = "Đang tính lời giải…".to_string();
                            }
                        } else if self.job_rx.is_none() {
                            self.status = "Đợi khối xoay xong đã".to_string();
                        }
                    }
                    Key::H => {
                        if self.job_rx.is_none() && self.queue.is_empty() && self.pending.is_none()
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
                        } else if self.job_rx.is_none() {
                            self.status = "Đợi khối xoay xong đã".to_string();
                        }
                    }
                    // Ctrl+T = mở/đóng bảng công thức (toàn bộ lời giải
                    // tách theo từng chặng CFOP), giống phím T bản Python
                    // (nay thêm Ctrl để tránh lẫn với các phím xoay mặt).
                    Key::T if modifiers.ctrl || modifiers.command => {
                        if self.formula_open {
                            self.formula_open = false;
                        } else if self.job_rx.is_none()
                            && self.queue.is_empty()
                            && self.pending.is_none()
                        {
                            let fresh = self
                                .formula_table
                                .as_ref()
                                .map(|(s, _)| *s == self.state)
                                .unwrap_or(false);
                            if fresh {
                                self.formula_open = true;
                            } else {
                                rubik_core::cancel::clear();
                                let state_copy = self.state;
                                let (tx, rx) = std::sync::mpsc::channel();
                                std::thread::spawn(move || {
                                    let b =
                                        rubik_core::breakdown::full_solve_breakdown(&state_copy);
                                    let _ = tx.send(JobResult::Breakdown(state_copy, b));
                                });
                                self.job_rx = Some(rx);
                                self.status = "Đang lập bảng công thức…".to_string();
                            }
                        }
                    }
                    // Ctrl+J = mở/đóng bảng tra toàn bộ công thức chuẩn.
                    Key::J if modifiers.ctrl || modifiers.command => {
                        self.library_open = !self.library_open;
                    }
                    // Ctrl+Y = mở/đóng thống kê OLL/PLL đã gặp.
                    //
                    // KHÔNG dùng Ctrl+L: nhánh 6 phím mặt ở trên nhận
                    // Ctrl+L làm nước WIDE `l` (xoay 2 lớp), lấy đi thì
                    // mất hẳn nước đó -- mà chỉ mất riêng mặt L, 5 mặt
                    // kia vẫn có wide, thành ra chỏng chơ. Ctrl+Y thì
                    // ngược lại: nhánh xoay khối bỏ qua Ctrl nên Ctrl+Y
                    // vốn chỉ là bản trùng của Y, lấy đi không mất gì.
                    //
                    // Xử lý thẳng tại đây (không qua cờ như nút bấm) vì
                    // khối bàn phím nằm ngoài lượt dựng giao diện.
                    Key::Y if modifiers.ctrl || modifiers.command => {
                        if self.stats_open {
                            self.stats_open = false;
                        } else {
                            self.stats_counts = history::counts();
                            self.stats_open = true;
                        }
                    }
                    // Ctrl+K = mở cửa sổ quét trạng thái từ self.camera (không
                    // có nút bấm riêng trong giao diện theo yêu cầu --
                    // chỉ vào được bằng phím tắt này).
                    Key::K if modifiers.ctrl || modifiers.command => {
                        if !req.is_busy && !req.is_animating {
                            self.scan_state.open = true;
                        }
                    }
                    Key::Escape => {
                        if self.stats_open {
                            self.stats_open = false;
                        } else if self.library_open {
                            self.library_open = false;
                        } else if self.formula_open {
                            self.formula_open = false;
                        } else if self.job_rx.is_some() {
                            rubik_core::cancel::request();
                            self.auto_solve = false;
                            self.auto_plan_boundaries.clear();
                            self.status = "Đang huỷ…".to_string();
                        } else if self.auto_solve {
                            // Đang giải tự động nhưng KHÔNG phải lúc
                            // đang tính (self.job_rx rỗng) -- tức là đang ở
                            // giữa lúc chạy nước (self.queue/self.pending). Dừng
                            // ngay tại đây: nước đang xoay dở vẫn xoay
                            // nốt cho mượt, nhưng các nước còn lại
                            // trong hàng đợi bị huỷ và không tính tiếp
                            // bước kế.
                            self.auto_solve = false;
                            self.auto_plan_boundaries.clear();
                            self.queue.clear();
                            self.status = "Đã dừng giải tự động".to_string();
                        }
                    }
                    Key::Z if modifiers.command || modifiers.ctrl => {
                        if let Some(prev) = self.undo_stack.pop() {
                            self.pending = None;
                            self.queue.clear();
                            self.auto_solve = false;
                            self.auto_plan_boundaries.clear();
                            self.state = prev;
                            self.move_count = self.move_count.saturating_sub(1);
                            self.status = "Đã hoàn tác".to_string();
                        } else {
                            self.status = "Không có gì để hoàn tác".to_string();
                        }
                    }
                    // Mui ten: xoay mat DANG CHON (click sticker de chon
                    // truoc). Up/Right = chieu thuan, Down/Left = nghich
                    // dao -- giong K_UP/K_RIGHT va K_DOWN/K_LEFT trong
                    // main.py.
                    Key::ArrowUp | Key::ArrowRight => {
                        if let Some(face) = self.sel_face {
                            self.push_undo();
                            self.queue.push_back((face, false, false));
                        }
                    }
                    Key::ArrowDown | Key::ArrowLeft => {
                        if let Some(face) = self.sel_face {
                            self.push_undo();
                            self.queue.push_back((face, true, false));
                        }
                    }
                    _ => {}
                }
            } else if let Event::ModifiersChange { modifiers } = event {
                let ctrl_now = modifiers.ctrl || modifiers.command;
                if ctrl_now && !self.ctrl_held {
                    // Ctrl vừa được nhấn xuống -- bắt đầu theo dõi xem
                    // có phím nào khác đi kèm trong lúc giữ hay không.
                    self.ctrl_held = true;
                    self.ctrl_combo_used = false;
                } else if !ctrl_now && self.ctrl_held {
                    // Ctrl vừa được thả ra.
                    self.ctrl_held = false;
                    if !self.ctrl_combo_used
                        && !self.hint_moves.is_empty()
                        && !req.is_busy
                        && !req.is_animating
                    {
                        // Bấm Ctrl đơn (không kèm phím khác) trong khi
                        // đang có gợi ý hiển thị => THỰC HIỆN gợi ý đó,
                        // giống hệt hành vi của nút "Thực hiện gợi ý
                        // này" (mục đích: đỡ phải với chuột sau khi vừa
                        // bấm H bằng bàn phím).
                        self.push_undo();
                        self.queue.clear();
                        for m in &self.hint_moves {
                            queue_move(&mut self.queue, m);
                        }
                        self.status = "Đang thực hiện gợi ý…".to_string();
                    }
                }

                let alt_now = modifiers.alt;
                if alt_now && !self.alt_held {
                    // Alt vừa được nhấn xuống -- bắt đầu theo dõi.
                    self.alt_held = true;
                    self.alt_combo_used = false;
                } else if !alt_now && self.alt_held {
                    // Alt vừa được thả ra.
                    self.alt_held = false;
                    if !self.alt_combo_used
                        && !req.is_busy
                        && !req.is_animating
                        && self.queue.is_empty()
                        && !self.state.is_solved()
                    {
                        if !self.hint_moves.is_empty() {
                            // Đang có sẵn gợi ý (còn nước chưa chạy) =>
                            // chạy đúng 1 nước ĐẦU TIÊN của nó, để dành
                            // phần còn lại cho lần bấm Alt kế tiếp.
                            self.push_undo();
                            let first = self.hint_moves.remove(0);
                            queue_move(&mut self.queue, &first);
                            self.status = format!("Gợi ý (từng bước): {first}");
                        } else if self.job_rx.is_none() && self.pending.is_none() {
                            // Hết nước của gợi ý cũ (hoặc chưa bấm gợi ý
                            // lần nào) => tính một gợi ý MỚI y hệt phím
                            // H, chỉ khác là khi có kết quả sẽ tự chạy
                            // luôn nước đầu tiên của nó (xem
                            // `self.alt_step_pending` ở chỗ nhận JobResult::Hint).
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
                            self.alt_step_pending = true;
                            self.status = if retry {
                                "Đang tính gợi ý (thử hướng khác)…".to_string()
                            } else {
                                "Đang tính gợi ý…".to_string()
                            };
                        }
                    }
                }
            }
        }
    }
}

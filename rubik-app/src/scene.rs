//! Khung nhìn 3D, chọn mặt bằng chuột, hoạt ảnh xoay lớp, vẽ khung hình.

use crate::app::{App, Req};
use crate::mesh::build_cube_mesh;
use crate::solve_flow::{advance_auto_plan, PendingMove};
use crate::ui_widgets::pick_face;
use three_d::*;
use winit::event_loop::ControlFlow;

impl App {
    pub(crate) fn update_camera_and_pick(&mut self, frame_input: &mut FrameInput, req: &Req) {
        // 3D chi chiem phan con lai ben phai panel.
        let full = frame_input.viewport;
        // Panel được vẽ ở tỉ lệ req.gui_ppp, nên quy đổi bề rộng của nó
        // sang pixel vật lý PHẢI dùng cùng tỉ lệ đó -- nếu dùng dpr
        // thô, vùng 3D sẽ lệch khỏi mép panel.
        let left_px = (req.actual_left_w * req.gui_ppp) as i32;
        let cam_viewport = Viewport {
            x: full.x + left_px,
            y: full.y,
            width: (full.width as i32 - left_px).max(1) as u32,
            // `height` truoc day khong duoc .max(1) nhu `width` --
            // khi Windows gui 1 frame co viewport.height = 0 (thuong
            // gap ngay frame dau tien luc mo app, hoac khi minimize/
            // restore cua so), ma tran chieu phoi canh (perspective
            // projection) tro thanh suy bien (aspect ratio vo cuc),
            // khien view*projection khong nghich dao duoc -> three-d
            // panic o .invert().unwrap() trong update_screen2ray().
            // Ghim height >=1 giong het width de tranh crash nay.
            height: full.height.max(1),
        };
        self.camera.set_viewport(cam_viewport);

        // ── Click chon mat (chi khi cube dang ranh -- khong animate) --
        // dung three_d::pick() (GPU raycast co san) truoc khi
        // OrbitControl xu ly, danh dau handled=true neu trung mat de
        // KHONG bi hieu nham thanh keo xoay self.camera (giong Python:
        // click trung sticker = chon mat, khong trung = keo self.camera).
        // Chỉ dựng lưới phục vụ bắt chuột KHI THỰC SỰ có cú click --
        // trước đây dựng lại mỗi khung hình dù không ai bấm, rất tốn.
        let has_click = frame_input.events.iter().any(|e| {
            matches!(
                e,
                Event::MousePress {
                    button: MouseButton::Left,
                    handled: false,
                    ..
                }
            )
        });
        if has_click && self.pending.is_none() && !req.editing_formula {
            let pick_cpu_mesh = build_cube_mesh(&self.state, None);
            let pick_mesh = Mesh::new(&self.context, &pick_cpu_mesh);
            let pick_model = Gm::new(pick_mesh, ColorMaterial::default());
            for event in frame_input.events.iter_mut() {
                if let Event::MousePress {
                    button: MouseButton::Left,
                    position,
                    handled,
                    ..
                } = event
                {
                    if *handled {
                        continue;
                    }
                    if position.x >= left_px as f32 {
                        if let Some(face) =
                            pick_face(&self.context, &self.camera, &pick_model, *position)
                        {
                            self.sel_face = Some(face);
                            *handled = true;
                        }
                    }
                }
            }
        }

        self.control
            .handle_events(&mut self.camera, &mut frame_input.events);
    }

    pub(crate) fn step_animation(&mut self, frame_input: &FrameInput) -> Option<(char, f32)> {
        if self.queue.len() > self.total_queued {
            self.total_queued = self.queue.len();
        }
        if self.pending.is_none() {
            if let Some((mv, prime, double)) = self.queue.pop_front() {
                let quarter = std::f32::consts::FRAC_PI_2;
                let mag = if double { 2.0 * quarter } else { quarter };
                let target = if prime { mag } else { -mag };
                self.pending = Some(PendingMove {
                    mv,
                    prime,
                    double,
                    target,
                    current: 0.0,
                    // Nước đúp quay gấp đôi góc nên tăng tốc để không
                    // mất gấp đôi thời gian.
                    speed: if double {
                        self.anim_speed * 1.6
                    } else {
                        self.anim_speed
                    },
                });
            }
        }

        let mut anim = self.pending.as_mut().map(|p| {
            let dt = (frame_input.elapsed_time / 1000.0) as f32;
            p.current += p.speed * dt * p.target.signum();
            if p.current.abs() >= p.target.abs() {
                p.current = p.target;
            }
            (p.mv, p.current)
        });
        if let Some(p) = &self.pending {
            if p.current == p.target {
                let mv_str = if p.double {
                    format!("{}2", p.mv)
                } else if p.prime {
                    format!("{}'", p.mv)
                } else {
                    p.mv.to_string()
                };
                self.state.do_move(&mv_str);
                self.move_count += 1;
                self.pending = None;
                // "Giải tự động" (self.auto_solve): đổi nhãn/công thức hiển
                // thị (self.hint_label/self.hint_moves) đúng lúc chặng kế tiếp
                // trong kế hoạch (đã tính sẵn 1 lần từ JobResult::AutoPlan)
                // bắt đầu -- KHÔNG tính toán gì thêm ở đây, chỉ so mốc.
                if self.auto_solve {
                    let progress = (self.move_count - self.auto_plan_start_count) as usize;
                    if let Some((label, moves)) =
                        advance_auto_plan(&mut self.auto_plan_boundaries, progress)
                    {
                        self.hint_label = Some(label);
                        self.hint_moves = moves;
                    }
                }
                // QUAN TRONG: phai XOA anim o day. `self.state` vua duoc cap
                // nhat da BAO GOM nuoc di nay roi, nen neu van giu anim
                // (dang o goc 90 do day du) thi build_cube_mesh se xoay
                // THEM 90 do nua len trang thai moi -> lop do bi xoay du
                // trong dung 1 khung hinh, gay hien tuong NHAY/GIAT o
                // moi nuoc khi may tu xoay. Da tim ra va sua.
                anim = None;
                if self.queue.is_empty() {
                    self.total_queued = 0;
                    if self.status.starts_with("Đang xáo") {
                        self.status = "Đã xáo xong".to_string();
                    }
                }
            }
        }
        anim
    }

    pub(crate) fn render_frame(
        &mut self,
        frame_input: &mut FrameInput,
        gui: &GUI,
        anim: Option<(char, f32)>,
    ) {
        let cpu_mesh = build_cube_mesh(&self.state, anim);
        let mesh = Mesh::new(&self.context, &cpu_mesh);
        let cube_model = Gm::new(mesh, ColorMaterial::default());

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.11, 0.11, 0.16, 1.0, 1.0))
            .render(&self.camera, &[&cube_model], &[&self.ambient, &self.sun])
            .write(|| gui.render())
            .unwrap();

        self.gl.swap_buffers().unwrap();
    }

    pub(crate) fn end_frame(&mut self, control_flow: &mut ControlFlow) {
        // Chỉ chạy hết tốc lực KHI CẦN (đang xoay khối, đang tính, hoặc
        // giao diện đang có hoạt ảnh). Lúc rảnh thì ngủ chờ sự kiện --
        // trước đây luôn Poll + request_redraw nên ngốn ~97% CPU dù
        // không làm gì. Đây là thay đổi quan trọng nhất cho máy yếu.
        let busy_now = self.pending.is_some() || !self.queue.is_empty() || self.job_rx.is_some();
        self.needs_redraw = busy_now;
        if busy_now {
            *control_flow = ControlFlow::Poll;
            self.winit_window.request_redraw();
        } else {
            // Vẫn thức dậy định kỳ (~20 lần/giây) để kịp nhận kết quả
            // luồng nền và giữ con trỏ chuột phản hồi mượt.
            *control_flow = ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(50),
            );
        }
    }
}

//! rubik-app — ứng dụng Rubik's Cube 3D tương tác.
//!
//! File này chỉ chứa phần GIAO DIỆN: dựng hình học cube cho GPU, vòng lặp
//! winit, camera, panel egui (khung 6 mặt + thanh nhập công thức), và xử
//! lý phím/chuột. TOÀN BỘ thuật toán giải nằm trong crate `rubik-core`.
//!
//! Xem README.md ở thư mục gốc workspace để biết cách chạy.


mod theme;

use rubik_core::cube::{self, CubeState};
use std::collections::VecDeque;
use three_d::*;

const CUBIE_H: f32 = 0.96;
const STICKER_H: f32 = 0.76;
const PLASTIC: [u8; 3] = [20, 20, 20];
const FACES: [char; 6] = ['U', 'D', 'F', 'B', 'L', 'R'];

fn face_color(face: char) -> [u8; 3] {
    match face {
        'U' => [255, 210, 0],
        'D' => [255, 255, 255],
        'F' => [15, 160, 55],
        'B' => [15, 75, 210],
        'L' => [210, 30, 45],
        'R' => [255, 100, 0],
        _ => unreachable!(),
    }
}

fn face_info(face: char) -> (Vec3, Vec3, Vec3) {
    match face {
        'U' => (vec3(0., 1., 0.), vec3(1., 0., 0.), vec3(0., 0., -1.)),
        'D' => (vec3(0., -1., 0.), vec3(1., 0., 0.), vec3(0., 0., 1.)),
        'F' => (vec3(0., 0., 1.), vec3(1., 0., 0.), vec3(0., 1., 0.)),
        'B' => (vec3(0., 0., -1.), vec3(-1., 0., 0.), vec3(0., 1., 0.)),
        'L' => (vec3(-1., 0., 0.), vec3(0., 0., 1.), vec3(0., 1., 0.)),
        'R' => (vec3(1., 0., 0.), vec3(0., 0., -1.), vec3(0., 1., 0.)),
        _ => unreachable!(),
    }
}

fn move_spec(mv: char) -> Option<(Vec3, usize, Vec<i32>)> {
    let (u, _, _) = face_info('U');
    let (d, _, _) = face_info('D');
    let (f, _, _) = face_info('F');
    let (b, _, _) = face_info('B');
    let (l, _, _) = face_info('L');
    let (r, _, _) = face_info('R');
    Some(match mv {
        'U' => (u, 1, vec![1]),
        'D' => (d, 1, vec![-1]),
        'F' => (f, 2, vec![1]),
        'B' => (b, 2, vec![-1]),
        'L' => (l, 0, vec![-1]),
        'R' => (r, 0, vec![1]),
        'M' => (l, 0, vec![0]),
        'E' => (d, 1, vec![0]),
        'S' => (f, 2, vec![0]),
        'u' => (u, 1, vec![0, 1]),
        'd' => (d, 1, vec![-1, 0]),
        'f' => (f, 2, vec![0, 1]),
        'b' => (b, 2, vec![-1, 0]),
        'l' => (l, 0, vec![-1, 0]),
        'r' => (r, 0, vec![0, 1]),
        'x' => (r, 0, vec![-1, 0, 1]),
        'y' => (u, 1, vec![-1, 0, 1]),
        'z' => (f, 2, vec![-1, 0, 1]),
        _ => return None,
    })
}

fn quad_corners(center: Vec3, right: Vec3, up: Vec3, half: f32) -> [Vec3; 4] {
    [
        center - right * half - up * half,
        center + right * half - up * half,
        center + right * half + up * half,
        center - right * half + up * half,
    ]
}

fn cubie_to_rc(ix: i32, iy: i32, iz: i32, rv: Vec3, uv: Vec3) -> (usize, usize) {
    let pos = vec3(ix as f32, iy as f32, iz as f32);
    let col = (pos.dot(rv)).round() as i32 + 1;
    let row = 1 - (pos.dot(uv)).round() as i32;
    (row.clamp(0, 2) as usize, col.clamp(0, 2) as usize)
}

fn build_cube_mesh(state: &CubeState, anim: Option<(char, f32)>) -> CpuMesh {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut colors: Vec<Srgba> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    let spec = anim.and_then(|(mv, ang)| move_spec(mv).map(|(axis, idx, allowed)| (axis, idx, allowed, ang)));

    let mut push_quad = |corners: [Vec3; 4], normal: Vec3, color: [u8; 3], rot: Option<Mat3>| {
        let c = Srgba::new(color[0], color[1], color[2], 255);
        for &i in &[0usize, 1, 2, 0, 2, 3] {
            let p = match rot { Some(m) => m * corners[i], None => corners[i] };
            let n = match rot { Some(m) => m * normal, None => normal };
            positions.push(p);
            colors.push(c);
            normals.push(n);
        }
    };

    for ix in -1..=1i32 {
        for iy in -1..=1i32 {
            for iz in -1..=1i32 {
                if ix == 0 && iy == 0 && iz == 0 {
                    continue;
                }
                let center = vec3(ix as f32, iy as f32, iz as f32);
                let rot: Option<Mat3> = spec.as_ref().and_then(|(axis, idx, allowed, ang)| {
                    let v = [ix, iy, iz][*idx];
                    if allowed.contains(&v) {
                        Some(Mat3::from_axis_angle(axis.normalize(), Rad(*ang)))
                    } else {
                        None
                    }
                });

                for &face in &FACES {
                    let (n, rv, uv) = face_info(face);
                    let (axis_val, layer_val): (i32, i32) = match face {
                        'U' => (iy, 1), 'D' => (iy, -1),
                        'F' => (iz, 1), 'B' => (iz, -1),
                        'L' => (ix, -1), 'R' => (ix, 1),
                        _ => unreachable!(),
                    };
                    let is_sticker = axis_val == layer_val;
                    if !is_sticker && rot.is_none() {
                        continue;
                    }
                    let panel_center = center + n * (CUBIE_H / 2. + 0.001);
                    let panel = quad_corners(panel_center, rv, uv, CUBIE_H / 2.);
                    push_quad(panel, n, PLASTIC, rot);
                    if is_sticker {
                        let (row, col) = cubie_to_rc(ix, iy, iz, rv, uv);
                        let ck = cube::face_char(state.facelet_at(cube::idx_of(face), row, col) as usize);
                        let sticker_center = center + n * (CUBIE_H / 2. + 0.015);
                        let sticker = quad_corners(sticker_center, rv, uv, STICKER_H / 2.);
                        push_quad(sticker, n, face_color(ck), rot);
                    }
                }
            }
        }
    }

    CpuMesh {
        positions: Positions::F32(positions),
        colors: Some(colors),
        normals: Some(normals),
        ..Default::default()
    }
}

/// 1 nước đang animate.
struct PendingMove {
    mv: char,
    prime: bool,
    target: f32,
    current: f32,
    speed: f32,
}

/// Parse "R", "R'", "R2" thanh 1-2 muc hang doi (mv, prime). R2 tach thanh
/// 2 vong quay 90 do cung chieu (dung vi R2 = R,R = R',R' ve mat trang thai).
fn queue_move(queue: &mut VecDeque<(char, bool)>, mv_str: &str) {
    let base = mv_str.chars().next().unwrap();
    if mv_str.ends_with('2') {
        queue.push_back((base, false));
        queue.push_back((base, false));
    } else if mv_str.ends_with('\'') {
        queue.push_back((base, true));
    } else {
        queue.push_back((base, false));
    }
}

/// Kết quả 1 job nền (chạy trên thread riêng, gửi về qua channel).
enum JobResult {
    Solve(Option<Vec<String>>),
    Hint(rubik_core::hint::Hint),
}

fn main() {
    use winit::event::{Event as WinitEvent, WindowEvent as WinitWindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::{Fullscreen, WindowBuilder};

    let event_loop = EventLoop::new();
    let winit_window = WindowBuilder::new()
        .with_title("Rubik's Cube (Rust/three-d)")
        .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 800.0))
        .with_min_inner_size(winit::dpi::LogicalSize::new(500.0, 400.0))
        .with_resizable(true)
        .build(&event_loop)
        .expect("Khong tao duoc cua so -- kiem tra driver GPU/man hinh");

    // Dung API cap thap (WindowedContext + FrameInputGenerator) thay vi
    // Window::render_loop() tien loi -- vi Window cua three-d KHONG expose
    // winit window goc sau khi tao (private field, khong co getter), nen
    // KHONG THE goi set_fullscreen() cho F11 neu dung API cao cap. Doi lai:
    // tu giu winit_window de co toan quyen (F11, resize deu tu tay xu ly,
    // giong dung logic ben trong render_loop() cua three-d -- xem
    // winit_window.rs ham render_loop() ma toi doi chieu khi viet lai).
    let surface_settings = SurfaceSettings::default();
    let gl = WindowedContext::from_winit_window(&winit_window, surface_settings)
        .expect("Khong tao duoc GL context");
    let context: Context = (*gl).clone();

    let mut camera = Camera::new_perspective(
        Viewport::new_at_origo(1100, 800),
        vec3(4.5, 4.0, 5.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(35.0),
        0.1,
        100.0,
    );
    let mut control = OrbitControl::new(vec3(0.0, 0.0, 0.0), 4.0, 15.0);
    let mut gui = three_d::GUI::new(&context);
    {
        // Nạp font tiếng Việt + bảng màu. GUI::new() đã tạo sẵn context egui
        // nên chỉ cần lấy ra và cấu hình 1 lần ở đây.
        let ctx = gui.context();
        theme::install_fonts(&ctx);
        theme::install_style(&ctx);
    }

    let mut state = CubeState::solved();
    let mut pending: Option<PendingMove> = None;
    let mut queue: VecDeque<(char, bool)> = VecDeque::new();
    let mut status = "Sẵn sàng".to_string();
    let mut formula_text = String::new();
    let mut hint_label: Option<String> = None;
    let mut hint_moves: Vec<String> = Vec::new();
    let mut undo_stack: Vec<CubeState> = Vec::new();
    const UNDO_MAX: usize = 120;
    let mut is_fullscreen = false;

    // ── Job nen (Enter=giai / H=goi y) chay tren thread rieng, khong chan
    // khung hinh -- ket qua nhan qua channel, kiem tra khong-blocking moi
    // frame (try_recv). job_rx.is_some() <=> dang co job chay (giong
    // cfop_busy ben Python: chi 1 job tai 1 thoi diem).
    let mut job_rx: Option<std::sync::mpsc::Receiver<JobResult>> = None;
    let mut move_count: u32 = 0;
    let mut last_hint_stage: Option<&'static str> = None;
    let mut last_hint_move_count: Option<u32> = None;
    let mut last_hint_failed: bool = false;
    let mut retry_counter: u64 = 0;
    let mut anim_speed: f32 = 10.0;
    let mut sel_face: Option<char> = None;
    let mut total_queued: usize = 0;

    // Luu 1 snapshot state HIEN TAI (truoc khi thay doi) vao undo_stack,
    // kem gioi han do sau -- dung dung 1 lan cho MOI "hanh dong" (giong
    // push_undo() trong main.py Python: 1 nuoc tay = 1 lan, ca cong thuc/
    // AI giai/goi y = 1 lan cho CA khoi, khong phai tung nuoc con trong do).
    macro_rules! push_undo {
        () => {
            undo_stack.push(state);
            if undo_stack.len() > UNDO_MAX {
                undo_stack.remove(0);
            }
        };
    }

    rubik_core::cross_solver::get_pdb();
    rubik_core::f2l_solver::warm_pair_pdbs();
    rubik_core::oll_algorithms::table();
    rubik_core::pll_algorithms::table();

    let ambient = AmbientLight::new(&context, 0.5, Srgba::WHITE);
    let sun = DirectionalLight::new(&context, 1.2, Srgba::WHITE, &vec3(-0.45, -0.85, -0.65));

    let mut frame_input_generator = FrameInputGenerator::from_winit_window(&winit_window);

    event_loop.run(move |event, _, control_flow| match event {
        WinitEvent::MainEventsCleared => {
            winit_window.request_redraw();
        }
        WinitEvent::RedrawRequested(_) => {
            let mut frame_input = frame_input_generator.generate(&context);

            // ── Tỉ lệ giao diện theo kích thước cửa sổ ──────────────
            // gui.update() nhận device_pixel_ratio và dùng nó làm
            // pixels_per_point cho egui, nên nhân thêm `ui_scale` là cách
            // phóng to/thu nhỏ TOÀN BỘ giao diện (chữ, nút, khoảng cách)
            // theo cỡ màn hình. Mốc chuẩn: cao 800px = tỉ lệ 1.0.
            let dpr = frame_input.device_pixel_ratio;
            let win_h_logical = frame_input.viewport.height as f32 / dpr;
            let win_w_logical = frame_input.viewport.width as f32 / dpr;
            let ui_scale = (win_h_logical / 800.0).clamp(0.80, 1.60);
            let gui_ppp = dpr * ui_scale;

            // Panel rộng theo tỉ lệ cửa sổ, có chặn trên/dưới để không quá
            // hẹp (vỡ bố cục) hay quá rộng (lấn hết chỗ của khối 3D).
            let left_w = (win_w_logical / ui_scale * 0.30).clamp(300.0, 430.0);
            let mut submit_formula = false;
            let mut editing_formula = false;
            let mut request_hint = false;
            let mut apply_hint = false;
            let mut request_cancel = false;
            let mut request_solve = false;
            let mut request_scramble = false;
            let mut request_undo = false;
            let is_busy = job_rx.is_some();
            let is_animating = pending.is_some();
            let is_solved_now = state.is_solved();
            gui.update(
                &mut frame_input.events,
                frame_input.accumulated_time,
                frame_input.viewport,
                gui_ppp,
                |gui_context| {
                    egui::SidePanel::left("faces_panel")
                        .exact_width(left_w)
                        .resizable(false)
                        .frame(
                            egui::Frame::none()
                                .fill(theme::BG_PANEL)
                                .inner_margin(egui::Margin::symmetric(13.0, 12.0)),
                        )
                        .show(gui_context, |ui| {
                            // Cuộn được khi cửa sổ thấp / nội dung dài hơn
                            // chiều cao panel (trước đây bị cắt mất phần dưới).
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                            // ── Tiêu đề ứng dụng ────────────────────────
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("RUBIK")
                                        .size(21.0)
                                        .strong()
                                        .color(theme::ACCENT),
                                );
                                ui.label(
                                    egui::RichText::new("CFOP Solver")
                                        .size(11.5)
                                        .color(theme::TEXT_DIM),
                                );
                            });
                            ui.add_space(11.0);

                            // ── Sơ đồ 6 mặt ─────────────────────────────
                            theme::card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    theme::section_label(ui, "Sơ đồ 6 mặt");
                                    if let Some(f) = sel_face {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("● mặt {f}"))
                                                        .size(11.0)
                                                        .color(theme::ACCENT),
                                                );
                                            },
                                        );
                                    }
                                });
                                ui.add_space(7.0);
                                draw_six_face_net(ui, &state, sel_face);
                            });
                            ui.add_space(10.0);

                            // ── Hành động chính ─────────────────────────
                            theme::card(ui, |ui| {
                                theme::section_label(ui, "Điều khiển");
                                ui.add_space(7.0);
                                ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                    if theme::primary_button(ui, "▶  Giải tự động", "Phím tắt: Enter").clicked() {
                                        request_solve = true;
                                    }
                                });
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 8.0) / 2.0;
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        if ui
                                            .add_sized([w, 30.0], egui::Button::new("🔀  Xáo"))
                                            .on_hover_text("Phím tắt: Space")
                                            .clicked()
                                        {
                                            request_scramble = true;
                                        }
                                    });
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        if ui
                                            .add_sized([w, 30.0], egui::Button::new("💡  Gợi ý"))
                                            .on_hover_text("Gợi ý 1 bước tiếp theo — Phím tắt: H")
                                            .clicked()
                                        {
                                            request_hint = true;
                                        }
                                    });
                                });
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 8.0) / 2.0;
                                    ui.add_enabled_ui(!undo_stack.is_empty(), |ui| {
                                        if ui
                                            .add_sized(
                                                [w, 28.0],
                                                egui::Button::new(format!("↶  Hoàn tác ({})", undo_stack.len())),
                                            )
                                            .on_hover_text("Phím tắt: Ctrl+Z")
                                            .clicked()
                                        {
                                            request_undo = true;
                                        }
                                    });
                                    ui.add_enabled_ui(is_busy, |ui| {
                                        if ui
                                            .add_sized([w, 28.0], egui::Button::new("✖  Huỷ"))
                                            .on_hover_text("Dừng việc tính toán — Phím tắt: Esc")
                                            .clicked()
                                        {
                                            request_cancel = true;
                                        }
                                    });
                                });
                            });
                            ui.add_space(10.0);

                            // ── Nhập công thức ──────────────────────────
                            theme::card(ui, |ui| {
                                theme::section_label(ui, "Công thức Singmaster");
                                ui.add_space(6.0);
                                let resp = ui.add_sized(
                                    [ui.available_width(), 26.0],
                                    egui::TextEdit::singleline(&mut formula_text)
                                        .hint_text("VD:  R U R' U'   hoặc   (R U)3"),
                                );
                                editing_formula = resp.has_focus() || resp.lost_focus();
                                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    submit_formula = true;
                                }
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 8.0) / 2.0;
                                    if ui.add_sized([w, 26.0], egui::Button::new("Áp dụng")).clicked() {
                                        submit_formula = true;
                                    }
                                    if ui.add_sized([w, 26.0], egui::Button::new("Xoá")).clicked() {
                                        formula_text.clear();
                                    }
                                });
                            });

                            // ── Kết quả gợi ý ───────────────────────────
                            if let Some(lbl) = &hint_label {
                                ui.add_space(10.0);
                                theme::card(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("💡").size(13.0));
                                        ui.label(
                                            egui::RichText::new(lbl)
                                                .size(13.0)
                                                .strong()
                                                .color(theme::INFO),
                                        );
                                    });
                                    if !hint_moves.is_empty() {
                                        ui.add_space(5.0);
                                        ui.label(
                                            egui::RichText::new(hint_moves.join("  "))
                                                .monospace()
                                                .size(13.5)
                                                .color(theme::ACCENT),
                                        );
                                        ui.add_space(6.0);
                                        ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                            if theme::wide_button(ui, "Thực hiện gợi ý này", "").clicked() {
                                                apply_hint = true;
                                            }
                                        });
                                    }
                                });
                            }

                            // ── Trạng thái (luôn ở cuối) ────────────────
                            ui.add_space(10.0);
                            theme::card(ui, |ui| {
                                ui.set_max_width(ui.available_width());
                                ui.horizontal_wrapped(|ui| {
                                    if is_busy {
                                        ui.spinner();
                                    }
                                    let color = if is_busy {
                                        theme::ACCENT
                                    } else if status.contains("Không") || status.contains("Lỗi") {
                                        theme::DANGER
                                    } else if is_solved_now {
                                        theme::SUCCESS
                                    } else {
                                        theme::TEXT_DIM
                                    };
                                    ui.label(egui::RichText::new(&status).size(12.0).color(color));
                                });
                                if !queue.is_empty() || is_animating {
                                    ui.add_space(5.0);
                                    let done = total_queued.saturating_sub(queue.len());
                                    let frac = if total_queued > 0 {
                                        done as f32 / total_queued as f32
                                    } else {
                                        0.0
                                    };
                                    ui.add(
                                        egui::ProgressBar::new(frac)
                                            .desired_height(16.0)
                                            .fill(theme::ACCENT)
                                            .text(
                                                egui::RichText::new(format!("{done}/{total_queued} nước"))
                                                    .size(10.0),
                                            ),
                                    );
                                }
                                if is_solved_now && !is_animating && queue.is_empty() {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new("✔  Khối đã hoàn thành")
                                            .size(12.5)
                                            .strong()
                                            .color(theme::SUCCESS),
                                    );
                                }
                            });

                            // ── Phím tắt (thu gọn được) ─────────────────
                            ui.add_space(10.0);
                            egui::CollapsingHeader::new(
                                egui::RichText::new("Phím tắt & hướng dẫn").size(12.5),
                            )
                            .default_open(false)
                            .show(ui, |ui| {
                                let row = |ui: &mut egui::Ui, k: &str, d: &str| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(k)
                                                .monospace()
                                                .size(11.5)
                                                .color(theme::ACCENT),
                                        );
                                        ui.label(
                                            egui::RichText::new(d).size(11.5).color(theme::TEXT_DIM),
                                        );
                                    });
                                };
                                row(ui, "U D F B L R", "xoay lớp tương ứng");
                                row(ui, "Shift + phím", "xoay ngược lại (U', R'…)");
                                row(ui, "Space", "xáo ngẫu nhiên 25 nước");
                                row(ui, "Enter", "AI tự giải");
                                row(ui, "H", "gợi ý 1 bước");
                                row(ui, "Esc", "huỷ khi đang tính");
                                row(ui, "Ctrl+Z", "hoàn tác");
                                row(ui, "F11", "toàn màn hình");
                                row(ui, "[  ]", &format!("tốc độ quay (hiện tại {:.0})", anim_speed));
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Kéo chuột để xoay góc nhìn · cuộn để phóng to\n\
                                         Bấm vào một ô màu rồi dùng phím mũi tên để xoay mặt đó",
                                    )
                                    .size(11.0)
                                    .color(theme::TEXT_DIM),
                                );
                            });
                            }); // het ScrollArea
                        });
                },
            );

            if request_cancel {
                rubik_core::cancel::request();
                status = "Đang huỷ…".to_string();
            }
            if request_undo && !undo_stack.is_empty() {
                if let Some(prev) = undo_stack.pop() {
                    pending = None;
                    queue.clear();
                    state = prev;
                    move_count = move_count.saturating_sub(1);
                    status = "Đã hoàn tác".to_string();
                }
            }
            if request_scramble && !is_busy && !is_animating && queue.is_empty() {
                push_undo!();
                let mut rng = rand::thread_rng();
                let mut tmp = state;
                let mvs = cube::random_scramble_moves(&mut tmp, 25, &mut rng);
                queue.clear();
                for m in mvs {
                    queue_move(&mut queue, m);
                }
                total_queued = queue.len();
                status = "Đang xáo…".to_string();
            }
            if request_solve && !is_busy && !is_animating && queue.is_empty() {
                rubik_core::cancel::clear();
                let state_copy = state;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let res = rubik_core::solve::solve_full_cube(&state_copy, 2);
                    let _ = tx.send(JobResult::Solve(res));
                });
                job_rx = Some(rx);
                status = "Đang tính lời giải…".to_string();
            }

            if request_hint && job_rx.is_none() && queue.is_empty() && pending.is_none() {
                rubik_core::cancel::clear();
                let cur_stage = rubik_core::hint::stage_of(&state);
                let retry = last_hint_failed
                    && last_hint_stage == Some(cur_stage)
                    && last_hint_move_count == Some(move_count);
                let retry_seed = if retry {
                    retry_counter += 1;
                    Some(retry_counter)
                } else {
                    None
                };
                let state_copy = state;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let h = rubik_core::hint::compute_hint(&state_copy, retry_seed);
                    let _ = tx.send(JobResult::Hint(h));
                });
                job_rx = Some(rx);
                status = if retry {
                    "Đang tính gợi ý (thử hướng khác)…".to_string()
                } else {
                    "Đang tính gợi ý…".to_string()
                };
            }
            if apply_hint && !hint_moves.is_empty() {
                push_undo!();
                queue.clear();
                for m in &hint_moves {
                    queue_move(&mut queue, m);
                }
                status = "Đang thực hiện gợi ý…".to_string();
            }

            if submit_formula {
                match cube::parse_singmaster(&formula_text) {
                    Ok(mvs) if !mvs.is_empty() => {
                        push_undo!();
                        queue.clear();
                        for m in &mvs {
                            queue_move(&mut queue, m);
                        }
                        status = format!("Đã nạp {} nước từ công thức", mvs.len());
                    }
                    Ok(_) => {
                        status = "Công thức trống".to_string();
                    }
                    Err(e) => {
                        status = format!("Lỗi công thức: {e}");
                    }
                }
                formula_text.clear();
            }

            // 3D chi chiem phan con lai ben phai panel.
            let full = frame_input.viewport;
            // Panel được vẽ ở tỉ lệ gui_ppp, nên quy đổi bề rộng của nó
            // sang pixel vật lý PHẢI dùng cùng tỉ lệ đó -- nếu dùng dpr
            // thô, vùng 3D sẽ lệch khỏi mép panel.
            let left_px = (left_w * gui_ppp) as i32;
            let cam_viewport = Viewport {
                x: full.x + left_px,
                y: full.y,
                width: (full.width as i32 - left_px).max(1) as u32,
                height: full.height,
            };
            camera.set_viewport(cam_viewport);

            // ── Click chon mat (chi khi cube dang ranh -- khong animate) --
            // dung three_d::pick() (GPU raycast co san) truoc khi
            // OrbitControl xu ly, danh dau handled=true neu trung mat de
            // KHONG bi hieu nham thanh keo xoay camera (giong Python:
            // click trung sticker = chon mat, khong trung = keo camera).
            if pending.is_none() && !editing_formula {
                let pick_cpu_mesh = build_cube_mesh(&state, None);
                let pick_mesh = Mesh::new(&context, &pick_cpu_mesh);
                let pick_model = Gm::new(pick_mesh, ColorMaterial::default());
                for event in frame_input.events.iter_mut() {
                    if let Event::MousePress { button: MouseButton::Left, position, handled, .. } = event {
                        if *handled {
                            continue;
                        }
                        if position.x >= left_px as f32 {
                            if let Some(face) = pick_face(&context, &camera, &pick_model, *position) {
                                sel_face = Some(face);
                                *handled = true;
                            }
                        }
                    }
                }
            }

            control.handle_events(&mut camera, &mut frame_input.events);

            for event in &frame_input.events {
                if editing_formula {
                    break;
                }
                if let Event::KeyPress { kind, modifiers, handled, .. } = event {
                    if *handled {
                        continue;
                    }
                    match kind {
                        Key::U | Key::D | Key::F | Key::B | Key::L | Key::R => {
                            let base = match kind {
                                Key::U => 'U', Key::D => 'D', Key::F => 'F',
                                Key::B => 'B', Key::L => 'L', Key::R => 'R',
                                _ => unreachable!(),
                            };
                            push_undo!();
                            queue.push_back((base, modifiers.shift));
                        }
                        Key::Space => {
                            push_undo!();
                            let mut rng = rand::thread_rng();
                            let mut tmp = state;
                            let mvs = cube::random_scramble_moves(&mut tmp, 25, &mut rng);
                            queue.clear();
                            for m in mvs {
                                queue_move(&mut queue, m);
                            }
                            status = "Đang xáo…".to_string();
                        }
                        Key::Enter => {
                            if job_rx.is_none() && queue.is_empty() && pending.is_none() {
                                rubik_core::cancel::clear();
                                let state_copy = state;
                                let (tx, rx) = std::sync::mpsc::channel();
                                std::thread::spawn(move || {
                                    let res = rubik_core::solve::solve_full_cube(&state_copy, 2);
                                    let _ = tx.send(JobResult::Solve(res));
                                });
                                job_rx = Some(rx);
                                status = "Đang tính lời giải…".to_string();
                            } else if job_rx.is_none() {
                                status = "Đợi khối xoay xong đã".to_string();
                            }
                        }
                        Key::H => {
                            if job_rx.is_none() && queue.is_empty() && pending.is_none() {
                                rubik_core::cancel::clear();
                                let cur_stage = rubik_core::hint::stage_of(&state);
                                let retry = last_hint_failed
                                    && last_hint_stage == Some(cur_stage)
                                    && last_hint_move_count == Some(move_count);
                                let retry_seed = if retry {
                                    retry_counter += 1;
                                    Some(retry_counter)
                                } else {
                                    None
                                };
                                let state_copy = state;
                                let (tx, rx) = std::sync::mpsc::channel();
                                std::thread::spawn(move || {
                                    let h = rubik_core::hint::compute_hint(&state_copy, retry_seed);
                                    let _ = tx.send(JobResult::Hint(h));
                                });
                                job_rx = Some(rx);
                                status = if retry {
                                    "Đang tính gợi ý (thử hướng khác)…".to_string()
                                } else {
                                    "Đang tính gợi ý…".to_string()
                                };
                            } else if job_rx.is_none() {
                                status = "Đợi khối xoay xong đã".to_string();
                            }
                        }
                        Key::Escape => {
                            if job_rx.is_some() {
                                rubik_core::cancel::request();
                                status = "Đang huỷ…".to_string();
                            }
                        }
                        Key::Z if modifiers.command || modifiers.ctrl => {
                            if let Some(prev) = undo_stack.pop() {
                                pending = None;
                                queue.clear();
                                state = prev;
                                move_count = move_count.saturating_sub(1);
                                status = "Đã hoàn tác".to_string();
                            } else {
                                status = "Không có gì để hoàn tác".to_string();
                            }
                        }
                        // Mui ten: xoay mat DANG CHON (click sticker de chon
                        // truoc). Up/Right = chieu thuan, Down/Left = nghich
                        // dao -- giong K_UP/K_RIGHT va K_DOWN/K_LEFT trong
                        // main.py.
                        Key::ArrowUp | Key::ArrowRight => {
                            if let Some(face) = sel_face {
                                push_undo!();
                                queue.push_back((face, false));
                            }
                        }
                        Key::ArrowDown | Key::ArrowLeft => {
                            if let Some(face) = sel_face {
                                push_undo!();
                                queue.push_back((face, true));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // ── Nhan ket qua job nen (khong chan khung hinh) ────────────
            if let Some(rx) = &job_rx {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        JobResult::Solve(Some(mvs)) if !mvs.is_empty() => {
                            push_undo!();
                            queue.clear();
                            for m in &mvs {
                                queue_move(&mut queue, m);
                            }
                            status = format!("Lời giải: {} nước", mvs.len());
                        }
                        JobResult::Solve(Some(_)) => {
                            status = "Khối đã ở trạng thái hoàn thành".to_string();
                        }
                        JobResult::Solve(None) => {
                            status = if rubik_core::cancel::is_cancelled() {
                                "Đã huỷ".to_string()
                            } else {
                                "Không tìm được lời giải — thử xáo lại".to_string()
                            };
                        }
                        JobResult::Hint(h) => {
                            last_hint_stage = Some(h.stage);
                            last_hint_move_count = Some(move_count);
                            last_hint_failed = h.moves.is_empty() && h.stage != "done";
                            status = if h.moves.is_empty() && h.label.contains("giai xong") {
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
                            hint_label = Some(h.label);
                            hint_moves = h.moves;
                        }
                    }
                    job_rx = None;
                }
            }

            if queue.len() > total_queued {
                total_queued = queue.len();
            }
            if pending.is_none() {
                if let Some((mv, prime)) = queue.pop_front() {
                    let target = if prime { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 };
                    pending = Some(PendingMove { mv, prime, target, current: 0.0, speed: anim_speed });
                }
            }

            let mut anim = pending.as_mut().map(|p| {
                let dt = (frame_input.elapsed_time / 1000.0) as f32;
                p.current += p.speed * dt * p.target.signum();
                if p.current.abs() >= p.target.abs() {
                    p.current = p.target;
                }
                (p.mv, p.current)
            });
            if let Some(p) = &pending {
                if p.current == p.target {
                    let mv_str = if p.prime { format!("{}'", p.mv) } else { p.mv.to_string() };
                    state.do_move(&mv_str);
                    move_count += 1;
                    pending = None;
                    // QUAN TRONG: phai XOA anim o day. `state` vua duoc cap
                    // nhat da BAO GOM nuoc di nay roi, nen neu van giu anim
                    // (dang o goc 90 do day du) thi build_cube_mesh se xoay
                    // THEM 90 do nua len trang thai moi -> lop do bi xoay du
                    // trong dung 1 khung hinh, gay hien tuong NHAY/GIAT o
                    // moi nuoc khi may tu xoay. Da tim ra va sua.
                    anim = None;
                    if queue.is_empty() {
                        total_queued = 0;
                        if status.starts_with("Đang xáo") {
                            status = "Đã xáo xong".to_string();
                        }
                    }
                }
            }

            let cpu_mesh = build_cube_mesh(&state, anim);
            let mesh = Mesh::new(&context, &cpu_mesh);
            let cube_model = Gm::new(mesh, ColorMaterial::default());

            frame_input.screen()
                .clear(ClearState::color_and_depth(0.11, 0.11, 0.16, 1.0, 1.0))
                .render(&camera, &[&cube_model], &[&ambient, &sun])
                .write(|| gui.render())
                .unwrap();

            gl.swap_buffers().unwrap();
            *control_flow = ControlFlow::Poll;
            winit_window.request_redraw();
        }
        WinitEvent::WindowEvent { ref event, .. } => {
            frame_input_generator.handle_winit_window_event(event);
            match event {
                WinitWindowEvent::Resized(physical_size) => {
                    gl.resize(*physical_size);
                }
                WinitWindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    gl.resize(**new_inner_size);
                }
                WinitWindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                // F11: toggle fullscreen thuc su. three_d::Key (API rut gon)
                // KHONG co bien the cho phim F -- phai bat truc tiep tu
                // KeyboardInput winit goc. Day cung la ly do chinh phai tu
                // quan ly vong lap thay vi dung Window::render_loop() co
                // san (khong expose winit window de goi set_fullscreen()).
                WinitWindowEvent::KeyboardInput { input, .. } => {
                    if input.state == winit::event::ElementState::Pressed {
                        match input.virtual_keycode {
                            Some(winit::event::VirtualKeyCode::F11) => {
                                is_fullscreen = !is_fullscreen;
                                winit_window.set_fullscreen(if is_fullscreen {
                                    Some(Fullscreen::Borderless(None))
                                } else {
                                    None
                                });
                            }
                            // Toc do animation: [ giam, ] tang. Ba muc gioi
                            // han 2.0-30.0 rad/s (~1.6s -- ~0.1s cho 1 phan
                            // tu 90 do) de tranh qua cham/qua nhanh khong
                            // xem duoc.
                            Some(winit::event::VirtualKeyCode::LBracket) => {
                                anim_speed = (anim_speed - 2.0).max(2.0);
                            }
                            Some(winit::event::VirtualKeyCode::RBracket) => {
                                anim_speed = (anim_speed + 2.0).min(30.0);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    });
}

/// Ve khung 6 mat (bo cuc chu thap: U tren, L F R B mot hang, D duoi) --
/// dung egui::Painter ve hinh vuong mau truc tiep, khong can 3D. Doc
/// facelet TRUC TIEP tu CubeState that (cube.rs) qua facelet_at()/idx_of().
/// Bắn tia từ vị trí chuột (pixel vật lý) qua camera, tìm điểm chạm gần
/// nhất trên `model` (dùng three_d::pick() -- raycasting GPU có sẵn,
/// không cần tự viết ray-plane), rồi suy ra mặt nào trong 6 mặt lớn (U D
/// F B L R) chứa điểm đó: mặt có dot(diem_cham, normal_mat) LỚN NHẤT
/// (điểm luôn nằm trên đúng 1 trong 6 mặt ngoài vì mesh tĩnh dùng để
/// pick chỉ vẽ panel/sticker mặt ngoài, xem build_cube_mesh).
fn pick_face(
    context: &Context,
    camera: &Camera,
    model: &Gm<Mesh, ColorMaterial>,
    pixel: PhysicalPoint,
) -> Option<char> {
    let hit = three_d::pick(context, camera, pixel, std::iter::once(model))?;
    let mut best_face = 'U';
    let mut best_dot = f32::NEG_INFINITY;
    for &face in &FACES {
        let (n, _, _) = face_info(face);
        let d = hit.dot(n);
        if d > best_dot {
            best_dot = d;
            best_face = face;
        }
    }
    Some(best_face)
}

fn draw_six_face_net(ui: &mut egui::Ui, state: &CubeState, sel_face: Option<char>) {
    // Ô co giãn theo bề rộng thực tế còn lại: sơ đồ gồm 4 cột mặt, mỗi
    // cột = 3 ô + 2 khe + lề. Giải ngược ra cỡ ô để sơ đồ luôn vừa khít
    // panel dù cửa sổ to nhỏ thế nào (trước đây cố định 21px nên panel
    // rộng thì thừa chỗ, panel hẹp thì tràn).
    let avail = ui.available_width();
    let gap = 2.5;
    let pad = 7.0;
    let cell = (((avail / 4.0) - pad - 2.0 * gap) / 3.0).clamp(13.0, 30.0);
    let panel = cell * 3.0 + gap * 2.0;
    let grid_pos: [(char, i32, i32); 6] = [
        ('U', 1, 0),
        ('L', 0, 1), ('F', 1, 1), ('R', 2, 1), ('B', 3, 1),
        ('D', 1, 2),
    ];
    let origin = ui.cursor().min;
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(4.0 * (panel + pad), 3.0 * (panel + pad)),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    for (face, gc, gr) in grid_pos {
        let fx = origin.x + gc as f32 * (panel + pad);
        let fy = origin.y + gr as f32 * (panel + pad);
        let selected = Some(face) == sel_face;

        // Nền mờ phía sau mỗi mặt cho tách bạch khỏi nền panel
        let panel_rect = egui::Rect::from_min_size(
            egui::pos2(fx - 3.0, fy - 3.0),
            egui::vec2(panel + 6.0, panel + 6.0),
        );
        painter.rect_filled(
            panel_rect,
            egui::Rounding::same(5.0),
            egui::Color32::from_rgb(16, 17, 25),
        );

        for r in 0..3usize {
            for c in 0..3usize {
                let ck = cube::face_char(state.facelet_at(cube::idx_of(face), r, c) as usize);
                let [cr, cg, cb] = face_color(ck);
                let x = fx + c as f32 * (cell + gap);
                let y = fy + r as f32 * (cell + gap);
                let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell, cell));
                painter.rect_filled(
                    cell_rect,
                    egui::Rounding::same((cell * 0.17).clamp(2.0, 6.0)),
                    egui::Color32::from_rgb(cr, cg, cb),
                );
            }
        }

        // Chữ tên mặt ở ô giữa (ô tâm luôn giữ nguyên màu mặt đó)
        let center = egui::pos2(fx + panel / 2.0, fy + panel / 2.0);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            face,
            egui::FontId::proportional((cell * 0.52).clamp(8.0, 15.0)),
            egui::Color32::from_black_alpha(150),
        );

        if selected {
            painter.rect_stroke(
                panel_rect,
                egui::Rounding::same(5.0),
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        }
    }
}

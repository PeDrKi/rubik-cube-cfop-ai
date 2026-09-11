//! rubik-app — ứng dụng Rubik's Cube 3D tương tác.
//!
//! File này chỉ chứa phần GIAO DIỆN: dựng hình học cube cho GPU, vòng lặp
//! winit, camera, panel egui (khung 6 mặt + thanh nhập công thức), và xử
//! lý phím/chuột. TOÀN BỘ thuật toán giải nằm trong crate `rubik-core`.
//!
//! Xem README.md ở thư mục gốc workspace để biết cách chạy.


mod theme;
mod camera_capture;
mod scan_ui;
mod paint_ui;
mod history;

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
    /// Nước đúp: xoay liền 180° trong 1 lần, áp 2 lần khi xong.
    double: bool,
    target: f32,
    current: f32,
    speed: f32,
}

/// Parse "R", "R'", "R2" thanh 1-2 muc hang doi (mv, prime). R2 tach thanh
/// 2 vong quay 90 do cung chieu (dung vi R2 = R,R = R',R' ve mat trang thai).
fn queue_move(queue: &mut VecDeque<(char, bool, bool)>, mv_str: &str) {
    let base = mv_str.chars().next().unwrap();
    // Nước đúp (R2) đưa vào hàng đợi thành MỘT mục xoay 180°, không tách
    // thành 2 lần 90° -- nếu tách, số mục trong hàng đợi sẽ nhiều hơn số
    // nước của lời giải, khiến thanh tiến trình đếm lệch (VD lời giải 67
    // nước nhưng hiện "31/78"). Xoay liền 180° cũng đúng cách người chơi
    // thật thực hiện.
    if mv_str.ends_with('2') {
        queue.push_back((base, false, true));
    } else if mv_str.ends_with('\'') {
        queue.push_back((base, true, false));
    } else {
        queue.push_back((base, false, false));
    }
}

/// Kết quả 1 job nền (chạy trên thread riêng, gửi về qua channel).
enum JobResult {
    Solve(Option<Vec<String>>),
    Hint(rubik_core::hint::Hint),
    /// Kèm theo CHÍNH trạng thái đã dùng để tính, để biết bảng còn khớp
    /// với khối hiện tại hay không (xem `formula_table`).
    Breakdown(CubeState, rubik_core::breakdown::Breakdown),
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
    let mut queue: VecDeque<(char, bool, bool)> = VecDeque::new();
    let mut status = "Sẵn sàng".to_string();
    let mut formula_text = String::new();
    let mut hint_label: Option<String> = None;
    let mut hint_moves: Vec<String> = Vec::new();
    let mut undo_stack: Vec<CubeState> = Vec::new();
    const UNDO_MAX: usize = 120;
    let mut is_fullscreen = false;
    let mut scan_state = scan_ui::ScanState::default();
    let mut paint_state = paint_ui::PaintState::default();

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
    // Cờ cho MainEventsCleared biết có cần vẽ khung mới hay không.
    let mut needs_redraw = true;
    let mut sel_face: Option<char> = None;
    let mut total_queued: usize = 0;
    // Bảng công thức (phím T): None = chưa tính lần nào.
    //
    // Nhớ kèm TRẠNG THÁI đã dùng để tính. Trước đây chỉ giữ mỗi bảng, nên
    // sau lần tính đầu tiên, bấm "Bảng công thức" chỉ bật/tắt cửa sổ và
    // KHÔNG tính lại — khối đã xoay mà bảng vẫn là bảng cũ. Có trạng thái
    // đi kèm thì so sánh được: khác thì tính lại, giống thì chỉ mở ra.
    // Việc này cũng làm cho nhật ký không đếm trùng một ván nhiều lần.
    let mut formula_table: Option<(CubeState, rubik_core::breakdown::Breakdown)> = None;
    let mut formula_open = false;
    // Cửa sổ thống kê OLL/PLL (Ctrl+Y). `stats_counts` được đọc lại từ file
    // mỗi lần mở, để thấy ngay số vừa ghi.
    let mut stats_open = false;
    let mut stats_counts: std::collections::HashMap<(String, String), u32> =
        std::collections::HashMap::new();
    let mut stats_tab_pll = false;
    // Các ca ĐÃ GHI nhật ký cho ván hiện tại, dạng (loại, tên ca).
    //
    // Vì sao cần: bấm H nhiều lần ở cùng một thế OLL sẽ trả về cùng một
    // gợi ý, và bấm Ctrl+T rồi bấm H cũng ra cùng ca đó. Không chặn thì
    // giữ phím H một lúc là số đếm nhảy vọt, thống kê thành vô nghĩa.
    //
    // Mỗi ca chỉ tính MỘT LẦN cho mỗi ván. Trong một ván vẫn ghi được
    // nhiều ca khác nhau (qua OLL rồi tới PLL là hai ca). Danh sách được
    // xoá khi bắt đầu ván mới — xáo, hoặc nạp trạng thái mới từ camera.
    let mut case_log = history::CaseLog::default();
    // Bảng tra công thức chuẩn (57 OLL + 21 PLL) — phím V.
    let mut library_open = false;
    let mut shortcuts_open = false;
    let mut library_query = String::new();
    let mut library_tab_pll = false;
    let mut auto_apply_hint = false;

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
            // CHỈ yêu cầu vẽ lại khi thực sự cần. Trước đây gọi vô điều
            // kiện ở đây nên vòng lặp chạy hết tốc lực (~97% CPU) dù đặt
            // ControlFlow::WaitUntil ở chỗ khác -- đo thực tế mới lộ ra.
            if needs_redraw {
                winit_window.request_redraw();
            }
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
            let mut request_solve = false;
            let mut request_scramble = false;
            let mut close_formula = false;
            let mut request_table = false;
            let mut request_stats = false;
            let mut close_stats = false;
            let mut close_library = false;
            let mut load_into_bar: Option<String> = None;
            let mut scanned_state: Option<CubeState> = None;
            let mut state_source_label: &str = "camera";
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

                            // ── Đầu vào ──────────────────────────────────
                            theme::card(ui, |ui| {
                                theme::section_label(ui, "Đầu vào");
                                ui.add_space(7.0);
                                ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                    if ui
                                        .add_sized([ui.available_width(), 30.0], egui::Button::new("🎨  Điền màu thủ công"))
                                        .on_hover_text("Tự chọn màu từng ô để mô phỏng trạng thái cube")
                                        .clicked()
                                    {
                                        paint_state.open = true;
                                    }
                                });
                            });
                            ui.add_space(10.0);

                            // ── Hành động chính ─────────────────────────
                            theme::card(ui, |ui| {
                                theme::section_label(ui, "Điều khiển");
                                ui.add_space(7.0);
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 8.0) / 2.0;
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        if ui
                                            .add_sized([w, 32.0], egui::Button::new("🔀  Xáo"))
                                            .on_hover_text("Phím tắt: Space")
                                            .clicked()
                                        {
                                            request_scramble = true;
                                        }
                                    });
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        let text = egui::RichText::new("▶  Giải tự động")
                                            .strong()
                                            .color(egui::Color32::from_rgb(28, 20, 6));
                                        if ui
                                            .add_sized(
                                                [w, 32.0],
                                                egui::Button::new(text)
                                                    .fill(theme::ACCENT)
                                                    .rounding(egui::Rounding::same(8.0)),
                                            )
                                            .on_hover_text("Phím tắt: Enter")
                                            .clicked()
                                        {
                                            request_solve = true;
                                        }
                                    });
                                });
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 8.0) / 2.0;
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        if ui
                                            .add_sized([w, 30.0], egui::Button::new("💡  Gợi ý"))
                                            .on_hover_text("Gợi ý 1 bước tiếp theo — Phím tắt: H")
                                            .clicked()
                                        {
                                            request_hint = true;
                                        }
                                    });
                                    ui.add_enabled_ui(!is_busy && !is_animating, |ui| {
                                        if ui
                                            .add_sized([w, 30.0], egui::Button::new("📋  Bảng công thức"))
                                            .on_hover_text(
                                                "Xem lời giải tách theo từng chặng CFOP — Phím tắt: Ctrl+T",
                                            )
                                            .clicked()
                                        {
                                            request_table = true;
                                        }
                                    });
                                });
                                ui.add_space(6.0);
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 28.0],
                                        egui::Button::new("📊  Thống kê OLL / PLL"),
                                    )
                                    .on_hover_text(
                                        "Số lần đã gặp từng thế OLL/PLL — Phím tắt: Ctrl+Y",
                                    )
                                    .clicked()
                                {
                                    request_stats = true;
                                }
                            });
                            ui.add_space(10.0);

                            // ── Kết quả gợi ý ───────────────────────────
                            // Đặt TRƯỚC ô nhập công thức: khi vừa bấm H,
                            // gợi ý là thứ người dùng đang chờ đọc, nên nó
                            // phải nằm ngay dưới cụm nút thay vì bị đẩy
                            // xuống dưới ô nhập. Khối này chỉ hiện khi CÓ
                            // gợi ý, nên lúc không có thì bố cục y như cũ.
                            if let Some(lbl) = &hint_label {
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
                                ui.add_space(10.0);
                            }

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

                            // ── Phím tắt: đã chuyển sang cửa sổ riêng, mở
                            // bằng icon bánh răng ở góc màn hình (xem phía
                            // dưới, gần "Bảng công thức").
                            }); // het ScrollArea
                        });

                    // ── Bảng công thức (phím T) ─────────────────────────
                    // Cửa sổ nổi, KHÔNG chặn thao tác khác (giống bản Python:
                    // "không modal") -- vẫn xoay được khối, vẫn gõ được.
                    if formula_open {
                        if let Some((_, bd)) = &formula_table {
                            let mut open = true;
                            egui::Window::new("Bảng công thức CFOP")
                                .open(&mut open)
                                .default_width(390.0)
                                .default_pos(egui::pos2(left_w + 24.0, 24.0))
                                .collapsible(false)
                                .frame(
                                    egui::Frame::none()
                                        .fill(theme::BG_PANEL)
                                        .rounding(egui::Rounding::same(10.0))
                                        .inner_margin(egui::Margin::same(13.0))
                                        .stroke(egui::Stroke::new(1.0, theme::BG_CARD_HI)),
                                )
                                .show(gui_context, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Tổng cộng {} nước",
                                            bd.total_moves()
                                        ))
                                        .size(12.5)
                                        .color(theme::TEXT_DIM),
                                    );
                                    ui.add_space(8.0);
                                    egui::ScrollArea::vertical()
                                        .max_height(430.0)
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                            stage_row(ui, "1) CROSS", "Cross", &bd.cross);
                                            ui.add_space(9.0);
                                            ui.label(
                                                egui::RichText::new("2) F2L  (4 cặp góc–cạnh)")
                                                    .size(11.5)
                                                    .strong()
                                                    .color(theme::ACCENT),
                                            );
                                            for (slot, st) in &bd.f2l {
                                                stage_row(ui, "", slot, st);
                                            }
                                            ui.add_space(9.0);
                                            stage_row(ui, "3) OLL  (định hướng lớp cuối)", "OLL", &bd.oll);
                                            ui.add_space(9.0);
                                            stage_row(ui, "4) PLL  (hoán vị lớp cuối)", "PLL", &bd.pll);
                                        });
                                });
                            if !open {
                                close_formula = true;
                            }
                        }
                    }

                    // ── Thống kê OLL/PLL đã gặp (Ctrl+Y) ────────────────
                    // Liệt kê TOÀN BỘ thế trong bảng chuẩn, kể cả thế chưa
                    // gặp lần nào (số 0) -- để thấy được mình còn thiếu thế
                    // nào, chứ không chỉ thấy những thế đã gặp.
                    if stats_open {
                        let mut open = true;
                        egui::Window::new("📊 Thống kê OLL / PLL")
                            .open(&mut open)
                            .default_width(330.0)
                            .default_pos(egui::pos2(left_w + 24.0, 24.0))
                            .collapsible(false)
                            .frame(
                                egui::Frame::none()
                                    .fill(theme::BG_PANEL)
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::same(13.0))
                                    .stroke(egui::Stroke::new(1.0, theme::BG_CARD_HI)),
                            )
                            .show(gui_context, |ui| {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut stats_tab_pll, false, "OLL");
                                    ui.selectable_value(&mut stats_tab_pll, true, "PLL");
                                });
                                ui.add_space(6.0);

                                let kind = if stats_tab_pll { "PLL" } else { "OLL" };
                                let algs = if stats_tab_pll {
                                    rubik_core::pll_algorithms::all_algorithms()
                                } else {
                                    rubik_core::oll_algorithms::all_algorithms()
                                };
                                // Ghép bảng chuẩn với số đếm rồi sắp giảm dần.
                                // Thế cùng số lần thì xếp theo tên cho ổn định
                                // (không nhảy lung tung giữa các lần mở).
                                let mut rows: Vec<(&str, u32)> = algs
                                    .iter()
                                    .map(|(name, _)| {
                                        let n = stats_counts
                                            .get(&(kind.to_string(), name.to_string()))
                                            .copied()
                                            .unwrap_or(0);
                                        (*name, n)
                                    })
                                    .collect();
                                let n_standard = rows.len();
                                // Nhật ký có thể chứa tên KHÔNG nằm trong bảng
                                // chuẩn: khi không tra được thế, breakdown lùi
                                // về cách giải 2 bước ("2-look") hoặc tìm kiếm
                                // ("macro"). Phải hiện ra, nếu không thì tổng
                                // số lần sẽ lớn hơn tổng các dòng mà không hiểu
                                // vì sao.
                                let mut extra: Vec<(String, u32)> = stats_counts
                                    .iter()
                                    .filter(|((k, name), _)| {
                                        k == kind && !algs.iter().any(|(a, _)| a == name)
                                    })
                                    .map(|((_, name), n)| (name.clone(), *n))
                                    .collect();
                                extra.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                                rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

                                let total = history::total(&stats_counts, kind);
                                let seen = rows.iter().filter(|(_, n)| *n > 0).count();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Đã gặp {seen}/{n_standard} thế · tổng {total} lần"
                                    ))
                                    .size(12.5)
                                    .color(theme::TEXT_DIM),
                                );
                                ui.add_space(8.0);

                                let max = rows
                                    .first()
                                    .map(|(_, n)| *n)
                                    .unwrap_or(0)
                                    .max(extra.first().map(|(_, n)| *n).unwrap_or(0))
                                    .max(1);
                                // Vẽ chung một vòng: bảng chuẩn trước, rồi tới
                                // các tên ngoài bảng (nếu có).
                                let all_rows: Vec<(&str, u32, bool)> = rows
                                    .iter()
                                    .map(|(n, c)| (*n, *c, false))
                                    .chain(extra.iter().map(|(n, c)| (n.as_str(), *c, true)))
                                    .collect();
                                egui::ScrollArea::vertical()
                                    .max_height(420.0)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        let mut sep_done = false;
                                        for (name, n, is_extra) in &all_rows {
                                            if *is_extra && !sep_done {
                                                sep_done = true;
                                                ui.add_space(8.0);
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Ngoài bảng chuẩn (giải bằng cách dự phòng)",
                                                    )
                                                    .size(10.5)
                                                    .color(theme::TEXT_DIM),
                                                );
                                            }
                                            ui.horizontal(|ui| {
                                                let col = if *n > 0 {
                                                    theme::TEXT
                                                } else {
                                                    theme::TEXT_DIM
                                                };
                                                ui.add_sized(
                                                    [124.0, 16.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(*name)
                                                            .size(11.5)
                                                            .color(col),
                                                    )
                                                    .wrap(false),
                                                );
                                                ui.add_sized(
                                                    [30.0, 16.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(n.to_string())
                                                            .monospace()
                                                            .size(11.5)
                                                            .color(if *n > 0 {
                                                                theme::ACCENT
                                                            } else {
                                                                theme::TEXT_DIM
                                                            }),
                                                    ),
                                                );
                                                // Thanh ngang so sánh tương đối
                                                // với thế gặp nhiều nhất.
                                                let (rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(ui.available_width().max(4.0), 8.0),
                                                    egui::Sense::hover(),
                                                );
                                                if *n > 0 {
                                                    let w = rect.width() * (*n as f32 / max as f32);
                                                    ui.painter().rect_filled(
                                                        egui::Rect::from_min_size(
                                                            rect.left_center()
                                                                - egui::vec2(0.0, 3.0),
                                                            egui::vec2(w.max(2.0), 6.0),
                                                        ),
                                                        2.0,
                                                        theme::ACCENT,
                                                    );
                                                }
                                            });
                                        }
                                    });
                                ui.add_space(8.0);
                                let where_txt = match history::path() {
                                    Some(p) => format!("Nhật ký: {}", p.display()),
                                    None => "Chưa xác định được nơi lưu nhật ký".to_string(),
                                };
                                ui.label(
                                    egui::RichText::new(where_txt).size(10.5).color(theme::TEXT_DIM),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Mỗi lần bấm “Bảng công thức” cho một ván mới sẽ ghi thêm \
                                         một dòng cho thế OLL và thế PLL của ván đó.",
                                    )
                                    .size(10.5)
                                    .color(theme::TEXT_DIM),
                                );
                            });
                        if !open {
                            close_stats = true;
                        }
                    }

                    // ── Bảng tra toàn bộ công thức chuẩn (phím V) ───────
                    if library_open {
                        let mut open = true;
                        egui::Window::new("Tra công thức chuẩn")
                            .open(&mut open)
                            .default_width(420.0)
                            .default_pos(egui::pos2(left_w + 40.0, 60.0))
                            .collapsible(false)
                            .frame(
                                egui::Frame::none()
                                    .fill(theme::BG_PANEL)
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::same(13.0))
                                    .stroke(egui::Stroke::new(1.0, theme::BG_CARD_HI)),
                            )
                            .show(gui_context, |ui| {
                                let oll = rubik_core::oll_algorithms::all_algorithms();
                                let pll = rubik_core::pll_algorithms::all_algorithms();
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(!library_tab_pll, format!("OLL ({})", oll.len()))
                                        .clicked()
                                    {
                                        library_tab_pll = false;
                                    }
                                    if ui
                                        .selectable_label(library_tab_pll, format!("PLL ({})", pll.len()))
                                        .clicked()
                                    {
                                        library_tab_pll = true;
                                    }
                                });
                                ui.add_space(7.0);
                                ui.add_sized(
                                    [ui.available_width(), 25.0],
                                    egui::TextEdit::singleline(&mut library_query)
                                        .hint_text("Tìm theo tên hoặc nước đi… VD: Sune, R U R'"),
                                );
                                ui.add_space(8.0);

                                let list = if library_tab_pll { pll } else { oll };
                                let q = library_query.trim().to_lowercase();
                                let filtered: Vec<&(&str, &str)> = list
                                    .iter()
                                    .filter(|(n, m)| {
                                        q.is_empty()
                                            || n.to_lowercase().contains(&q)
                                            || m.to_lowercase().contains(&q)
                                    })
                                    .collect();

                                ui.label(
                                    egui::RichText::new(format!("{} công thức", filtered.len()))
                                        .size(11.0)
                                        .color(theme::TEXT_DIM),
                                );
                                ui.add_space(5.0);

                                egui::ScrollArea::vertical()
                                    .max_height(400.0)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        for (name, moves) in filtered {
                                            egui::Frame::none()
                                                .fill(theme::BG_CARD)
                                                .rounding(egui::Rounding::same(7.0))
                                                .inner_margin(egui::Margin::symmetric(9.0, 7.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(*name)
                                                                .size(12.5)
                                                                .strong()
                                                                .color(theme::INFO),
                                                        );
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                if ui
                                                                    .small_button("Dùng")
                                                                    .on_hover_text(
                                                                        "Nạp vào ô công thức bên trái",
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    load_into_bar =
                                                                        Some(moves.to_string());
                                                                }
                                                            },
                                                        );
                                                    });
                                                    ui.label(
                                                        egui::RichText::new(*moves)
                                                            .monospace()
                                                            .size(12.0)
                                                            .color(theme::ACCENT),
                                                    );
                                                });
                                            ui.add_space(5.0);
                                        }
                                    });
                            });
                        if !open {
                            close_library = true;
                        }
                    }

                    // ── Icon bánh răng ở góc màn hình -- mở bảng phím tắt ──
                    egui::Area::new("shortcuts_gear_icon")
                        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-14.0, 14.0))
                        .order(egui::Order::Foreground)
                        .show(gui_context, |ui| {
                            let resp = ui.add(
                                egui::Button::new(egui::RichText::new("⚙").size(18.0))
                                    .fill(theme::BG_CARD)
                                    .stroke(egui::Stroke::new(1.0, theme::BG_CARD_HI))
                                    .rounding(egui::Rounding::same(8.0)),
                            );
                            if resp.on_hover_text("Phím tắt & hướng dẫn").clicked() {
                                shortcuts_open = !shortcuts_open;
                            }
                        });

                    // ── Cửa sổ phím tắt & hướng dẫn (mở bằng icon bánh răng) ──
                    if shortcuts_open {
                        let mut open = true;
                        egui::Window::new("⚙ Phím tắt & hướng dẫn")
                            .open(&mut open)
                            .default_width(340.0)
                            .default_pos(egui::pos2(left_w + 40.0, 60.0))
                            .collapsible(false)
                            .frame(
                                egui::Frame::none()
                                    .fill(theme::BG_PANEL)
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::same(13.0))
                                    .stroke(egui::Stroke::new(1.0, theme::BG_CARD_HI)),
                            )
                            .show(gui_context, |ui| {
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
                                row(ui, "Ctrl+T", "bảng công thức cho ván này");
                                row(ui, "Ctrl+J", "tra toàn bộ công thức chuẩn");
                                row(ui, "Ctrl+Y", "thống kê OLL/PLL đã gặp");
                                row(ui, "Ctrl+K", "quét trạng thái từ camera");
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
                        if !open {
                            shortcuts_open = false;
                        }
                    }

                    // ── Cửa sổ quét trạng thái từ camera ─────────────────
                    if let Some(new_state) = scan_ui::show(gui_context, &mut scan_state) {
                        scanned_state = Some(new_state);
                        state_source_label = "camera";
                    }

                    // ── Cửa sổ điền màu thủ công ──────────────────────────
                    if let Some(new_state) = paint_ui::show(gui_context, &mut paint_state) {
                        scanned_state = Some(new_state);
                        state_source_label = "điền màu thủ công";
                    }

                    // Chặn phím tắt toàn cục khi BẤT KỲ ô nhập nào của egui
                    // đang nhận bàn phím (ô công thức ở panel, ô tìm kiếm
                    // trong bảng tra…). Trước đây chỉ chặn riêng ô công
                    // thức, nên gõ vào ô tìm kiếm bị hiểu nhầm thành phím
                    // tắt (gõ "Sune" làm chữ "u" xoay mặt U) -- lỗi thật đã
                    // phát hiện khi thử.
                    if gui_context.wants_keyboard_input() {
                        editing_formula = true;
                    }
                },
            );

            if let Some(new_state) = scanned_state.take() {
                push_undo!();
                pending = None;
                queue.clear();
                state = new_state;
                move_count = 0;
                case_log.van_moi();
                formula_table = None;
                status = format!("Đã nạp trạng thái từ {state_source_label}");
            }

            if close_formula {
                formula_open = false;
            }
            if close_stats {
                stats_open = false;
            }
            if request_stats {
                if stats_open {
                    stats_open = false;
                } else {
                    // Đọc lại file mỗi lần mở: nhật ký có thể vừa được ghi
                    // thêm, và người dùng cũng có thể tự sửa file.
                    stats_counts = history::counts();
                    stats_open = true;
                }
            }
            if close_library {
                library_open = false;
            }
            if let Some(mvs) = load_into_bar.take() {
                formula_text = mvs;
                status = "Đã nạp công thức — bấm Áp dụng để chạy".to_string();
            }
            if request_table && job_rx.is_none() && queue.is_empty() && pending.is_none() {
                let fresh = formula_table.as_ref().map(|(s, _)| *s == state).unwrap_or(false);
                if fresh {
                    formula_open = !formula_open;
                } else {
                    rubik_core::cancel::clear();
                    let state_copy = state;
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let b = rubik_core::breakdown::full_solve_breakdown(&state_copy);
                        let _ = tx.send(JobResult::Breakdown(state_copy, b));
                    });
                    job_rx = Some(rx);
                    status = "Đang lập bảng công thức…".to_string();
                }
            }
            if request_scramble && !is_busy && !is_animating && queue.is_empty() {
                push_undo!();
                case_log.van_moi();
                formula_table = None;
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
            camera.set_viewport(cam_viewport);

            // ── Click chon mat (chi khi cube dang ranh -- khong animate) --
            // dung three_d::pick() (GPU raycast co san) truoc khi
            // OrbitControl xu ly, danh dau handled=true neu trung mat de
            // KHONG bi hieu nham thanh keo xoay camera (giong Python:
            // click trung sticker = chon mat, khong trung = keo camera).
            // Chỉ dựng lưới phục vụ bắt chuột KHI THỰC SỰ có cú click --
            // trước đây dựng lại mỗi khung hình dù không ai bấm, rất tốn.
            let has_click = frame_input.events.iter().any(|e| {
                matches!(e, Event::MousePress { button: MouseButton::Left, handled: false, .. })
            });
            if has_click && pending.is_none() && !editing_formula {
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
                if editing_formula || scan_state.open || paint_state.open {
                    break;
                }
                if let Event::KeyPress { kind, modifiers, handled, .. } = event {
                    if *handled {
                        continue;
                    }
                    match kind {
                        // 6 mặt cơ bản. Kèm phím bổ trợ:
                        //   Shift = nước ngược (U')
                        //   Ctrl  = nước wide, xoay 2 lớp (u, r…)
                        //   Alt   = nước đúp 180° (U2) -- nạp 2 lần 90°
                        Key::U | Key::D | Key::F | Key::B | Key::L | Key::R => {
                            let base = match kind {
                                Key::U => 'U', Key::D => 'D', Key::F => 'F',
                                Key::B => 'B', Key::L => 'L', Key::R => 'R',
                                _ => unreachable!(),
                            };
                            // Ctrl -> chữ thường = nước wide (move_spec đã
                            // hỗ trợ sẵn u/d/f/b/l/r).
                            let mv = if modifiers.ctrl || modifiers.command {
                                base.to_ascii_lowercase()
                            } else {
                                base
                            };
                            push_undo!();
                            queue.push_back((mv, modifiers.shift, modifiers.alt));
                        }
                        // Lớp giữa: M (giữa L-R), E (giữa U-D), S (giữa F-B).
                        Key::M => {
                            push_undo!();
                            queue.push_back(('M', modifiers.shift, false));
                        }
                        Key::E => {
                            push_undo!();
                            queue.push_back(('E', modifiers.shift, false));
                        }
                        Key::S => {
                            push_undo!();
                            queue.push_back(('S', modifiers.shift, false));
                        }
                        // Xoay CẢ KHỐI: x (quanh trục R), y (quanh U), z (quanh F).
                        Key::X => {
                            push_undo!();
                            queue.push_back(('x', modifiers.shift, false));
                        }
                        // Chốt chặn Ctrl giống hệt `Key::Z` ngay dưới: Ctrl+Y
                        // đã nhường cho cửa sổ thống kê. Không mất gì, vì
                        // nhánh này vốn BỎ QUA Ctrl -- Ctrl+Y trước đây chỉ là
                        // bản trùng lặp của Y, xoay y y hệt.
                        Key::Y if !(modifiers.ctrl || modifiers.command) => {
                            push_undo!();
                            queue.push_back(('y', modifiers.shift, false));
                        }
                        Key::Z if !(modifiers.ctrl || modifiers.command) => {
                            push_undo!();
                            queue.push_back(('z', modifiers.shift, false));
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
                        // Ctrl+T = mở/đóng bảng công thức (toàn bộ lời giải
                        // tách theo từng chặng CFOP), giống phím T bản Python
                        // (nay thêm Ctrl để tránh lẫn với các phím xoay mặt).
                        Key::T if modifiers.ctrl || modifiers.command => {
                            if formula_open {
                                formula_open = false;
                            } else if job_rx.is_none() && queue.is_empty() && pending.is_none() {
                                let fresh = formula_table
                                    .as_ref()
                                    .map(|(s, _)| *s == state)
                                    .unwrap_or(false);
                                if fresh {
                                    formula_open = true;
                                } else {
                                    rubik_core::cancel::clear();
                                    let state_copy = state;
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    std::thread::spawn(move || {
                                        let b = rubik_core::breakdown::full_solve_breakdown(&state_copy);
                                        let _ = tx.send(JobResult::Breakdown(state_copy, b));
                                    });
                                    job_rx = Some(rx);
                                    status = "Đang lập bảng công thức…".to_string();
                                }
                            }
                        }
                        // Ctrl+J = mở/đóng bảng tra toàn bộ công thức chuẩn.
                        Key::J if modifiers.ctrl || modifiers.command => {
                            library_open = !library_open;
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
                            if stats_open {
                                stats_open = false;
                            } else {
                                stats_counts = history::counts();
                                stats_open = true;
                            }
                        }
                        // Ctrl+K = mở cửa sổ quét trạng thái từ camera (không
                        // có nút bấm riêng trong giao diện theo yêu cầu --
                        // chỉ vào được bằng phím tắt này).
                        Key::K if modifiers.ctrl || modifiers.command => {
                            if !is_busy && !is_animating {
                                scan_state.open = true;
                            }
                        }
                        Key::Escape => {
                            if stats_open {
                                stats_open = false;
                            } else if library_open {
                                library_open = false;
                            } else if formula_open {
                                formula_open = false;
                            } else if job_rx.is_some() {
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
                                queue.push_back((face, false, false));
                            }
                        }
                        Key::ArrowDown | Key::ArrowLeft => {
                            if let Some(face) = sel_face {
                                push_undo!();
                                queue.push_back((face, true, false));
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
                        JobResult::Breakdown(st, b) => {
                            status = format!("Bảng công thức: {} nước", b.total_moves());
                            // Ghi nhật ký NGAY tại đây, chỗ duy nhất bảng
                            // được tính xong. Vì `formula_table` chỉ tính lại
                            // khi trạng thái đổi, mở đi mở lại cùng một ván
                            // sẽ KHÔNG đếm trùng.
                            use rubik_core::breakdown::StageStatus;
                            let mut entries: Vec<(&str, String, usize)> = Vec::new();
                            for (kind, stage) in [("OLL", &b.oll), ("PLL", &b.pll)] {
                                if let StageStatus::Moves { moves, case_name } = stage {
                                    if let Some(name) = case_name {
                                        // Chỉ ghi nếu ván này chưa ghi ca đó.
                                        if case_log.nen_ghi(kind, name) {
                                            entries.push((kind, name.clone(), moves.len()));
                                        }
                                    }
                                }
                            }
                            if let Err(e) = history::append(&entries) {
                                // Không nuốt lỗi: nếu không ghi được (thư mục
                                // chỉ đọc chẳng hạn) thì phải nói ra, chứ để
                                // im lặng thì người dùng tưởng đã ghi.
                                status = format!("Bảng công thức xong, nhưng KHÔNG ghi được nhật ký: {e}");
                            }
                            if stats_open {
                                stats_counts = history::counts();
                            }
                            formula_table = Some((st, b));
                            formula_open = true;
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
                                    if case_log.nen_ghi(kind, name) {
                                        let e = [(kind, name.clone(), h.moves.len())];
                                        if let Err(err) = history::append(&e) {
                                            // KHÔNG đặt `status` ở đây: đoạn
                                            // ngay dưới gán đè `status` bằng
                                            // nhãn gợi ý, nên lỗi sẽ bị nuốt
                                            // mất. Giữ lại rồi nối vào sau.
                                            log_err = Some(err);
                                        } else if stats_open {
                                            stats_counts = history::counts();
                                        }
                                    }
                                }
                            }
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
                            if let Some(err) = log_err {
                                status = format!("{status} — KHÔNG ghi được nhật ký: {err}");
                            }
                            hint_label = Some(h.label);
                            hint_moves = h.moves;
                            if auto_apply_hint {
                                auto_apply_hint = false;
                                if !hint_moves.is_empty() {
                                    push_undo!();
                                    queue.clear();
                                    for m in &hint_moves {
                                        queue_move(&mut queue, m);
                                    }
                                    total_queued = queue.len();
                                }
                            }
                        }
                    }
                    job_rx = None;
                }
            }

            if queue.len() > total_queued {
                total_queued = queue.len();
            }
            if pending.is_none() {
                if let Some((mv, prime, double)) = queue.pop_front() {
                    let quarter = std::f32::consts::FRAC_PI_2;
                    let mag = if double { 2.0 * quarter } else { quarter };
                    let target = if prime { mag } else { -mag };
                    pending = Some(PendingMove {
                        mv,
                        prime,
                        double,
                        target,
                        current: 0.0,
                        // Nước đúp quay gấp đôi góc nên tăng tốc để không
                        // mất gấp đôi thời gian.
                        speed: if double { anim_speed * 1.6 } else { anim_speed },
                    });
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
                    let mv_str = if p.double {
                        format!("{}2", p.mv)
                    } else if p.prime {
                        format!("{}'", p.mv)
                    } else {
                        p.mv.to_string()
                    };
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

            // Chỉ chạy hết tốc lực KHI CẦN (đang xoay khối, đang tính, hoặc
            // giao diện đang có hoạt ảnh). Lúc rảnh thì ngủ chờ sự kiện --
            // trước đây luôn Poll + request_redraw nên ngốn ~97% CPU dù
            // không làm gì. Đây là thay đổi quan trọng nhất cho máy yếu.
            let busy_now = pending.is_some()
                || !queue.is_empty()
                || job_rx.is_some()
;
            needs_redraw = busy_now;
            if busy_now {
                *control_flow = ControlFlow::Poll;
                winit_window.request_redraw();
            } else {
                // Vẫn thức dậy định kỳ (~20 lần/giây) để kịp nhận kết quả
                // luồng nền và giữ con trỏ chuột phản hồi mượt.
                *control_flow = ControlFlow::WaitUntil(
                    std::time::Instant::now() + std::time::Duration::from_millis(50),
                );
            }
        }
        WinitEvent::WindowEvent { ref event, .. } => {
            // Có tương tác -> cần vẽ lại ngay.
            needs_redraw = true;
            winit_window.request_redraw();
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

/// Vẽ 1 dòng chặng trong bảng công thức. `header` để trống nếu không cần
/// tiêu đề nhóm riêng (VD các cặp F2L nằm chung dưới 1 tiêu đề).
fn stage_row(
    ui: &mut egui::Ui,
    header: &str,
    label: &str,
    st: &rubik_core::breakdown::StageStatus,
) {
    use rubik_core::breakdown::StageStatus as S;
    if !header.is_empty() {
        ui.label(
            egui::RichText::new(header)
                .size(11.5)
                .strong()
                .color(theme::ACCENT),
        );
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{label}"))
                .size(12.5)
                .strong()
                .color(theme::TEXT),
        );
        match st {
            S::Done => {
                ui.label(
                    egui::RichText::new("✔ đã xong sẵn")
                        .size(12.0)
                        .color(theme::SUCCESS),
                );
            }
            S::Moves { moves, case_name } => {
                if let Some(n) = case_name {
                    ui.label(
                        egui::RichText::new(format!("[{n}]"))
                            .size(11.0)
                            .color(theme::INFO),
                    );
                }
                ui.label(
                    egui::RichText::new(moves.join(" "))
                        .monospace()
                        .size(12.5)
                        .color(theme::ACCENT),
                );
                ui.label(
                    egui::RichText::new(format!("({} nước)", moves.len()))
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            }
            S::Failed => {
                ui.label(
                    egui::RichText::new("chưa tìm được — thử mở lại bảng (T)")
                        .size(12.0)
                        .color(theme::DANGER),
                );
            }
            S::Pending => {
                ui.label(
                    egui::RichText::new("chưa tới lượt — cần xong bước trước")
                        .size(12.0)
                        .color(theme::TEXT_DIM),
                );
            }
        }
    });
}

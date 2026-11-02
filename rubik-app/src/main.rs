//! rubik-app — ứng dụng Rubik's Cube 3D tương tác.
//!
//! File này chỉ chứa phần GIAO DIỆN: dựng hình học cube cho GPU, vòng lặp
//! winit, camera, panel egui (khung 6 mặt + thanh nhập công thức), và xử
//! lý phím/chuột. TOÀN BỘ thuật toán giải nằm trong crate `rubik-core`.
//!
//! Xem README.md ở thư mục gốc workspace để biết cách chạy.
//!
//! Bản RELEASE trên Windows không mở cửa sổ console (giống file .pyw) --
//! chỉ khi build DEBUG (`cargo run` không kèm `--release`, dùng lúc phát
//! triển) mới giữ console lại để còn thấy println!/panic! khi cần soi lỗi.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod camera_capture;
mod history;
mod input;
mod jobs;
mod mesh;
mod paint_ui;
mod scan_ui;
mod scene;
mod solve_flow;
mod theme;
mod ui_library;
mod ui_panel;
mod ui_stats;
mod ui_widgets;
mod ui_windows;

use app::App;
use three_d::*;

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
    let surface_settings = SurfaceSettings {
        // Mặc định của three-d là 4x MSAA (khử răng cưa cạnh khối) --
        // giảm xuống 2x: cube toàn mặt phẳng màu đặc, cạnh khối vẫn khá
        // mượt ở 2x, nhưng chi phí GPU cho mỗi lần vẽ giảm đáng kể so
        // với 4x -- rõ nhất trên GPU tích hợp/máy cấu hình thấp. vsync
        // giữ mặc định (true) -- đã tự giới hạn khung hình theo tần số
        // quét màn hình, không cần Poll liên tục.
        multisamples: 2,
        ..SurfaceSettings::default()
    };
    let gl = WindowedContext::from_winit_window(&winit_window, surface_settings)
        .expect("Khong tao duoc GL context");
    let context: Context = (*gl).clone();

    let camera = Camera::new_perspective(
        Viewport::new_at_origo(1100, 800),
        vec3(4.5, 4.0, 5.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(35.0),
        0.1,
        100.0,
    );
    let control = OrbitControl::new(vec3(0.0, 0.0, 0.0), 4.0, 15.0);
    let mut gui = three_d::GUI::new(&context);
    {
        // Nạp font tiếng Việt + bảng màu. GUI::new() đã tạo sẵn context egui
        // nên chỉ cần lấy ra và cấu hình 1 lần ở đây.
        let ctx = gui.context();
        theme::install_fonts(&ctx);
        theme::install_style(&ctx);
    }

    let mut app = App::new(winit_window, gl, context, camera, control);
    let mut frame_input_generator = FrameInputGenerator::from_winit_window(&app.winit_window);

    event_loop.run(move |event, _, control_flow| match event {
        WinitEvent::MainEventsCleared => {
            // CHỈ yêu cầu vẽ lại khi thực sự cần. Trước đây gọi vô điều
            // kiện ở đây nên vòng lặp chạy hết tốc lực (~97% CPU) dù đặt
            // ControlFlow::WaitUntil ở chỗ khác -- đo thực tế mới lộ ra.
            if app.needs_redraw {
                app.winit_window.request_redraw();
            }
        }
        WinitEvent::RedrawRequested(_) => {
            let mut frame_input = frame_input_generator.generate(&app.context);
            let mut req = app.begin_frame(&frame_input);
            let gui_ppp = req.gui_ppp;
            gui.update(
                &mut frame_input.events,
                frame_input.accumulated_time,
                frame_input.viewport,
                gui_ppp,
                |gui_context| {
                    app.ui_left_panel(gui_context, &mut req);
                    app.ui_formula_table(gui_context, &mut req);
                    app.ui_stats(gui_context, &mut req);
                    app.ui_library(gui_context, &mut req);
                    app.ui_shortcuts(gui_context, &mut req);
                    app.ui_dialogs(gui_context, &mut req);
                },
            );
            app.apply_requests(&mut req);
            app.update_camera_and_pick(&mut frame_input, &req);
            app.handle_keys(&frame_input, &req);
            app.poll_job();
            let anim = app.step_animation(&frame_input);
            app.render_frame(&mut frame_input, &gui, anim);
            app.end_frame(control_flow);
        }
        WinitEvent::WindowEvent { ref event, .. } => {
            // Có tương tác -> cần vẽ lại ngay.
            app.needs_redraw = true;
            app.winit_window.request_redraw();
            frame_input_generator.handle_winit_window_event(event);
            match event {
                WinitWindowEvent::Resized(physical_size) => {
                    app.gl.resize(*physical_size);
                }
                WinitWindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    app.gl.resize(**new_inner_size);
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
                                app.is_fullscreen = !app.is_fullscreen;
                                app.winit_window.set_fullscreen(if app.is_fullscreen {
                                    Some(Fullscreen::Borderless(None))
                                } else {
                                    None
                                });
                            }
                            // Toc do animation: [ giam (cham hon), ] tang
                            // (nhanh hon). Gioi han duoi cho phep mot phan tu
                            // (90 do) cham toi da 2 giay -- gioi han tren van
                            // giu nhu cu (~0.05s cho 1 phan tu, du nhanh de
                            // xem muot).
                            Some(winit::event::VirtualKeyCode::LBracket) => {
                                app.anim_speed =
                                    (app.anim_speed - 2.0).max(std::f32::consts::FRAC_PI_2 / 2.0);
                            }
                            Some(winit::event::VirtualKeyCode::RBracket) => {
                                app.anim_speed = (app.anim_speed + 2.0).min(30.0);
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

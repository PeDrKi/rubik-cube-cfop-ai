//! main.rs — Rubik's Cube: cửa sổ tương tác thật (không phải headless).
//! Dùng ĐÚNG hình học đã kiểm chứng ở rubik-render (build_cube_mesh,
//! move_spec, cubie_to_rc) -- chỉ khác phần vỏ ngoài: thay HeadlessContext
//! + ghi PNG bằng Window + render_loop + OrbitControl thật của three-d.
//!
//! CHƯA TỰ CHẠY THỬ ĐƯỢC (cần màn hình thật) -- xem README_CHAY.md đi kèm
//! để biết cách chạy và báo lại kết quả.

mod cube;

use cube::CubeState;
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

/// Hàng đợi nước đi đang chạy animation.
struct PendingMove {
    mv: char,
    prime: bool,
    target: f32,
    current: f32,
    speed: f32,
}

fn main() {
    let window = Window::new(WindowSettings {
        title: "Rubik's Cube (Rust/three-d)".to_string(),
        max_size: Some((900, 900)),
        ..Default::default()
    })
    .expect("Khong tao duoc cua so -- kiem tra driver GPU/man hinh");

    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(4.5, 4.0, 5.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(35.0),
        0.1,
        100.0,
    );
    let mut control = OrbitControl::new(vec3(0.0, 0.0, 0.0), 4.0, 15.0);

    let mut state = CubeState::solved();
    let mut pending: Option<PendingMove> = None;

    let ambient = AmbientLight::new(&context, 0.5, Srgba::WHITE);
    let sun = DirectionalLight::new(&context, 1.2, Srgba::WHITE, &vec3(-0.45, -0.85, -0.65));

    window.render_loop(move |mut frame_input| {
        camera.set_viewport(frame_input.viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        // Xu ly phim tat: U D F B L R (+Shift = nghich dao ') kich hoat
        // animation 1 nuoc (chi khi khong co animation nao dang chay).
        if pending.is_none() {
            for event in &frame_input.events {
                if let Event::KeyPress { kind, modifiers, .. } = event {
                    let base = match kind {
                        Key::U => Some('U'), Key::D => Some('D'),
                        Key::F => Some('F'), Key::B => Some('B'),
                        Key::L => Some('L'), Key::R => Some('R'),
                        _ => None,
                    };
                    if let Some(mv) = base {
                        let prime = modifiers.shift;
                        // Khong-prime: -90 do (CW nhin tu ngoai vao, dung quy
                        // uoc cube_engine.py -- da kiem chung). Prime: nguoc
                        // dau, +90 do.
                        let target = if prime {
                            std::f32::consts::FRAC_PI_2
                        } else {
                            -std::f32::consts::FRAC_PI_2
                        };
                        pending = Some(PendingMove { mv, prime, target, current: 0.0, speed: 6.0 });
                    }
                }
            }
        }

        // Tien animation dang chay (neu co) theo thoi gian thuc.
        let anim = pending.as_mut().map(|p| {
            let dt = (frame_input.elapsed_time / 1000.0) as f32;
            p.current += p.speed * dt * p.target.signum();
            if p.current.abs() >= p.target.abs() {
                p.current = p.target;
            }
            (p.mv, p.current)
        });
        if let Some(p) = &pending {
            if p.current == p.target {
                // Animation xong: AP THAT nuoc di vao state facelet, dung
                // dung engine da kiem chung o rubik-rs (cube.rs).
                let mv_str = if p.prime { format!("{}'", p.mv) } else { p.mv.to_string() };
                state.do_move(&mv_str);
                pending = None;
            }
        }

        let cpu_mesh = build_cube_mesh(&state, anim);
        let mesh = Mesh::new(&context, &cpu_mesh);
        let cube_model = Gm::new(mesh, ColorMaterial::default());

        frame_input.screen()
            .clear(ClearState::color_and_depth(0.11, 0.11, 0.16, 1.0, 1.0))
            .render(&camera, &[&cube_model], &[&ambient, &sun]);

        FrameOutput::default()
    });
}

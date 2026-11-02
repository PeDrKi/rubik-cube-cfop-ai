//! mesh.rs — dựng hình học cube cho GPU: màu mỗi mặt, trục/lớp bị xoay
//! ứng với mỗi ký tự nước đi, và hàm chính `build_cube_mesh` gom tất cả
//! 27 cubie thành 1 mesh duy nhất (tách riêng khỏi main.rs vì đây là
//! phần THUẦN -- không đụng tới cửa sổ/sự kiện/state của app -- cho dễ
//! đọc và test độc lập).

use rubik_core::cube::{self, CubeState};
use three_d::*;

pub(crate) const CUBIE_H: f32 = 0.96;
pub(crate) const STICKER_H: f32 = 0.76;
pub(crate) const PLASTIC: [u8; 3] = [20, 20, 20];
pub(crate) const FACES: [char; 6] = ['U', 'D', 'F', 'B', 'L', 'R'];

pub(crate) fn face_color(face: char) -> [u8; 3] {
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

pub(crate) fn face_info(face: char) -> (Vec3, Vec3, Vec3) {
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

pub(crate) fn move_spec(mv: char) -> Option<(Vec3, usize, Vec<i32>)> {
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

pub(crate) fn quad_corners(center: Vec3, right: Vec3, up: Vec3, half: f32) -> [Vec3; 4] {
    [
        center - right * half - up * half,
        center + right * half - up * half,
        center + right * half + up * half,
        center - right * half + up * half,
    ]
}

pub(crate) fn cubie_to_rc(ix: i32, iy: i32, iz: i32, rv: Vec3, uv: Vec3) -> (usize, usize) {
    let p = vec3(ix as f32, iy as f32, iz as f32);
    let c = p.dot(rv).round() as i32;
    let r = p.dot(uv).round() as i32;
    let row = 1 - r;
    let col = c + 1;
    (row as usize, col as usize)
}

pub(crate) fn build_cube_mesh(state: &CubeState, anim: Option<(char, f32)>) -> CpuMesh {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut colors: Vec<Srgba> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    let spec = anim
        .and_then(|(mv, ang)| move_spec(mv).map(|(axis, idx, allowed)| (axis, idx, allowed, ang)));

    let mut push_quad = |corners: [Vec3; 4], normal: Vec3, color: [u8; 3], rot: Option<Mat3>| {
        let c = Srgba::new(color[0], color[1], color[2], 255);
        for &i in &[0usize, 1, 2, 0, 2, 3] {
            let p = match rot {
                Some(m) => m * corners[i],
                None => corners[i],
            };
            let n = match rot {
                Some(m) => m * normal,
                None => normal,
            };
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
                        'U' => (iy, 1),
                        'D' => (iy, -1),
                        'F' => (iz, 1),
                        'B' => (iz, -1),
                        'L' => (ix, -1),
                        'R' => (ix, 1),
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
                        let ck = cube::face_char(
                            state.facelet_at(cube::idx_of(face), row, col) as usize
                        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_spec_6_mat_co_ban_deu_hop_le_va_dung_1_lop() {
        for &c in &['U', 'D', 'F', 'B', 'L', 'R'] {
            let (_, _axis, layers) =
                move_spec(c).unwrap_or_else(|| panic!("move_spec('{c}') = None"));
            assert_eq!(layers.len(), 1, "mặt '{c}' phải xoay đúng 1 lớp");
        }
    }

    #[test]
    fn move_spec_nuoc_giua_khong_lam_xoay_lop_ngoai() {
        // M/E/S xoay lớp GIỮA (layer index 0) -- không phải lớp ngoài
        // (-1 hoặc 1), nếu lẫn sẽ vô tình xoay nhầm mặt ngoài khi bấm M/E/S.
        for &c in &['M', 'E', 'S'] {
            let (_, _axis, layers) = move_spec(c).unwrap();
            assert_eq!(layers, vec![0], "'{c}' phải chỉ xoay lớp giữa (0)");
        }
    }

    #[test]
    fn move_spec_nuoc_wide_xoay_2_lop_gom_ca_lop_ngoai_va_giua() {
        // u = U + lớp giữa (E ngược), tức 2 trong 3 lớp trên trục đó.
        for &c in &['u', 'd', 'f', 'b', 'l', 'r'] {
            let (_, _axis, layers) = move_spec(c).unwrap();
            assert_eq!(layers.len(), 2, "nước wide '{c}' phải xoay 2 lớp");
        }
    }

    #[test]
    fn move_spec_xoay_ca_khoi_xoay_du_3_lop() {
        for &c in &['x', 'y', 'z'] {
            let (_, _axis, layers) = move_spec(c).unwrap();
            assert_eq!(
                layers.len(),
                3,
                "xoay cả khối '{c}' phải xoay đủ 3 lớp trên trục đó"
            );
        }
    }

    #[test]
    fn move_spec_ky_tu_khong_hop_le_tra_ve_none() {
        for c in ['Q', ' ', '2', '\'', 'a'] {
            assert_eq!(
                move_spec(c),
                None,
                "'{c}' không phải 1 nước hợp lệ, phải trả None"
            );
        }
    }

    #[test]
    fn face_color_dung_quy_uoc_va_phan_biet() {
        let mapping = [
            ('U', [255, 210, 0]),
            ('D', [255, 255, 255]),
            ('F', [15, 160, 55]),
            ('B', [15, 75, 210]),
            ('L', [210, 30, 45]),
            ('R', [255, 100, 0]),
        ];
        let mut seen = std::collections::HashSet::new();
        for (face, expected) in mapping {
            assert_eq!(face_color(face), expected);
            assert!(
                seen.insert(expected),
                "màu của mặt '{face}' trùng với 1 mặt khác"
            );
        }
    }
}

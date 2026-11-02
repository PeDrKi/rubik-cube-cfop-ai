//! Chống tấn công từ chối dịch vụ qua cú pháp lặp `(...)N` của công thức.
//! Trước đây `(R)99999999999999999999` làm panic (tràn số khi parse) và
//! `(R U R' U')99999999` cấp phát hàng GB bộ nhớ -- cả server lẫn trang web đều có thể bị đánh sập
//! chỉ bằng một chuỗi gõ vào ô công thức hoặc gửi lên /api/apply, /api/game/finish.
use rubik_core::cube::{parse_singmaster, MAX_PARSED_MOVES};
use std::time::Instant;

#[test]
fn so_lap_qua_lon_khong_panic() {
    let r = parse_singmaster("(R)99999999999999999999999999");
    assert!(r.is_err(), "phải trả lỗi thay vì panic");
}

#[test]
fn bung_chuoi_qua_dai_bi_tu_choi_nhanh() {
    let t = Instant::now();
    let r = parse_singmaster("(R U R' U')999999999");
    assert!(r.is_err());
    assert!(
        t.elapsed().as_millis() < 200,
        "phải từ chối ngay, không bung chuỗi rồi mới kiểm tra"
    );
}

#[test]
fn nhieu_nhom_lap_cong_don_van_bi_chan() {
    // Mỗi nhóm nằm dưới giới hạn, nhưng cộng lại vượt quá.
    let s = "(R U R' U')900 ".repeat(10);
    assert!(parse_singmaster(&s).is_err());
}

#[test]
fn cong_thuc_binh_thuong_van_chay() {
    assert_eq!(parse_singmaster("(R U R' U')6").unwrap().len(), 24);
    assert_eq!(parse_singmaster("(R U)3 (F)2").unwrap().len(), 8);
    let max_ok = format!("(R){MAX_PARSED_MOVES}");
    assert_eq!(parse_singmaster(&max_ok).unwrap().len(), MAX_PARSED_MOVES);
}

#[test]
fn qua_nhieu_nuoc_khong_dung_lap_cung_bi_chan() {
    let s = "R ".repeat(MAX_PARSED_MOVES + 1);
    assert!(parse_singmaster(&s).is_err());
}

#[test]
fn thong_bao_loi_ky_hieu_la_tieng_viet() {
    let e = parse_singmaster("R Q U").unwrap_err();
    assert!(e.contains("không hợp lệ") && e.contains('Q'), "{e}");
}

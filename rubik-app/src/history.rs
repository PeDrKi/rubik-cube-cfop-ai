//! history.rs — nhật ký các thế OLL/PLL đã gặp.
//!
//! Mỗi lần `Bảng công thức` nhận diện được thế OLL và PLL của ván hiện
//! tại, ghi thêm 1 dòng vào file văn bản thuần đặt CẠNH FILE .EXE. Không
//! dùng thư viện ngoài nào (dự án không có `serde`/`chrono`, kéo thêm
//! dependency chỉ để ghi vài dòng là không đáng).
//!
//! Định dạng TSV, mở được bằng Notepad lẫn Excel:
//!
//! ```text
//! epoch_giay	thoi_gian_utc	loai	ten_the	so_nuoc
//! 1789020000	2026-09-10 06:40:00	OLL	OLL27	7
//! 1789020000	2026-09-10 06:40:00	PLL	T	14
//! ```
//!
//! GHI CHÚ VỀ MÚI GIỜ: `std::time` không cho biết lệch múi giờ của máy,
//! và dự án không có `chrono`, nên cột thời gian là **UTC** — đặt tên cột
//! rõ như vậy để không hiểu nhầm là giờ địa phương. Cột `epoch_giay` giữ
//! nguyên số giây để sau này muốn xử lý lại thì không mất thông tin.
//!
//! CẤU TRÚC: phần việc thật nằm ở `append_to` / `counts_from` nhận vào
//! đường dẫn, còn `append` / `counts` chỉ là lớp mỏng tra ra đường dẫn
//! cạnh .exe. Tách vậy để test được bằng file tạm — nếu mọi thứ đi qua
//! `current_exe()` thì không viết test nổi.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "lich_su_oll_pll.tsv";
const HEADER: &str = "epoch_giay\tthoi_gian_utc\tloai\tten_the\tso_nuoc\n";

/// Đường dẫn file nhật ký: cùng thư mục với file .exe đang chạy.
///
/// LƯU Ý: khi chạy bằng `cargo run`, .exe nằm trong `target/release/`, nên
/// file cũng nằm ở đó — `cargo clean` sẽ xoá mất. UI có hiện đường dẫn đầy
/// đủ để người dùng biết chính xác file đang ở đâu.
pub fn path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(FILE_NAME))
}

/// Đổi số giây từ 1970 sang chuỗi "YYYY-MM-DD HH:MM:SS" (UTC).
/// Thuật toán civil-from-days chuẩn của Howard Hinnant — đúng cho mọi năm
/// trong phạm vi quan tâm, kể cả năm nhuận.
fn utc_string(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let secs = epoch.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Bỏ ký tự làm hỏng định dạng TSV khỏi tên thế.
///
/// Tên hiện tại (OLL01…OLL57, T, Ja, Gb…) không chứa tab hay xuống dòng,
/// nhưng một tên có tab sẽ làm lệch hẳn cột và phá phép đếm về sau. Chặn
/// ngay lúc ghi rẻ hơn nhiều so với đi sửa file đã hỏng.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c == '\t' || c == '\n' || c == '\r' { '_' } else { c })
        .collect()
}

/// Ghi thêm vào ĐÚNG file này. Tách riêng để test được bằng file tạm.
fn append_to(p: &Path, entries: &[(&str, String, usize)], epoch: i64) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let is_new = !p.exists();
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .map_err(|e| format!("không mở được {}: {e}", p.display()))?;
    let mut buf = String::new();
    if is_new {
        buf.push_str(HEADER);
    }
    let ts = utc_string(epoch);
    for (kind, name, moves) in entries {
        let _ = writeln!(buf, "{epoch}\t{ts}\t{kind}\t{}\t{moves}", sanitize(name));
    }
    f.write_all(buf.as_bytes())
        .map_err(|e| format!("không ghi được {}: {e}", p.display()))
}

/// Ghi thêm các thế vừa gặp. `entries` là danh sách (loại, tên thế, số nước).
/// Trả `Err` kèm mô tả để UI báo cho người dùng thay vì im lặng nuốt lỗi.
pub fn append(entries: &[(&str, String, usize)]) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let p = path().ok_or_else(|| "không xác định được thư mục chứa .exe".to_string())?;
    append_to(&p, entries, now_epoch())
}

/// Đếm số lần gặp từng thế trong ĐÚNG file này.
///
/// Dòng hỏng hoặc dòng tiêu đề bị BỎ QUA thay vì làm hỏng cả phép đếm —
/// file này người dùng có thể mở sửa tay, nên phải chịu được dữ liệu bẩn.
fn counts_from(p: &Path) -> HashMap<(String, String), u32> {
    let mut out = HashMap::new();
    let Ok(text) = std::fs::read_to_string(p) else { return out };
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 {
            continue;
        }
        let kind = cols[2];
        if kind != "OLL" && kind != "PLL" {
            continue; // bỏ dòng tiêu đề và dòng rác
        }
        *out.entry((kind.to_string(), cols[3].to_string())).or_insert(0) += 1;
    }
    out
}

/// Đếm số lần gặp từng thế. Khoá là (loại, tên thế).
pub fn counts() -> HashMap<(String, String), u32> {
    match path() {
        Some(p) => counts_from(&p),
        None => HashMap::new(),
    }
}

// ── Chống đếm trùng trong một ván ────────────────────────────────────

/// Quyết định ca OLL/PLL nào được ghi vào nhật ký trong ván hiện tại.
///
/// VÌ SAO CẦN: cả `Ctrl+T` lẫn `H` đều có thể nhận ra cùng một ca, và bấm
/// `H` nhiều lần ở cùng một thế sẽ trả về cùng một gợi ý. Không chặn thì
/// giữ phím H một lúc là số đếm nhảy vọt, thống kê thành vô nghĩa.
///
/// QUY TẮC: mỗi ca tính MỘT LẦN cho mỗi ván. Trong một ván vẫn ghi được
/// nhiều ca khác nhau — qua OLL rồi tới PLL là hai ca, cả hai đều tính.
///
/// VÌ SAO NẰM Ở ĐÂY chứ không nằm trong `main()`: trước đây nó là một
/// `HashSet` cục bộ giữa vòng lặp sự kiện, nên không cách nào gọi tới từ
/// test — mà đây lại chính là logic quyết định con số thống kê có nghĩa
/// hay không. Tách ra thành kiểu riêng thì test được, và `main()` chỉ còn
/// gọi hai phương thức.
#[derive(Default)]
pub struct CaseLog {
    da_ghi: std::collections::HashSet<(String, String)>,
}

impl CaseLog {
    /// Bắt đầu ván mới — xáo, hoặc nạp trạng thái mới từ camera.
    pub fn van_moi(&mut self) {
        self.da_ghi.clear();
    }

    /// `true` nếu ca này CHƯA được ghi trong ván hiện tại, tức là nên ghi.
    ///
    /// Gọi hàm này ĐÃ TÍNH là đã ghi: gọi lần hai với cùng ca trả `false`.
    /// Đặt tên `nen_ghi` chứ không phải `da_ghi` để chỗ gọi đọc xuôi:
    /// `if case_log.nen_ghi("OLL", name) { ...ghi... }`.
    pub fn nen_ghi(&mut self, loai: &str, ten: &str) -> bool {
        self.da_ghi.insert((loai.to_string(), ten.to_string()))
    }

    /// Số ca đã ghi trong ván hiện tại.
    pub fn so_ca(&self) -> usize {
        self.da_ghi.len()
    }
}

/// Tổng số lần đã ghi cho một loại.
pub fn total(counts: &HashMap<(String, String), u32>, kind: &str) -> u32 {
    counts.iter().filter(|((k, _), _)| k == kind).map(|(_, v)| *v).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// File tạm riêng cho từng test — test chạy song song nên không được
    /// dùng chung tên file.
    fn tmp(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("rubik_history_test_{tag}_{}.tsv", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn doi_gio_utc_dung_ke_ca_nam_nhuan() {
        // Mốc biên và các năm nhuận, gồm cả năm nhuận thế kỷ 2000.
        let cases: [(i64, &str); 8] = [
            (0, "1970-01-01 00:00:00"),
            (86_399, "1970-01-01 23:59:59"),
            (86_400, "1970-01-02 00:00:00"),
            (951_782_400, "2000-02-29 00:00:00"),
            (1_078_012_800, "2004-02-29 00:00:00"),
            (1_709_164_800, "2024-02-29 00:00:00"),
            (1_757_462_400, "2025-09-10 00:00:00"),
            (1_789_020_000, "2026-09-10 06:00:00"),
        ];
        for (e, want) in cases {
            assert_eq!(utc_string(e), want, "epoch {e}");
        }
    }

    #[test]
    fn ghi_tieu_de_dung_mot_lan_roi_noi_them() {
        let p = tmp("header");
        append_to(&p, &[("OLL", "OLL27".into(), 7)], 1_789_020_000).unwrap();
        append_to(&p, &[("PLL", "T".into(), 14)], 1_789_020_000).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.matches("epoch_giay").count(), 1, "tiêu đề bị ghi lại");
        assert_eq!(text.lines().count(), 3, "phải có tiêu đề + 2 dòng");
        assert!(text.contains("\tOLL\tOLL27\t7"));
        assert!(text.contains("\tPLL\tT\t14"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn danh_sach_rong_thi_khong_tao_file() {
        let p = tmp("empty");
        append_to(&p, &[], 0).unwrap();
        assert!(!p.exists(), "không được tạo file khi không có gì để ghi");
    }

    #[test]
    fn dem_dung_va_bo_qua_dong_rac() {
        let p = tmp("counts");
        append_to(&p, &[("OLL", "OLL27".into(), 7), ("PLL", "T".into(), 14)], 1).unwrap();
        append_to(&p, &[("OLL", "OLL27".into(), 7), ("PLL", "Ua".into(), 9)], 2).unwrap();
        // Giả lập người dùng mở file sửa tay rồi để lại rác.
        {
            let mut f = OpenOptions::new().append(true).open(&p).unwrap();
            writeln!(f, "rac").unwrap();
            writeln!(f, "cot\tthieu").unwrap();
            writeln!(f, "9\t2020-01-01 00:00:00\tXYZ\tKhong_phai_OLL\t3").unwrap();
        }
        let c = counts_from(&p);
        let g = |k: &str, n: &str| c.get(&(k.to_string(), n.to_string())).copied().unwrap_or(0);
        assert_eq!(g("OLL", "OLL27"), 2);
        assert_eq!(g("PLL", "T"), 1);
        assert_eq!(g("PLL", "Ua"), 1);
        assert_eq!(c.iter().filter(|((k, _), _)| k == "XYZ").count(), 0, "dòng rác lọt vào");
        assert_eq!(total(&c, "OLL"), 2);
        assert_eq!(total(&c, "PLL"), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn file_khong_ton_tai_thi_dem_ra_rong() {
        let p = tmp("missing");
        assert!(counts_from(&p).is_empty());
    }

    // ── CaseLog: logic quyết định con số thống kê ────────────────────
    //
    // Đây chính là phần trước đây nằm kẹt trong `fn main()` nên không test
    // được. Bốn test dưới là bốn tình huống thật khi dùng app.

    #[test]
    fn bam_H_nhieu_lan_chi_dem_mot() {
        let mut l = CaseLog::default();
        assert!(l.nen_ghi("OLL", "OLL27"), "lần đầu gặp thì phải ghi");
        assert!(!l.nen_ghi("OLL", "OLL27"), "lần hai không được ghi nữa");
        assert!(!l.nen_ghi("OLL", "OLL27"), "lần ba cũng vậy");
        assert_eq!(l.so_ca(), 1);
    }

    #[test]
    fn ctrl_t_roi_bam_H_khong_dem_hai_lan() {
        // Ctrl+T ghi cả OLL lẫn PLL của ván. Sau đó bấm H ở thế OLL thì
        // ra đúng ca đó -- không được tính thêm.
        let mut l = CaseLog::default();
        assert!(l.nen_ghi("OLL", "OLL27"));
        assert!(l.nen_ghi("PLL", "T"));
        assert!(!l.nen_ghi("OLL", "OLL27"), "H sau Ctrl+T không được đếm lại");
        assert_eq!(l.so_ca(), 2);
    }

    #[test]
    fn mot_van_van_ghi_duoc_nhieu_ca_khac_nhau() {
        // Qua OLL rồi tới PLL là hai ca khác nhau, cả hai đều phải tính.
        // Và OLL với PLL trùng tên vẫn là hai ca khác nhau.
        let mut l = CaseLog::default();
        assert!(l.nen_ghi("OLL", "OLL27"));
        assert!(l.nen_ghi("PLL", "T"));
        assert!(l.nen_ghi("OLL", "OLL21"));
        assert!(l.nen_ghi("PLL", "OLL27"), "khác loại thì là ca khác");
        assert_eq!(l.so_ca(), 4);
    }

    #[test]
    fn van_moi_thi_dem_lai_tu_dau() {
        let mut l = CaseLog::default();
        assert!(l.nen_ghi("OLL", "OLL27"));
        assert!(!l.nen_ghi("OLL", "OLL27"));
        l.van_moi(); // xáo, hoặc nạp trạng thái mới từ camera
        assert_eq!(l.so_ca(), 0);
        assert!(l.nen_ghi("OLL", "OLL27"), "ván mới thì gặp lại phải tính lại");
    }

    #[test]
    fn ten_co_tab_khong_lam_lech_cot() {
        let p = tmp("tab");
        append_to(&p, &[("OLL", "co\ttab\ntrong ten".into(), 5)], 3).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.lines().count(), 2, "tên có tab/xuống dòng làm vỡ dòng");
        let c = counts_from(&p);
        assert_eq!(total(&c, "OLL"), 1, "đếm vẫn phải ra 1");
        let _ = std::fs::remove_file(&p);
    }
}

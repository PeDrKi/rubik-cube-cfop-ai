//! Trạng thái toàn ứng dụng (`App`) và dữ liệu theo từng khung hình (`Req`).
//!
//! Trước đây mọi thứ nằm trong một hàm `main` ~1900 dòng, các biến cục bộ được vòng lặp sự kiện
//! "bắt" vào closure. Giờ các biến đó là trường của `App`; từng phần xử lý nằm ở một module riêng
//! (ui_*.rs, input.rs, jobs.rs, actions.rs, scene.rs) dưới dạng phương thức của `App`.

use crate::solve_flow::{JobResult, PendingMove};
use crate::{history, paint_ui, scan_ui};
use rubik_core::cube::CubeState;
use std::collections::{HashMap, VecDeque};
use three_d::*;

/// Số bước hoàn tác tối đa.
pub(crate) const UNDO_MAX: usize = 120;

pub(crate) struct App {
    pub(crate) state: CubeState,
    pub(crate) pending: Option<PendingMove>,
    pub(crate) queue: VecDeque<(char, bool, bool)>,
    pub(crate) status: String,
    pub(crate) formula_text: String,
    pub(crate) hint_label: Option<String>,
    pub(crate) scramble_formula: Option<Vec<String>>,
    pub(crate) hint_moves: Vec<String>,
    pub(crate) undo_stack: Vec<CubeState>,
    pub(crate) is_fullscreen: bool,
    pub(crate) scan_state: scan_ui::ScanState,
    pub(crate) paint_state: paint_ui::PaintState,
    pub(crate) job_rx: Option<std::sync::mpsc::Receiver<JobResult>>,
    pub(crate) move_count: u32,
    pub(crate) last_hint_stage: Option<&'static str>,
    pub(crate) last_hint_move_count: Option<u32>,
    pub(crate) last_hint_failed: bool,
    pub(crate) retry_counter: u64,
    pub(crate) anim_speed: f32,
    pub(crate) needs_redraw: bool,
    pub(crate) sel_face: Option<char>,
    pub(crate) total_queued: usize,
    pub(crate) formula_table: Option<(CubeState, rubik_core::breakdown::Breakdown)>,
    pub(crate) formula_open: bool,
    pub(crate) stats_open: bool,
    pub(crate) stats_counts: HashMap<(String, String), u32>,
    pub(crate) stats_tab_pll: bool,
    pub(crate) case_log: history::CaseLog,
    pub(crate) library_open: bool,
    pub(crate) shortcuts_open: bool,
    pub(crate) library_query: String,
    pub(crate) library_tab_pll: bool,
    pub(crate) auto_apply_hint: bool,
    pub(crate) ctrl_held: bool,
    pub(crate) ctrl_combo_used: bool,
    pub(crate) alt_held: bool,
    pub(crate) alt_combo_used: bool,
    pub(crate) alt_step_pending: bool,
    pub(crate) auto_solve: bool,
    pub(crate) auto_plan_boundaries: VecDeque<(usize, String, Vec<String>)>,
    pub(crate) auto_plan_start_count: u32,
    pub(crate) camera: Camera,
    pub(crate) control: OrbitControl,
    pub(crate) context: Context,
    pub(crate) gl: WindowedContext,
    pub(crate) winit_window: winit::window::Window,
    pub(crate) ambient: AmbientLight,
    pub(crate) sun: DirectionalLight,
}

/// Dữ liệu của MỘT khung hình: các yêu cầu do giao diện đặt ra, xử lý sau khi vẽ xong giao diện.
pub(crate) struct Req {
    pub(crate) submit_formula: bool,
    pub(crate) editing_formula: bool,
    pub(crate) request_hint: bool,
    pub(crate) apply_hint: bool,
    pub(crate) request_solve: bool,
    pub(crate) request_scramble: bool,
    pub(crate) close_formula: bool,
    pub(crate) request_table: bool,
    pub(crate) request_stats: bool,
    pub(crate) close_stats: bool,
    pub(crate) close_library: bool,
    pub(crate) load_into_bar: Option<String>,
    pub(crate) scanned_state: Option<CubeState>,
    pub(crate) state_source_label: &'static str,
    pub(crate) is_busy: bool,
    pub(crate) is_animating: bool,
    pub(crate) is_solved_now: bool,
    /// Bề rộng THỰC TẾ của panel trái sau khi vẽ (panel kéo giãn được bằng tay).
    pub(crate) actual_left_w: f32,
    pub(crate) left_w: f32,
    pub(crate) gui_ppp: f32,
}

impl App {
    pub(crate) fn new(
        winit_window: winit::window::Window,
        gl: WindowedContext,
        context: Context,
        camera: Camera,
        control: OrbitControl,
    ) -> Self {
        // Xây trước mọi bảng tra/PDB cho gợi ý trên THREAD NỀN để cửa sổ hiện ra ngay (xem rubik_core::hint::warmup()).
        std::thread::spawn(rubik_core::hint::warmup);
        let ambient = AmbientLight::new(&context, 0.5, Srgba::WHITE);
        let sun = DirectionalLight::new(&context, 1.2, Srgba::WHITE, &vec3(-0.45, -0.85, -0.65));
        App {
            state: CubeState::solved(),
            pending: None,
            queue: VecDeque::new(),
            status: "Sẵn sàng".to_string(),
            formula_text: String::new(),
            hint_label: None,
            // Công thức của lần xáo GẦN NHẤT (giữ nguyên cho tới lần xáo kế tiếp,
            // không bị hint/giải tự động ghi đè vì đây là biến riêng).
            scramble_formula: None,
            hint_moves: Vec::new(),
            undo_stack: Vec::new(),
            is_fullscreen: false,
            scan_state: scan_ui::ScanState::default(),
            paint_state: paint_ui::PaintState::default(),

            // ── Job nen (Enter=giai / H=goi y) chay tren thread rieng, khong chan
            // khung hinh -- ket qua nhan qua channel, kiem tra khong-blocking moi
            // frame (try_recv). job_rx.is_some() <=> dang co job chay (giong
            // cfop_busy ben Python: chi 1 job tai 1 thoi diem).
            job_rx: None,
            move_count: 0,
            last_hint_stage: None,
            last_hint_move_count: None,
            last_hint_failed: false,
            retry_counter: 0,
            anim_speed: 10.0,
            // Cờ cho MainEventsCleared biết có cần vẽ khung mới hay không.
            needs_redraw: true,
            sel_face: None,
            total_queued: 0,
            // Bảng công thức (phím T): None = chưa tính lần nào.
            //
            // Nhớ kèm TRẠNG THÁI đã dùng để tính. Trước đây chỉ giữ mỗi bảng, nên
            // sau lần tính đầu tiên, bấm "Bảng công thức" chỉ bật/tắt cửa sổ và
            // KHÔNG tính lại — khối đã xoay mà bảng vẫn là bảng cũ. Có trạng thái
            // đi kèm thì so sánh được: khác thì tính lại, giống thì chỉ mở ra.
            // Việc này cũng làm cho nhật ký không đếm trùng một ván nhiều lần.
            formula_table: None,
            formula_open: false,
            // Cửa sổ thống kê OLL/PLL (Ctrl+Y). `stats_counts` được đọc lại từ file
            // mỗi lần mở, để thấy ngay số vừa ghi.
            stats_open: false,
            stats_counts: std::collections::HashMap::new(),
            stats_tab_pll: false,
            // Các ca ĐÃ GHI nhật ký cho ván hiện tại, dạng (loại, tên ca).
            //
            // Vì sao cần: bấm H nhiều lần ở cùng một thế OLL sẽ trả về cùng một
            // gợi ý, và bấm Ctrl+T rồi bấm H cũng ra cùng ca đó. Không chặn thì
            // giữ phím H một lúc là số đếm nhảy vọt, thống kê thành vô nghĩa.
            //
            // Mỗi ca chỉ tính MỘT LẦN cho mỗi ván. Trong một ván vẫn ghi được
            // nhiều ca khác nhau (qua OLL rồi tới PLL là hai ca). Danh sách được
            // xoá khi bắt đầu ván mới — xáo, hoặc nạp trạng thái mới từ camera.
            case_log: history::CaseLog::default(),
            // Bảng tra công thức chuẩn (57 OLL + 21 PLL) — phím V.
            library_open: false,
            shortcuts_open: false,
            library_query: String::new(),
            library_tab_pll: false,
            auto_apply_hint: false,
            // Theo dõi phím Ctrl để dùng làm phím tắt "thực hiện gợi ý" (thay cho
            // phải bấm chuột vào nút "Thực hiện gợi ý này" sau khi đã bấm H).
            // Ctrl không phải là một `Key` riêng trong three-d — nó chỉ đổi
            // Modifiers, báo qua Event::ModifiersChange — nên phải tự bắt biến cố
            // nhấn/thả và tự loại trừ trường hợp Ctrl đang được giữ để bấm một tổ
            // hợp KHÁC (Ctrl+T/J/K/Y/Z...). Chỉ tính là "bấm Ctrl để thực hiện gợi
            // ý" khi THẢ Ctrl ra mà không có phím nào khác đi kèm lúc đang giữ.
            ctrl_held: false,
            ctrl_combo_used: false,
            // Alt: y het co che cua Ctrl o tren, nhung thay vi THUC HIEN CA gop
            // y dang hien, moi lan bam-tha Alt sach chi THUC HIEN 1 nuoc DAU
            // TIEN trong danh sach nuoc cua goi y do (VD goi y la "U U' R R2" thi
            // bam Alt lan 1 -> chay U, lan 2 -> chay U', lan 3 -> chay R,...).
            // Khi danh sach nuoc cua goi y hien tai da het, lan bam Alt tiep theo
            // moi tinh mot goi y MOI (giong H) roi tu dong chay nuoc dau cua goi
            // y moi do. Panel goi y (hint_label/hint_moves) van hien thi binh
            // thuong nen luon thay duoc phan con lai cua cong thuc sau moi lan
            // bam.
            alt_held: false,
            alt_combo_used: false,
            alt_step_pending: false,
            // Enter / nut "Giai tu dong": thay vi tinh nguyen loi giai roi chay 1
            // lan (khong thay duoc dang giai buoc nao), gio DUA VAO CHINH co che
            // hint (H): moi lan tinh xong 1 buoc (Cross / 1 cap F2L / OLL / PLL)
            // thi HIEN THI cong thuc do (giong H) VA chay het buoc do, xong roi
            // tu dong tinh tiep buoc ke tiep -- lap lai cho toi khi giai xong.
            // Co flag nay bat = dang trong vong lap do; tat khi giai xong hoac
            // khi nguoi dung huy (Esc) / hoan tac (Ctrl+Z).
            auto_solve: false,
            // "Giải tự động" (Enter) giờ tính NGUYÊN lời giải 1 LẦN DUY NHẤT
            // trước khi chạy nước nào (dùng lại `breakdown::full_solve_breakdown`
            // -- đúng hàm build "Bảng công thức"), rồi xếp HẾT các nước vào
            // `queue` luôn một thể -- thay vì tính từng chặng một xen kẽ lúc
            // animate như bản trước (dù đã có "tính trước" ở giữa các chặng,
            // chặng nào tính lâu hơn animate chặng trước vẫn lộ ra khoảng dừng).
            // Cách này chạy liền mạch tuyệt đối một khi đã bắt đầu di chuyển --
            // đánh đổi bằng 1 khoảng chờ (có spinner) DUY NHẤT lúc đầu thay vì
            // nhiều khoảng chờ nhỏ rải rác giữa chừng.
            //
            // `auto_plan_boundaries`: các chặng CÒN LẠI (sau chặng đầu, đã hiện
            // ngay khi vừa xếp xong) kèm mốc "đã chạy xong bao nhiêu nước kể từ
            // lúc bắt đầu thì tới lượt hiện nhãn chặng này" -- chỉ để ĐỔI NHÃN
            // hiển thị (hint_label/hint_moves) đúng lúc cube thực sự bắt đầu
            // chặng đó, không cần tính toán gì thêm.
            auto_plan_boundaries: std::collections::VecDeque::new(),
            auto_plan_start_count: 0,
            camera,
            control,
            context,
            gl,
            winit_window,
            ambient,
            sun,
        }
    }

    /// Lưu 1 snapshot trạng thái HIỆN TẠI vào undo_stack, kèm giới hạn độ sâu. Gọi đúng 1 lần cho MỖI "hành động"
    /// (1 nước tay = 1 lần; cả công thức / giải / gợi ý = 1 lần cho CẢ khối).
    pub(crate) fn push_undo(&mut self) {
        self.undo_stack.push(self.state);
        if self.undo_stack.len() > UNDO_MAX {
            self.undo_stack.remove(0);
        }
    }

    /// Tính tỉ lệ giao diện theo cỡ cửa sổ (chuẩn: cao 800px = 1.0) và khởi tạo dữ liệu khung hình.
    pub(crate) fn begin_frame(&self, frame_input: &FrameInput) -> Req {
        let dpr = frame_input.device_pixel_ratio;
        let win_h_logical = frame_input.viewport.height as f32 / dpr;
        let win_w_logical = frame_input.viewport.width as f32 / dpr;
        let ui_scale = (win_h_logical / 800.0).clamp(0.80, 1.60);
        let gui_ppp = dpr * ui_scale;
        // Panel rộng theo tỉ lệ cửa sổ, có chặn trên/dưới.
        let left_w = (win_w_logical / ui_scale * 0.30).clamp(300.0, 430.0);
        Req {
            submit_formula: false,
            editing_formula: false,
            request_hint: false,
            apply_hint: false,
            request_solve: false,
            request_scramble: false,
            close_formula: false,
            request_table: false,
            request_stats: false,
            close_stats: false,
            close_library: false,
            load_into_bar: None,
            scanned_state: None,
            state_source_label: "camera",
            is_busy: self.job_rx.is_some(),
            is_animating: self.pending.is_some(),
            is_solved_now: self.state.is_solved(),
            actual_left_w: left_w,
            left_w,
            gui_ppp,
        }
    }
}

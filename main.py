"""
main.py — Rubik's Cube Simulator
6-Face View (cột trái) và 3D View (cột phải).
Tích hợp CFOP AI (Cross + F2L, phase MVP): phím A = auto-solve, H = hint,
Tab = copy nhanh gợi ý hiện tại xuống thanh công thức (Singmaster),
T = mở/đóng bảng công thức tham khảo Cross→F2L→OLL→PLL.
"""

import pygame
from pygame.locals import *
import math
import copy
import sys
import time
import threading
import queue

# ── Cửa sổ OS riêng cho bảng công thức (THỬ NGHIỆM / best-effort) ──────────
# pygame >= 2.5 hỗ trợ đa cửa sổ qua pygame._sdl2.video.Window, nhưng hành
# vi có thể khác nhau giữa các hệ điều hành/driver và KHÔNG được test trực
# tiếp ở đây. Nếu import/tạo cửa sổ thất bại bất kỳ lúc nào, toàn bộ tính
# năng tự vô hiệu hoá và app quay lại dùng panel nổi (docked) như bình
# thường -- không bao giờ làm crash app chính.
try:
    from pygame._sdl2.video import Window as _SDLWindow
except Exception:
    _SDLWindow = None

from constants   import FPS, BG, GOLD, HINT, DIV
from layout      import Layout
from cube_engine import (make_solved, scramble_cube, cube_solved,
                         do_move, parse_singmaster)
from renderer_3d import Rx, Ry, draw_cube_3d, hit_test_3d, ANIMATABLE_BASES
from draw_helpers import draw_panels, panel_hit, draw_bar
from solver import cfop_ai


ANIM_DUR      = 0.13   # giây, 3D move animation (ở tốc độ x1)
BAR_MAX_LEN   = 80      # giới hạn ký tự Singmaster bar, tránh queue move khổng lồ
UNDO_MAX      = 120     # giới hạn độ sâu undo
MIN_W, MIN_H  = 800, 520   # kích thước cửa sổ tối thiểu, tránh UI vỡ hình khi resize nhỏ
SPEED_STEPS   = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0]   # các mức tốc độ animation
SPEED_DEFAULT_IDX = SPEED_STEPS.index(1.0)


def _clipboard_set(text):
    """
    Copy `text` (str, có thể chứa tiếng Việt có dấu) vào clipboard hệ
    thống, ưu tiên cách ĐẢM BẢO ĐÚNG UNICODE trên từng nền tảng thay vì chỉ
    dùng pygame.scrap (trên Windows, pygame.scrap ánh xạ SCRAP_TEXT sang
    CF_TEXT/ANSI -- ghi thẳng bytes UTF-8 vào đó khiến ứng dụng dán ra hiểu
    nhầm theo bảng mã ANSI/CP1252, gây lỗi hiển thị kiểu "cáº·p" thay vì
    "cặp"). Trả về True nếu copy thành công.
    """
    if sys.platform.startswith('win'):
        try:
            import ctypes
            CF_UNICODETEXT = 13
            GMEM_MOVEABLE  = 0x0002
            data = (text.replace('\n', '\r\n') + '\0').encode('utf-16-le')
            kernel32 = ctypes.windll.kernel32
            user32   = ctypes.windll.user32
            h = kernel32.GlobalAlloc(GMEM_MOVEABLE, len(data))
            ptr = kernel32.GlobalLock(h)
            ctypes.memmove(ptr, data, len(data))
            kernel32.GlobalUnlock(h)
            user32.OpenClipboard(0)
            user32.EmptyClipboard()
            user32.SetClipboardData(CF_UNICODETEXT, h)
            user32.CloseClipboard()
            return True
        except Exception:
            pass   # rot xuong thu pygame.scrap ben duoi

    try:
        pygame.scrap.put(pygame.SCRAP_TEXT, text.encode('utf-8'))
        return True
    except Exception:
        return False


# ── Bảng công thức CFOP (Cross -> F2L -> OLL -> PLL), hiển thị khi bấm T ────
# Khác với 1 bảng tra cứu tĩnh: nội dung được TÍNH TỪ TRẠNG THÁI CUBE HIỆN
# TẠI (xem cfop_ai.full_solve_breakdown), nên mỗi bước hiện đúng chuỗi nước
# đi cần làm để đi tới khi hoàn thành; bước nào đã xong sẵn ghi "DONE!".
_ROW_GREEN = (110, 230, 140)   # DONE!
_ROW_AMBER = (235, 180, 90)    # that bai trong ngan sach hien tai
_ROW_GREY  = (140, 140, 175)   # chua toi luot
_ROW_WHITE = (225, 225, 235)   # co chuoi nuoc di can lam


def _stage_row(label, info):
    """1 dong (kind='row') the hien 1 buoc/1 cap F2L, dua tren dict
    {'status': 'done'|'moves'|'failed'|'not_reached', 'moves': [...]}."""
    status = info.get('status')
    if status == 'done':
        return ('row', label, 'DONE!', _ROW_GREEN)
    if status == 'moves':
        return ('row', label, ' '.join(info['moves']), _ROW_WHITE)
    if status == 'failed':
        return ('row', label,
                 '(chưa tìm được trong ngân sách hiện tại -- mở lại bảng (T) để thử lại)',
                 _ROW_AMBER)
    return ('row', label, '(chưa tới lượt -- cần xong bước trước)', _ROW_GREY)


def build_formula_lines(breakdown):
    """Tra ve danh sach (kind, *data) mo ta noi dung bang cong thuc CFOP tu
    KET QUA cfop_ai.full_solve_breakdown(state), dung de ve trong panel
    bat/tat bang phim T. kind:
      'header' -> (kind, tieu de)
      'row'    -> (kind, nhan, noi dung, mau)
      'spacer' -> (kind,)  dong trong nho de tach nhom
    """
    lines = []
    lines.append(('header', "1) CROSS"))
    lines.append(_stage_row('Cross', breakdown['cross']))
    lines.append(('spacer',))

    lines.append(('header', "2) F2L  (4 cặp góc-cạnh)"))
    for slot in ('DFR', 'DFL', 'DBR', 'DBL'):
        lines.append(_stage_row(slot, breakdown['f2l'][slot]))
    lines.append(('spacer',))

    lines.append(('header', "3) OLL  (Orientation of Last Layer)"))
    lines.append(_stage_row('OLL', breakdown['oll']))
    lines.append(('spacer',))

    lines.append(('header', "4) PLL  (Permutation of Last Layer)"))
    lines.append(_stage_row('PLL', breakdown['pll']))

    return lines


def _formula_lines_to_text(formula_lines):
    """Chuyen danh sach (kind, ...) tu build_formula_lines() thanh 1 doan
    van ban thuong (plain text), dung de copy toan bo noi dung bang cong
    thuc vao clipboard he thong (vi ban than cua so pygame khong ho tro
    boi den/chon van ban truc tiep tren canvas ve)."""
    out = []
    for row in formula_lines:
        kind = row[0]
        if kind == 'header':
            out.append(row[1])
        elif kind == 'row':
            _, label, text, _color = row
            out.append(f"{label}: {text}")
        elif kind == 'spacer':
            out.append('')
    return '\n'.join(out)


def _formula_row_data(formula_lines):
    """Tra ve 2 list SONG SONG (cung thu tu/row_idx ma
    _draw_formula_panel_content() dung de gan nhan khi ve):
      labels   -- ["Cross", "DFR", "DFL", ..., "PLL"]
      formulas -- ["U' F2 B2 L D", "DONE!", "R U R'...", ...]  (CHỈ nội
                  dung, KHÔNG kèm nhãn -- dùng để copy/áp dụng đúng 1 công
                  thức mà không dính chữ "Cross: "/"DFR: " ở đầu).
    """
    labels, formulas = [], []
    for row in formula_lines:
        if row[0] == 'row':
            _, label, text, _color = row
            labels.append(label)
            formulas.append(text)
    return labels, formulas


def _looks_like_moves(text):
    """True neu `text` la 1 chuoi nuoc di Singmaster that su (khong phai
    'DONE!' hay ghi chu dang '(...)'), dung de an/hien nut 'Ap dung -> bar'."""
    t = (text or '').strip()
    return bool(t) and t != 'DONE!' and not t.startswith('(')


def _wrap_text(text, font, max_w):
    """Chia `text` thanh nhieu dong sao cho moi dong vua voi chieu rong
    max_w khi render bang `font` (wrap theo tu, cach nhau boi dau cach --
    phu hop voi chuoi Singmaster vi moi nuoc di la 1 'tu'). Luon tra ve
    it nhat 1 dong (co the rong)."""
    words = text.split(' ')
    lines = []
    cur = ''
    for w in words:
        trial = w if not cur else cur + ' ' + w
        if not cur or font.size(trial)[0] <= max_w:
            cur = trial
        else:
            lines.append(cur)
            cur = w
    lines.append(cur)
    return lines


def _draw_formula_panel_content(surface, rect, fonts, formula_lines, scroll, gold_color,
                                 selected_row=None, highlight_color=(90, 74, 20)):
    """
    Vẽ NỘI DUNG (header/rows, có scroll+clip) của bảng công thức vào
    `surface`, giới hạn trong `rect`. KHÔNG vẽ khung ngoài / nút đóng / nút
    cuộn -- những phần đó do nơi gọi tự vẽ. Tách riêng để dùng chung cho cả
    panel nổi (docked, không modal) và -- nếu tạo được -- 1 cửa sổ OS riêng.

    Mỗi dòng công thức ('row' -- vd Cross/DFR/OLL/PLL) có 1 chỉ số thứ tự
    (row_idx, đếm theo thứ tự xuất hiện). Nếu `selected_row` khớp row_idx
    của 1 dòng, dòng đó (và các dòng phụ do wrap dài) được TÔ NỀN để thể
    hiện "đã chọn" (thay thế cho việc bôi đen văn bản thật không khả dụng
    trên canvas pygame).

    fonts: dict {'header': Font, 'row': Font, 'name_col_w': int,
                 'row_gap': int, 'spacer_h': int}
    Trả về (scroll_max, scroll_da_duoc_gioi_han, hit_rows) trong đó
    hit_rows là list [(pygame.Rect_toa_do_man_hinh, row_idx), ...] cho MỌI
    dòng 'row'/'rowcont' đã vẽ (kể cả nằm ngoài rect do đã cuộn) -- nơi gọi
    tự lọc theo rect khi hit-test click.
    """
    bfont, mfont   = fonts['header'], fonts['row']
    name_col_w     = fonts['name_col_w']
    row_gap        = fonts['row_gap']
    spacer_h       = fonts['spacer_h']
    text_x0        = rect.x + 10 + name_col_w
    wrap_w         = max(50, rect.width - 10 - name_col_w - 10)

    draw_items = []
    row_idx = -1
    for row in formula_lines:
        kind = row[0]
        if kind == 'header':
            h = bfont.get_height() + 14
            draw_items.append({'kind': 'header', 'text': row[1], 'h': h})
        elif kind == 'spacer':
            draw_items.append({'kind': 'spacer', 'h': spacer_h})
        elif kind == 'row':
            row_idx += 1
            _, label, text, color = row
            wrapped = _wrap_text(text, mfont, wrap_w)
            h0 = mfont.get_height() + row_gap
            draw_items.append({'kind': 'row', 'label': label, 'row_idx': row_idx,
                                'text': wrapped[0], 'color': color, 'h': h0})
            for extra in wrapped[1:]:
                draw_items.append({'kind': 'rowcont', 'text': extra, 'row_idx': row_idx,
                                    'color': color, 'h': h0})

    total_h    = sum(it['h'] for it in draw_items)
    scroll_max = max(0, total_h - rect.height)
    scroll     = max(0, min(scroll, scroll_max))

    hit_rows  = []
    prev_clip = surface.get_clip()
    surface.set_clip(rect)
    y = rect.y - scroll
    for it in draw_items:
        h = it['h']
        if it['kind'] in ('row', 'rowcont'):
            hit_rows.append((pygame.Rect(rect.x, y, rect.width, h), it['row_idx']))
        if y + h >= rect.y and y <= rect.bottom:
            if it['kind'] == 'header':
                txt = bfont.render(it['text'], True, gold_color)
                surface.blit(txt, (rect.x, y))
            elif it['kind'] == 'row':
                if it['row_idx'] == selected_row:
                    pygame.draw.rect(surface, highlight_color, (rect.x, y, rect.width, h))
                n_txt = mfont.render(it['label'], True, gold_color)
                s_txt = mfont.render(it['text'], True, it['color'])
                surface.blit(n_txt, (rect.x + 10, y))
                surface.blit(s_txt, (text_x0, y))
            elif it['kind'] == 'rowcont':
                if it['row_idx'] == selected_row:
                    pygame.draw.rect(surface, highlight_color, (rect.x, y, rect.width, h))
                s_txt = mfont.render(it['text'], True, it['color'])
                surface.blit(s_txt, (text_x0, y))
        y += h
    surface.set_clip(prev_clip)
    return scroll_max, scroll, hit_rows


def _try_open_formula_window():
    """Cố tạo 1 cửa sổ OS THẬT SỰ riêng (dùng pygame._sdl2.video.Window) để
    hiển thị bảng công thức, độc lập với cửa sổ chính. TÍNH NĂNG THỬ
    NGHIỆM (best-effort): nếu thất bại ở bất kỳ bước nào (import không có,
    driver không hỗ trợ đa cửa sổ, v.v.), trả về (None, None) -- nơi gọi sẽ
    tự động dùng panel nổi (docked) thay thế, KHÔNG làm crash app.
    """
    if _SDLWindow is None:
        return None, None
    try:
        win  = _SDLWindow("Công thức CFOP - Rubik AI", size=(460, 720), resizable=True)
        surf = win.get_surface()
        return win, surf
    except Exception:
        return None, None


def _bar_index_at_x(text, mouse_x, text_start_x, font):
    """Tra ve vi tri ky tu (0..len(text)) gan nhat voi toa do x cua chuot,
    dung khi bam/keo chuot trong Singmaster bar de dat con tro/chon text."""
    best_i = len(text)
    best_dist = abs(text_start_x + font.size(text)[0] - mouse_x)
    for i in range(len(text) + 1):
        w = font.size(text[:i])[0]
        dist = abs(text_start_x + w - mouse_x)
        if dist < best_dist:
            best_dist = dist
            best_i = i
    return best_i


def main():
    pygame.init()

    info   = pygame.display.Info()
    init_w = max(MIN_W, int(info.current_w * 0.92))
    init_h = max(MIN_H, int(info.current_h * 0.92))
    screen = pygame.display.set_mode((init_w, init_h), RESIZABLE)
    try:
        pygame.scrap.init()   # can cho copy/paste he thong (Ctrl+C/V) trong Singmaster bar
    except Exception:
        pass   # mot so moi truong (vd headless/sandbox) khong ho tro clipboard -- bo qua an toan
    pygame.display.set_caption(
        "Rubik's Cube — 6-Face · 3D  [F11=fullscreen]"
    )
    clock      = pygame.time.Clock()
    fullscreen = False

    def make_layout():
        w, h = screen.get_size()
        return Layout(w, h)

    lo = make_layout()

    # ── trạng thái cube ───────────────────────────────────────────────────────
    state      = make_solved()
    undo_stack = []
    state_version = 0   # tăng mỗi khi state bị mutate -> ép renderer 3D rebuild cache
    speed_idx  = SPEED_DEFAULT_IDX          # index vào SPEED_STEPS
    speed_flash = 0                          # đếm ngược frame hiển thị overlay tốc độ

    # ── Move counter & timer ──────────────────────────────────────────────────
    move_count   = 0      # số nước đã đi từ lần scramble gần nhất
    timer_start  = None   # time.perf_counter() khi bắt đầu solve
    timer_end    = None   # time.perf_counter() khi solved (None = đang chạy)
    scrambled    = False  # True sau khi scramble, False khi solved/reset

    # ── 3D view state ─────────────────────────────────────────────────────────
    yaw        = math.radians(-38)
    pitch      = math.radians(28)
    zoom       = lo.ZOOM0
    dragging3d = False
    drag_start = (0, 0)
    yaw0 = pitch0 = 0.0
    sel_face   = None

    anim_queue  = []
    anim_face   = None
    anim_ang    = 0.
    anim_target = 0.
    anim_t      = 0.
    anim_mv     = None
    solved_flash = 0
    undo_empty_flash = 0   # đếm ngược hiển thị "nothing to undo"

    # ── Singmaster bar ────────────────────────────────────────────────────────
    bar_text         = ''
    bar_active       = False
    bar_status       = None
    bar_status_timer = 0
    history          = []
    MAX_HIST         = 5
    bar_cursor       = 0      # vi tri con tro trong bar_text (0..len(bar_text))
    bar_sel_anchor   = None   # vi tri bat dau vung dang chon (None = khong chon gi)
    bar_dragging     = False  # dang giu chuot keo de chon text trong bar
    hint_copy_rect   = None   # pygame.Rect cua nut "Copy -> thanh cong thuc"
                               # (None neu khong co hint dang hien thi), duoc
                               # ve lai moi frame, dung de hit-test click.

    # ── Bảng công thức CFOP (mở/đóng bằng phím T) ─────────────────────────────
    # Panel KHÔNG modal: nổi (floating) ở góc trên-phải, không làm mờ/khoá
    # phần còn lại của app -- cube vẫn xoay/giải được bình thường khi panel mở.
    formula_panel_open   = False
    formula_scroll       = 0     # độ lệch cuộn hiện tại (px)
    formula_scroll_max   = 0     # cập nhật mỗi frame khi vẽ, dùng để giới hạn cuộn
    formula_lines        = []    # nội dung hiện tại, dựng lại mỗi lần bấm T
    formula_state_ver    = None  # state_version tại lúc tính xong -> phát hiện dữ liệu cũ
    formula_panel_rect   = None  # cập nhật mỗi frame khi vẽ, dùng hit-test chuột frame sau
    formula_close_rect   = None
    formula_up_rect      = None
    formula_down_rect    = None
    formula_copy_rect    = None   # nút "Copy" trên panel nổi -- hit-test frame sau
    formula_copy_note        = None   # thông báo "Đã copy..." hiển thị tạm thời
    formula_copy_note_timer  = 0
    formula_selected_row  = None  # row_idx (int) đang được "bôi đen"/chọn, hoặc None
    formula_row_labels    = []    # ["Cross", "DFR", ...] song song với row_idx
    formula_row_formulas  = []    # ["U' F2 B2 L D", "DONE!", ...] -- KHÔNG kèm nhãn
    formula_hit_rows      = []    # [(Rect, row_idx), ...] cập nhật mỗi frame -> hit-test click
    formula_apply_rect    = None  # nút "Áp dụng -> bar" -- chỉ hiện khi có dòng đang chọn
    formula_content_rect  = None  # vùng nội dung (không tính header/footer) -- hit-test click trống để bỏ chọn

    # ── Cửa sổ OS riêng (thử nghiệm) cho bảng công thức, xem show_formula_window() ──
    formula_window        = None   # pygame._sdl2.video.Window hoặc None nếu không tạo được
    formula_win_surface   = None
    formula_win_supported = True   # False nếu mở lần đầu thất bại -> không thử lại nữa
    formula_win_last_w    = -1     # để phát hiện đổi kích thước -> dựng lại font
    formula_win_fonts     = None

    # ── CFOP AI (Cross + F2L, chay nen bang thread de khong dong UI) ─────────
    cfop_busy       = False    # True trong khi thread AI dang tinh
    cfop_job_kind   = None     # 'solve' hoac 'hint'
    cfop_result_q   = queue.Queue()
    hint_label      = None     # nhan hien thi cua goi y gan nhat (vd "F2L - cap DFR")
    hint_moves_str  = None     # chuoi Singmaster cua goi y (khong tu dong thuc thi)
    cfop_note       = None     # thong bao ngan (vd "Da giai xong Cross+F2L")
    cfop_note_timer = 0
    # theo doi de nhan biet "bam H lai cho CUNG 1 giai doan vua that bai ma
    # cube CHUA doi gi" -> tim kiem von tat dinh nen phai xao tron nuoc di
    # (retry=True) moi co y nghia, khong thi se that bai y het lan truoc.
    last_hint_stage      = None
    last_hint_move_count = None
    last_hint_failed     = False

    def start_cfop_job(kind):
        nonlocal cfop_busy, cfop_job_kind
        if cfop_busy:
            return
        cfop_busy = True
        cfop_job_kind = kind
        snapshot = copy.deepcopy(state)

        retry = False
        if kind == 'hint':
            cur_stage = cfop_ai.stage_of(snapshot)   # re, khong search, an toan goi dong bo
            retry = (last_hint_failed and last_hint_stage == cur_stage
                     and last_hint_move_count == move_count)

        def worker():
            try:
                if kind == 'solve':
                    res = cfop_ai.full_solve(snapshot, retry=retry)
                elif kind == 'formula':
                    res = cfop_ai.full_solve_breakdown(snapshot, retry=retry)
                else:
                    res = cfop_ai.hint(snapshot, retry=retry)
                cfop_result_q.put((kind, res, None))
            except Exception as exc:
                cfop_result_q.put((kind, None, str(exc)))

        threading.Thread(target=worker, daemon=True).start()

    def apply_cfop_solution(moves):
        nonlocal move_count, timer_start, scrambled
        if not moves:
            return
        push_undo()
        anim_queue.extend(moves)
        move_count += len(moves)
        if scrambled and timer_start is None:
            timer_start = time.perf_counter()

    def Rm():
        return Ry(yaw) @ Rx(pitch)

    def push_undo():
        """Lưu state hiện tại vào undo_stack, áp dụng giới hạn UNDO_MAX nhất quán."""
        undo_stack.append(copy.deepcopy(state))
        if len(undo_stack) > UNDO_MAX:
            undo_stack.pop(0)

    def enqueue(mv):
        nonlocal move_count, timer_start, scrambled
        push_undo()
        anim_queue.append(mv)
        move_count += 1
        # Bắt đầu timer khi nước đầu tiên sau scramble
        if scrambled and timer_start is None:
            timer_start = time.perf_counter()

    def execute_bar():
        nonlocal bar_text, bar_status, bar_status_timer, bar_active
        nonlocal move_count, timer_start, scrambled
        moves, err = parse_singmaster(bar_text)
        if err or not moves:
            bar_status = 'error'; bar_status_timer = 120; return
        push_undo()
        for mv in moves:
            anim_queue.append(mv)
        move_count += len(moves)
        if scrambled and timer_start is None:
            timer_start = time.perf_counter()
        history.append(bar_text.strip())
        if len(history) > MAX_HIST:
            history.pop(0)
        bar_status = 'ok'; bar_status_timer = 90
        bar_text   = ''; bar_active = False

    # ══════════════════════════════════════════════════════════════════════════
    #  GAME LOOP
    # ══════════════════════════════════════════════════════════════════════════
    running = True
    while running:
        dt = clock.tick(FPS) / 1000.0

        if solved_flash > 0:
            solved_flash -= 1
        if undo_empty_flash > 0:
            undo_empty_flash -= 1
        if speed_flash > 0:
            speed_flash -= 1
        if bar_status_timer > 0:
            bar_status_timer -= 1
            if bar_status_timer == 0:
                bar_status = None
        if cfop_note_timer > 0:
            cfop_note_timer -= 1
            if cfop_note_timer == 0:
                cfop_note = None
        if formula_copy_note_timer > 0:
            formula_copy_note_timer -= 1
            if formula_copy_note_timer == 0:
                formula_copy_note = None

        try:
            kind, res, err = cfop_result_q.get_nowait()
        except queue.Empty:
            pass
        else:
            cfop_busy = False
            if err is not None:
                cfop_note = f'Lỗi AI: {err}'
                cfop_note_timer = 200
            elif kind == 'solve':
                apply_cfop_solution(res['all_moves'])
                msgs = {
                    'solved':      f"AI: 🎉 GIẢI XONG cả cube ({len(res['all_moves'])} nước)!",
                    'pll_partial': "AI: xong Cross+F2L+OLL, PLL còn dở (case khó, thử lại 'A')",
                    'oll_partial': "AI: xong Cross+F2L, OLL còn dở (case khó, thử lại 'A')",
                    'f2l_done':    f"AI: đã giải Cross + F2L ({len(res['all_moves'])} nước)",
                    'f2l_partial': "AI: giải được Cross + một phần F2L (cặp khó, thử lại 'A' lần nữa)",
                }
                cfop_note = msgs.get(res['reached'], f"AI: đã đi {len(res['all_moves'])} nước")
                cfop_note_timer = 220
            elif kind == 'formula':
                formula_lines       = build_formula_lines(res)
                formula_row_labels, formula_row_formulas = _formula_row_data(formula_lines)
                formula_selected_row = None   # dữ liệu mới -> bỏ chọn dòng cũ
                formula_scroll      = 0
                formula_state_ver   = state_version

                used_real_window = False
                if formula_win_supported and _SDLWindow is not None:
                    if formula_window is None:
                        formula_window, formula_win_surface = _try_open_formula_window()
                        if formula_window is None:
                            formula_win_supported = False   # thử 1 lần/phiên, tránh spam lỗi
                    if formula_window is not None:
                        used_real_window   = True
                        formula_win_last_w = -1   # ép dựng lại font ở frame kế tiếp

                # Chỉ dùng 1 trong 2 nơi hiển thị cùng lúc (tránh trùng lặp):
                # cửa sổ OS riêng nếu tạo được, ngược lại panel nổi (docked).
                formula_panel_open = not used_real_window
            else:   # hint
                hint_label = res['label']
                hint_moves_str = ' '.join(res['moves']) if res['moves'] else None
                last_hint_stage = res.get('stage')
                last_hint_move_count = move_count
                last_hint_failed = not res['moves']

        mx, my = pygame.mouse.get_pos()
        R = Rm()

        for ev in pygame.event.get():

            if ev.type == QUIT:
                running = False

            elif ev.type == VIDEORESIZE:
                new_w, new_h = max(MIN_W, ev.w), max(MIN_H, ev.h)
                if (new_w, new_h) != (ev.w, ev.h):
                    screen = pygame.display.set_mode((new_w, new_h), RESIZABLE)
                lo = make_layout()
                zoom_ratio = zoom / lo.ZOOM0
                zoom = lo.ZOOM0 * zoom_ratio

            elif (formula_panel_open and ev.type == MOUSEBUTTONDOWN
                  and ev.button in (4, 5)
                  and formula_panel_rect and formula_panel_rect.collidepoint(mx, my)):
                step = max(30, int(60 * lo.s))
                if ev.button == 4:
                    formula_scroll = max(0, formula_scroll - step)
                else:
                    formula_scroll = min(formula_scroll_max, formula_scroll + step)

            elif (formula_panel_open and ev.type == MOUSEBUTTONDOWN and ev.button == 1
                  and formula_panel_rect and formula_panel_rect.collidepoint(mx, my)):
                # Click nằm TRONG panel công thức -- xử lý nút đóng/cuộn tại đây.
                # Panel không modal: click ở NGOÀI panel (kể cả khi panel đang
                # mở) sẽ không rơi vào đây, mà rơi xuống nhánh cube/bar bên dưới
                # như bình thường -- đây chính là điều làm panel "không chặn".
                step = max(30, int(60 * lo.s))
                if formula_close_rect and formula_close_rect.collidepoint(mx, my):
                    formula_panel_open = False
                elif formula_copy_rect and formula_copy_rect.collidepoint(mx, my):
                    if formula_selected_row is not None and formula_selected_row < len(formula_row_formulas):
                        cp_txt  = formula_row_formulas[formula_selected_row]
                        cp_note = f"Đã copy: {formula_row_labels[formula_selected_row]}"
                    else:
                        cp_txt  = _formula_lines_to_text(formula_lines)
                        cp_note = "Đã copy toàn bộ công thức vào clipboard!"
                    formula_copy_note = (cp_note if _clipboard_set(cp_txt) else
                                          "Không copy được (clipboard không khả dụng ở máy này)")
                    formula_copy_note_timer = 150
                elif formula_apply_rect and formula_apply_rect.collidepoint(mx, my):
                    # Áp dụng công thức đang chọn thẳng vào thanh Singmaster
                    # (giống nút "Copy -> bar" của gợi ý H) để xem/sửa rồi
                    # Enter thực thi trên cube.
                    if (formula_selected_row is not None
                            and formula_selected_row < len(formula_row_formulas)
                            and _looks_like_moves(formula_row_formulas[formula_selected_row])):
                        bar_text       = formula_row_formulas[formula_selected_row][:BAR_MAX_LEN]
                        bar_active     = True
                        bar_status     = None
                        bar_cursor     = len(bar_text)
                        bar_sel_anchor = None
                elif formula_up_rect and formula_up_rect.collidepoint(mx, my):
                    formula_scroll = max(0, formula_scroll - step)
                elif formula_down_rect and formula_down_rect.collidepoint(mx, my):
                    formula_scroll = min(formula_scroll_max, formula_scroll + step)
                elif formula_content_rect and formula_content_rect.collidepoint(mx, my):
                    # Click vào NỘI DUNG (không phải nút) -- chọn/bỏ chọn 1
                    # dòng công thức (Cross/DFR/.../PLL) theo vị trí click.
                    # Đây là cách "bôi đen" 1 công thức trên canvas pygame
                    # (không hỗ trợ kéo-chọn văn bản thật như ứng dụng thường).
                    clicked_idx = None
                    for hit_rect, hit_idx in formula_hit_rows:
                        if hit_rect.collidepoint(mx, my):
                            clicked_idx = hit_idx
                            break
                    formula_selected_row = (
                        None if clicked_idx == formula_selected_row else clicked_idx)

            elif ev.type == MOUSEBUTTONDOWN and ev.button == 1:
                if hint_copy_rect and hint_copy_rect.collidepoint(mx, my):
                    # Chep gợi ý hien tai vao thanh Singmaster va focus vao do
                    # de nguoi dung xem/sua truoc khi bam Enter thuc thi.
                    bar_text       = hint_moves_str[:BAR_MAX_LEN]
                    bar_active     = True
                    bar_status     = None
                    bar_cursor     = len(bar_text)
                    bar_sel_anchor = None

                elif lo.bar_rect().collidepoint(mx, my):
                    bar_active     = True
                    text_start_x   = lo.BAR_X + max(60, int(130 * lo.s))
                    bar_cursor     = _bar_index_at_x(bar_text, mx, text_start_x, lo.mfont)
                    bar_sel_anchor = bar_cursor   # bat dau 1 vung chon rong -- keo chuot se mo rong
                    bar_dragging   = True

                elif lo.AREA_3D.collidepoint(mx, my):
                    bar_active = False
                    hf = hit_test_3d(mx, my, R, zoom, lo)
                    if hf:
                        sel_face = hf
                    else:
                        dragging3d = True
                        drag_start = (mx, my)
                        yaw0, pitch0 = yaw, pitch

                else:
                    bar_active = False
                    pf = panel_hit(mx, my, lo)
                    if pf:
                        sel_face = pf

            elif ev.type == MOUSEBUTTONDOWN:
                if ev.button == 4 and lo.AREA_3D.collidepoint(mx, my):
                    zoom = min(zoom * 1.1, 280 * lo.s)
                elif ev.button == 5 and lo.AREA_3D.collidepoint(mx, my):
                    zoom = max(zoom / 1.1, 45 * lo.s)

            elif ev.type == MOUSEBUTTONUP and ev.button == 1:
                dragging3d   = False
                bar_dragging = False

            elif ev.type == MOUSEMOTION:
                if dragging3d:
                    yaw   = yaw0   + (mx - drag_start[0]) * 0.007
                    pitch = max(-1.48, min(1.48,
                                pitch0 - (my - drag_start[1]) * 0.007))
                elif bar_dragging:
                    text_start_x = lo.BAR_X + max(60, int(130 * lo.s))
                    bar_cursor   = _bar_index_at_x(bar_text, mx, text_start_x, lo.mfont)

            elif ev.type == KEYDOWN:
                mods = pygame.key.get_mods()

                if ev.key == K_F11:
                    fullscreen = not fullscreen
                    if fullscreen:
                        screen = pygame.display.set_mode((0, 0), FULLSCREEN)
                    else:
                        screen = pygame.display.set_mode((init_w, init_h), RESIZABLE)
                    lo = make_layout()
                    zoom_ratio2 = zoom / lo.ZOOM0
                    zoom = lo.ZOOM0 * zoom_ratio2

                elif ev.key == K_TAB and hint_moves_str:
                    # Tab = copy nhanh gợi ý hiện tại xuống thanh Singmaster,
                    # tương đương bấm nút "Copy -> thanh công thức".
                    # Hoạt động dù đang gõ trong bar hay không (bar_active True/False).
                    bar_text       = hint_moves_str[:BAR_MAX_LEN]
                    bar_active     = True
                    bar_status     = None
                    bar_cursor     = len(bar_text)
                    bar_sel_anchor = None

                elif (ev.key == K_c and not (mods & (KMOD_CTRL | KMOD_META))
                      and formula_lines):
                    # C (không Ctrl) = copy bảng công thức vào clipboard hệ
                    # thống. Nếu đã CLICK CHỌN 1 dòng trong panel nổi (bôi
                    # nền vàng) thì chỉ copy ĐÚNG chuỗi công thức của dòng đó
                    # (không kèm nhãn "Cross: "/"DFR: "); ngược lại copy toàn
                    # bộ. Cần thiết vì nội dung được vẽ dưới dạng hình
                    # (canvas), KHÔNG phải văn bản thật nên không kéo-chọn
                    # bằng chuột được như 1 ứng dụng thông thường.
                    if formula_selected_row is not None and formula_selected_row < len(formula_row_formulas):
                        txt  = formula_row_formulas[formula_selected_row]
                        note = f"Đã copy: {formula_row_labels[formula_selected_row]}"
                    else:
                        txt  = _formula_lines_to_text(formula_lines)
                        note = "Đã copy toàn bộ công thức vào clipboard!"
                    formula_copy_note = (note if _clipboard_set(txt) else
                                          "Không copy được (clipboard không khả dụng ở máy này)")
                    formula_copy_note_timer = 150

                elif ev.key == K_t:
                    # T = mở/đóng bảng công thức (KHÔNG modal -- hoạt động dù
                    # đang gõ trong bar hay không, không chặn phím nào khác).
                    if formula_panel_open or formula_window is not None:
                        formula_panel_open = False
                        if formula_window is not None:
                            try:
                                formula_window.destroy()
                            except Exception:
                                pass
                            formula_window = None
                    else:
                        start_cfop_job('formula')

                elif (formula_panel_open or formula_window is not None) and ev.key == K_ESCAPE:
                    # Đóng panel/cửa sổ trước (không thoát app), giống quy ước
                    # đóng popup/overlay trên cùng trước tiên.
                    formula_panel_open = False
                    if formula_window is not None:
                        try:
                            formula_window.destroy()
                        except Exception:
                            pass
                        formula_window = None

                elif bar_active:
                    shift = bool(mods & KMOD_SHIFT)
                    ctrl  = bool(mods & (KMOD_CTRL | KMOD_META))  # META cho Mac (Cmd)

                    def _sel_range():
                        if bar_sel_anchor is None or bar_sel_anchor == bar_cursor:
                            return None
                        return (min(bar_sel_anchor, bar_cursor), max(bar_sel_anchor, bar_cursor))

                    if ev.key in (K_RETURN, K_KP_ENTER):
                        execute_bar()
                        if not bar_text:
                            bar_cursor = 0; bar_sel_anchor = None
                        else:
                            bar_cursor = min(bar_cursor, len(bar_text))
                    elif ev.key == K_ESCAPE:
                        bar_active = False; bar_text = ""
                        bar_cursor = 0; bar_sel_anchor = None

                    elif ctrl and ev.key == K_a:
                        bar_sel_anchor = 0; bar_cursor = len(bar_text)

                    elif ctrl and ev.key in (K_c, K_x):
                        sel = _sel_range()
                        copy_text = bar_text[sel[0]:sel[1]] if sel else bar_text
                        _clipboard_set(copy_text)
                        if ev.key == K_x and sel:
                            a, b = sel
                            bar_text = bar_text[:a] + bar_text[b:]
                            bar_cursor = a; bar_sel_anchor = None

                    elif ctrl and ev.key == K_v:
                        pasted = ''
                        try:
                            raw = pygame.scrap.get(pygame.SCRAP_TEXT)
                            if raw:
                                pasted = raw.decode('utf-8', errors='ignore').split('\x00')[0]
                                pasted = ''.join(ch for ch in pasted if ch.isprintable())
                        except Exception:
                            pasted = ''
                        if pasted:
                            sel = _sel_range()
                            if sel:
                                a, b = sel
                                bar_text = bar_text[:a] + bar_text[b:]
                                bar_cursor = a; bar_sel_anchor = None
                            room = BAR_MAX_LEN - len(bar_text)
                            pasted = pasted[:max(0, room)]
                            bar_text = bar_text[:bar_cursor] + pasted + bar_text[bar_cursor:]
                            bar_cursor += len(pasted)

                    elif ev.key == K_LEFT:
                        sel = _sel_range()
                        if shift:
                            if bar_sel_anchor is None:
                                bar_sel_anchor = bar_cursor
                            bar_cursor = max(0, bar_cursor - 1)
                        elif sel:
                            bar_cursor = sel[0]; bar_sel_anchor = None
                        else:
                            bar_cursor = max(0, bar_cursor - 1)
                    elif ev.key == K_RIGHT:
                        sel = _sel_range()
                        if shift:
                            if bar_sel_anchor is None:
                                bar_sel_anchor = bar_cursor
                            bar_cursor = min(len(bar_text), bar_cursor + 1)
                        elif sel:
                            bar_cursor = sel[1]; bar_sel_anchor = None
                        else:
                            bar_cursor = min(len(bar_text), bar_cursor + 1)
                    elif ev.key == K_HOME:
                        bar_sel_anchor = bar_cursor if shift else None
                        bar_cursor = 0
                    elif ev.key == K_END:
                        bar_sel_anchor = bar_cursor if shift else None
                        bar_cursor = len(bar_text)

                    elif ev.key == K_BACKSPACE:
                        sel = _sel_range()
                        if sel:
                            a, b = sel
                            bar_text = bar_text[:a] + bar_text[b:]
                            bar_cursor = a; bar_sel_anchor = None
                        elif bar_cursor > 0:
                            bar_text = bar_text[:bar_cursor - 1] + bar_text[bar_cursor:]
                            bar_cursor -= 1
                    elif ev.key == K_DELETE:
                        sel = _sel_range()
                        if sel:
                            a, b = sel
                            bar_text = bar_text[:a] + bar_text[b:]
                            bar_cursor = a; bar_sel_anchor = None
                        elif bar_cursor < len(bar_text):
                            bar_text = bar_text[:bar_cursor] + bar_text[bar_cursor + 1:]

                    else:
                        ch = ev.unicode
                        if ch and ch.isprintable():
                            sel = _sel_range()
                            if sel:
                                a, b = sel
                                bar_text = bar_text[:a] + bar_text[b:]
                                bar_cursor = a; bar_sel_anchor = None
                            if len(bar_text) < BAR_MAX_LEN:
                                bar_text = bar_text[:bar_cursor] + ch + bar_text[bar_cursor:]
                                bar_cursor += 1

                else:
                    if ev.key == K_ESCAPE:
                        running = False

                    elif ev.key == K_SPACE:
                        push_undo()
                        state = make_solved(); scramble_cube(state)
                        state_version += 1
                        anim_queue.clear()
                        move_count  = 0
                        timer_start = None
                        timer_end   = None
                        scrambled   = True

                    elif ev.key == K_0:
                        push_undo()
                        state = make_solved(); anim_queue.clear()
                        state_version += 1
                        move_count  = 0
                        timer_start = None
                        timer_end   = None
                        scrambled   = False

                    elif ev.key == K_z and (mods & KMOD_CTRL):
                        if undo_stack:
                            anim_queue.clear()
                            state = undo_stack.pop()
                            state_version += 1
                            move_count = max(0, move_count - 1)
                        else:
                            undo_empty_flash = 90

                    elif ev.key in (K_MINUS, K_KP_MINUS, K_LEFTBRACKET):
                        speed_idx   = max(0, speed_idx - 1)
                        speed_flash = 90

                    elif ev.key in (K_EQUALS, K_PLUS, K_KP_PLUS, K_RIGHTBRACKET):
                        speed_idx   = min(len(SPEED_STEPS) - 1, speed_idx + 1)
                        speed_flash = 90

                    elif ev.key in (K_UP, K_RIGHT) and sel_face:
                        enqueue(sel_face)
                    elif ev.key in (K_DOWN, K_LEFT) and sel_face:
                        enqueue(sel_face + "'")

                    elif ev.key == K_SLASH:
                        bar_active = True

                    elif ev.key == K_a:
                        start_cfop_job('solve')

                    elif ev.key == K_h:
                        start_cfop_job('hint')

        # ── 3D ANIMATION ──────────────────────────────────────────────────────
        if anim_face is None and anim_queue:
            mv   = anim_queue.pop(0)
            base = mv[0]   # ký tự mặt/gốc, luôn là 1 ký tự đầu (vd "R2" -> "R")

            if base in ANIMATABLE_BASES:
                # renderer_3d hỗ trợ animate đầy đủ cả 18 base move: outer
                # (U D F B L R), slice (M E S), wide (u d f b l r) và
                # rotation cả khối (x y z).
                anim_mv     = mv
                anim_face   = base
                is_double   = '2' in mv
                anim_target = math.pi if is_double else (math.pi / 2 * (1 if "'" in mv else -1))
                anim_ang    = 0.
                anim_t      = 0.
            else:
                # Fallback phòng thủ: base lạ (không nên xảy ra vì parser đã
                # validate) -> áp dụng ngay, không animate, để tránh crash.
                do_move(state, mv)
                state_version += 1
                if cube_solved(state):
                    solved_flash = 220
                    if scrambled and timer_start is not None and timer_end is None:
                        timer_end = time.perf_counter()
                    scrambled = False

        if anim_face:
            # Khi còn nhiều move chờ trong queue (vd chuỗi Singmaster dài),
            # tăng tốc animation để tránh người dùng phải đợi quá lâu,
            # nhưng vẫn giữ animation mượt cho thao tác đơn lẻ.
            # user_speed: hệ số người dùng tự chỉnh bằng phím [ / ] (hoặc -/+).
            user_speed   = SPEED_STEPS[speed_idx]
            speed_factor = 1.0 if len(anim_queue) < 4 else min(4.0, 1.0 + len(anim_queue) * 0.15)
            anim_t += dt * speed_factor * user_speed / ANIM_DUR
            if anim_t >= 1.:
                anim_t = 1.
            anim_ang = anim_target * (1 - (1 - anim_t) ** 3)
            if anim_t >= 1.:
                do_move(state, anim_mv)
                state_version += 1
                anim_face = None; anim_ang = 0.
                if cube_solved(state):
                    solved_flash = 220
                    if scrambled and timer_start is not None and timer_end is None:
                        timer_end = time.perf_counter()
                    scrambled = False

        # ══════════════════════════════════════════════════════════════════════
        #  DRAW
        # ══════════════════════════════════════════════════════════════════════
        W, H = screen.get_size()
        screen.fill(BG)

        pygame.draw.line(screen, DIV, (lo.RIGHT_X, 40), (lo.RIGHT_X, H - 10), 1)

        # ── LEFT: 6-Face view ─────────────────────────────────────────────────
        lbl = lo.sfont.render(
            "6-Face View  ·  click to select  ·  ↑↓←→ rotate", True, HINT
        )
        screen.blit(lbl, (lo.LEFT_X + 8, lo.PY0 - max(14, int(28 * lo.s))))
        draw_panels(screen, state, sel_face, lo)

        # history
        hy = lo.PY0 + lo.LBL_H + 3 * (lo.PANEL + lo.LBL_H + 5) + 14
        # Khoang trong danh cho shortcuts panel: TINH THEO SO DONG THAT (9
        # phim tat + 1 dong ky hieu U/u D/d...), khong hardcode nhu truoc
        # (tung bi loi che khuat dong cuoi vi so voi noi dung thuc te). Dung
        # CHUNG 1 gia tri nay o CA hy_limit lan shortcuts_top_y ben duoi de
        # khong bao gio lech nhau.
        _N_SHORTCUT_LINES = 9   # dung dung so dong trong list o duoi (khong tinh dong ky hieu)
        _shortcut_line_h = max(10, int(15 * lo.s))
        shortcuts_reserved_h = (_N_SHORTCUT_LINES + 1) * _shortcut_line_h + max(10, int(14 * lo.s))

        hy_limit = lo.BAR_Y - shortcuts_reserved_h - max(10, int(14 * lo.s)) \
            - max(70, int(108 * lo.s)) - int(4 * lo.s)
        if history:
            screen.blit(
                lo.sfont.render("History:", True, (90, 90, 130)),
                (lo.LEFT_X + 10, hy)
            )
            hy += int(16 * lo.s)
            for h in reversed(history):
                if hy + int(15 * lo.s) > hy_limit:
                    break
                screen.blit(
                    lo.sfont.render(h, True, (150, 150, 195)),
                    (lo.LEFT_X + 16, hy)
                )
                hy += int(15 * lo.s)

        # ── CFOP AI panel: vi tri CO DINH neo tu day (doc lap voi chieu cao
        # thay doi cua History o tren) -> khong bao gio de len shortcuts. ────
        shortcuts_top_y = lo.BAR_Y - shortcuts_reserved_h
        CFOP_H = max(70, int(108 * lo.s))
        cfop_top = shortcuts_top_y - max(10, int(14 * lo.s)) - CFOP_H
        cfop_box = pygame.Rect(lo.LEFT_X + 6, cfop_top, lo.LEFT_W - 12, CFOP_H)
        pygame.draw.rect(screen, (24, 26, 38), cfop_box, border_radius=6)
        pygame.draw.rect(screen, (55, 58, 80), cfop_box, width=1, border_radius=6)

        ax = cfop_box.x + 10
        ay = cfop_box.y + 8

        # Badge tien do 4 buoc CFOP (✓ = xong, ● = dang lam, ○ = chua toi)
        try:
            _stage = cfop_ai.stage_of(state)
        except Exception:
            _stage = None
        _order = ['cross', 'f2l', 'oll', 'pll']
        _cur_idx = _order.index(_stage) if _stage in _order else 4
        bx = ax
        for i, label in enumerate(['Cross', 'F2L', 'OLL', 'PLL']):
            if _stage == 'done' or i < _cur_idx:
                mark, col = '✓', (90, 220, 130)
            elif i == _cur_idx:
                mark, col = '●', (255, 210, 0)
            else:
                mark, col = '○', (90, 90, 115)
            btxt = lo.sfont.render(f"{mark} {label}", True, col)
            screen.blit(btxt, (bx, ay))
            bx += btxt.get_width() + max(10, int(14 * lo.s))
        ay += int(18 * lo.s)

        if cfop_busy:
            dots = '.' * (1 + (pygame.time.get_ticks() // 300) % 3)
            busy_txt = lo.sfont.render(f"AI đang tính{dots}", True, (255, 210, 0))
            screen.blit(busy_txt, (ax, ay))
            ay += int(15 * lo.s)
        elif cfop_note:
            note_txt = lo.sfont.render(cfop_note, True, (120, 220, 140))
            screen.blit(note_txt, (ax, ay))
            ay += int(15 * lo.s)

        if hint_label:
            hl_txt = lo.sfont.render(f"Gợi ý: {hint_label}", True, (230, 190, 255))
            screen.blit(hl_txt, (ax, ay))
            ay += int(15 * lo.s)
            if hint_moves_str:
                hm_txt = lo.sfont.render(f"  {hint_moves_str}", True, (255, 255, 255))
                screen.blit(hm_txt, (ax, ay))

                # ── Nút "Copy -> thanh công thức" ──────────────────────────
                btn_w = max(90, int(118 * lo.s))
                btn_h = max(16, int(20 * lo.s))
                btn_x = ax + hm_txt.get_width() + max(8, int(10 * lo.s))
                # Nếu nút bị tràn ra ngoài cfop_box, xuống dòng dưới thay vì
                # đè lên nội dung khác.
                if btn_x + btn_w > cfop_box.right - 6:
                    btn_x = ax
                    ay += int(18 * lo.s)
                hint_copy_rect = pygame.Rect(btn_x, ay - int(2 * lo.s), btn_w, btn_h)
                hovered = hint_copy_rect.collidepoint(mx, my)
                pygame.draw.rect(
                    screen,
                    (70, 70, 100) if hovered else (48, 48, 68),
                    hint_copy_rect, border_radius=max(3, int(5 * lo.s)),
                )
                pygame.draw.rect(
                    screen, GOLD if hovered else (110, 110, 155),
                    hint_copy_rect, 1, border_radius=max(3, int(5 * lo.s)),
                )
                cp_txt = lo.sfont.render("Copy -> bar", True,
                                         GOLD if hovered else (200, 200, 225))
                screen.blit(cp_txt, (
                    hint_copy_rect.centerx - cp_txt.get_width() // 2,
                    hint_copy_rect.centery - cp_txt.get_height() // 2,
                ))
            else:
                hint_copy_rect = None
        else:
            hint_copy_rect = None

        # phím tắt hint
        sy = shortcuts_top_y
        for k, v in [
            ("Space",  "Scramble"),
            ("Ctrl+Z", "Undo"),
            ("0",      "Reset"),
            ("[ / ]",  "Tốc độ animation"),
            ("Esc",    "Quit"),
            ("F11",    "Fullscreen"),
            ("/",      "Type moves"),
            ("A",      "AI tự giải (Cross→F2L→OLL→PLL)"),
            ("H",      "AI gợi ý bước tiếp"),
            ("Tab",    "Copy gợi ý -> thanh công thức"),
            ("T",      "Bảng công thức (cửa sổ riêng nếu được, hoặc panel nổi)"),
            ("C",      "Copy toàn bộ bảng công thức vào clipboard"),
        ]:
            ks = lo.sfont.render(k, True, GOLD)
            vs = lo.sfont.render(f"  {v}", True, HINT)
            screen.blit(ks, (lo.LEFT_X + 10, sy))
            screen.blit(vs, (lo.LEFT_X + 10 + ks.get_width(), sy))
            sy += _shortcut_line_h

        # Notation hint dưới shortcut list
        nh = lo.sfont.render("  U/u D/d F/f B/b L/l R/r M E S x y z", True, (80, 80, 115))
        screen.blit(nh, (lo.LEFT_X + 6, sy))

        _bar_sel = None
        if bar_sel_anchor is not None and bar_sel_anchor != bar_cursor:
            _bar_sel = (min(bar_sel_anchor, bar_cursor), max(bar_sel_anchor, bar_cursor))
        draw_bar(screen, bar_text, bar_active, bar_status, lo,
                 cursor_pos=bar_cursor, sel_range=_bar_sel)

        if undo_empty_flash > 0:
            msg = lo.sfont.render("nothing to undo", True, (200, 90, 90))
            screen.blit(msg, (lo.LEFT_X + 10, lo.BAR_Y - max(78, int(126 * lo.s))))

        # ── RIGHT: 3D view ────────────────────────────────────────────────────
        lbl3 = lo.font.render("3D View", True, (110, 110, 155))
        screen.blit(lbl3, (lo.CX3 - lbl3.get_width() // 2, max(8, int(18 * lo.s))))

        # Chỉ báo tốc độ animation ([ / ] hoặc - / + để chỉnh), luôn hiện nhỏ
        # gọn cạnh nhãn 3D View; sáng/to hơn trong giây lát khi vừa đổi.
        speed_val = SPEED_STEPS[speed_idx]
        speed_txt = f"tốc độ {speed_val:g}x"
        speed_col = (255, 210, 0) if speed_flash > 0 else (95, 95, 130)
        speed_fnt = lo.font if speed_flash > 0 else lo.sfont
        speed_lbl = speed_fnt.render(speed_txt, True, speed_col)
        screen.blit(speed_lbl, (lo.CX3 + lbl3.get_width() // 2 + max(10, int(14 * lo.s)),
                                 max(8, int(18 * lo.s))))

        # Move counter + timer
        now = time.perf_counter()
        if timer_end is not None:
            elapsed = timer_end - timer_start
            t_col   = (60, 230, 100)   # xanh = solved
        elif timer_start is not None:
            elapsed = now - timer_start
            t_col   = (255, 210, 0)    # vàng = đang chạy
        else:
            elapsed = 0.0
            t_col   = HINT

        mins = int(elapsed) // 60
        secs = elapsed % 60
        if mins > 0:
            t_str = f"{mins}:{secs:05.2f}"
        else:
            t_str = f"{secs:.2f}s"

        # Hiển thị bên phải: moves | timer
        info_str = f"{move_count} moves  {t_str}"
        info_lbl = lo.sfont.render(info_str, True, t_col)
        screen.blit(info_lbl, (lo.CX3 - info_lbl.get_width() // 2,
                                max(8, int(18 * lo.s)) + lbl3.get_height() + 2))

        draw_cube_3d(screen, state, R, zoom, sel_face, anim_face, anim_ang, lo, state_version)

        if solved_flash > 0:
            t = lo.bfont.render("✓  SOLVED!", True, (60, 230, 100))
            screen.blit(t, (lo.CX3 - t.get_width() // 2, lo.CY3 - 14))

        if formula_copy_note:
            note_s = lo.sfont.render(formula_copy_note, True, (110, 230, 140))
            note_pad = max(6, int(8 * lo.s))
            note_bg  = pygame.Rect(0, 0, note_s.get_width() + note_pad * 2,
                                    note_s.get_height() + note_pad * 2)
            note_bg.topright = (lo.W - max(10, int(14 * lo.s)), max(10, int(14 * lo.s)))
            pygame.draw.rect(screen, (18, 18, 28), note_bg, border_radius=6)
            pygame.draw.rect(screen, (110, 230, 140), note_bg, width=1, border_radius=6)
            screen.blit(note_s, (note_bg.x + note_pad, note_bg.y + note_pad))

        if formula_panel_open:
            # ── Panel NỔI (docked), KHÔNG modal: không dim/khoá phần còn lại
            # của màn hình -- cube vẫn xoay/giải bình thường khi panel mở.
            panel_w = min(int(420 * max(lo.s, 1.0)), lo.RIGHT_W - 20, lo.W - 20)
            panel_w = max(panel_w, min(320, lo.W - 20))
            panel_h = min(int(lo.H * 0.78), int(640 * max(lo.s, 1.0)))
            panel_h = max(panel_h, min(360, lo.H - 20))
            margin  = max(10, int(14 * lo.s))
            panel_x = lo.W - panel_w - margin
            panel_y = max(46, int(50 * lo.s))

            formula_panel_rect = pygame.Rect(panel_x, panel_y, panel_w, panel_h)

            panel_surf = pygame.Surface((panel_w, panel_h), pygame.SRCALPHA)
            panel_surf.fill((18, 18, 28, 235))
            screen.blit(panel_surf, (panel_x, panel_y))
            pygame.draw.rect(screen, GOLD, formula_panel_rect, width=2, border_radius=10)

            title = lo.mfont.render("Công thức CFOP (tham khảo)", True, GOLD)
            screen.blit(title, (panel_x + 14, panel_y + 10))

            close_sz = max(18, int(22 * lo.s))
            formula_close_rect = pygame.Rect(
                panel_x + panel_w - close_sz - 8, panel_y + 8, close_sz, close_sz)
            pygame.draw.rect(screen, (60, 60, 75), formula_close_rect, border_radius=5)
            xt = lo.sfont.render("X", True, (230, 230, 230))
            screen.blit(xt, (formula_close_rect.centerx - xt.get_width() // 2,
                              formula_close_rect.centery - xt.get_height() // 2))

            copy_label = "Copy dòng" if formula_selected_row is not None else "Copy tất cả"
            copy_txt_s = lo.sfont.render(copy_label, True, (230, 230, 230))
            copy_w = copy_txt_s.get_width() + 14
            formula_copy_rect = pygame.Rect(
                formula_close_rect.x - copy_w - 6, panel_y + 8, copy_w, close_sz)
            pygame.draw.rect(screen, (60, 60, 75), formula_copy_rect, border_radius=5)
            screen.blit(copy_txt_s, (formula_copy_rect.centerx - copy_txt_s.get_width() // 2,
                                      formula_copy_rect.centery - copy_txt_s.get_height() // 2))

            can_apply = (formula_selected_row is not None
                         and formula_selected_row < len(formula_row_formulas)
                         and _looks_like_moves(formula_row_formulas[formula_selected_row]))
            if can_apply:
                apply_txt_s = lo.sfont.render("Áp dụng -> bar", True, (30, 30, 20))
                apply_w = apply_txt_s.get_width() + 14
                formula_apply_rect = pygame.Rect(
                    formula_copy_rect.x - apply_w - 6, panel_y + 8, apply_w, close_sz)
                pygame.draw.rect(screen, GOLD, formula_apply_rect, border_radius=5)
                screen.blit(apply_txt_s,
                            (formula_apply_rect.centerx - apply_txt_s.get_width() // 2,
                             formula_apply_rect.centery - apply_txt_s.get_height() // 2))
            else:
                formula_apply_rect = None

            header_h = max(title.get_height(), close_sz) + 16

            stale = (formula_state_ver is not None and formula_state_ver != state_version)
            stale_h = 0
            if stale:
                stale_txt = lo.sfont.render(
                    "⚠ Cube đã đổi kể từ lúc tính -- bấm T để làm mới", True, (235, 180, 90))
                screen.blit(stale_txt, (panel_x + 14, panel_y + header_h))
                stale_h = stale_txt.get_height() + 6

            footer_txt = lo.sfont.render(
                "T/Esc đóng • click chọn dòng • C: copy • nút vàng: dán vào bar",
                True, HINT)
            footer_h = footer_txt.get_height() + 10
            screen.blit(footer_txt, (panel_x + 14, panel_y + panel_h - footer_h))

            btn_sz = max(20, int(24 * lo.s))
            formula_down_rect = pygame.Rect(
                panel_x + panel_w - btn_sz - 8,
                panel_y + panel_h - footer_h - btn_sz - 4, btn_sz, btn_sz)
            formula_up_rect = pygame.Rect(
                formula_down_rect.x, formula_down_rect.y - btn_sz - 4, btn_sz, btn_sz)
            for rct, sym in ((formula_up_rect, "^"), (formula_down_rect, "v")):
                pygame.draw.rect(screen, (55, 55, 70), rct, border_radius=4)
                st = lo.sfont.render(sym, True, (220, 220, 220))
                screen.blit(st, (rct.centerx - st.get_width() // 2,
                                  rct.centery - st.get_height() // 2))

            content_rect = pygame.Rect(
                panel_x + 14, panel_y + header_h + stale_h,
                panel_w - 28, panel_h - header_h - stale_h - footer_h - 4)
            formula_content_rect = content_rect

            fonts = {'header': lo.bfont, 'row': lo.mfont,
                     'name_col_w': max(70, int(80 * lo.s)),
                     'row_gap': max(2, int(3 * lo.s)),
                     'spacer_h': max(6, int(10 * lo.s))}
            formula_scroll_max, formula_scroll, formula_hit_rows = _draw_formula_panel_content(
                screen, content_rect, fonts, formula_lines, formula_scroll, GOLD,
                selected_row=formula_selected_row)

        if formula_window is not None:
            # ── Vẽ + hiện cửa sổ OS riêng (thử nghiệm) ───────────────────────
            # Cửa sổ này CHỈ ĐỂ XEM (không nhận click/cuộn riêng) -- đóng bằng
            # phím T hoặc Esc ở cửa sổ chính. Bất kỳ lỗi nào cũng tự đóng
            # cửa sổ và quay lại panel nổi (docked), không làm crash app.
            try:
                win_w, win_h = formula_window.size
                if formula_win_fonts is None or formula_win_last_w != win_w:
                    formula_win_surface = formula_window.get_surface()
                    formula_win_last_w  = win_w
                    fs = max(0.6, min(1.6, win_w / 460))
                    formula_win_fonts = {
                        'title':  pygame.font.SysFont("consolas", max(12, int(17 * fs)), bold=True),
                        'header': pygame.font.SysFont("consolas", max(11, int(16 * fs)), bold=True),
                        'row':    pygame.font.SysFont("consolas", max(10, int(13 * fs))),
                        'footer': pygame.font.SysFont("consolas", max(9,  int(11 * fs))),
                    }

                surf = formula_win_surface
                surf.fill((18, 18, 28))
                pad = max(8, int(14 * (win_w / 460)))

                title_txt = formula_win_fonts['title'].render(
                    "Công thức CFOP: Cross → F2L → OLL → PLL", True, GOLD)
                surf.blit(title_txt, (pad, pad))
                header_h = title_txt.get_height() + pad + 6

                win_stale = (formula_state_ver is not None and formula_state_ver != state_version)
                win_stale_h = 0
                if win_stale:
                    stxt = formula_win_fonts['footer'].render(
                        "⚠ Cube đã đổi -- mở lại bằng T ở cửa sổ chính để làm mới",
                        True, (235, 180, 90))
                    surf.blit(stxt, (pad, header_h))
                    win_stale_h = stxt.get_height() + 6

                foot_txt = formula_win_fonts['footer'].render(
                    "Đóng: T/Esc  •  Copy toàn bộ: C  (bấm ở cửa sổ chính)",
                    True, HINT)
                foot_h = foot_txt.get_height() + pad
                surf.blit(foot_txt, (pad, win_h - foot_h))

                win_content_rect = pygame.Rect(
                    pad, header_h + win_stale_h, win_w - 2 * pad,
                    max(10, win_h - header_h - win_stale_h - foot_h - 4))
                win_fonts_arg = {'header': formula_win_fonts['header'],
                                  'row': formula_win_fonts['row'],
                                  'name_col_w': max(60, int(80 * (win_w / 460))),
                                  'row_gap': 2,
                                  'spacer_h': max(6, int(8 * (win_w / 460)))}
                # Cửa sổ chỉ xem -- không cuộn (scroll=0); nội dung dài quá sẽ
                # bị cắt gọn ở cạnh dưới (set_clip) thay vì tràn ra ngoài.
                _draw_formula_panel_content(
                    surf, win_content_rect, win_fonts_arg, formula_lines, 0, GOLD)

                formula_window.flip()
            except Exception:
                try:
                    formula_window.destroy()
                except Exception:
                    pass
                formula_window        = None
                formula_win_supported = False
                if formula_lines:
                    formula_panel_open = True   # fallback về panel nổi khi cửa sổ lỗi

        pygame.display.flip()

    if formula_window is not None:
        try:
            formula_window.destroy()
        except Exception:
            pass

    pygame.quit()
    sys.exit()


if __name__ == "__main__":
    main()

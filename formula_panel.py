"""
formula_panel.py
==================
Bang cong thuc CFOP tham khao (mo/dong bang phim T trong main.py).

TACH RA TU main.py (truoc day toan bo o day nam trong 1 file main.py duy
nhat 1400 dong) de:
  1) Giam kich thuoc main.py (van con lon, nhung tach duoc 1 khoi chuc nang
     doc lap ro rang la buoc dau).
  2) Cho phep TEST cac ham logic thuan (build_formula_lines, _looks_like_moves,
     _formula_row_data, _formula_lines_to_text) MA KHONG can import toan bo
     main.py -- xem test_app_logic.py.

Cac ham trong file nay chia lam 2 nhom:
  - THUAN (khong dung pygame display, chi nhan/tra du lieu): _stage_row,
    build_formula_lines, _formula_lines_to_text, _formula_row_data,
    _looks_like_moves. -> de test, khong can pygame that.
  - CAN RENDERER (dung pygame.Surface/Font that): _wrap_text (can font.size),
    draw_formula_panel_content, try_open_formula_window. -> chi chay dung
    trong app that (main.py), khong test truc tiep o day.
"""

import pygame

from solver.oll_algorithms import PRETTY_CASE_NAME as _PRETTY_CASE_NAME

# ── Cửa sổ OS riêng (THỬ NGHIỆM / best-effort) ──────────────────────────────
# Xem ghi chu tuong tu trong main.py: neu import/tao cua so that bai, tinh
# nang tu vo hieu hoa va quay lai dung panel noi (docked), khong bao gio
# lam crash app chinh.
try:
    from pygame._sdl2.video import Window as _SDLWindow
except Exception:
    _SDLWindow = None


_ROW_GREEN = (110, 230, 140)   # DONE!
_ROW_AMBER = (235, 180, 90)    # that bai trong ngan sach hien tai
_ROW_GREY  = (140, 140, 175)   # chua toi luot
_ROW_WHITE = (225, 225, 235)   # co chuoi nuoc di can lam


# ══════════════════════════════════════════════════════════════════════════
#  Nhom THUAN -- khong dung pygame display, de test doc lap
# ══════════════════════════════════════════════════════════════════════════

def _stage_row(label, info):
    """1 dong (kind='row') the hien 1 buoc/1 cap F2L, dua tren dict
    {'status': 'done'|'moves'|'failed'|'not_reached', 'moves': [...],
     'case_name': ten case (chi co o oll/pll) hoac None}.
    Khi da nhan dien duoc TEN case cu the (vd 'Sune', 'T', 'OCLL'), THAY
    THE nhan hien thi bang ten do (gon hon so voi nhan chung "OLL"/"PLL"
    vốn đã lặp lại tiêu đề nhóm ngay phía trên) -- giúp người dùng HỌC
    tên case thay vì chỉ thấy 1 chuỗi nước vô danh."""
    status    = info.get('status')
    case_name = info.get('case_name')
    if status == 'moves' and case_name:
        pretty = _PRETTY_CASE_NAME.get(case_name, case_name)
        disp_label = f"{label}:{pretty}"
    else:
        disp_label = label
    if status == 'done':
        return ('row', label, 'DONE!', _ROW_GREEN)
    if status == 'moves':
        return ('row', disp_label, ' '.join(info['moves']), _ROW_WHITE)
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

    HAM THUAN: khong sua doi `breakdown` truyen vao, khong dung pygame
    display -- xem test_app_logic.py:test_build_formula_lines_does_not_mutate_input.
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


def formula_lines_to_text(formula_lines):
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


def formula_row_data(formula_lines):
    """Tra ve 2 list SONG SONG (cung thu tu/row_idx ma
    draw_formula_panel_content() dung de gan nhan khi ve):
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


def looks_like_moves(text):
    """True neu `text` la 1 chuoi nuoc di Singmaster that su (khong phai
    'DONE!' hay ghi chu dang '(...)'), dung de an/hien nut 'Ap dung -> bar'."""
    t = (text or '').strip()
    return bool(t) and t != 'DONE!' and not t.startswith('(')


# ══════════════════════════════════════════════════════════════════════════
#  Nhom CAN RENDERER -- dung pygame.Surface/Font that
# ══════════════════════════════════════════════════════════════════════════

def wrap_text(text, font, max_w):
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


def draw_formula_panel_content(surface, rect, fonts, formula_lines, scroll, gold_color,
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
            wrapped = wrap_text(text, mfont, wrap_w)
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


def try_open_formula_window():
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


def sdl_window_available():
    """True neu co the (ve mat import) thu mo cua so OS rieng -- dung o
    main.py de quyet dinh co goi try_open_formula_window() hay khong."""
    return _SDLWindow is not None

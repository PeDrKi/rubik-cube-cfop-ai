"""
main.py — Rubik's Cube Simulator
Chỉ còn: 6-Face View (cột trái) và 3D View (cột phải).
Đã loại bỏ: CFOP Tutor, AI Advisor, F2L Highlighter, 2D Circle View.
"""

import pygame
from pygame.locals import *
import math
import copy
import sys
import time

from constants   import FPS, BG, GOLD, HINT, DIV
from layout      import Layout
from cube_engine import (make_solved, scramble_cube, cube_solved,
                         do_move, parse_singmaster)
from renderer_3d import Rx, Ry, draw_cube_3d, hit_test_3d, ANIMATABLE_BASES
from draw_helpers import draw_panels, panel_hit, draw_bar


ANIM_DUR      = 0.13   # giây, 3D move animation (ở tốc độ x1)
BAR_MAX_LEN   = 80      # giới hạn ký tự Singmaster bar, tránh queue move khổng lồ
UNDO_MAX      = 120     # giới hạn độ sâu undo
MIN_W, MIN_H  = 800, 520   # kích thước cửa sổ tối thiểu, tránh UI vỡ hình khi resize nhỏ
SPEED_STEPS   = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0]   # các mức tốc độ animation
SPEED_DEFAULT_IDX = SPEED_STEPS.index(1.0)


def main():
    pygame.init()

    info   = pygame.display.Info()
    init_w = max(MIN_W, int(info.current_w * 0.92))
    init_h = max(MIN_H, int(info.current_h * 0.92))
    screen = pygame.display.set_mode((init_w, init_h), RESIZABLE)
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

            elif ev.type == MOUSEBUTTONDOWN and ev.button == 1:
                if lo.bar_rect().collidepoint(mx, my):
                    bar_active = True

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
                dragging3d = False

            elif ev.type == MOUSEMOTION:
                if dragging3d:
                    yaw   = yaw0   + (mx - drag_start[0]) * 0.007
                    pitch = max(-1.48, min(1.48,
                                pitch0 - (my - drag_start[1]) * 0.007))

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

                elif bar_active:
                    if ev.key in (K_RETURN, K_KP_ENTER):
                        execute_bar()
                    elif ev.key == K_ESCAPE:
                        bar_active = False; bar_text = ""
                    elif ev.key == K_BACKSPACE:
                        bar_text = bar_text[:-1]
                    else:
                        ch = ev.unicode
                        if ch and ch.isprintable() and len(bar_text) < BAR_MAX_LEN:
                            bar_text += ch

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

                    elif ev.key in (K_SLASH, K_t):
                        bar_active = True

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
        if history:
            screen.blit(
                lo.sfont.render("History:", True, (90, 90, 130)),
                (lo.LEFT_X + 10, hy)
            )
            hy += int(16 * lo.s)
            for h in reversed(history):
                screen.blit(
                    lo.sfont.render(h, True, (150, 150, 195)),
                    (lo.LEFT_X + 16, hy)
                )
                hy += int(15 * lo.s)

        # phím tắt hint
        sy = lo.BAR_Y - max(60, int(123 * lo.s))
        for k, v in [
            ("Space",  "Scramble"),
            ("Ctrl+Z", "Undo"),
            ("0",      "Reset"),
            ("[ / ]",  "Tốc độ animation"),
            ("Esc",    "Quit"),
            ("F11",    "Fullscreen"),
            ("/",      "Type moves"),
        ]:
            ks = lo.sfont.render(k, True, GOLD)
            vs = lo.sfont.render(f"  {v}", True, HINT)
            screen.blit(ks, (lo.LEFT_X + 10, sy))
            screen.blit(vs, (lo.LEFT_X + 10 + ks.get_width(), sy))
            sy += max(10, int(15 * lo.s))

        # Notation hint dưới shortcut list
        nh = lo.sfont.render("  U/u D/d F/f B/b L/l R/r M E S x y z", True, (80, 80, 115))
        screen.blit(nh, (lo.LEFT_X + 6, sy))

        draw_bar(screen, bar_text, bar_active, bar_status, lo)

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

        pygame.display.flip()

    pygame.quit()
    sys.exit()


if __name__ == "__main__":
    main()

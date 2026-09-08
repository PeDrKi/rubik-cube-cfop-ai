"""
draw_helpers.py
===============
Tất cả hàm vẽ UI của simulator:
  - draw_panels     : 6-face view (cột LEFT)
  - panel_hit       : hit-test click vào mặt trong 6-face view
  - draw_bar        : Singmaster input bar

Phụ thuộc: pygame, constants
"""

import pygame

from constants import (
    FACES, COL3D, FACE_LC,
    BLACK, GOLD,
    BAR_BG, BAR_ACT, BAR_BOK, BAR_BER,
)


# ── 6-Face view ───────────────────────────────────────────────────────────────

def draw_panels(surf: pygame.Surface, state: dict, sel_face, lo,
                highlight_set: set | None = None) -> None:
    """
    Vẽ lưới 6 mặt (chữ thập) ở cột LEFT.

    Params:
        state         : cube state dict
        sel_face      : tên mặt đang chọn (highlight vàng) hoặc None
        lo            : Layout
        highlight_set : set of (face, r, c) cần highlight (F2L pair)
    """
    if highlight_set is None:
        highlight_set = set()
    s = lo.s
    for face in FACES:
        gc, gr = lo.PGRID[face]
        lx = lo.PX0 + gc * (lo.PANEL + 5)
        ly = lo.PY0 + gr * (lo.PANEL + lo.LBL_H + 5)
        sel = (face == sel_face)

        # label tên mặt
        lb = lo.font.render(face, True, GOLD if sel else FACE_LC[face])
        surf.blit(lb, (lx + lo.PANEL // 2 - lb.get_width() // 2, ly))

        # nền panel
        pr = lo.panel_rect(face)
        pygame.draw.rect(
            surf,
            (55, 55, 72) if sel else (38, 38, 52),
            pr,
            border_radius=max(2, int(4 * s)),
        )
        if sel:
            pygame.draw.rect(surf, GOLD, pr, 2, border_radius=max(2, int(4 * s)))

        # stickers
        for r in range(3):
            for c in range(3):
                col    = COL3D[state[face][r, c]]
                rect   = lo.sticker_rect(face, r, c)
                is_hl  = (face, r, c) in highlight_set
                pygame.draw.rect(surf, col, rect, border_radius=max(1, int(2 * s)))

                # Viền highlight F2L: trắng sáng nhấp nháy
                if is_hl:
                    t        = pygame.time.get_ticks()
                    pulse    = int(180 + 75 * abs(__import__('math').sin(t / 300)))
                    hl_col   = (pulse, pulse, 80)
                    pygame.draw.rect(surf, hl_col, rect,
                                     max(2, int(3 * s)),
                                     border_radius=max(1, int(2 * s)))
                else:
                    pygame.draw.rect(surf, BLACK, rect, 1,
                                     border_radius=max(1, int(2 * s)))


def panel_hit(mx: int, my: int, lo) -> str | None:
    """Trả về tên mặt nếu click vào panel của nó, ngược lại None."""
    for f in FACES:
        if lo.panel_rect(f).collidepoint(mx, my):
            return f
    return None


# ── Singmaster bar ────────────────────────────────────────────────────────────

def draw_bar(surf: pygame.Surface, text: str, active: bool, status, lo) -> None:
    """
    Vẽ Singmaster input bar ở cuối cột LEFT.

    Params:
        text   : nội dung đang gõ
        active : True nếu bar đang focus
        status : None | 'ok' | 'error'
        lo     : Layout
    """
    s = lo.s
    col_brd = {None: (80, 80, 110), 'ok': BAR_BOK, 'error': BAR_BER}[status]
    r = lo.bar_rect()

    pygame.draw.rect(surf, BAR_ACT if active else BAR_BG, r,
                     border_radius=max(3, int(5 * s)))
    pygame.draw.rect(surf, col_brd, r, 2,
                     border_radius=max(3, int(5 * s)))

    lbl = lo.mfont.render("Singmaster ›", True, (110, 110, 155))
    surf.blit(lbl, (lo.BAR_X + max(4, int(8 * s)),
                    lo.BAR_Y + lo.BAR_H // 2 - lbl.get_height() // 2))

    cursor = '|' if active and (pygame.time.get_ticks() // 500) % 2 == 0 else ' '
    ts = lo.mfont.render(text + cursor, True,
                         (255, 255, 255) if active else (180, 180, 210))
    surf.blit(ts, (lo.BAR_X + max(60, int(130 * s)),
                   lo.BAR_Y + lo.BAR_H // 2 - ts.get_height() // 2))

    if status == 'ok':
        m = lo.sfont.render("✓ ok", True, BAR_BOK)
        surf.blit(m, (lo.BAR_X + lo.BAR_W - m.get_width() - 8,
                      lo.BAR_Y + lo.BAR_H // 2 - m.get_height() // 2))
    elif status == 'error':
        m = lo.sfont.render("✗ invalid", True, BAR_BER)
        surf.blit(m, (lo.BAR_X + lo.BAR_W - m.get_width() - 8,
                      lo.BAR_Y + lo.BAR_H // 2 - m.get_height() // 2))

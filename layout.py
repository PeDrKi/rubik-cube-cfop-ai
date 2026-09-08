"""
layout.py
=========
Class Layout — tính toán lại toàn bộ geometry phụ thuộc kích thước cửa sổ.
Gọi lại Layout(W, H) mỗi khi cửa sổ bị resize.

Chỉ còn 2 cột: LEFT (6-Face View) và RIGHT (3D View).
"""

import pygame


class Layout:
    """
    Tất cả kích thước, tọa độ và font được tính từ W×H hiện tại.
    Kích thước tham chiếu thiết kế gốc: REF_W × REF_H.
    """
    REF_W = 1300
    REF_H = 880

    def __init__(self, W: int, H: int):
        self.W = W
        self.H = H

        sx = W / self.REF_W
        sy = H / self.REF_H
        s  = min(sx, sy)   # scale đồng đều
        self.s = s

        # ── Hai cột chính ─────────────────────────────────────────────────────
        self.LEFT_W  = int(530 * s)
        self.RIGHT_W = W - self.LEFT_W
        self.LEFT_X  = 0
        self.RIGHT_X = self.LEFT_W

        # ── 6-Face panel (cột LEFT) ───────────────────────────────────────────
        self.CELL  = max(12, int(32 * s))
        self.GAP   = max(1,  int( 3 * s))
        self.PAD   = max(2,  int( 5 * s))
        self.LBL_H = max(10, int(19 * s))
        self.PANEL = 3 * self.CELL + 2 * self.GAP + 2 * self.PAD
        self.PX0   = self.LEFT_X + max(6, int(18 * s))
        self.PY0   = max(20, int(52 * s))
        # Vị trí lưới của mỗi mặt: (col, row) trong bố cục chữ thập
        self.PGRID = {
            'U': (1, 0),
            'L': (0, 1), 'F': (1, 1), 'R': (2, 1), 'B': (3, 1),
            'D': (1, 2),
        }

        # ── Singmaster bar ────────────────────────────────────────────────────
        self.BAR_H = max(22, int(38 * s))
        self.BAR_Y = H - self.BAR_H - max(3, int(6 * s))
        self.BAR_X = self.LEFT_X + max(3, int(6 * s))
        self.BAR_W = self.LEFT_W - max(6, int(12 * s))

        # ── 3D panel (cột RIGHT) ──────────────────────────────────────────────
        self.CX3   = self.RIGHT_X + self.RIGHT_W // 2
        self.CY3   = H // 2 - max(8, int(20 * s))
        self.ZOOM0 = 130.0 * s

        # ── Rect vùng tương tác ───────────────────────────────────────────────
        self.AREA_3D = pygame.Rect(self.RIGHT_X, 0, self.RIGHT_W, H - 50)

        # ── Fonts ─────────────────────────────────────────────────────────────
        fs = lambda base, mn: max(mn, int(base * s))
        self.font   = pygame.font.SysFont("consolas", fs(17, 9),  bold=True)
        self.sfont  = pygame.font.SysFont("consolas", fs(13, 7))
        self.mfont  = pygame.font.SysFont("consolas", fs(15, 8))
        self.bfont  = pygame.font.SysFont("consolas", fs(21, 11), bold=True)
        self.bfont2 = pygame.font.SysFont("consolas", fs(22, 11), bold=True)

    # ── Helpers geometry ──────────────────────────────────────────────────────

    def panel_tl(self, face: str):
        """Tọa độ góc trên-trái của panel mặt face trong 6-face view."""
        gc, gr = self.PGRID[face]
        return (
            self.PX0 + gc * (self.PANEL + 5),
            self.PY0 + self.LBL_H + gr * (self.PANEL + self.LBL_H + 5),
        )

    def panel_rect(self, face: str) -> pygame.Rect:
        """pygame.Rect bao toàn bộ panel của mặt face."""
        x, y = self.panel_tl(face)
        return pygame.Rect(x, y, self.PANEL, self.PANEL)

    def sticker_rect(self, face: str, r: int, c: int) -> pygame.Rect:
        """pygame.Rect của ô sticker (r, c) trên mặt face."""
        x, y = self.panel_tl(face)
        return pygame.Rect(
            x + self.PAD + c * (self.CELL + self.GAP),
            y + self.PAD + r * (self.CELL + self.GAP),
            self.CELL, self.CELL,
        )

    def bar_rect(self) -> pygame.Rect:
        """pygame.Rect của Singmaster input bar."""
        return pygame.Rect(self.BAR_X, self.BAR_Y, self.BAR_W, self.BAR_H)

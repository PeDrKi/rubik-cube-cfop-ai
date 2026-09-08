"""Fake minimal pygame stub - CHI de cho phep import formula_panel.py /
main.py trong sandbox khong co internet (khong the pip install pygame that).
KHONG dung de chay app that (khong co rendering/window that su) -- chi de
chay cac test logic THUAN (khong dung display) trong test_app_logic.py.
"""
import types, sys

class Rect:
    def __init__(self, x=0, y=0, w=0, h=0):
        self.x, self.y, self.width, self.height = x, y, w, h
    @property
    def centerx(self): return self.x + self.width // 2
    @property
    def centery(self): return self.y + self.height // 2
    @property
    def bottom(self): return self.y + self.height
    def collidepoint(self, x, y=None):
        if y is None:
            x, y = x
        return self.x <= x <= self.x + self.width and self.y <= y <= self.y + self.height

class _Surface:
    def __init__(self, *a, **k): pass
    def get_clip(self): return None
    def set_clip(self, *a): pass
    def blit(self, *a, **k): pass
    def fill(self, *a, **k): pass
    def set_alpha(self, *a): pass
    def get_width(self): return 0
    def get_height(self): return 0

def Surface(*a, **k): return _Surface()

class font:
    @staticmethod
    def SysFont(*a, **k):
        class F:
            def size(self, t): return (len(t) * 7, 12)
            def render(self, *a, **k): return _Surface()
            def get_height(self): return 12
        return F()
    @staticmethod
    def init(): pass

class draw:
    @staticmethod
    def rect(*a, **k): pass

class _sdl2:
    class video:
        Window = None

locals_mod = types.ModuleType("pygame.locals")
sys.modules['pygame.locals'] = locals_mod
sys.modules['pygame._sdl2'] = _sdl2
sys.modules['pygame._sdl2.video'] = _sdl2.video

SCRAP_TEXT = 'text'
class scrap:
    @staticmethod
    def init(): pass
    @staticmethod
    def put(*a, **k): pass
    @staticmethod
    def get(*a, **k): return None

def init(): pass
def quit(): pass

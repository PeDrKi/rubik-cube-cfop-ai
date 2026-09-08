"""
solver/move_simplify.py
=========================
Rut gon chuoi nuoc di sau khi giai (khong doi ket qua cuoi cung, chi lam
gon hon). Huu ich vi cac solver (dac biet PLL, ghep nhieu "khoi" thuat toan
lai voi nhau) co the sinh ra nuoc di du thua o ranh gioi giua cac khoi.

Ky thuat: 2 nuoc tren MAT DOI DIEN (U/D, F/B, L/R) GIAO HOAN duoc voi nhau
(doi thu tu khong lam thay doi ket qua), nen co the "bubble" 1 nuoc qua cac
nuoc doi dien lien ke de tim co hoi gop VOI nuoc CUNG MAT o xa hon. Sau khi
gop, cac nuoc cung mat duoc CONG DON (VD R + R = R2, R + R' = huy, R2 + R'
= R). Lap lai toi khi khong con rut gon duoc nua.
"""

_OPPOSITE = {'U': 'D', 'D': 'U', 'F': 'B', 'B': 'F', 'L': 'R', 'R': 'L'}


def _face(mv):
    return mv[0]


def _turns(mv):
    if mv.endswith('2'):
        return 2
    if mv.endswith("'"):
        return 3
    return 1


def _make(face, turns):
    turns = turns % 4
    if turns == 0:
        return None
    if turns == 1:
        return face
    if turns == 2:
        return face + '2'
    return face + "'"


def simplify(moves):
    """Rut gon 1 danh sach nuoc di Singmaster. Tra ve danh sach moi (co the
    ngan hon), KHONG thay doi hieu ung cuoi cung len cube."""
    moves = list(moves)
    changed = True
    while changed:
        changed = False
        # buoc 1: gop 2 nuoc CUNG MAT lien ke (R,R->R2 ; R,R'->huy ; ...)
        out = []
        i = 0
        while i < len(moves):
            if i + 1 < len(moves) and _face(moves[i]) == _face(moves[i + 1]):
                nm = _make(_face(moves[i]), _turns(moves[i]) + _turns(moves[i + 1]))
                if nm:
                    out.append(nm)
                i += 2
                changed = True
            else:
                out.append(moves[i])
                i += 1
        moves = out

        # buoc 2: thu doi cho 1 cap nuoc tren MAT DOI DIEN lien ke (giao
        # hoan, khong doi ket qua) NEU lam vay tao ra 1 cap CUNG MAT moi de
        # gop o buoc sau. Chi doi 1 cap moi lan de giu don gian & an toan.
        for i in range(len(moves) - 2):
            a, b, c = moves[i], moves[i + 1], moves[i + 2]
            if _face(b) == _OPPOSITE.get(_face(a)) and _face(a) == _face(c):
                moves[i + 1], moves[i] = moves[i], moves[i + 1]
                changed = True
                break
    return moves

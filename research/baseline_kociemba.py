"""
research/baseline_kociemba.py
==============================
Nhom B (computer-like baseline) cho thi nghiem so sanh Human-Likeness.

Dung thu vien `kociemba` (Two-Phase Algorithm, Kociemba 1992) -- thuat toan
KINH DIEN thuoc dung nhanh "computer-like": khong quan tam cau truc CFOP,
chi toi uu so nuoc (thuong 18-23 nuoc, rat gan God's Number = 20).

Day la baseline KHACH QUAN -- khong tu viet lai de tranh thien vi so sanh
(tu viet 1 optimal solver rieng se kho dam bao dung chat luong nhu thu vien
da duoc cong dong kiem chung qua nhieu nam).

Cai dat: pip install kociemba
"""

import kociemba

# Thu vien kociemba dung thu tu mat U R F D L B, moi mat 9 ky tu doc theo
# hang, tu tren-trai xuong duoi-phai. Mau trung tam cua moi mat = ten mat do
# (vi engine hien tai luon giu center co dinh dung mau -- kiem tra ngay
# trong _facelets_to_kstring de bao loi som neu khong dung).
_KOCIEMBA_FACE_ORDER = ['U', 'R', 'F', 'D', 'L', 'B']


def _facelets_to_kstring(state: dict) -> str:
    """Chuyen facelet-state (dict[str, np.ndarray 3x3]) cua repo nay sang
    chuoi 54 ky tu ma thu vien kociemba doi hoi."""
    out = []
    for face in _KOCIEMBA_FACE_ORDER:
        grid = state[face]
        assert grid[1, 1] == face, (
            f"Center mat {face} khong dung mau ({grid[1, 1]}) -- "
            f"kiem tra lai orientation cua state truoc khi goi baseline"
        )
        for r in range(3):
            for c in range(3):
                out.append(str(grid[r, c]))
    return ''.join(out)


def solve_baseline(state: dict) -> list[str]:
    """Tra ve list nuoc di (Singmaster, vd 'R', "U'", 'F2') giai TOI UU
    GAN-TUYET-DOI (Two-Phase Kociemba, thuong <= 2 nuoc so voi God's Number
    20 trong hau het truong hop) tu state hien tai. Khong doi state truyen
    vao."""
    kstr = _facelets_to_kstring(state)
    sol_str = kociemba.solve(kstr)   # vd: "R L U2 R L' B2 U2 R2 F2 L2 D2 L2 F2"
    if not sol_str or sol_str.startswith('Error'):
        raise RuntimeError(f'Kociemba solve failed: {sol_str!r}')
    return sol_str.split()


if __name__ == '__main__':
    # Smoke test doc lap, khong phu thuoc module nao khac cua repo.
    solved = 'U' * 9 + 'R' * 9 + 'F' * 9 + 'D' * 9 + 'L' * 9 + 'B' * 9
    print('Test cube da giai:', kociemba.solve(solved))

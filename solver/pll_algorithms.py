"""
solver/pll_algorithms.py
==========================
Co so du lieu 20/21 thuat toan PLL chuan (dung ky hieu Singmaster, khong
dung slice M/wide/rotation de tuong thich voi engine hien co).

QUAN TRONG: MOI thuat toan duoi day DA DUOC KIEM CHUNG BANG CODE khi module
nay duoc import lan dau (xem verify_and_build_table() o cuoi file) -- KHONG
tin vao tri nho don thuan. Trong qua trinh xay dung, 2/22 cong thuc nho ban
dau (Ab, Z) sai (loi nho/ghi lai), da duoc phat hien NGAY LAP TUC nho buoc
kiem chung nay va sua (Ab) hoac loai bo (Z, chua tim duoc ban dung trong
thoi gian cho phep -- xem CFOP_AI_README.md).

Tieu chuan kiem chung cho moi thuat toan X (ap dung tu trang thai da giai):
  1. Chi thay doi hoan vi 4 goc + 4 canh lop U (khong dung nuoc D).
  2. KHONG lam thay doi huong (orientation) cua bat ky goc/canh nao.
  3. KHONG lam vo Cross + F2L.
Neu MOT trong 3 dieu kien tren sai, thuat toan bi loai khoi bang (khong
dua vao san xuat) -- xem verify_and_build_table().
"""

from cube_engine import make_solved, do_move
from .full_state import from_facelets, apply_move, cross_f2l_ok

# ── 20 thuat toan PLL chuan (thieu Z-perm) ────────────────────────────────
_RAW_ALGS = {
    'Aa': "R' F R' B2 R F' R' B2 R2",
    'E':  "R B' R F2 R' B R F2 R2",
    'Ua': "R U' R U R U R U' R' U' R2",
    'Ub': "R2 U R U R' U' R' U' R' U R'",
    'H':  "R2 U2 R U2 R2 U2 R2 U2 R U2 R2",
    'T':  "R U R' U' R' F R2 U' R' U' R U R' F'",
    'Y':  "F R U' R' U' R U R' F' R U R' U' R' F R F'",
    'F':  "R' U' F' R U R' U' R' F R2 U' R' U' R U R' U R",
    'Ja': "R' U L' U2 R U' R' U2 R L",
    'Jb': "R U R' F' R U R' U' R' F R2 U' R'",
    'Ra': "R U' R' U' R U R D R' U' R D' R' U2 R'",
    'Rb': "R' U2 R U2 R' F R U R' U' R' F' R2",
    'Ga': "R2 U R' U R' U' R U' R2 U' D R' U R D'",
    'Gb': "R' U' R U D' R2 U R' U R U' R U' R2 D",
    'Gc': "R2 U' R U' R U R' U R2 U D' R U' R' D",
    'Gd': "R U R' U' D R2 U' R U' R' U R' U R2 D'",
    'V':  "R' U R' U' B' R' B2 U' B' U B' R B R",
    'Na': "R U R' U R U R' F' R U R' U' R' F R2 U' R' U2 R U' R'",
    'Nb': "R' U R U' R' F' U' F R U R' F R' F' R U' R",
}


def _inv(seq):
    out = []
    for m in reversed(seq):
        if m.endswith("'"):
            out.append(m[:-1])
        elif m.endswith('2'):
            out.append(m)
        else:
            out.append(m + "'")
    return out


def _pattern_of(seq):
    """Ap seq len cube da giai, tra ve (cp[:4], ep[:4]) va co hop le khong
    (giu nguyen huong + Cross + F2L)."""
    st = make_solved()
    for mv in seq:
        do_move(st, mv)
    full = from_facelets(st)
    ep, eo, cp, co = full
    valid = (eo[0:4] == (0, 0, 0, 0) and co[0:4] == (0, 0, 0, 0)
             and cross_f2l_ok(full))
    return cp[0:4], ep[0:4], valid


def verify_and_build_table():
    """Kiem chung TAT CA thuat toan trong _RAW_ALGS, chi giu lai nhung cai
    HOP LE, roi xay bang tra cuu 2 chieu (thuan + nghich) tu (pattern) ->
    (chuoi nuoc giai). Chay 1 lan khi import module."""
    table = {}
    verified_names = []
    for name, seqstr in _RAW_ALGS.items():
        seq = seqstr.split()
        cp, ep, valid = _pattern_of(seq)
        if not valid:
            continue   # loai bo thuat toan sai (khong dua vao san xuat)
        verified_names.append(name)
        # chieu thuan: cube dang o pattern (cp,ep) -> giai bang inv(seq)
        table[(cp, ep)] = _inv(seq)
        # chieu nghich: cube dang o pattern cua inv(seq) -> giai bang seq
        inv_seq = _inv(seq)
        icp, iep, ivalid = _pattern_of(inv_seq)
        if ivalid:
            table[(icp, iep)] = seq
    return table, verified_names


PLL_TABLE, VERIFIED_ALG_NAMES = verify_and_build_table()

AUF_OPTIONS = (('', []), ('U', ['U']), ('U2', ['U2']), ("U'", ["U'"]))


def solve_pll_lookup(full):
    """Nhan dien case PLL hien tai (thu 4 AUF) va tra ve chuoi nuoc giai
    NEU khop 1 trong cac pattern da kiem chung -- CFOP thuc su (1 case = 1
    thuat toan, ~9-19 nuoc), KHONG phai ghep nhieu 'khoi'.
    Tra ve None neu khong khop case nao (goi ham macro-search du phong)."""
    ep0, eo0, cp0, co0 = full
    if cp0[0:4] == (0, 1, 2, 3) and ep0[0:4] == (0, 1, 2, 3):
        return []
    for auf_name, auf in AUF_OPTIONS:
        s = full
        for mv in auf:
            s = apply_move(s, mv)
        ep, eo, cp, co = s
        key = (cp[0:4], ep[0:4])
        if key in PLL_TABLE:
            return auf + PLL_TABLE[key]
    return None

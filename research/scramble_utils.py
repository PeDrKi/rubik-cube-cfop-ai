"""
research/scramble_utils.py
=============================
Sinh scramble theo dung phuong phap WCA su dung: RANDOM-STATE, khong phai
RANDOM-MOVE (da dung trong cac experiment truoc -- xem Threats to
Validity trong paper/main.tex).

Khac biet quan trong:
  - random-move (cach cu): ap N nuoc ngau nhien (vd 20 nuoc) tu cube da
    giai. VAN DE: N nuoc ngau nhien co the "trung hoa" lan nhau (vd R R'
    khong lien tiep nhung vAn huy nhau ve mat hoan vi), khien scramble
    THUONG DE HON scramble thi dau thuc te, va PHAN BO DO KHO khong deu.
  - random-state (dung theo WCA): sinh 1 TRANG THAI ngau nhien GAN NHU
    DEU tren toan bo khong gian ~4.3x10^19 trang thai, roi lay NGHICH DAO
    cua 1 loi giai (gan) toi uu cho trang thai do lam scramble. Dam bao
    scramble luon "kho toi da" (can dung so nuoc toi uu de giai), dung
    tinh than cua bo sinh scramble WCA that (dung thuat toan Kociemba/Korf
    tuong tu, chi khac ho dung thuat toan toi uu tuyet doi con o day dung
    Kociemba xap xi de kha thi ve toc do).

Ky thuat: thay vi cai dat lai toan bo ly thuyet nhom (sampling deu tren
S8 x S12 x Z3^7 x Z2^11 voi rang buoc parity), ta dung 2 buoc don gian
nhung dung ve mat thong ke:
  1. Random walk DAI (vd 300 nuoc) tu cube da giai -- voi metric group co
     duong kinh chi 20, 300 nuoc la qua du de hoi tu gan nhu deu
     (mixing) tren toan bo khong gian trang thai dat duoc.
  2. Dung Kociemba tim loi giai (gan) toi uu cho trang thai do, LAY
     NGHICH DAO lam scramble cuoi cung -- dam bao scramble nay THUC SU
     kho (~18-23 nuoc de giai toi uu), khong bi "trung hoa ngau nhien"
     nhu cach random-move truoc day.
"""

import sys

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES, random_scramble_moves


def _random_walk(st, n, rng):
    """Wrapper mong cho tuong thich nguoc (khong tra ve gi, chi ap len
    st) -- logic that su nam trong cube_engine.random_scramble_moves,
    dung chung voi scramble_cube(), demo_ai_search.py, benchmark.py,
    research/run_experiment.py (truoc day 5 noi nay moi noi tu viet lai
    cung 1 doan logic, xem CHANGELOG_SESSION.md)."""
    random_scramble_moves(st, n, rng=rng)


def _invert(moves):
    inv_map = {}
    out = []
    for mv in reversed(moves):
        if mv.endswith('2'):
            out.append(mv)
        elif mv.endswith("'"):
            out.append(mv[:-1])
        else:
            out.append(mv + "'")
    return out


def random_state_scramble(rng, mix_moves=300):
    """Tra ve (scramble_moves, state) theo phuong phap random-state chuan
    WCA (xem docstring dau file). `rng` la random.Random(seed) de
    reproducible. scramble_moves la list Singmaster; state la facelet-state
    SAU KHI da scramble (ap scramble_moves tu cube da giai)."""
    from research.baseline_kociemba import solve_baseline

    st = make_solved()
    _random_walk(st, mix_moves, rng)

    # Tim loi giai (gan) toi uu cho trang thai "gan deu ngau nhien" nay
    near_optimal_solve = solve_baseline(st)
    scramble_moves = _invert(near_optimal_solve)

    # Ap scramble_moves tu cube da giai de co state cuoi cung dung de tra ve
    st2 = make_solved()
    for mv in scramble_moves:
        do_move(st2, mv)
    return scramble_moves, st2


if __name__ == '__main__':
    import random
    from cube_engine import cube_solved

    rng = random.Random(42)
    scramble, st = random_state_scramble(rng)
    print('Scramble (random-state, WCA-style):', ' '.join(scramble))
    print('So nuoc scramble:', len(scramble))

    # Kiem chung: giai lai bang Kociemba, phai ra dung so nuoc GAN BANG
    # do dai scramble (vi scramble = nghich dao cua 1 loi giai gan-toi-uu,
    # nen giai lai cung se ra do dai tuong duong -- neu ngan hon han thi
    # co gi do sai).
    from research.baseline_kociemba import solve_baseline
    check = solve_baseline(st)
    for mv in check:
        do_move(st, mv)
    print('Giai lai bang Kociemba:', len(check), 'nuoc -- da giai dung chua?', cube_solved(st))

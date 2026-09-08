"""
research/trigger_biased_search.py
====================================
Dong gop thuat toan cua de tai: THIEN VI A* search (F2L) huong ve cac
chuoi nuoc trung khop trigger nguoi quen thuoc (finger_tricks.py), bang
cach TRU BOT gia tri heuristic cho cac nhanh da hoan thanh 1 trigger.

Khong sua doi truc tiep solver/f2l_solver.py logic mac dinh -- chi dung
hook trigger_bonus_fn/lam da duoc them (backward-compatible, xem
solver/f2l_solver.py docstring).

Cong thuc heuristic moi (xem solver/f2l_solver.py::_heuristic):
    f'(n) = g(n) + h_PDB(n) + penalty(n) - lambda * bonus(path)
trong do bonus(path) = so nuoc trong `path` hien tai thuoc ve it nhat 1
trigger da biet (dung greedy longest-match, TAI SU DUNG dung logic
research/hli_metrics.py::trigger_overlap de dam bao NHAT QUAN giua luc
search va luc do luong HLI sau do).
"""

import sys
sys.path.insert(0, '.')

from research.finger_tricks import all_trigger_variants

_TRIGGER_VARIANTS = all_trigger_variants()
_ALL_TRIGGERS_BY_LEN = sorted(
    ((name, seq) for name, seqs in _TRIGGER_VARIANTS.items() for seq in seqs),
    key=lambda x: -len(x[1])
)


def trigger_bonus_count(path: tuple, cap=None) -> int:
    """So nuoc trong `path` (tuple nuoc di) duoc bao phu boi it nhat 1
    trigger da biet (greedy longest-match trai->phai). Dung y het thuat
    toan trong hli_metrics.trigger_overlap() nhung tra ve SO NUOC (int)
    thay vi ty le, vi day duoc goi hang ngan lan/giay trong vong lap A*
    -- can nhanh, tranh chia float khong can thiet o hot path.

    `cap`: [NGHIEN CUU -- capped-bonus admissibility experiment, xem
    research/run_capped_bonus_sweep.py] neu duoc cho (khong phai None),
    gioi han gia tri tra ve toi da la `cap`. Muc dich: neu heuristic
    goc (base = single-piece PDB) la admissible, thi h' = h - lam*bonus
    voi bonus KHONG cap co the giam khong gioi han khi path dai ra,
    lam mat admissibility hoan toan (khong co can). Voi bonus_capped =
    min(bonus, cap), do lech toi da so voi h admissible bi chan boi
    lam*cap -- cho phep phat bieu 1 CAN THUC NGHIEM ro rang thay vi chi
    quan sat dinh tinh 've mat cham lai/di nhieu nuoc hon'. Mac dinh
    cap=None -> hanh vi y het ban khong cap (KHONG anh huong ket qua da
    co trong Exp2)."""
    n = len(path)
    if n == 0:
        return 0
    covered = [False] * n
    i = 0
    while i < n:
        matched_len = 0
        for _name, seq in _ALL_TRIGGERS_BY_LEN:
            L = len(seq)
            if i + L <= n and tuple(path[i:i + L]) == seq:
                matched_len = L
                break
        if matched_len:
            for k in range(i, i + matched_len):
                covered[k] = True
            i += matched_len
        else:
            i += 1
    total = sum(covered)
    return total if cap is None else min(total, cap)


def solve_f2l_trigger_biased(state, lam, cap=None, **kwargs):
    """Wrapper mong: goi thang solver.f2l_solver.solve_f2l voi
    trigger_bonus_fn=trigger_bonus_count va lam duoc chi dinh. lam=0.0
    tuong duong solve_f2l() goc (baseline, khong thien vi).

    `cap`: [NGHIEN CUU] xem trigger_bonus_count() -- None (mac dinh) =
    hanh vi goc khong doi. Neu khac None, bonus bi cap tai gia tri nay
    truoc khi tru vao heuristic."""
    from functools import partial
    from solver.f2l_solver import solve_f2l
    bonus_fn = partial(trigger_bonus_count, cap=cap) if cap is not None else trigger_bonus_count
    return solve_f2l(state, trigger_bonus_fn=bonus_fn, lam=lam, **kwargs)


if __name__ == '__main__':
    # Smoke test: ham bonus phai nhan dien duoc 1 trigger biet truoc.
    sexy = ('R', 'U', "R'", "U'")
    print('bonus cho 1 sexy-move day du:', trigger_bonus_count(sexy), '/', len(sexy))
    mixed = ('R', 'U', "R'", "U'", 'F', 'B2')
    print('bonus cho sexy-move + 2 nuoc la:', trigger_bonus_count(mixed), '/', len(mixed))

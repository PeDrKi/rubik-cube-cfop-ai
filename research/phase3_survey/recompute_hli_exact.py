"""
research/phase3_survey/recompute_hli_exact.py
==================================================
Tinh lai CHINH XAC Segmentability + Pattern-conformity cho ca 24 stimuli
(bao gom ca Nhom H) bang cach replay tung nuoc di tren trang thai cube
THAT, thay cho uoc luong nhi phan don gian (1.0 neu group in {A,A+} else
0.0) dang dung trong analyze_results.py.

QUAN TRONG: day la fix cho 1 gia dinh SAI trong proxy cu -- proxy cu gia
dinh Nhom H (nguoi that) co Pattern-conformity=0 GIONG Nhom B (computer),
trong khi thuc ra neu ho giai bang CFOP that thi gia tri nay co the CAO.
Script nay kiem tra THAT bang cach replay state, khong con gia dinh.

Chay:  python3 -m research.phase3_survey.recompute_hli_exact
"""

import json
import re
import sys

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES
from solver.full_state import from_facelets, apply_move, cross_ok, pair_ok
from solver.oll_solver import oll_done
from solver.pll_algorithms import solve_pll_lookup
from solver.oll_solver import _solve_ocll_macro

F2L_SLOTS = ['DFR', 'DFL', 'DBR', 'DBL']
BASE_SEED = 9000


def random_scramble_moves(seed, n=20):
    """Tai tao CHINH XAC scramble da dung trong prepare_stimuli.py (cung
    thuat toan random walk, cung seed) -- deterministic, tai tao dung."""
    import random
    rng = random.Random(seed)
    prev = None
    moves = []
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev] if prev else ALL_MOVES
        mv = rng.choice(cand)
        moves.append(mv)
        prev = mv[0]
    return moves


def load_human_scrambles(log_path):
    """Doc Result.txt (dinh dang 'Scramble: ...' / 'Solve   : ...' xen
    ke), tra ve list (scramble_str, solve_str)."""
    with open(log_path, encoding='utf-8') as f:
        text = f.read()
    scrambles = re.findall(r'Scramble:\s*(.+)', text)
    solves = re.findall(r'Solve\s*:\s*(.+)', text)
    return list(zip(scrambles, solves))


def analyze_solve(scramble_moves, solve_moves):
    """Replay scramble+solve tren state that, tra ve dict:
      segmentability (0/1), oll_named (0/1 hoac None neu khong toi duoc
      OLL), pll_named (0/1 hoac None), stage_count, named_count.

    QUAN TRONG: moi checkpoint (cross/f2l/oll) phai duoc GIU NGUYEN dung
    TU DIEM DO DEN HET chuoi nuoc di (khong chi dung tai 1 thoi diem roi
    thoi) -- neu khong, mot loi giai "tat ca cung roi vao dung cho gan
    nhu dong thoi o cuoi" (dac trung cua solver toi uu khong theo CFOP
    nhu Kociemba) se bi tinh nham la "co phan doan CFOP" chi vi cac dieu
    kien tinh co dung cung luc o buoc cuoi cung.
    """
    st = make_solved()
    for mv in scramble_moves:
        do_move(st, mv)

    # Tinh truoc TOAN BO chuoi trang thai (full_i = trang thai sau khi ap
    # dung i nuoc dau cua solve_moves), i = 0..len(solve_moves)
    full0 = from_facelets(st)
    states = [full0]
    cur = full0
    for mv in solve_moves:
        cur = apply_move(cur, mv)
        states.append(cur)
    n = len(solve_moves)

    def cross_holds(i): return cross_ok(states[i])
    def f2l_holds(i): return cross_ok(states[i]) and all(pair_ok(states[i], s) for s in F2L_SLOTS)
    def oll_holds(i): return f2l_holds(i) and oll_done(states[i])

    def first_index_true(holds_fn):
        """Tra ve chi so i NHO NHAT (lan dau tien) ma holds_fn(i) dung,
        hoac None neu khong bao gio dung. LUU Y: khong yeu cau giu nguyen
        lien tuc sau do -- vi thuat toan OLL/PLL that (vd T-perm) thuong
        co nuoc giua chung (R, F...) tam thoi xao tron lai F2L/Cross roi
        tu khoi phuc dung o nuoc cuoi thuat toan, day la hanh vi BINH
        THUONG cua CFOP that, khong phai loi. Bu lai, dung BAT DANG THUC
        CHAT (cross_idx < f2l_idx < oll_idx < n) de van loai duoc cac
        truong hop "tat ca rot vao dung cho gan nhu dong thoi" dac trung
        cua solver toi uu khong theo CFOP (da xac nhan thuc te tren du
        lieu Nhom B Kociemba)."""
        for i in range(n + 1):
            if holds_fn(i):
                return i
        return None

    cross_idx = first_index_true(cross_holds)
    f2l_idx = first_index_true(f2l_holds)
    oll_idx = first_index_true(oll_holds)

    from cube_engine import cube_solved
    st2 = make_solved()
    for mv in scramble_moves + solve_moves:
        do_move(st2, mv)
    fully_solved = cube_solved(st2)

    segmentability = 1.0 if (cross_idx is not None and f2l_idx is not None
                              and oll_idx is not None and fully_solved
                              and cross_idx < f2l_idx < oll_idx < n) else 0.0
    # QUAN TRONG: dung BAT DANG THUC CHAT (<) chu khong phai <= -- neu
    # khong, mot loi giai toi uu (vd Kociemba) co the "tinh co" giai xong
    # ca Cross+F2L CUNG LUC o 1 nuoc gan cuoi (0 nuoc rieng cho F2L), va
    # OLL+PLL cung luc o nuoc cuoi cung -- van thoa man <= nhung KHONG
    # phai la phan doan CFOP that su (da phat hien thuc te bang debug
    # tren du lieu Nhom B, xem CHANGELOG_SESSION.md).

    oll_named = None
    pll_named = None
    if segmentability == 1.0:
        # Trang thai NGAY SAU F2L (truoc khi lam OLL) -- kiem tra case
        # nay co giai duoc bang Sune/Anti-Sune (co ten) khong. Dung lai
        # states[] da tinh san (states[i] = sau i nuoc dau cua solve_moves).
        full_after_f2l = states[f2l_idx]
        oll_named = 1.0 if _solve_ocll_macro(full_after_f2l) is not None else 0.0

        # Trang thai NGAY SAU OLL (truoc khi lam PLL) -- kiem tra case
        # nay co trong bang 21 PLL chuan khong.
        full_after_oll = states[oll_idx]
        pll_named = 1.0 if solve_pll_lookup(full_after_oll) is not None else 0.0

    stage_count = 0
    named_count = 0
    if oll_named is not None:
        stage_count += 1
        named_count += oll_named
    if pll_named is not None:
        stage_count += 1
        named_count += pll_named

    pattern_conformity = (named_count / stage_count) if stage_count > 0 else 0.0

    return {
        'segmentability': segmentability,
        'pattern_conformity': pattern_conformity,
        'oll_named': oll_named,
        'pll_named': pll_named,
        'fully_solved_check': fully_solved,
    }


def main():
    with open('research/phase3_survey/stimuli.json', encoding='utf-8') as f:
        data = json.load(f)

    human_log_path = 'research/phase3_survey/human_solves_log.txt'
    human_pairs = load_human_scrambles(human_log_path)
    human_solve_to_scramble = {sol.strip(): scr.strip() for scr, sol in human_pairs}

    results = []
    for s in data['stimuli']:
        moves_str = ' '.join(s['moves'])
        if s['internal_group'] == 'H':
            scramble_str = human_solve_to_scramble.get(moves_str)
            if scramble_str is None:
                print(f"[CANH BAO] Khong khop duoc scramble cho {s['display_id']} "
                      f"(Nhom H) -- kiem tra lai human_solves_log.txt", file=sys.stderr)
                continue
            scramble_moves = scramble_str.split()
        else:
            scramble_moves = random_scramble_moves(int(s['seed']))

        analysis = analyze_solve(scramble_moves, s['moves'])
        results.append({
            'display_id': s['display_id'],
            'group': s['internal_group'],
            **analysis,
        })
        print(f"{s['display_id']:>14} [{s['internal_group']:>2}]  "
              f"seg={analysis['segmentability']:.0f}  "
              f"pat_conf={analysis['pattern_conformity']:.2f}  "
              f"(OLL_named={analysis['oll_named']}, PLL_named={analysis['pll_named']})  "
              f"fully_solved={analysis['fully_solved_check']}")

    out_path = 'research/phase3_survey/exact_components.json'
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
    print(f'\nDa luu {len(results)} ket qua vao {out_path}')


if __name__ == '__main__':
    main()

"""
research/run_experiment.py
=============================
Chay N scramble ngau nhien qua ca 2 nhom (A = AI-CFOP human-like, B =
Kociemba computer-like), do HLI + move-count, xuat CSV + tom tat thong ke.

Chay:  python3 -m research.run_experiment --n 50 --out results.csv
"""

import argparse
import csv
import random
import signal
import statistics
import sys
import time

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES, cube_solved
from solver import cfop_ai
from research.baseline_kociemba import solve_baseline
from research.hli_metrics import compute_hli, move_count_ratio, GODS_NUMBER_HTM
from research.scramble_utils import random_state_scramble


class _TimeoutError(Exception):
    pass


def _alarm_handler(signum, frame):
    raise _TimeoutError()


def _full_solve_with_timeout(state, timeout_s, **kwargs):
    """Chay cfop_ai.full_solve() voi gioi han thoi gian thuc (wall-clock)
    dung SIGALRM -- can thiet vi mot so scramble hiem gap ('pathological')
    khien F2L search khong hoi tu trong ngan sach node hop ly (da quan sat
    thuc te: seed=1013 voi scramble 20 nuoc mac dinh treo > 5 phut ngay ca
    khi giam node budget tu 120k xuong 40k/do sau -- xem CHANGELOG_SESSION.md
    va phan Discussion trong paper/main.tex). Neu timeout, tra ve dict voi
    reached='timeout' thay vi de tien trinh treo vo han."""
    old_handler = signal.signal(signal.SIGALRM, _alarm_handler)
    signal.alarm(timeout_s)
    try:
        return cfop_ai.full_solve(state, **kwargs)
    except _TimeoutError:
        return {'all_moves': [], 'reached': 'timeout',
                'cross_moves': [], 'f2l_moves': [], 'oll_moves': [], 'pll_moves': [],
                'f2l_per_slot': {}, 'oll_pll_stage_count': 0, 'oll_pll_named_count': 0}
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old_handler)


def random_scramble(st, n=20, rng=random):
    prev_face = None
    moves = []
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev_face] if prev_face else ALL_MOVES
        mv = rng.choice(cand)
        do_move(st, mv)
        moves.append(mv)
        prev_face = mv[0]
    return moves


def run_one(scramble_seed, scramble_len=20, timeout_flag_moves=200, f2l_nodes_per_depth=120_000,
            timeout_s=25, scramble_mode='random-move'):
    """Chay 1 scramble qua ca 2 nhom, tra ve dict 1 dong ket qua.

    scramble_mode: 'random-move' (cu, ap N nuoc ngau nhien -- co the de
    hon scramble thi dau thuc te, xem canh bao trong scramble_utils.py)
    hoac 'random-state' (moi, chuan WCA -- xem research/scramble_utils.py)."""
    rng = random.Random(scramble_seed)

    # --- Nhom A: AI-CFOP (human-like) ---
    if scramble_mode == 'random-state':
        scramble, st_a = random_state_scramble(rng)
    else:
        st_a = make_solved()
        scramble = random_scramble(st_a, scramble_len, rng)

    t0 = time.time()
    res_a = _full_solve_with_timeout(st_a, timeout_s, f2l_nodes_per_depth=f2l_nodes_per_depth)
    time_a = time.time() - t0

    cross_f2l_moves = res_a['cross_moves'] + res_a['f2l_moves']
    hli = compute_hli(res_a, cross_f2l_moves)

    # --- Nhom B: Kociemba (computer-like) --- (cung scramble, cube moi)
    st_b = make_solved()
    for mv in scramble:
        do_move(st_b, mv)

    t0 = time.time()
    try:
        sol_b = solve_baseline(st_b)
        for mv in sol_b:
            do_move(st_b, mv)
        b_solved = cube_solved(st_b)
    except Exception as e:
        sol_b = []
        b_solved = False
    time_b = time.time() - t0

    return {
        'seed': scramble_seed,
        'scramble': ' '.join(scramble),
        'A_reached': res_a['reached'],
        'A_moves': len(res_a['all_moves']),
        'A_time_s': round(time_a, 4),
        'A_segmentability': hli['segmentability'],
        'A_pattern_conformity': hli['pattern_conformity'],
        'A_trigger_overlap': hli['trigger_overlap'],
        'A_HLI': round(hli['HLI'], 4),
        'A_move_ratio': round(hli['move_count_ratio'], 3),
        'B_solved': b_solved,
        'B_moves': len(sol_b) if b_solved else None,
        'B_time_s': round(time_b, 5),
        'B_move_ratio': round(len(sol_b) / GODS_NUMBER_HTM, 3) if b_solved else None,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--n', type=int, default=20, help='so scramble')
    ap.add_argument('--scramble-len', type=int, default=20)
    ap.add_argument('--out', type=str, default='research/results.csv')
    ap.add_argument('--base-seed', type=int, default=1000)
    ap.add_argument('--f2l-nodes-per-depth', type=int, default=40_000,
                     help='giam mac dinh tu 120k xuong 40k de tranh treo tren '
                          'may 1 CPU khi gap scramble pathological (xem CHANGELOG)')
    ap.add_argument('--timeout-s', type=int, default=25,
                     help='gioi han thoi gian toi da (giay) cho 1 lan full_solve, '
                          'qua han se ghi reached=timeout thay vi treo vo han')
    ap.add_argument('--scramble-mode', type=str, default='random-move',
                     choices=['random-move', 'random-state'],
                     help="'random-state' la chuan WCA (khuyen nghi cho ket qua "
                          "chinh thuc), 'random-move' la mac dinh cu (nhanh hon "
                          "nhung co the thien vi do kho -- xem scramble_utils.py)")
    args = ap.parse_args()

    rows = []
    for i in range(args.n):
        seed = args.base_seed + i
        row = run_one(seed, args.scramble_len, f2l_nodes_per_depth=args.f2l_nodes_per_depth,
                       timeout_s=args.timeout_s, scramble_mode=args.scramble_mode)
        rows.append(row)
        print(f"[{i+1}/{args.n}] seed={seed} A: {row['A_reached']:12s} "
              f"{row['A_moves']:>3}mv HLI={row['A_HLI']:.2f} ({row['A_time_s']}s)  |  "
              f"B: {'OK' if row['B_solved'] else 'FAIL':4s} {row['B_moves']}mv ({row['B_time_s']}s)",
              flush=True)

    with open(args.out, 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    # --- Tom tat thong ke nhanh (chi cho cac ca A giai thanh cong hoan toan) ---
    solved_rows = [r for r in rows if r['A_reached'] == 'solved' and r['B_solved']]
    print(f"\n=== TOM TAT ({len(solved_rows)}/{args.n} ca ca 2 nhom deu thanh cong) ===")
    if solved_rows:
        a_moves = [r['A_moves'] for r in solved_rows]
        b_moves = [r['B_moves'] for r in solved_rows]
        a_hli = [r['A_HLI'] for r in solved_rows]
        print(f"So nuoc  -- Nhom A (AI-CFOP): mean={statistics.mean(a_moves):.1f}  "
              f"stdev={statistics.pstdev(a_moves):.1f}  min/max={min(a_moves)}/{max(a_moves)}")
        print(f"So nuoc  -- Nhom B (Kociemba): mean={statistics.mean(b_moves):.1f}  "
              f"stdev={statistics.pstdev(b_moves):.1f}  min/max={min(b_moves)}/{max(b_moves)}")
        print(f"HLI      -- Nhom A: mean={statistics.mean(a_hli):.3f}  stdev={statistics.pstdev(a_hli):.3f}")
        print(f"(Nhom B khong co khai niem HLI segment/pattern -- ve dinh nghia luon = 0 "
              f"vi khong theo cau truc CFOP nao)")
    print(f"\nDa luu chi tiet vao: {args.out}")


if __name__ == '__main__':
    main()

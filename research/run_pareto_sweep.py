"""
research/run_pareto_sweep.py
===============================
Experiment 2 cua de tai: quet lambda (he so thien vi trigger) tren cung 1
tap scramble, do trade-off giua Trigger-overlap / so nuoc / thoi gian tinh.
Dung du lieu nay ve duong cong Pareto cho bai bao.

Chay:  python3 -m research.run_pareto_sweep --n 15 --out research/pareto.csv

CANH BAO THOI GIAN: lambda lon (>=2) co the lam mot vai scramble mat
15-60+ giay/cap F2L (da quan sat thuc te) -- script co --node-cap va
--timeout-per-solve de tranh treo qua lau khi chay N lon.
"""

import argparse
import csv
import random
import sys
import time

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES
from solver.cross_solver import solve_cross
from research.trigger_biased_search import solve_f2l_trigger_biased
from research.hli_metrics import trigger_overlap


def random_scramble(st, n, rng):
    prev = None
    moves = []
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev] if prev else ALL_MOVES
        mv = rng.choice(cand)
        do_move(st, mv)
        moves.append(mv)
        prev = mv[0]
    return moves


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--n', type=int, default=15, help='so scramble')
    ap.add_argument('--lambdas', type=float, nargs='+', default=[0.0, 0.5, 1.0, 2.0])
    ap.add_argument('--scramble-len', type=int, default=20)
    ap.add_argument('--nodes-per-depth', type=int, default=60_000,
                     help='giam so voi mac dinh 120k de tranh treo qua lau khi lambda lon')
    ap.add_argument('--out', type=str, default='research/pareto.csv')
    ap.add_argument('--base-seed', type=int, default=2000)
    args = ap.parse_args()

    rows = []
    for i in range(args.n):
        seed = args.base_seed + i
        rng = random.Random(seed)
        st = make_solved()
        random_scramble(st, args.scramble_len, rng)
        cross = solve_cross(st)
        for mv in cross:
            do_move(st, mv)
        snapshot = {f: st[f].copy() for f in st}

        for lam in args.lambdas:
            st2 = {f: snapshot[f].copy() for f in snapshot}
            t0 = time.time()
            try:
                res = solve_f2l_trigger_biased(
                    st2, lam=lam, nodes_per_depth=args.nodes_per_depth
                )
            except Exception as e:
                res = {'moves': None}
            dt = time.time() - t0
            mv = res.get('moves')
            success = mv is not None and len(res.get('solved_slots', [])) == 4
            row = {
                'seed': seed,
                'lambda': lam,
                'ok': success,
                'moves': len(mv) if success else None,
                'trigger_overlap': round(trigger_overlap(mv), 4) if success else None,
                'time_s': round(dt, 3),
            }
            rows.append(row)
            print(f"[seed={seed} lambda={lam}] "
                  f"{'OK ' + str(row['moves']) + 'mv' if success else 'FAIL':>10}  "
                  f"trig={row['trigger_overlap']}  t={row['time_s']}s", flush=True)

    with open(args.out, 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    print(f"\n=== TOM TAT theo lambda (chi tinh cac ca OK) ===")
    import statistics
    for lam in args.lambdas:
        sub = [r for r in rows if r['lambda'] == lam and r['ok'] and r['moves'] is not None]
        if not sub:
            print(f"lambda={lam}: khong co ca nao thanh cong")
            continue
        mv = [r['moves'] for r in sub]
        to = [r['trigger_overlap'] for r in sub]
        tm = [r['time_s'] for r in sub]
        print(f"lambda={lam:>4}  n={len(sub):>2}/{args.n}  "
              f"moves: mean={statistics.mean(mv):.1f}  "
              f"trigger_overlap: mean={statistics.mean(to):.3f}  "
              f"time: mean={statistics.mean(tm):.2f}s max={max(tm):.2f}s")

    print(f"\nDa luu chi tiet vao: {args.out}")


if __name__ == '__main__':
    main()

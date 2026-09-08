"""
research/run_capped_bonus_sweep.py
====================================
Mo rong Experiment 2 (research/run_pareto_sweep.py): quet ca lambda VA
cap (gioi han bonus toi da) tren cung 1 tap scramble, de kiem tra
thuc nghiem xem cap bonus co giam duoc chi phi so nuoc / thoi gian
tinh "khong tuyen tinh, thinh thoang rat lon" da quan sat o Exp2 hay
khong, dong thoi cho phep phat bieu 1 CAN suboptimality thuc nghiem
ro rang (thay vi chi quan sat dinh tinh).

Dung LAI dung ham random_scramble/seed/scramble-len nhu
run_pareto_sweep.py de tap scramble co the doi chieu truc tiep voi
pareto_n60.csv da co (cap=None, cung lambda -> phai ra dung so lieu
da co trong paper, day la cach tu-kiem-chung script nay dung truoc
khi tin ket qua moi).

Chay:  python3 -m research.run_capped_bonus_sweep --n 60 \
           --lambdas 0.5 1.0 --caps 2 4 6 none \
           --out research/capped_bonus_n60.csv
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


def parse_cap(s):
    return None if s.lower() == 'none' else int(s)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--n', type=int, default=15, help='so scramble (dung voi --base-seed)')
    ap.add_argument('--seed-start', type=int, default=None,
                     help='neu dat, ghi de --base-seed: seed bat dau cho lo nay')
    ap.add_argument('--seed-count', type=int, default=None,
                     help='neu dat, ghi de --n: so seed trong lo nay')
    ap.add_argument('--lambdas', type=float, nargs='+', default=[0.5, 1.0])
    ap.add_argument('--caps', type=parse_cap, nargs='+', default=[1, 2, 4, None])
    ap.add_argument('--scramble-len', type=int, default=20)
    ap.add_argument('--nodes-per-depth', type=int, default=60_000)
    ap.add_argument('--out', type=str, default='research/capped_bonus.csv')
    ap.add_argument('--base-seed', type=int, default=2000)
    ap.add_argument('--append', action='store_true',
                     help='noi tiep vao file da co (khong ghi lai header)')
    args = ap.parse_args()

    seed_start = args.seed_start if args.seed_start is not None else args.base_seed
    seed_count = args.seed_count if args.seed_count is not None else args.n

    fieldnames = ['seed', 'lambda', 'cap', 'ok', 'moves', 'trigger_overlap', 'time_s']
    mode = 'a' if args.append else 'w'
    fout = open(args.out, mode, newline='')
    w = csv.DictWriter(fout, fieldnames=fieldnames)
    if not args.append:
        w.writeheader()
        fout.flush()

    for i in range(seed_count):
        seed = seed_start + i
        rng = random.Random(seed)
        st = make_solved()
        random_scramble(st, args.scramble_len, rng)
        cross = solve_cross(st)
        for mv in cross:
            do_move(st, mv)
        snapshot = {f: st[f].copy() for f in st}

        for lam in args.lambdas:
            for cap in args.caps:
                st2 = {f: snapshot[f].copy() for f in snapshot}
                t0 = time.time()
                try:
                    res = solve_f2l_trigger_biased(
                        st2, lam=lam, cap=cap, nodes_per_depth=args.nodes_per_depth
                    )
                except Exception:
                    res = {'moves': None}
                dt = time.time() - t0
                mv = res.get('moves')
                success = mv is not None and len(res.get('solved_slots', [])) == 4
                row = {
                    'seed': seed,
                    'lambda': lam,
                    'cap': 'none' if cap is None else cap,
                    'ok': success,
                    'moves': len(mv) if success else None,
                    'trigger_overlap': round(trigger_overlap(mv), 4) if success else None,
                    'time_s': round(dt, 3),
                }
                w.writerow(row)
                fout.flush()
                print(f"[seed={seed} lambda={lam} cap={row['cap']}] "
                      f"{'OK ' + str(row['moves']) + 'mv' if success else 'FAIL':>10}  "
                      f"trig={row['trigger_overlap']}  t={row['time_s']}s", flush=True)

    fout.close()
    print(f"\nDa luu (append={args.append}) vao: {args.out}")


if __name__ == '__main__':
    main()

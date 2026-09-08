"""
research/analyze_experiment2.py
===================================
Tinh lai thong ke Experiment 2 (Pareto trade-off) tu file CSV da co
san (research/pareto_n60.csv), tu-doi-chieu voi paper/main.tex.

File pareto_n60.csv NAY tai lap duoc chinh xac tu dau bang
`research/run_pareto_sweep.py` (da tu-kiem-chung trong phien lam viec
truoc -- khac voi results_randomstate_n60.csv/results_n100.csv cua
Experiment 1, xem ghi chu trong analyze_experiment1.py).

Chay:  python3 -m research.analyze_experiment2
"""

import csv
import sys
from collections import defaultdict

sys.path.insert(0, '.')

import numpy as np
from scipy.stats import wilcoxon


def main():
    csv_path = 'research/pareto_n60.csv'
    rows = list(csv.DictReader(open(csv_path)))
    print(f'Doc duoc {len(rows)} dong tu {csv_path}\n')

    by_lambda = defaultdict(dict)
    for r in rows:
        by_lambda[r['lambda']][r['seed']] = r

    print('=== Trung binh theo lambda (cac ca "ok") ===')
    for lam in ['0.0', '0.5', '1.0']:
        grp = by_lambda[lam]
        ok = [r for r in grp.values() if r['ok'] in ('True', '1', 'true')]
        to = np.array([float(r['trigger_overlap']) for r in ok])
        mv = np.array([float(r['moves']) for r in ok])
        print(f'lambda={lam}: n_ok={len(ok)}/60, trigger_overlap={to.mean():.3f}, moves={mv.mean():.1f}')

    l0 = by_lambda['0.0']
    l1 = by_lambda['1.0']
    ok0 = {s for s, r in l0.items() if r['ok'] in ('True', '1', 'true')}
    ok1 = {s for s, r in l1.items() if r['ok'] in ('True', '1', 'true')}
    paired = sorted(ok0 & ok1, key=int)
    n = len(paired)

    to0 = np.array([float(l0[s]['trigger_overlap']) for s in paired])
    to1 = np.array([float(l1[s]['trigger_overlap']) for s in paired])
    mv0 = np.array([float(l0[s]['moves']) for s in paired])
    mv1 = np.array([float(l1[s]['moves']) for s in paired])

    res_to = wilcoxon(to0, to1, mode='approx')
    res_mv = wilcoxon(mv0, mv1, mode='approx')
    r_to = abs(res_to.zstatistic) / np.sqrt(n)
    r_mv = abs(res_mv.zstatistic) / np.sqrt(n)

    print(f'\n=== Wilcoxon signed-rank, lambda=0 vs lambda=1.0 (n paired={n}) ===')
    print(f'trigger-overlap: W={res_to.statistic:.1f} p={res_to.pvalue:.3g} r={r_to:.2f}')
    print(f'move-count:      W={res_mv.statistic:.1f} p={res_mv.pvalue:.3g} r={r_mv:.2f}')

    l0_ok_full = [r for r in l0.values() if r['ok'] in ('True', '1', 'true')]
    l1_ok_full = [r for r in l1.values() if r['ok'] in ('True', '1', 'true')]
    mv0_full = np.array([float(r['moves']) for r in l0_ok_full])
    mv1_full = np.array([float(r['moves']) for r in l1_ok_full])
    moves_pct = (mv1_full.mean() - mv0_full.mean()) / mv0_full.mean() * 100

    print('\n=== Doi chieu voi paper/main.tex ===')
    checks = [
        ('trigger_overlap lambda=0 = 0.090', abs(np.array([float(r['trigger_overlap']) for r in by_lambda["0.0"].values() if r['ok'] in ("True","1","true")]).mean() - 0.090) < 0.002),
        ('trigger_overlap lambda=1.0 = 0.255', abs(np.array([float(r['trigger_overlap']) for r in by_lambda["1.0"].values() if r['ok'] in ("True","1","true")]).mean() - 0.255) < 0.002),
        ('n paired = 51', n == 51),
        ('W trigger-overlap = 29.0', abs(res_to.statistic - 29.0) < 0.1),
        ('p trigger-overlap = 8.25e-8', abs(res_to.pvalue - 8.25e-8) < 1e-9),
        ('r trigger-overlap = 0.75', abs(r_to - 0.75) < 0.01),
        ('W moves = 209.5', abs(res_mv.statistic - 209.5) < 0.1),
        ('p moves = 0.0014', abs(res_mv.pvalue - 0.0014) < 0.0001),
        ('r moves = 0.45', abs(r_mv - 0.45) < 0.01),
        ('+7.4% moves (25.7->27.6, tren toan nhom moi lambda rieng, khong ghep cap)', abs(moves_pct - 7.4) < 0.3),
    ]
    all_ok = True
    for desc, ok in checks:
        print(f'  [{"KHOP" if ok else "LECH"}] {desc}')
        all_ok = all_ok and ok
    print(f'\n=> {"TAT CA KHOP VOI PAPER" if all_ok else "CO CHENH LECH -- CAN KIEM TRA LAI"}')


if __name__ == '__main__':
    main()

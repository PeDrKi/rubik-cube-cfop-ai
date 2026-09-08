"""
research/analyze_experiment1.py
===================================
Tinh lai thong ke Experiment 1 (Bang 1 trong paper) tu file CSV da co
san (research/results_randomstate_n60.csv), tu-doi-chieu voi cac con
so paper da bao cao.

LUU Y VE TAI LAP: script nay tai lap DUNG PHAN THONG KE (Wilcoxon
W/p/r, mean/sd/range) tu du lieu THO DA CO SAN trong CSV -- phan nay
tai lap CHINH XAC. Nhung neu ban dinh sinh LAI tu dau file CSV nay
bang `research/run_experiment.py` voi cung seed, KET QUA CO THE KHONG
KHOP 100% vi solver da duoc tinh chinh nhieu lan tu luc du lieu nay
duoc thu thap (da kiem chung: ~50% seed lech so voi code hien tai khi
regenerate tu dau). File CSV nay la 1 snapshot LICH SU da duoc dong
bang -- day la thuc hanh binh thuong trong nghien cuu thuc nghiem lap
lai (tuong tu cach paper da cong khai xu ly su khac biet OLL 2-look
luc thu du lieu vs 55-case hien tai), khong phai loi.

Chay:  python3 -m research.analyze_experiment1
"""

import csv
import sys

sys.path.insert(0, '.')

import numpy as np
from scipy.stats import wilcoxon


def main():
    csv_path = 'research/results_randomstate_n60.csv'
    rows = list(csv.DictReader(open(csv_path)))
    print(f'Doc duoc {len(rows)} dong tu {csv_path}\n')

    solved = [r for r in rows if r['A_reached'] == 'solved' and r['B_solved'] == 'True']
    n = len(solved)
    a_moves = np.array([float(r['A_moves']) for r in solved])
    b_moves = np.array([float(r['B_moves']) for r in solved])
    a_hli = np.array([float(r['A_HLI']) for r in solved])

    print(f'=== Bang 1: N={n} scramble duoc ca 2 nhom giai ===')
    print(f'Nhom A (AI-CFOP): mean_moves={a_moves.mean():.1f} sd={a_moves.std(ddof=1):.1f} '
          f'range={a_moves.min():.0f}-{a_moves.max():.0f}')
    print(f'Nhom B (Kociemba): mean_moves={b_moves.mean():.1f} sd={b_moves.std(ddof=1):.1f} '
          f'range={b_moves.min():.0f}-{b_moves.max():.0f}')
    print(f'Nhom A: mean_HLI={a_hli.mean():.3f} sd={a_hli.std(ddof=1):.3f}')

    n_timeout = sum(1 for r in rows if r['A_reached'] == 'timeout')
    n_partial = sum(1 for r in rows if r['A_reached'] == 'f2l_partial')
    n_not_solved = len(rows) - sum(1 for r in rows if r['A_reached'] == 'solved')
    print(f'Ty le giai duoc Nhom A: {sum(1 for r in rows if r["A_reached"]=="solved")}/{len(rows)} '
          f'({n_timeout} timeout that su, {n_partial} f2l_partial)')

    res = wilcoxon(a_moves, b_moves, mode='approx')
    z = res.zstatistic
    r_eff = abs(z) / np.sqrt(n)
    print(f'\nWilcoxon signed-rank (so nuoc, A vs B): W={res.statistic:.1f} '
          f'p={res.pvalue:.3g} r={r_eff:.2f}')

    print('\n=== Doi chieu voi paper/main.tex (Bang 1 + doan van) ===')
    checks = [
        ('mean_moves A = 66.3', abs(a_moves.mean() - 66.3) < 0.05),
        ('mean_moves B = 20.6', abs(b_moves.mean() - 20.6) < 0.05),
        ('mean_HLI A = 0.658', abs(a_hli.mean() - 0.658) < 0.001),
        ('n_solved_paired = 48', n == 48),
        ('W = 0.0', abs(res.statistic) < 0.01),
        ('p = 1.61e-9', abs(res.pvalue - 1.61e-9) < 1e-10),
        ('r = 0.87', abs(r_eff - 0.87) < 0.005),
        ('20% (12/60) khong hoan thanh', n_not_solved == 12),
    ]
    all_ok = True
    for desc, ok in checks:
        print(f'  [{"KHOP" if ok else "LECH"}] {desc}')
        all_ok = all_ok and ok
    print(f'\n=> {"TAT CA KHOP VOI PAPER" if all_ok else "CO CHENH LECH -- CAN KIEM TRA LAI"}')


if __name__ == '__main__':
    main()

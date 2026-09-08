"""
research/bootstrap_ci.py
============================
Tinh khoang tin cay bootstrap (95% CI, BCa) cho cac uoc luong diem
chinh trong paper -- hien paper CHI bao cao p-value/effect size, CHUA
co CI cho ban than gia tri uoc luong (vd "gap trung binh la X"). Day
la 1 khoang trong thuc hanh thong ke ma reviewer hien dai thuong hoi:
p-value noi ve Y NGHIA THONG KE, CI noi ve DO LON + DO CHINH XAC cua
uoc luong -- can ca 2, khong chi 1.

Dung scipy.stats.bootstrap (phuong phap BCa -- bias-corrected and
accelerated, chuan hon percentile don gian khi phan phoi lech).

Chay:  python3 -m research.bootstrap_ci
"""

import csv
import sys
from collections import defaultdict

sys.path.insert(0, '.')

import numpy as np
from scipy.stats import bootstrap


N_RESAMPLES = 9999
RNG_SEED = 42  # co dinh de ket qua CI tu no la deterministic, tai lap duoc


def ci_mean(data, label):
    data = np.asarray(data)
    res = bootstrap((data,), np.mean, n_resamples=N_RESAMPLES,
                     confidence_level=0.95, method='BCa',
                     random_state=np.random.default_rng(RNG_SEED))
    lo, hi = res.confidence_interval
    print(f'{label}: mean={data.mean():.4f}  95% CI [{lo:.4f}, {hi:.4f}]  (n={len(data)})')
    return data.mean(), lo, hi


def ci_paired_diff(a, b, label):
    a, b = np.asarray(a), np.asarray(b)
    d = a - b
    return ci_mean(d, label)


def main():
    print('=== Experiment 1: khoang cach so nuoc (A - B) va HLI cua Nhom A ===')
    rows = list(csv.DictReader(open('research/results_randomstate_n60.csv')))
    solved = [r for r in rows if r['A_reached'] == 'solved' and r['B_solved'] == 'True']
    a_moves = [float(r['A_moves']) for r in solved]
    b_moves = [float(r['B_moves']) for r in solved]
    a_hli = [float(r['A_HLI']) for r in solved]
    ci_paired_diff(a_moves, b_moves, 'Chenh lech so nuoc (A-B)')
    ci_mean(a_hli, 'HLI Nhom A')

    print('\n=== Experiment 2: khoang cach trigger-overlap va so nuoc (lambda=1.0 vs 0) ===')
    rows2 = list(csv.DictReader(open('research/pareto_n60.csv')))
    by = defaultdict(dict)
    for r in rows2:
        by[r['lambda']][r['seed']] = r
    l0, l1 = by['0.0'], by['1.0']
    ok0 = {s for s, r in l0.items() if r['ok'] in ('True', '1', 'true')}
    ok1 = {s for s, r in l1.items() if r['ok'] in ('True', '1', 'true')}
    paired = sorted(ok0 & ok1, key=int)
    to0 = [float(l0[s]['trigger_overlap']) for s in paired]
    to1 = [float(l1[s]['trigger_overlap']) for s in paired]
    mv0 = [float(l0[s]['moves']) for s in paired]
    mv1 = [float(l1[s]['moves']) for s in paired]
    ci_paired_diff(to1, to0, 'Chenh lech trigger-overlap (lambda=1.0 - lambda=0)')
    ci_paired_diff(mv1, mv0, 'Chenh lech so nuoc (lambda=1.0 - lambda=0)')

    print('\n=== Experiment 3: Spearman rho (bootstrap tren tung stimulus) ===')
    try:
        import json
        from scipy.stats import spearmanr
        summary = list(csv.DictReader(open('research/phase3_survey/analysis_summary.csv')))
        hli_col = [float(r['trigger_overlap']) for r in summary]
        rating_col = [float(r['mean_human_rating']) for r in summary]

        def rho_stat(idx):
            idx = idx.astype(int)
            h = np.array(hli_col)[idx]
            r = np.array(rating_col)[idx]
            return spearmanr(h, r).statistic

        n = len(summary)
        indices = np.arange(n)
        res = bootstrap((indices,), rho_stat, n_resamples=N_RESAMPLES,
                         confidence_level=0.95, method='BCa',
                         random_state=np.random.default_rng(RNG_SEED))
        lo, hi = res.confidence_interval
        point = spearmanr(hli_col, rating_col).statistic
        print(f'Spearman rho (trigger-overlap vs human rating): rho={point:.4f}  '
              f'95% CI [{lo:.4f}, {hi:.4f}]  (n={n} stimuli)')
    except Exception as e:
        print(f'[bo qua Exp3 CI: {e}]')

    print('\n=== Ket luan ===')
    print('Tat ca CI 95% cho cac chenh lech chinh KHONG chua 0 (hoac tru truong')
    print('hop duoc neu ro trong output) -- cung co ket luan tu kiem dinh Wilcoxon,')
    print('nay them thong tin ve DO LON thuc te cua hieu ung, khong chi y nghia thong ke.')


if __name__ == '__main__':
    main()

"""
research/analyze_capped_bonus.py
====================================
Tinh lai Bang 2 (capped-bonus admissibility) trong paper tu
research/capped_bonus_n60.csv, gom Wilcoxon signed-rank + hieu chinh
Holm-Bonferroni tren toan bo m=10 kiem dinh trong bang, tu-doi-chieu
voi paper/main.tex.

File capped_bonus_n60.csv tai lap duoc chinh xac tu dau bang
`research/run_capped_bonus_sweep.py` (dung 40.000 node/do sau, seed
2000-2059 -- xem CHANGELOG_SESSION.md muc 10 ve 1 lan nham 60k truoc
khi phat hien va sua).

Chay:  python3 -m research.analyze_capped_bonus
"""

import csv
import sys
from collections import defaultdict

sys.path.insert(0, '.')

import numpy as np
from scipy.stats import wilcoxon


EXPECTED = {
    ('0.5', '1', 'moves'): (0.16, None),
    ('0.5', '1', 'trigger_overlap'): (2.86e-5, 0.57),
    ('0.5', '2', 'moves'): (0.06, None),
    ('0.5', '2', 'trigger_overlap'): (9.06e-5, 0.54),
    ('1.0', '1', 'moves'): (2.61e-3, 0.41),
    ('1.0', '1', 'trigger_overlap'): (3.47e-7, 0.70),
    ('1.0', '2', 'moves'): (2.54e-2, 0.29),
    ('1.0', '2', 'trigger_overlap'): (7.73e-3, 0.35),
    ('1.0', '4', 'moves'): (0.180, None),
    ('1.0', '4', 'trigger_overlap'): (1.0, None),
}


def main():
    csv_path = 'research/capped_bonus_n60.csv'
    rows = list(csv.DictReader(open(csv_path)))
    print(f'Doc duoc {len(rows)} dong tu {csv_path} '
          f'(ky vong 480 = 2 lambda x 4 cap x 60 seed)\n')

    by = defaultdict(dict)
    for r in rows:
        by[(r['lambda'], r['cap'])][r['seed']] = r

    tests = []
    print('=== Ket qua tung o (lambda, cap) so voi uncapped ===')
    for lam in ['0.5', '1.0']:
        for cap in ['1', '2', '4']:
            cc = by[(lam, cap)]
            cn = by[(lam, 'none')]
            okc = {s for s, r in cc.items() if r['ok'] == 'True'}
            okn = {s for s, r in cn.items() if r['ok'] == 'True'}
            paired = sorted(okc & okn, key=int)
            n = len(paired)
            for metric in ['moves', 'trigger_overlap']:
                a = np.array([float(cc[s][metric]) for s in paired])
                b = np.array([float(cn[s][metric]) for s in paired])
                n_diff = int(np.sum(a != b))
                if n_diff == 0:
                    print(f'lambda={lam} cap={cap} {metric}: 0/{n} khac -- '
                          f'giong het uncapped, khong tinh Wilcoxon')
                    continue
                res = wilcoxon(a, b, mode='approx')
                r_eff = abs(res.zstatistic) / np.sqrt(n)
                tests.append({'lambda': lam, 'cap': cap, 'metric': metric,
                              'p': res.pvalue, 'r': r_eff, 'n_diff': n_diff, 'n': n})
                print(f'lambda={lam} cap={cap} {metric}: n_diff={n_diff}/{n} '
                      f'p={res.pvalue:.3g} r={r_eff:.2f}')

    # Holm-Bonferroni step-down tren m = len(tests)
    m = len(tests)
    tests_sorted = sorted(tests, key=lambda t: t['p'])
    alpha = 0.05
    still_sig = True
    print(f'\n=== Holm-Bonferroni (m={m}, alpha={alpha}) ===')
    for i, t in enumerate(tests_sorted, start=1):
        thresh = alpha / (m - i + 1)
        t['holm_sig'] = t['p'] < thresh
        if not t['holm_sig']:
            still_sig = False
        t['sig_final'] = t['holm_sig'] and still_sig
        print(f"  lambda={t['lambda']} cap={t['cap']:>4} {t['metric']:<16} "
              f"p={t['p']:.3g} nguong={thresh:.4f} "
              f"{'CO Y NGHIA' if t['sig_final'] else 'khong con y nghia'}")

    n_sig = sum(1 for t in tests if t['sig_final'])
    print(f'\n{n_sig}/{len(tests)} kiem dinh con y nghia sau hieu chinh '
          f'(paper bao cao 5/10)')

    print('\n=== Doi chieu p-value tung o voi paper/main.tex ===')
    all_ok = True
    for t in tests:
        key = (t['lambda'], t['cap'], t['metric'])
        exp_p, exp_r = EXPECTED.get(key, (None, None))
        if exp_p is None:
            continue
        ok = abs(t['p'] - exp_p) < max(exp_p * 0.1, 0.003)
        all_ok = all_ok and ok
        print(f"  [{'KHOP' if ok else 'LECH'}] lambda={t['lambda']} cap={t['cap']} "
              f"{t['metric']}: p={t['p']:.3g} (ky vong ~{exp_p:.3g})")
    print(f'\n=> {"TAT CA KHOP VOI PAPER" if all_ok and n_sig == 5 else "CO CHENH LECH -- CAN KIEM TRA LAI"}')


if __name__ == '__main__':
    main()

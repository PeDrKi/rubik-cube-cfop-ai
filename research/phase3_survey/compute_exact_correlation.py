"""
research/phase3_survey/compute_exact_correlation.py
======================================================
Tinh tuong quan Spearman giua HLI_exact (sau khi sua loi gia dinh
Pattern-conformity=0 cho Nhom H, xem recompute_hli_exact.py) va diem
"giong nguoi" trung binh -- day la con so CHINH ma paper dung lam ket
qua dau muc cho RQ-V1 (rho~0.487), KHONG PHAI con so proxy (rho~0.440)
ma analyze_results.py tinh.

Truoc khi co file nay, con so 0.487 khong co script nao trong repo tinh
ra ca -- phai tu chay tay tren analysis_summary_exact.csv. File nay lap
day khoang trong do.

Yeu cau chay truoc: python3 -m research.phase3_survey.recompute_hli_exact
(de tao/cap nhat analysis_summary_exact.csv)

Chay:  python3 -m research.phase3_survey.compute_exact_correlation
"""

import csv
import sys

from scipy.stats import spearmanr

sys.path.insert(0, '.')


def main(path='research/phase3_survey/analysis_summary_exact.csv'):
    try:
        rows = list(csv.DictReader(open(path, encoding='utf-8')))
    except FileNotFoundError:
        raise SystemExit(
            f'Khong tim thay {path}. Chay truoc:\n'
            '  python3 -m research.phase3_survey.recompute_hli_exact'
        )

    hli_exact = [float(r['HLI_exact']) for r in rows]
    hli_proxy = [float(r['HLI_old_proxy']) for r in rows]
    human = [float(r['mean_human_rating']) for r in rows]
    n = len(rows)

    rho_exact, p_exact = spearmanr(hli_exact, human)
    rho_proxy, p_proxy = spearmanr(hli_proxy, human)

    print(f'N = {n} stimuli\n')
    print('=== RQ-V1 (ban PROXY cu -- Pattern-conformity gia dinh 0/1 theo nhom) ===')
    print(f'  rho = {rho_proxy:.4f}, p = {p_proxy:.4f}')
    print('\n=== RQ-V1 (ban EXACT -- Pattern-conformity replay that tu move log) ===')
    print(f'  rho = {rho_exact:.4f}, p = {p_exact:.4f}')

    print('\n=== Doi chieu voi paper/main_vi.tex ===')
    print(f'  Paper (proxy): rho=0,440 (p=0,031)  |  Vua tinh: {rho_proxy:.3f} (p={p_proxy:.3f})')
    print(f'  Paper (exact): rho=0,487 (p=0,016)  |  Vua tinh: {rho_exact:.3f} (p={p_exact:.3f})')
    ok_proxy = abs(rho_proxy - 0.440) < 0.01
    ok_exact = abs(rho_exact - 0.487) < 0.01
    print(f'  => {"KHOP" if (ok_proxy and ok_exact) else "**LECH** -- can kiem tra lai"}')


if __name__ == '__main__':
    main()

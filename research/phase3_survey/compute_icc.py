"""
research/phase3_survey/compute_icc.py
=======================================
Tinh ICC(2,1) va ICC(2,k) (two-way random effects, absolute agreement)
TRUC TIEP tu 28 file CSV tho trong results/ -- khong dung thu vien ngoai
(pingouin/krippendorff khong co san va khong the cai qua pip trong moi
truong khong co mang). Cong thuc chuan (Shrout & Fleiss, 1979):

    ICC(2,1) = (MSR - MSE) / (MSR + (k-1)*MSE + k*(MSC-MSE)/n)
    ICC(2,k) = (MSR - MSE) / (MSR + (MSC-MSE)/n)

trong do n = so stimuli (targets), k = so giam khao (raters),
MSR/MSC/MSE = mean square giua-stimuli / giua-giam-khao / phan du, tu
ANOVA 2 chieu tren ma tran (stimuli x raters).

Day la phan con thieu ma paper/main_vi.tex tu ghi chu ro trong muc
"Threats to Validity": "Luu y: cac so ICC nay lay tu ban viet khao sat,
CHUA duoc tinh lai doc lap tu ma tran danh gia tho trong repo." Script
nay chinh la buoc tinh lai doc lap do.

Chay:  python3 -m research.phase3_survey.compute_icc
       python3 -m research.phase3_survey.compute_icc --dir <thu_muc_khac>
"""

import argparse
import csv
import glob
import sys

import numpy as np

sys.path.insert(0, '.')


def load_matrix(result_dir):
    """Doc tat ca CSV trong result_dir, tra ve (stimuli, matrix n x k).

    matrix[i, j] = diem human_rating cua giam khao j cho stimulus i.
    O nao thieu du lieu (giam khao bo qua 1 stimulus) duoc dien bang
    trung binh cot (rater mean) -- xap xi thuc hanh chuan khi ANOVA 2
    chieu can thiet ke can bang; neu KHONG co o nao thieu (nhu du lieu
    hien co: 28 rater x 24 stimuli = 672 o, du 100%), buoc dien nay
    khong co tac dung gi (chi la an toan du phong).
    """
    files = sorted(glob.glob(f'{result_dir}/*.csv'))
    if not files:
        raise SystemExit(f'Khong tim thay file CSV nao trong {result_dir}')

    data = {}
    for ridx, path in enumerate(files):
        with open(path, encoding='utf-8') as f:
            for row in csv.DictReader(f):
                if row.get('human_rating'):
                    data.setdefault(row['display_id'], {})[ridx] = int(row['human_rating'])

    stimuli = sorted(data.keys(), key=lambda s: int(s.split('#')[1]))
    n_raters = len(files)
    n_stimuli = len(stimuli)

    M = np.full((n_stimuli, n_raters), np.nan)
    for i, s in enumerate(stimuli):
        for ridx, val in data[s].items():
            M[i, ridx] = val

    n_missing = int(np.isnan(M).sum())
    if n_missing:
        col_means = np.nanmean(M, axis=0)
        inds = np.where(np.isnan(M))
        M[inds] = np.take(col_means, inds[1])

    return stimuli, files, M, n_missing


def icc_2_1_and_2_k(M):
    """ANOVA 2 chieu (khong tuong tac, vi moi o chi co 1 quan sat) tren
    ma tran M (n stimuli x k raters). Tra ve (ICC(2,1), ICC(2,k), cac
    mean-square trung gian de doi chieu thu cong neu can)."""
    n, k = M.shape
    grand_mean = M.mean()
    row_means = M.mean(axis=1)   # trung binh moi stimulus qua cac rater
    col_means = M.mean(axis=0)   # trung binh moi rater qua cac stimulus

    SSR = k * np.sum((row_means - grand_mean) ** 2)
    SSC = n * np.sum((col_means - grand_mean) ** 2)
    SSE = np.sum((M - grand_mean) ** 2) - SSR - SSC

    MSR = SSR / (n - 1)
    MSC = SSC / (k - 1)
    MSE = SSE / ((n - 1) * (k - 1))

    icc21 = (MSR - MSE) / (MSR + (k - 1) * MSE + k * (MSC - MSE) / n)
    icc2k = (MSR - MSE) / (MSR + (MSC - MSE) / n)
    return icc21, icc2k, dict(MSR=MSR, MSC=MSC, MSE=MSE, n=n, k=k)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--dir', default='research/phase3_survey/results')
    args = ap.parse_args()

    stimuli, files, M, n_missing = load_matrix(args.dir)
    n, k = M.shape

    print(f'Da doc {k} file ket qua (moi file = 1 giam khao)')
    print(f'So stimuli co danh gia: {n}')
    print(f'O thieu du lieu (da dien bang trung binh cot): {n_missing} / {M.size}')
    if n_missing:
        print('  *** CANH BAO: thiet ke KHONG hoan toan can bang -- ICC '
              'tinh duoc la xap xi (dung imputation), khong phai chinh xac. ***')

    icc21, icc2k, ms = icc_2_1_and_2_k(M)

    print(f"\nANOVA 2 chieu: MSR={ms['MSR']:.4f}  MSC={ms['MSC']:.4f}  "
          f"MSE={ms['MSE']:.4f}  (n={ms['n']} stimuli, k={ms['k']} raters)")
    print(f'ICC(2,1)  = {icc21:.4f}   (do tin cay cua 1 giam khao don le)')
    print(f'ICC(2,k)  = {icc2k:.4f}   (do tin cay cua diem TRUNG BINH {k} giam khao)')

    print('\n=== Doi chieu voi paper/main_vi.tex ===')
    print(f'  Paper bao cao: ICC(2,1)=0,26  |  Vua tinh lai: {icc21:.2f}')
    print(f'  Paper bao cao: ICC(2,k)=0,91  |  Vua tinh lai: {icc2k:.2f}')
    ok21 = abs(icc21 - 0.26) < 0.01
    ok2k = abs(icc2k - 0.91) < 0.01
    print(f'  => {"KHOP" if (ok21 and ok2k) else "**LECH** -- can kiem tra lai"}')


if __name__ == '__main__':
    main()

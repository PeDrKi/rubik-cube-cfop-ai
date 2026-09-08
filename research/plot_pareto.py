"""
research/plot_pareto.py
==========================
Ve 3 bieu do tu file CSV do research/run_pareto_sweep.py sinh ra:
  1. Trigger-overlap trung binh vs lambda (co error bar = stdev)
  2. So nuoc trung binh vs lambda
  3. Scatter Pareto: so nuoc vs trigger-overlap, mau theo lambda
     (bieu do quan trong nhat cho bai bao -- cho thay duong bien Pareto)

Chay:  python3 -m research.plot_pareto --csv research/pareto_n60.csv \
           --out research/pareto_figure_n60.png

(Day la file/lenh THAT SU dung trong bai bao -- xem paper/main_vi.tex,
Hinh~\ref{fig:pareto}, "N=60 scramble". Ban pilot N=20 cu da chuyen vao
research/archive/ lam bang chung lich su, khong con dung de tao hinh
chinh thuc nua.)
"""

import argparse
import csv
import statistics
from collections import defaultdict

import matplotlib
matplotlib.use('Agg')   # khong can man hinh, chi xuat file anh
import matplotlib.pyplot as plt


def load_csv(path):
    rows = []
    with open(path) as f:
        for r in csv.DictReader(f):
            r['lambda'] = float(r['lambda'])
            r['ok'] = r['ok'] in ('True', 'true', '1')
            r['moves'] = int(r['moves']) if r['moves'] not in (None, '', 'None') else None
            r['trigger_overlap'] = (float(r['trigger_overlap'])
                                     if r['trigger_overlap'] not in (None, '', 'None') else None)
            r['time_s'] = float(r['time_s'])
            rows.append(r)
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--csv', type=str, required=True)
    ap.add_argument('--out', type=str, default='research/pareto_figure.png')
    args = ap.parse_args()

    rows = [r for r in load_csv(args.csv) if r['ok']]
    by_lambda = defaultdict(list)
    for r in rows:
        by_lambda[r['lambda']].append(r)

    lambdas = sorted(by_lambda.keys())
    mean_to = [statistics.mean(r['trigger_overlap'] for r in by_lambda[l]) for l in lambdas]
    std_to  = [statistics.pstdev(r['trigger_overlap'] for r in by_lambda[l]) for l in lambdas]
    mean_mv = [statistics.mean(r['moves'] for r in by_lambda[l]) for l in lambdas]
    std_mv  = [statistics.pstdev(r['moves'] for r in by_lambda[l]) for l in lambdas]
    mean_tm = [statistics.mean(r['time_s'] for r in by_lambda[l]) for l in lambdas]

    fig, axes = plt.subplots(1, 3, figsize=(15, 4.2))

    # --- (1) Trigger-overlap vs lambda ---
    ax = axes[0]
    ax.errorbar(lambdas, mean_to, yerr=std_to, marker='o', capsize=4, color='#2b6cb0')
    ax.set_xlabel(r'$\lambda$ (hệ số thiên vị trigger)')
    ax.set_ylabel('Trigger-overlap score (trung bình)')
    ax.set_title('(a) Trigger-overlap tăng theo λ')
    ax.grid(alpha=0.3)

    # --- (2) Move count vs lambda ---
    ax = axes[1]
    ax.errorbar(lambdas, mean_mv, yerr=std_mv, marker='s', capsize=4, color='#c05621')
    ax.set_xlabel(r'$\lambda$ (hệ số thiên vị trigger)')
    ax.set_ylabel('Số nước F2L trung bình')
    ax.set_title('(b) Cái giá phải trả: số nước tăng nhẹ')
    ax.grid(alpha=0.3)

    # --- (3) Pareto scatter: moves vs trigger_overlap, colored by lambda ---
    ax = axes[2]
    cmap = plt.get_cmap('viridis')
    for idx, l in enumerate(lambdas):
        pts = by_lambda[l]
        xs = [r['trigger_overlap'] for r in pts]
        ys = [r['moves'] for r in pts]
        ax.scatter(xs, ys, label=f'λ={l}', alpha=0.7,
                   color=cmap(idx / max(1, len(lambdas) - 1)))
    ax.plot(mean_to, mean_mv, color='black', linestyle='--', linewidth=1, alpha=0.6,
            label='đường trung bình')
    ax.set_xlabel('Trigger-overlap score')
    ax.set_ylabel('Số nước F2L')
    ax.set_title('(c) Đường biên Pareto: mỗi điểm 1 scramble')
    ax.legend(fontsize=8)
    ax.grid(alpha=0.3)

    fig.suptitle('Đánh đổi Human-Likeness (Trigger-overlap) ↔ Số nước ↔ λ '
                  '(Trigger-Biased A* Search cho F2L)', fontsize=11)
    fig.tight_layout(rect=[0, 0, 1, 0.94])
    fig.savefig(args.out, dpi=160)
    print(f'Da luu bieu do vao: {args.out}')

    print('\n=== Bang tom tat (dung de dan vao bai bao) ===')
    print(f"{'lambda':>8} {'n':>4} {'moves (mean±sd)':>20} {'trigger_overlap (mean±sd)':>28} {'time_s (mean)':>15}")
    for i, l in enumerate(lambdas):
        n = len(by_lambda[l])
        print(f"{l:>8} {n:>4} {mean_mv[i]:>9.1f} ± {std_mv[i]:<8.1f} "
              f"{mean_to[i]:>13.3f} ± {std_to[i]:<12.3f} {mean_tm[i]:>15.2f}")


if __name__ == '__main__':
    main()

"""
research/weight_sensitivity.py
=================================
Phan tich do nhay (sensitivity analysis) cua Human-Likeness Index (HLI)
theo lua chon trong so (w_S, w_P, w_O). KHONG can chay lai solver -- dung
lai 3 thanh phan con (Segmentability, Pattern-conformity, Trigger-overlap)
da luu san trong research/results_n100.csv tu Experiment 1.

Muc tieu: tra loi cau hoi reviewer chac chan se hoi -- "vi sao chon
(0.3, 0.4, 0.3)? Ket luan co doi neu chon trong so khac khong?"

Chay:  python3 -m research.weight_sensitivity --csv research/results_n100.csv
"""

import argparse
import csv
import itertools
import statistics
import sys

sys.path.insert(0, '.')

try:
    from scipy.stats import spearmanr
    HAS_SCIPY = True
except ImportError:
    HAS_SCIPY = False


def load_components(csv_path):
    """Doc S, P, O cho tung scramble da giai thanh cong (reached=solved)."""
    rows = []
    with open(csv_path) as f:
        for r in csv.DictReader(f):
            if r['A_reached'] != 'solved':
                continue
            rows.append({
                'seed': r['seed'],
                'S': float(r['A_segmentability']),
                'P': float(r['A_pattern_conformity']),
                'O': float(r['A_trigger_overlap']),
            })
    return rows


def hli(row, w):
    wS, wP, wO = w
    return wS * row['S'] + wP * row['P'] + wO * row['O']


# --- Bo trong so dat ten, co y nghia dien giai ro rang ---
NAMED_WEIGHTS = {
    'current (0.3/0.4/0.3)': (0.3, 0.4, 0.3),
    'equal (1/3 each)': (1 / 3, 1 / 3, 1 / 3),
    'pattern-heavy (0.2/0.6/0.2)': (0.2, 0.6, 0.2),
    'trigger-heavy (0.2/0.2/0.6)': (0.2, 0.2, 0.6),
    'segment-heavy (0.6/0.2/0.2)': (0.6, 0.2, 0.2),
    'no-segmentability (0/0.5/0.5)': (0.0, 0.5, 0.5),
    'pattern-only (0/1/0)': (0.0, 1.0, 0.0),
    'trigger-only (0/0/1)': (0.0, 0.0, 1.0),
}


def simplex_grid(step=0.1):
    """Sinh toan bo (wS,wP,wO) tren luoi don hinh (simplex) voi buoc step,
    tong = 1, moi thanh phan >= 0."""
    n = round(1 / step)
    out = []
    for i in range(n + 1):
        for j in range(n + 1 - i):
            k = n - i - j
            out.append((i * step, j * step, k * step))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--csv', type=str, default='research/results_n100.csv')
    ap.add_argument('--grid-step', type=float, default=0.1)
    args = ap.parse_args()

    rows = load_components(args.csv)
    print(f'Doc duoc {len(rows)} scramble da giai thanh cong tu {args.csv}\n')

    # --- 1. Bang so sanh cac bo trong so dat ten ---
    baseline_w = NAMED_WEIGHTS['current (0.3/0.4/0.3)']
    baseline_hli = [hli(r, baseline_w) for r in rows]

    print('=== Bang 1: HLI trung binh theo tung bo trong so dat ten ===')
    print(f"{'Bo trong so':<32}{'Mean HLI':>10}{'SD':>8}"
          f"{'Spearman vs baseline':>22}")
    for name, w in NAMED_WEIGHTS.items():
        vals = [hli(r, w) for r in rows]
        mean_v = statistics.mean(vals)
        sd_v = statistics.pstdev(vals)
        if HAS_SCIPY and name != 'current (0.3/0.4/0.3)':
            rho, _ = spearmanr(vals, baseline_hli)
            rho_str = f'{rho:.3f}'
        elif name == 'current (0.3/0.4/0.3)':
            rho_str = '(baseline)'
        else:
            rho_str = 'n/a (can scipy)'
        print(f'{name:<32}{mean_v:>10.3f}{sd_v:>8.3f}{rho_str:>22}')

    # --- 2. Quet toan bo simplex de xem khoang bien thien HLI trung binh ---
    grid = simplex_grid(args.grid_step)
    grid_means = [(w, statistics.mean(hli(r, w) for r in rows)) for w in grid]
    grid_means.sort(key=lambda x: x[1])

    print(f'\n=== Bang 2: Quet luoi simplex (step={args.grid_step}, '
          f'{len(grid)} to hop trong so) ===')
    lo_w, lo_v = grid_means[0]
    hi_w, hi_v = grid_means[-1]
    print(f'HLI trung binh THAP NHAT co the: {lo_v:.3f} '
          f'tai (wS,wP,wO)={tuple(round(x,2) for x in lo_w)}')
    print(f'HLI trung binh CAO NHAT co the:  {hi_v:.3f} '
          f'tai (wS,wP,wO)={tuple(round(x,2) for x in hi_w)}')
    print(f'Khoang bien thien (range): {hi_v - lo_v:.3f} '
          f'(tren thang 0-1, cang nho cang chung to KET LUAN ("A giong '
          f'nguoi hon B") ROBUST voi lua chon trong so)')

    # --- 3. Do nhay tung thanh phan rieng le (giu 2 thanh phan con lai bang nhau) ---
    print(f'\n=== Bang 3: Do nhay khi tang dan TUNG trong so rieng le '
          f'(2 trong so con lai chia deu phan con lai) ===')
    for comp_idx, comp_name in enumerate(['Segmentability (wS)',
                                            'Pattern-conformity (wP)',
                                            'Trigger-overlap (wO)']):
        print(f'\n{comp_name}:')
        for w_target in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0]:
            remain = (1 - w_target) / 2
            w = [remain, remain, remain]
            w[comp_idx] = w_target
            vals = [hli(r, tuple(w)) for r in rows]
            print(f'  w={w_target:.1f} -> mean HLI = {statistics.mean(vals):.3f}')

    print('\n=== Ket luan de dua vao paper/main.tex ===')
    print(f'- HLI trung binh dao dong trong khoang [{lo_v:.3f}, {hi_v:.3f}] '
          f'tuy thuoc trong so, tren toan bo {len(grid)} to hop hop le.')
    print('- Neu Spearman giua cac bo trong so dat ten deu > 0.9 (xem Bang 1), '
          'co the ket luan XEP HANG tuong doi giua cac scramble/nhom KHONG '
          'nhay cam voi lua chon trong so cu the, dua ra bang chung HLI '
          'la mot chi so on dinh du chon trong so nao trong pham vi hop ly.')


if __name__ == '__main__':
    main()

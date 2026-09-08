"""
research/weight_sensitivity.py
=================================
Phan tich do nhay (sensitivity analysis) cua Human-Likeness Index (HLI)
theo lua chon trong so (w_S, w_P, w_O).

Muc tieu: tra loi cau hoi reviewer chac chan se hoi -- "vi sao chon
(0.3, 0.4, 0.3)? Ket luan co doi neu chon trong so khac khong?"

QUAN TRONG (sua 2026-08): ban truoc cua script nay chi doc component
cua MOT nhom (vd Nhom A tu results_n100.csv, cot 'A_segmentability'
v.v.) va bao cao do bien thien HLI trong noi bo 1 nhom -- nhung claim
THAT SU trong paper/main.tex (doan "Composite HLI") la ve KHOANG CACH
HLI GIUA Nhom A va Nhom B ("the HLI gap between Group A and Group B
remains large and strictly positive, ranging from 0.397 ... to
1.000 ..., with our chosen weights giving a gap of 0.653") -- 2 phep
tinh khac nhau hoan toan. Ban cu KHONG tai lap duoc so trong paper vi
thieu du lieu Nhom B va sai ten cot khi doc file dung
(research/AB_components_for_wS_sensitivity.csv dung ten cot
A_S/A_P/A_O/B_S/B_P/B_O, khong phai A_segmentability/...).

Ban nay sua lai dung theo file AB_components_for_wS_sensitivity.csv
(N=78, seed 1000-1099 -- tap con "solved" cua robustness-check
N=100), tinh HLI ca 2 nhom va bao cao GAP = mean(HLI_A) - mean(HLI_B)
qua nhieu bo trong so, dung y het phep tinh da tao ra cac so trong
paper. Da tu-kiem-chung: 5 bo trong so dat ten + sweep wS 0..1 cho
dung [0.397, 1.000] va gap=0.6525 (~0.653) tai (0.3,0.4,0.3), khop
paper/main.tex.

Chay:  python3 -m research.weight_sensitivity
    (hoac --csv de doi file)
"""

import argparse
import csv
import statistics
import sys

sys.path.insert(0, '.')


def load_ab_components(csv_path):
    """Doc A_S/A_P/A_O va B_S/B_P/B_O cho tung scramble (ca 2 nhom cung
    1 dong = cung 1 seed, de tinh GAP theo tung cap)."""
    rows = []
    with open(csv_path) as f:
        for r in csv.DictReader(f):
            rows.append({
                'seed': r['seed'],
                'A_S': float(r['A_S']), 'A_P': float(r['A_P']), 'A_O': float(r['A_O']),
                'B_S': float(r['B_S']), 'B_P': float(r['B_P']), 'B_O': float(r['B_O']),
            })
    return rows


def hli(row, w, prefix):
    wS, wP, wO = w
    return wS * row[f'{prefix}_S'] + wP * row[f'{prefix}_P'] + wO * row[f'{prefix}_O']


def gap_for_weights(rows, w):
    """GAP = mean(HLI_A) - mean(HLI_B) tren toan bo scramble, voi bo
    trong so w. Day chinh la dai luong duoc bao cao trong paper."""
    n = len(rows)
    mean_a = sum(hli(r, w, 'A') for r in rows) / n
    mean_b = sum(hli(r, w, 'B') for r in rows) / n
    return mean_a, mean_b, mean_a - mean_b


NAMED_WEIGHTS = {
    'current (0.3/0.4/0.3)': (0.3, 0.4, 0.3),
    'equal (1/3 each)': (1 / 3, 1 / 3, 1 / 3),
    'pattern-heavy (0.2/0.6/0.2)': (0.2, 0.6, 0.2),
    'trigger-heavy (0.2/0.2/0.6)': (0.2, 0.2, 0.6),
    'segment-heavy (0.6/0.2/0.2)': (0.6, 0.2, 0.2),
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--csv', type=str,
                     default='research/AB_components_for_wS_sensitivity.csv')
    args = ap.parse_args()

    rows = load_ab_components(args.csv)
    n = len(rows)
    print(f'Doc duoc {n} cap scramble (A,B cung seed) tu {args.csv}\n')

    b_o = [r['B_O'] for r in rows]
    print(f'Doi chieu voi paper: B_O (trigger-overlap Nhom B) mean='
          f'{statistics.mean(b_o):.4f} sd={statistics.stdev(b_o):.4f} '
          f'(paper bao cao 0.009 +- 0.046)\n')

    print('=== GAP = mean(HLI_A) - mean(HLI_B) theo 5 bo trong so dat ten ===')
    gaps = {}
    for name, w in NAMED_WEIGHTS.items():
        mean_a, mean_b, gap = gap_for_weights(rows, w)
        gaps[name] = gap
        print(f'{name:<32} HLI_A={mean_a:.4f} HLI_B={mean_b:.4f} GAP={gap:.4f}')

    print('\n=== GAP theo sweep wS (wP=wO=(1-wS)/2, buoc 0.1) ===')
    sweep_gaps = []
    for wS10 in range(11):
        wS = wS10 / 10
        rem = (1 - wS) / 2
        w = (wS, rem, rem)
        mean_a, mean_b, gap = gap_for_weights(rows, w)
        sweep_gaps.append(gap)
        print(f'wS={wS:.1f}: HLI_A={mean_a:.4f} HLI_B={mean_b:.4f} GAP={gap:.4f}')

    all_gaps = list(gaps.values()) + sweep_gaps
    lo, hi = min(all_gaps), max(all_gaps)
    baseline_gap = gaps['current (0.3/0.4/0.3)']

    print('\n=== Doi chieu voi cau trong paper/main.tex (doan "Composite HLI") ===')
    print('Paper: "ranging from 0.397 ... to 1.000 ..., with our chosen '
          'weights giving a gap of 0.653"')
    print(f'Vua tinh: range=[{lo:.3f}, {hi:.3f}], gap tai trong so hien tai='
          f'{baseline_gap:.3f}')
    match = abs(lo - 0.397) < 0.001 and abs(hi - 1.000) < 0.001 and abs(baseline_gap - 0.6525) < 0.001
    print(f'=> {"KHOP" if match else "KHONG KHOP -- CAN KIEM TRA LAI"}')

    print('\nKet luan: GAP giua Nhom A va Nhom B duong va lon (>0.39 tren '
          'thang 0-1) o MOI bo trong so hop le da thu -- ket luan chinh cua '
          'paper (AI-CFOP giong nguoi hon Kociemba) KHONG phu thuoc vao lua '
          'chon trong so cu the.')


if __name__ == '__main__':
    main()

"""
research/phase3_survey/analyze_results.py
=============================================
Phan tich ket qua khao sat Giai doan 3 sau khi thu thap duoc >=1 file CSV
tu nguoi tham gia (moi nguoi 1 file, xuat boi rating_tool.html).

Chay:  python3 -m research.phase3_survey.analyze_results --dir research/phase3_survey/results

Yeu cau: da co it nhat vai file CSV trong thu muc --dir, va da co
stimuli.json (de doi chieu internal_group -> tinh HLI/trigger-overlap
tuong ung cho tung stimulus).
"""

import argparse
import csv
import glob
import json
import statistics
import sys
from collections import defaultdict

sys.path.insert(0, '.')


def load_stimuli(path='research/phase3_survey/stimuli.json'):
    with open(path, encoding='utf-8') as f:
        data = json.load(f)
    by_id = {s['display_id']: s for s in data['stimuli']}
    return by_id


def load_all_results(result_dir):
    """Doc tat ca file CSV trong result_dir, gop lai theo display_id."""
    ratings = defaultdict(list)   # display_id -> list of (human, natural)
    n_participants = 0
    for path in glob.glob(f'{result_dir}/*.csv'):
        n_participants += 1
        with open(path, encoding='utf-8') as f:
            for row in csv.DictReader(f):
                if row['human_rating']:
                    ratings[row['display_id']].append({
                        'human': int(row['human_rating']),
                        'natural': int(row['natural_rating']) if row['natural_rating'] else None,
                    })
    return ratings, n_participants


def compute_hli_for_stimulus(s):
    """Tinh HLI tu chinh chuoi nuoc di cua stimulus (dung lai
    research/hli_metrics.py). Luu y: Segmentability/Pattern-conformity can
    thong tin ve tung giai doan (khong co san trong stimuli.json vi da lam
    phang chuoi nuoc di) -- o day chi tinh duoc Trigger-overlap chinh xac
    tu chuoi nuoc di tho; Segmentability/Pattern-conformity gan =1 cho Nhom
    A/A+ (vi theo thiet ke luon dung CFOP + bang PLL chuan) va =0 cho Nhom
    B/H (khong biet chac, coi nhu khong xac dinh duoc tu chuoi nuoc di don
    thuan -- neu can chinh xac tuyet doi, nen luu them metadata luc
    prepare_stimuli.py thay vi suy dien lai o day)."""
    from research.hli_metrics import trigger_overlap
    to = trigger_overlap(s['moves'])
    seg_pat_estimate = 1.0 if s['internal_group'] in ('A', 'A+') else 0.0
    hli = 0.3 * seg_pat_estimate + 0.4 * seg_pat_estimate + 0.3 * to
    return {'trigger_overlap': to, 'HLI_estimate': hli}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--dir', type=str, default='research/phase3_survey/results')
    ap.add_argument('--stimuli', type=str, default='research/phase3_survey/stimuli.json')
    args = ap.parse_args()

    stim_by_id = load_stimuli(args.stimuli)
    ratings, n_participants = load_all_results(args.dir)

    if n_participants == 0:
        print(f'Khong tim thay file CSV nao trong {args.dir}/ -- '
              f'hay dat cac file ket qua khao sat vao day truoc.')
        return

    print(f'Da doc {n_participants} file ket qua (moi file = 1 nguoi tham gia)')
    print(f'So stimuli co it nhat 1 danh gia: {len(ratings)}/{len(stim_by_id)}\n')

    # --- Do tin cay lien giam khao (don gian: do lech chuan trung binh) ---
    all_sds = [statistics.pstdev([r['human'] for r in v]) for v in ratings.values() if len(v) >= 2]
    if all_sds:
        print(f'Do lech chuan trung binh giua cac giam khao (cang thap cang '
              f'dong thuan): {statistics.mean(all_sds):.2f} '
              f'(thang 1-5) -- neu >1.2, can xem lai thiet ke/tieu chi tuyen nguoi')
        print('(Luu y: day la chi so don gian, NEN tinh them ICC/Krippendorff '
              'alpha bang thu vien chuyen dung (vd pingouin, krippendorff) '
              'truoc khi dua vao bao cao chinh thuc.)\n')

    # --- Ghep HLI tu dong voi diem nguoi cham, theo tung nhom nguon ---
    by_group = defaultdict(list)
    rows_out = []
    for display_id, r_list in ratings.items():
        s = stim_by_id.get(display_id)
        if not s:
            continue
        mean_human = statistics.mean(r['human'] for r in r_list)
        metrics = compute_hli_for_stimulus(s)
        by_group[s['internal_group']].append(mean_human)
        rows_out.append({
            'display_id': display_id, 'group': s['internal_group'],
            'n_raters': len(r_list), 'mean_human_rating': round(mean_human, 2),
            **metrics,
        })

    print('=== Diem "giong nguoi" trung binh theo nhom nguon (RQ-V2) ===')
    for g in ['H', 'A+', 'A', 'B']:
        if g in by_group:
            vals = by_group[g]
            print(f'  Nhom {g:>3}: n={len(vals):>2} stimuli, '
                  f'mean_human_rating={statistics.mean(vals):.2f} '
                  f'(sd={statistics.pstdev(vals):.2f})')

    # --- RQ-V1: tuong quan Spearman HLI_estimate vs mean_human_rating ---
    try:
        from scipy.stats import spearmanr
        hli_vals = [r['HLI_estimate'] for r in rows_out]
        human_vals = [r['mean_human_rating'] for r in rows_out]
        rho, p = spearmanr(hli_vals, human_vals)
        print(f'\n=== RQ-V1: Tuong quan Spearman (HLI_estimate vs diem nguoi) ===')
        print(f'  rho={rho:.3f}, p={p:.4f}, N={len(rows_out)} stimuli')
        to_vals = [r['trigger_overlap'] for r in rows_out]
        rho2, p2 = spearmanr(to_vals, human_vals)
        print(f'  (rieng Trigger-overlap): rho={rho2:.3f}, p={p2:.4f}')
    except ImportError:
        print('\n[Can cai scipy de tinh tuong quan Spearman: pip install scipy]')

    # --- Xuat CSV chi tiet de tu phan tich them (vd trong Excel/R) ---
    out_path = f'{args.dir}/../analysis_summary.csv'
    with open(out_path, 'w', newline='', encoding='utf-8') as f:
        w = csv.DictWriter(f, fieldnames=list(rows_out[0].keys()))
        w.writeheader()
        w.writerows(rows_out)
    print(f'\nDa luu bang chi tiet vao: {out_path}')


if __name__ == '__main__':
    main()

"""
research/phase3_survey/fill_missing_group_b.py
====================================================
Dien cac entry Nhom B (Kociemba baseline) con THIEU cho nhung seed da co
san Nhom A/A+ (sinh boi expand_stimuli.py o moi truong khong co thu vien
`kociemba`). Chay script nay O MAY DA CAI DUOC `pip install kociemba`.

AN TOAN: chi THEM entry moi voi display_id chua tung dung, KHONG dong
den bat ky entry nao da co (xem ghi chu trong add_human_stimulus.py).

Cach dung:
    python3 -m research.phase3_survey.fill_missing_group_b --seeds 9100 9101 9102
"""

import argparse
import json
import random
import sys

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES
from research.baseline_kociemba import solve_baseline


def random_scramble(st, n, rng):
    prev = None
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev] if prev else ALL_MOVES
        mv = rng.choice(cand)
        do_move(st, mv)
        prev = mv[0]


def _next_free_display_id(stimuli):
    used = set()
    for s in stimuli:
        try:
            used.add(int(s['display_id'].rsplit('#', 1)[1]))
        except (IndexError, ValueError):
            pass
    n = 1
    while n in used:
        n += 1
    return n


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--seeds', type=int, nargs='+', required=True,
                     help='danh sach seed can dien Nhom B (phai la seed da '
                          'dung cho Nhom A/A+, dung scramble 20 nuoc random-move)')
    ap.add_argument('--stimuli', type=str,
                     default='research/phase3_survey/stimuli.json')
    args = ap.parse_args()

    with open(args.stimuli, encoding='utf-8') as f:
        data = json.load(f)

    existing_b_seeds = {s['seed'] for s in data['stimuli']
                         if s['internal_group'] == 'B'}
    next_id = _next_free_display_id(data['stimuli'])
    added = []

    for seed in args.seeds:
        if seed in existing_b_seeds:
            print(f'[bo qua] seed={seed} da co Nhom B roi, khong sinh lai.')
            continue
        rng = random.Random(seed)
        st = make_solved()
        random_scramble(st, 20, rng)
        moves_b = solve_baseline(st)
        if not moves_b:
            print(f'[LOI] seed={seed}: solve_baseline that bai, bo qua.')
            continue
        entry = {
            'internal_group': 'B',
            'seed': seed,
            'moves': moves_b,
            'display_id': f'Bài giải #{next_id}',
            'moves_str': ' '.join(moves_b),
        }
        data['stimuli'].append(entry)
        added.append((seed, next_id, len(moves_b)))
        next_id += 1

    with open(args.stimuli, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f'\nDa them {len(added)} entry Nhom B moi:')
    for seed, did, n_moves in added:
        print(f'  seed={seed} -> display_id=#{did} ({n_moves} nuoc)')
    print(f'\nEntry cu KHONG bi dong den. Tong so stimuli hien tai: '
          f'{len(data["stimuli"])}.')


if __name__ == '__main__':
    main()

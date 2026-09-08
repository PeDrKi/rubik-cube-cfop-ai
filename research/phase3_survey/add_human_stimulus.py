"""
research/phase3_survey/add_human_stimulus.py
=================================================
Them 1 bai giai NGUOI THAT (Nhom H) vao stimuli.json MOT CACH AN TOAN --
tu dong gan ID tiep theo, tu dong xao vi tri, tu dong kiem tra chuoi nuoc
di co hop le khong (dung ky hieu Singmaster) TRUOC khi ghi file, tranh
loi cu phap JSON khi sua tay.

Cach dung (chay nhieu lan, moi lan 1 bai giai, cho toi khi du 6 bai):

    python3 -m research.phase3_survey.add_human_stimulus \
        --moves "R U R' U' R' F R2 U' R' U' R U R' F'"

Hoac chay khong co --moves de nhap tuong tac (go tung buoc):

    python3 -m research.phase3_survey.add_human_stimulus
"""

import argparse
import json
import random
import sys

VALID_FACES = set('RLUDFB')
VALID_SUFFIXES = {'', "'", '2'}


def validate_moves(move_str):
    """Kiem tra chuoi nuoc di dung ky hieu Singmaster, tra ve (list_nuoc,
    thong_bao_loi_hoac_None)."""
    tokens = move_str.split()
    if not tokens:
        return None, 'Chuoi nuoc di rong.'
    for t in tokens:
        face = t[0]
        suffix = t[1:]
        if face not in VALID_FACES:
            return None, (f"'{t}' khong hop le: mat '{face}' phai la 1 trong "
                           f"R, L, U, D, F, B.")
        if suffix not in VALID_SUFFIXES:
            return None, (f"'{t}' khong hop le: sau ky tu mat chi duoc phep "
                           f"la rong (nuoc thuan), ' (nuoc nguoc), hoac 2 "
                           f"(nuoc doi). Vi du dung: R, R', R2.")
    if len(tokens) < 15:
        return None, (f'Chi co {len(tokens)} nuoc -- qua ngan de la 1 loi '
                       f'giai CFOP day du (Cross+F2L+OLL+PLL thuong >=35 '
                       f'nuoc). Kiem tra lai xem co paste thieu khong.')
    return tokens, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--moves', type=str, default=None,
                     help='Chuoi nuoc di Singmaster, vd "R U R\' U\' ..."')
    ap.add_argument('--stimuli', type=str,
                     default='research/phase3_survey/stimuli.json')
    args = ap.parse_args()

    with open(args.stimuli, encoding='utf-8') as f:
        data = json.load(f)

    n_h_before = sum(1 for s in data['stimuli'] if s['internal_group'] == 'H')
    print(f'Hien tai da co {n_h_before}/6 bai giai Nhom H (nguoi that).')

    move_str = args.moves
    if move_str is None:
        print('\nDan chuoi nuoc di Singmaster (vi du: R U R\' U\' F ...):')
        move_str = input('> ').strip()

    tokens, err = validate_moves(move_str)
    if err:
        print(f'\n[LOI] {err}')
        print('KHONG ghi gi vao file -- sua lai chuoi nuoc di roi chay lai lenh nay.')
        sys.exit(1)

    next_id = len(data['stimuli']) + 1
    new_entry = {
        'internal_group': 'H',
        'seed': 'human',
        'moves': tokens,
        'display_id': f'Bài giải #{next_id}',
        'moves_str': ' '.join(tokens),
    }
    data['stimuli'].append(new_entry)

    # Xao lai TOAN BO thu tu (khong chi phan tu moi) de khong lo thu tu
    # them vao trung voi vi tri nhom -- tranh nguoi cham doan duoc quy luat.
    random.shuffle(data['stimuli'])
    for idx, s in enumerate(data['stimuli'], start=1):
        s['display_id'] = f'Bài giải #{idx}'
        s['moves_str'] = ' '.join(s['moves'])

    with open(args.stimuli, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    n_h_after = n_h_before + 1
    print(f'\nDA THEM THANH CONG ({len(tokens)} nuoc). '
          f'Nhom H: {n_h_after}/6.')
    if n_h_after < 6:
        print(f'Con thieu {6 - n_h_after} bai -- chay lai lenh nay voi bai tiep theo.')
    else:
        print('DA DU 6/6 -- stimuli.json san sang gui cho nguoi tham gia khao sat!')


if __name__ == '__main__':
    main()

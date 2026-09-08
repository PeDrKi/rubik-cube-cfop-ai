"""
research/phase3_survey/add_human_stimulus.py
=================================================
Them 1 bai giai NGUOI THAT (Nhom H) vao stimuli.json MOT CACH AN TOAN --
tu dong gan ID tiep theo, tu dong kiem tra chuoi nuoc di co hop le khong
(dung ky hieu Singmaster) TRUOC khi ghi file, tranh loi cu phap JSON khi
sua tay.

QUAN TRONG (sua loi 2026-08): PHIEN BAN CU cua script nay xao tron +
danh so lai display_id cua TOAN BO danh sach moi lan chay. Dieu nay AN
TOAN neu chua co ai cham diem, nhung PHA HONG du lieu da thu duoc mot
khi da co rater that -- vi analyze_results.py tra cuu noi dung stimulus
theo display_id TU stimuli.json HIEN TAI (khong luu san trong tung file
CSV ket qua), nen danh so lai se lam moi rating cu bi gan nham noi
dung. Ban KHONG can xao display_id de chong doan-quy-luat: rating_tool.html
da tu xao THU TU HIEN THI o phia client (bien `order`, doc lap voi
display_id) moi phien cham, nen viec xao lai o day la THUA va RUI RO.
Tu ban nay tro di: CHI THEM entry moi voi display_id CHUA TUNG DUNG,
KHONG dung lai, KHONG doi display_id cua entry da co.

Cach dung (chay nhieu lan, moi lan 1 bai giai):

    python3 -m research.phase3_survey.add_human_stimulus \
        --moves "R U R' U' R' F R2 U' R' U' R U R' F'"

Hoac chay khong co --moves de nhap tuong tac (go tung buoc):

    python3 -m research.phase3_survey.add_human_stimulus
"""

import argparse
import json
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


def _next_free_display_id(stimuli):
    """Tim so ID nho nhat CHUA TUNG DUNG, quet toan bo lich su -- KHONG
    chi dua vao len(list) vi cac ID cu (0..len-1) da "chot" voi rater
    that, phai giu nguyen tuyet doi."""
    used = set()
    for s in stimuli:
        did = s['display_id']
        # dang "Bài giải #N"
        try:
            n = int(did.rsplit('#', 1)[1])
            used.add(n)
        except (IndexError, ValueError):
            pass
    n = 1
    while n in used:
        n += 1
    return n


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
    print(f'Hien tai da co {n_h_before} bai giai Nhom H (nguoi that).')

    move_str = args.moves
    if move_str is None:
        print('\nDan chuoi nuoc di Singmaster (vi du: R U R\' U\' F ...):')
        move_str = input('> ').strip()

    tokens, err = validate_moves(move_str)
    if err:
        print(f'\n[LOI] {err}')
        print('KHONG ghi gi vao file -- sua lai chuoi nuoc di roi chay lai lenh nay.')
        sys.exit(1)

    next_id = _next_free_display_id(data['stimuli'])
    new_entry = {
        'internal_group': 'H',
        'seed': 'human',
        'moves': tokens,
        'display_id': f'Bài giải #{next_id}',
        'moves_str': ' '.join(tokens),
    }
    data['stimuli'].append(new_entry)
    # KHONG shuffle, KHONG danh so lai cac entry da co -- xem ghi chu
    # dau file. rating_tool.html tu lo phan xao thu tu hien thi.

    with open(args.stimuli, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    n_h_after = n_h_before + 1
    print(f'\nDA THEM THANH CONG ({len(tokens)} nuoc) voi display_id='
          f'"Bài giải #{next_id}". Nhom H: {n_h_after} bai. '
          f'Cac entry cu KHONG bi dong den.')


if __name__ == '__main__':
    main()

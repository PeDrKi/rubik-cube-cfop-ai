"""
research/phase3_survey/expand_stimuli.py
=============================================
Mo rong stimuli.json bang cach THEM stimuli MOI cho Nhom A/A+/B (tu dong
sinh, khong can nguoi) -- dung LAI dung pipeline giai (Cross bang tra ->
F2L trigger-biased search -> OLL/PLL bang tra) va dung baseline Kociemba
y het prepare_stimuli.py, nhung AN TOAN cho du lieu da co: KHONG bao gio
xao tron hay danh so lai cac entry da ton tai trong stimuli.json (xem
ghi chu trong add_human_stimulus.py ve vi sao dieu nay quan trong mot
khi da co rater that cham diem).

Nhom H (nguoi that) KHONG the sinh o day -- dung
add_human_stimulus.py cho tung bai giai nguoi that moi.

Cach dung (them 6 seed moi = 18 stimuli moi cho A/A+/B):

    python3 -m research.phase3_survey.expand_stimuli --n-new-seeds 6

Mac dinh tu dong chon seed chua tung dung (quet seed trong stimuli.json
hien co, bat dau tim tu --base-seed tro di).
"""

import argparse
import json
import random
import signal
import sys

sys.path.insert(0, '.')

from cube_engine import make_solved, do_move, ALL_MOVES
from research.trigger_biased_search import solve_f2l_trigger_biased
from solver.cross_solver import solve_cross
from solver.oll_solver import solve_oll
from solver.pll_solver import solve_pll

try:
    from research.baseline_kociemba import solve_baseline
    _HAS_KOCIEMBA = True
except ImportError:
    _HAS_KOCIEMBA = False
    solve_baseline = None

TIMEOUT_S = 25


class _TimeoutError(Exception):
    pass


def _alarm(signum, frame):
    raise _TimeoutError()


def _with_timeout(fn, *args, **kwargs):
    old = signal.signal(signal.SIGALRM, _alarm)
    signal.alarm(TIMEOUT_S)
    try:
        return fn(*args, **kwargs)
    except _TimeoutError:
        return None
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old)


def random_scramble(st, n, rng):
    prev = None
    moves = []
    for _ in range(n):
        cand = [m for m in ALL_MOVES if m[0] != prev] if prev else ALL_MOVES
        mv = rng.choice(cand)
        do_move(st, mv)
        moves.append(mv)
        prev = mv[0]
    return moves


def solve_group_a(scramble_state, trigger_bias_lambda=0.0):
    """Y het prepare_stimuli.py: Cross -> F2L (co the trigger-bias) ->
    OLL -> PLL, tra ve chuoi nuoc di phang."""
    st = {f: scramble_state[f].copy() for f in scramble_state}
    cross = solve_cross(st)
    for mv in cross:
        do_move(st, mv)
    f2l_res = solve_f2l_trigger_biased(st, lam=trigger_bias_lambda, nodes_per_depth=40_000)
    if f2l_res['moves'] is None or len(f2l_res['solved_slots']) < 4:
        return None
    for mv in f2l_res['moves']:
        do_move(st, mv)
    oll_res = solve_oll(st)
    oll_moves = oll_res['moves'] or []
    for mv in oll_moves:
        do_move(st, mv)
    pll_res = solve_pll(st)
    pll_moves = pll_res['moves'] or []
    for mv in pll_moves:
        do_move(st, mv)
    return cross + f2l_res['moves'] + oll_moves + pll_moves


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
    ap.add_argument('--n-new-seeds', type=int, default=6,
                     help='so seed MOI can (moi seed sinh 3 stimuli: A, A+, B)')
    ap.add_argument('--base-seed', type=int, default=9100,
                     help='bat dau quet tu day de tim seed chua dung '
                          '(mac dinh 9100, cach xa 9000-9007 da dung)')
    ap.add_argument('--stimuli', type=str,
                     default='research/phase3_survey/stimuli.json')
    args = ap.parse_args()

    with open(args.stimuli, encoding='utf-8') as f:
        data = json.load(f)

    used_seeds = {s['seed'] for s in data['stimuli'] if s['seed'] != 'human'}
    n_before = len(data['stimuli'])
    print(f'Da co {n_before} stimuli ({len(used_seeds)} seed A/A+/B da dung: '
          f'{sorted(used_seeds)}).')
    if not _HAS_KOCIEMBA:
        print('\n[CANH BAO] Khong import duoc thu vien `kociemba` (pip install '
              'kociemba) trong moi truong nay -- se CHI sinh Nhom A/A+ cho '
              'seed moi, BO QUA Nhom B. Xem huong dan o cuoi de dien Nhom B '
              'sau, o may co san thu vien nay.\n')

    seed = args.base_seed
    collected = 0
    attempts = 0
    max_attempts = args.n_new_seeds * 4
    new_entries = []
    seeds_missing_b = []

    while collected < args.n_new_seeds and attempts < max_attempts:
        attempts += 1
        if seed in used_seeds:
            seed += 1
            continue
        rng = random.Random(seed)
        st = make_solved()
        random_scramble(st, 20, rng)
        snapshot = {f: st[f].copy() for f in st}

        moves_a = _with_timeout(solve_group_a, snapshot, trigger_bias_lambda=0.0)
        moves_ap = _with_timeout(solve_group_a, snapshot, trigger_bias_lambda=1.0)
        if _HAS_KOCIEMBA:
            st_b = {f: snapshot[f].copy() for f in snapshot}
            moves_b = _with_timeout(solve_baseline, st_b)
        else:
            moves_b = None

        required_ok = moves_a and moves_ap and (moves_b or not _HAS_KOCIEMBA)
        if required_ok:
            new_entries.append({'internal_group': 'A', 'seed': seed, 'moves': moves_a})
            new_entries.append({'internal_group': 'A+', 'seed': seed, 'moves': moves_ap})
            if moves_b:
                new_entries.append({'internal_group': 'B', 'seed': seed, 'moves': moves_b})
            else:
                seeds_missing_b.append(seed)
            collected += 1
            b_str = f'B={len(moves_b)}mv' if moves_b else 'B=(chua sinh, thieu kociemba)'
            print(f'[{collected}/{args.n_new_seeds}] seed={seed} OK: '
                  f'A={len(moves_a)}mv A+={len(moves_ap)}mv {b_str}', flush=True)
        else:
            print(f'[bo qua] seed={seed}: mot nhanh timeout/that bai '
                  f'(A={"OK" if moves_a else "FAIL"}, A+={"OK" if moves_ap else "FAIL"}) '
                  f'-- thu seed ke tiep', flush=True)
        used_seeds.add(seed)
        seed += 1

    if collected < args.n_new_seeds:
        print(f'\n[canh bao] chi thu duoc {collected}/{args.n_new_seeds} sau {attempts} lan thu.')

    # QUAN TRONG: xao thu tu CHI trong noi bo cac entry MOI (khong dong
    # den entry cu), roi gan display_id MOI hoan toan cho tung entry moi
    # -- khong bao gio doi display_id cua entry da co san.
    random.Random(args.base_seed).shuffle(new_entries)
    next_id = _next_free_display_id(data['stimuli'])
    for entry in new_entries:
        entry['display_id'] = f'Bài giải #{next_id}'
        entry['moves_str'] = ' '.join(entry['moves'])
        next_id += 1

    data['stimuli'].extend(new_entries)

    with open(args.stimuli, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f'\nDa them {len(new_entries)} stimuli moi -> tong {len(data["stimuli"])} '
          f'stimuli trong {args.stimuli}.')
    print(f'({n_before} entry cu giu nguyen display_id, KHONG bi dong den.)')
    print('\n=== VIEC CAN LAM TIEP ===')
    step = 1
    if seeds_missing_b:
        print(f'{step}. Nhom B con THIEU cho {len(seeds_missing_b)} seed: '
              f'{seeds_missing_b} (moi truong nay khong co thu vien kociemba).')
        print('   O may co san `pip install kociemba`, chay:')
        print(f'   python3 -m research.phase3_survey.fill_missing_group_b '
              f'--seeds {" ".join(str(s) for s in seeds_missing_b)}')
        step += 1
    n_h = sum(1 for s in data['stimuli'] if s['internal_group'] == 'H')
    print(f'{step}. Nhom H hien co {n_h} bai -- them bai nguoi that moi bang:')
    print('   python3 -m research.phase3_survey.add_human_stimulus --moves "..."')
    step += 1
    print(f'{step}. Gui stimuli.json + rating_tool.html cho nguoi tham gia khao sat MOI')
    print('   (nguoi cu KHONG can cham lai -- chi nguoi moi can file stimuli.json')
    print('   moi nay; ket qua cu va moi gop duoc voi nhau vi display_id on dinh).')


if __name__ == '__main__':
    main()
